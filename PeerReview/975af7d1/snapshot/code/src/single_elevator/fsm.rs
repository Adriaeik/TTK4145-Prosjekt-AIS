use driver_rust::elevio::elev as e;

use crate::config::NUM_BUTTONS;
use crate::single_elevator::elevator;
use crate::single_elevator::requests;
use crate::types::Direction;

pub fn set_all_lights(elevio_driver: &e::Elevator, elevator_state: &elevator::State) {
    for f in 0..elevator_state.config.number_of_floors {
        for c in 0..NUM_BUTTONS {
            let light = elevator_state.get_request(f.try_into().unwrap(), c.try_into().unwrap());
            elevio_driver.call_button_light(f.try_into().unwrap(), c, light);
        }
    }
}

pub fn on_init_between_floors(elevio_driver: &e::Elevator, elevator_state: &mut elevator::State) {
    elevio_driver.motor_direction(e::DIRN_DOWN);
    elevator_state.direction = Direction::Down;
    elevator_state.behaviour = elevator::Behaviour::Moving;
}

pub fn on_new_order_assignment(
    // This is almost a blind copy of fsm::on_request_button_press
    elevio_driver: &e::Elevator,
    mut elevator_state: &mut elevator::State,
    // request_floor: u8,
    // button: elevator::Button,
) {
    match elevator_state.behaviour {
        elevator::Behaviour::Idle => {
            // elevator_state.set_request(request_floor, button, true);
            let direction_behaviour_pair = requests::choose_direction(&elevator_state);
            elevator_state.direction = direction_behaviour_pair.0;
            elevator_state.behaviour = direction_behaviour_pair.1;
            match elevator_state.behaviour {
                elevator::Behaviour::Idle => {}
                elevator::Behaviour::DoorOpen => {
                    elevio_driver.door_light(true);
                    elevator_state.start_door_timer();
                    elevator_state = requests::clear_at_current_floor(elevator_state);
                    // THIS NEEDS TO BE BROADCAST AS WELL
                }
                elevator::Behaviour::Moving => {
                    elevio_driver.motor_direction(elevator_state.direction as u8);
                }
            }
        }
        elevator::Behaviour::DoorOpen => {
            // if requests::should_clear_immediately(&elevator_state, request_floor, button) {
            //     elevator_state.start_door_timer();
            // } else {
            //     elevator_state.set_request(request_floor, button, true);
            // }
        }
        elevator::Behaviour::Moving => {
            // elevator_state.set_request(request_floor, button, true);
        }
    }

    set_all_lights(elevio_driver, elevator_state);

    // debug!("New state: "\);
    // elevator_state.print();
}

pub fn on_arrival(
    elevio_driver: &e::Elevator,
    mut elevator_state: &mut elevator::State,
    new_floor: u8,
) {
    // debug!("Arrived at floor {}", new_floor);
    // elevator_state.print();

    elevator_state.set_floor(new_floor);
    elevio_driver.floor_indicator(new_floor);

    match elevator_state.behaviour {
        elevator::Behaviour::Moving => {
            if requests::should_stop(&elevator_state) {
                elevio_driver.motor_direction(e::DIRN_STOP);
                elevio_driver.door_light(true);
                elevator_state = requests::clear_at_current_floor(elevator_state);
                elevator_state.start_door_timer();
                set_all_lights(elevio_driver, elevator_state);
                elevator_state.behaviour = elevator::Behaviour::DoorOpen;
            }
        }
        _ => {}
    }

    // debug!("New state: "\);
    // elevator_state.print();
}

pub fn on_door_timeout(elevio_driver: &e::Elevator, mut elevator_state: &mut elevator::State) {
    // debug!("Door timeout");
    // elevator_state.print();

    match elevator_state.behaviour {
        elevator::Behaviour::DoorOpen => {
            let direction_behaviour_pair = requests::choose_direction(&elevator_state);
            elevator_state.direction = direction_behaviour_pair.0;
            elevator_state.behaviour = direction_behaviour_pair.1;

            match elevator_state.behaviour {
                elevator::Behaviour::Moving | elevator::Behaviour::Idle => {
                    elevio_driver.door_light(false);
                    elevio_driver.motor_direction(elevator_state.direction as u8);
                }
                elevator::Behaviour::DoorOpen => {
                    elevator_state.start_door_timer();
                    elevator_state = requests::clear_at_current_floor(elevator_state); // THIS NEEDS TO BE BROADCAST AS WELL
                    set_all_lights(elevio_driver, elevator_state);
                }
            }
        }
        _ => {}
    }

    // debug!("New state: "\);
    // elevator_state.print();
}
