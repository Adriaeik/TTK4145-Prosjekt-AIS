use core::panic;
use crossbeam_channel as cbc;
use log::{debug, error, info};
use std::thread;
use std::time;

use driver_rust::elevio;

use crate::config::Config;
use crate::message::{self, Message};
use crate::single_elevator::elevator;
use crate::single_elevator::fsm;
use crate::single_elevator::timer;
use crate::types;
use crate::types::Orders;
// use crate::single_elevator::requests;

pub fn run_controller(
    config: Config,
    elevio_driver: elevio::elev::Elevator,
    network_node_name: String,
    network_tx: cbc::Sender<message::DataMessage>,
    command_rx: cbc::Receiver<Orders>,
) {
    let polling_interval = time::Duration::from_millis(config.polling_interval_ms);

    // Call buttons
    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>();
    {
        let elevio_driver = elevio_driver.clone();
        thread::spawn(move || {
            elevio::poll::call_buttons(elevio_driver, call_button_tx, polling_interval)
        });
    }

    // Floor sensor
    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
    {
        let elevio_driver = elevio_driver.clone();
        thread::spawn(move || {
            elevio::poll::floor_sensor(elevio_driver, floor_sensor_tx, polling_interval)
        });
    }

    // Obstruction
    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();
    {
        let elevio_driver = elevio_driver.clone();
        thread::spawn(move || {
            elevio::poll::obstruction(elevio_driver, obstruction_tx, polling_interval)
        });
    }

    // Timer
    let (timer_elev_tx, timer_elev_rx) = cbc::unbounded::<timer::TimerMessage>();
    let (timer_time_tx, timer_time_rx) = cbc::unbounded::<timer::TimerMessage>();
    {
        let mut timer_instance = timer::Timer::new();
        thread::spawn(move || loop {
            let mut sel = cbc::Select::new();
            sel.recv(&timer_time_rx); // This IS NECESSARY, but why?

            let oper = sel.try_select();
            match oper {
                Err(_) => {
                    // Since try_select is non-blocking, this is the default case when no messages are available
                    if timer_instance.timed_out() {
                        timer_instance.stop();
                        timer_elev_tx.send(timer::TimerMessage::TimedOut).unwrap();
                    }
                }
                Ok(oper) => {
                    let timer_message = oper.recv(&timer_time_rx).unwrap(); // Sometimes get error "unwrap on Err value: RecvError", needs fixing
                    match timer_message {
                        timer::TimerMessage::Start(duration) => {
                            timer_instance.start(duration);
                        }
                        timer::TimerMessage::Stop => {
                            timer_instance.stop();
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    // Initialize elevator
    let mut elevator_state = elevator::State::new(config, timer_time_tx);
    let initial_obstruction = elevio_driver.obstruction();
    elevator_state.obstruction = initial_obstruction;
    // Check for initial floor
    let initial_floor = elevio_driver.floor_sensor();
    if initial_floor == None {
        fsm::on_init_between_floors(&elevio_driver, &mut elevator_state);
    }

    loop {
        cbc::select! {
            // Command from network
            recv(command_rx) -> received => {
                let requests = match received {
                    Ok(requests) => requests,
                    Err(e) => {
                        error!("Error receiving new requests: {e}");
                        continue;
                    }
                };

                // Update local array of requests
                debug!("Received new requests: {:#?}", requests);
                elevator_state.set_all_requests(requests);
                fsm::set_all_lights(&elevio_driver, &elevator_state);

                // Do we need to do something? Only if we're IDLE
                if elevator_state.behaviour == elevator::Behaviour::Idle {
                    info!("In IDLE, starting new request");
                    // let floor = elevator_state.get_floor().unwrap();
                    // fsm::on_arrival(&elevio_driver, &mut elevator_state, floor);
                    fsm::on_new_order_assignment(&elevio_driver, &mut elevator_state);
                } else {
                    info!("Not IDLE, ignoring new request");
                }
            },

            // Call button
            recv(call_button_rx) -> received => {
                let call_button = match received {
                    Ok(call_button) => call_button,
                    Err(e) => {
                        error!("Error receiving peer update: {e}");
                        continue;
                    }
                };
                let button = match call_button.call {
                    0 => elevator::Button::HallUp,
                    1 => elevator::Button::HallDown,
                    2 => elevator::Button::Cab,
                    _ => panic!("Invalid call button"),
                };
                let direction = match button {
                    elevator::Button::HallUp => types::Direction::Up,
                    elevator::Button::HallDown => types::Direction::Down,
                    _ => types::Direction::Up,
                };
                // debug!("{:#?}", call_button);
                // elevio_driver.call_button_light(call_button.floor, call_button.call, true);
                // fsm::on_request_button_press(&elevio_driver, &mut elevator_state, call_button.floor, button);
                match button {
                    elevator::Button::HallUp | elevator::Button::HallDown => {
                        // Notify of hall order
                        let event: message::HallOrderMessage = message::HallOrderMessage {
                            floor: call_button.floor,
                            direction: direction,
                        };
                        network_tx.send(event.to_data_message(&network_node_name)).unwrap();
                    },
                    elevator::Button::Cab => {
                        // Notify of cab order
                        let event: message::CabOrderMessage = message::CabOrderMessage {
                            floor: call_button.floor,
                        };
                        network_tx.send(event.to_data_message(&network_node_name)).unwrap();
                    },
                };
            },

            // Floor sensor
            recv(floor_sensor_rx) -> received => {
                let floor = match received {
                    Ok(floor) => floor,
                    Err(e) => {
                        error!("Error receiving floor: {e}");
                        continue;
                    }
                };
                // if elevator_state.get_floor() != Some(floor) {
                //     // TODO: What happens if elevator goes up, then down to same floor without reaching any other floor? Need to handle so arrival code executes again
                // }
                debug!("Arrival at floor");
                fsm::on_arrival(&elevio_driver, &mut elevator_state, floor);

                // Notify of an event (updated state)
                let event: message::ElevatorEventMessage = message::ElevatorEventMessage {
                    behaviour: elevator_state.behaviour,
                    floor: elevator_state.get_floor().unwrap(), // Can unwrap be an issue here?
                    direction: elevator_state.direction,
                };
                network_tx.send(event.to_data_message(&network_node_name)).unwrap();
            },

            // Timer
            recv(timer_elev_rx) -> received => {
                let timer_message = match received {
                    Ok(timer_message) => timer_message,
                    Err(e) => {
                        error!("Error receiving timer message: {e}");
                        continue;
                    }
                };
                debug!("Timer message");
                match timer_message {
                    timer::TimerMessage::TimedOut => {
                        if elevator_state.obstruction {
                            debug!("Obstruction detected, restarting door timer");
                            elevator_state.start_door_timer();
                        } else {
                            debug!("Door timeout");
                            fsm::on_door_timeout(&elevio_driver, &mut elevator_state);

                            // Notify of an event (updated state)
                            let event: message::ElevatorEventMessage = message::ElevatorEventMessage {
                                behaviour: elevator_state.behaviour,
                                floor: elevator_state.get_floor().unwrap(), // Can unwrap be an issue here?
                                direction: elevator_state.direction,
                            };
                            network_tx.send(event.to_data_message(&network_node_name)).unwrap();
                        }
                    },
                    _ => {},
                }
            },

            // Obstruction
            recv(obstruction_rx) -> received => {
                let obstr = match received {
                    Ok(obstr) => obstr,
                    Err(e) => {
                        error!("Error receiving obstruction: {e}");
                        continue;
                    }
                };
                elevator_state.obstruction = obstr;
            },

            // Default
            default(time::Duration::from_millis(500)) => {
                // debug!("Controller default");
            },
        }
    }
}
