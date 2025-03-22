pub mod fsm;
pub mod request;
pub mod timer;
mod lights;
mod self_elevator;

use std::time::Duration;
use std::u8::MAX;

use tokio::task::yield_now;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::sleep;
use crate::config;
use crate::elevio;
use crate::elevio::elev::Elevator;
use crate::elevio::ElevMessage;
use crate::print;
use crate::world_view;
use crate::world_view::Dirn;
use crate::world_view::ElevatorBehaviour;
use crate::world_view::ElevatorContainer;


pub async fn run_local_elevator(wv_watch_rx: watch::Receiver<Vec<u8>>, elevator_states_tx: mpsc::Sender<Vec<u8>>) {
    let (local_elev_tx, local_elev_rx) = mpsc::channel::<ElevMessage>(100);
    
    let elevator = self_elevator::init(local_elev_tx).await;

    
    // Task som utfører deligerte tasks (ikke implementert korrekt enda)
    {
        let elevator_c = elevator.clone();
        let wv_watch_rx_c = wv_watch_rx.clone();
        let _handle_task = tokio::spawn(async move {
            let _ = handle_elevator(wv_watch_rx_c, elevator_states_tx, local_elev_rx, elevator_c).await;
        });
    }  

    {
        let e = elevator.clone();
        let wv_watch_rx_c = wv_watch_rx.clone();
        // Task som setter på hall_lights
        tokio::spawn(async move {
            let mut wv = world_view::get_wv(wv_watch_rx);
            loop {
                world_view::update_wv(wv_watch_rx_c.clone(), &mut wv).await;
                match world_view::extract_self_elevator_container(wv.clone()) {
                    Some(self_elevator) => {
                        lights::set_hall_lights(wv.clone(), e.clone(), &self_elevator);
                    }
                    None => {
                        print::warn(format!("Failed to extract self elevator container"));
                    }
                }
                sleep(config::POLL_PERIOD).await;
            }
        });
    }  

    loop {
        yield_now().await;
    }
    
}




pub async fn handle_elevator(wv_watch_rx: watch::Receiver<Vec<u8>>, elevator_states_tx: mpsc::Sender<Vec<u8>>, mut local_elev_rx: mpsc::Receiver<elevio::ElevMessage>, e: Elevator) {
    
    let mut wv = world_view::get_wv(wv_watch_rx.clone());
    let mut self_container = await_valid_self_container(wv_watch_rx.clone()).await;

    
    let mut door_timer = timer::new(Duration::from_secs(3));
    let mut cab_call_timer = timer::new(Duration::from_secs(10));
    let mut error_timer = timer::new(Duration::from_secs(7));
    let mut prev_cab_call_timer_stat:bool = false;

    //init the state. this is blocking until we reach the closest foor in down direction
    fsm::onInit(&mut self_container, e.clone(), &mut local_elev_rx, &mut cab_call_timer, &mut error_timer, &mut door_timer).await;

    // self_container.dirn = Dirn::Stop;
    let mut prev_behavior:ElevatorBehaviour = self_container.behaviour;
    let mut prev_floor: u8 = self_container.last_floor_sensor;
    
    loop {
        /*OBS OBS!! krasjer når vi starter i 0 etasje..... uff da */
        //Les nye data fra heisen, putt de inn i self_container
        
        self_elevator::update_elev_container_from_msgs(&mut local_elev_rx, &mut self_container, &mut cab_call_timer , &mut error_timer ).await;
        
        /*======================================================================*/
        /*                           START: FSM Events                          */
        /*======================================================================*/
        handle_floor_sensor_update(
            &mut self_container,
            e.clone(),
            &mut prev_floor,
            &mut door_timer,
            &mut cab_call_timer,
            &mut error_timer,
        ).await;        

        
        handle_door_timeout_and_lights(
            &mut self_container,
            e.clone(),
            &door_timer,
            &mut cab_call_timer,
        ).await;
        
        handle_error_timeout(
            &self_container,
            &cab_call_timer,
            &mut error_timer,
            prev_cab_call_timer_stat,
        );
        
        // fsm::onIdle ?
        handle_idle_state(&mut self_container, e.clone(), &mut door_timer);
        /*======================================================================*/
        /*                           END: FSM Events                            */
        /*======================================================================*/

        /*============================================================================================================================================*/
        
        if self_container.behaviour != ElevatorBehaviour::DoorOpen {
            e.motor_direction(self_container.dirn as u8);  
        }
        if error_timer.timer_timeouted() {
            prev_cab_call_timer_stat = true;
            self_container.behaviour = ElevatorBehaviour::Error;
        } else {
            prev_cab_call_timer_stat = false;
        }
        
        // Lagre tidlegare status før oppdatering
        let last_behavior = prev_behavior;

        // Oppdater prev_behavior dersom statusen endrar seg
        if prev_behavior != self_container.behaviour {
            prev_behavior = self_container.behaviour;
            println!("Endra status: {:?} -> {:?}", last_behavior, prev_behavior);
        }

        // Sett motor til stopp når vi går frå DoorOpen til Error
        if last_behavior == ElevatorBehaviour::DoorOpen && prev_behavior == ElevatorBehaviour::Error {
            self_container.dirn = Dirn::Stop;
        }
        
        
        //Send til update_wv -> nye self_container
        let _ = elevator_states_tx.send(world_view::serial::serialize_elev_container(&self_container)).await;    
        
        //Hent nyeste worldview
        if world_view::update_wv(wv_watch_rx.clone(), &mut wv).await{
            update_tasks_and_hall_requests(&mut self_container, wv.clone()).await;
        }
        yield_now().await;
        sleep(config::POLL_PERIOD).await;

        
        
    }
}

async fn update_tasks_and_hall_requests(self_container: &mut ElevatorContainer, wv: Vec<u8>){
    if let Some(task_container) = world_view::extract_self_elevator_container(wv) {
        self_container.tasks = task_container.tasks;
        self_container.unsent_hall_request = task_container.unsent_hall_request;
    } else {
        print::warn(format!("Failed to extract self elevator container – keeping previous value"));
    }
}

async fn await_valid_self_container(wv_rx: watch::Receiver<Vec<u8>>) -> ElevatorContainer {
    loop {
        let wv = world_view::get_wv(wv_rx.clone());
        if let Some(container) = world_view::extract_self_elevator_container(wv) {
            return container;
        } else {
            print::warn(format!("Failed to extract self elevator container, retrying..."));
            sleep(Duration::from_millis(100)).await;
        }
    }
}


// Hjelpefunksjona til loopen
pub async fn handle_floor_sensor_update(
    self_container: &mut ElevatorContainer,
    e: Elevator,
    prev_floor: &mut u8,
    door_timer: &mut timer::Timer,
    cab_call_timer: &mut timer::Timer,
    error_timer: &mut timer::Timer,
) {
    if *prev_floor != self_container.last_floor_sensor {
        fsm::onFloorArrival(self_container, e, door_timer, cab_call_timer).await;
        error_timer.timer_start();

        // Skal ignorere cab_call_timer viss oppdraget kom frå ein inside-knapp
        if !request::was_outside(self_container) {
            cab_call_timer.release_timer();
        }
        *prev_floor = self_container.last_floor_sensor;
    }
}


async fn handle_door_timeout_and_lights(
    self_container: &mut ElevatorContainer,
    e: Elevator,
    door_timer: &timer::Timer,
    cab_call_timer: &mut timer::Timer,
) {
    if door_timer.timer_timeouted() && !self_container.obstruction {
        lights::clear_door_open_light(e.clone());

        if request::moving_towards_cab_call(&self_container.clone()) {
            cab_call_timer.release_timer();
        }

        if cab_call_timer.timer_timeouted() {
            fsm::onDoorTimeout(self_container, e.clone(), cab_call_timer).await;
        }
    }
}

fn handle_error_timeout(
    self_container: &ElevatorContainer,
    cab_call_timer: &timer::Timer,
    error_timer: &mut timer::Timer,
    prev_cab_call_timer_stat: bool,
) {
    if !cab_call_timer.timer_timeouted() || self_container.behaviour == ElevatorBehaviour::Idle {
        error_timer.timer_start();
    }

    if error_timer.timer_timeouted() && !prev_cab_call_timer_stat {
        print::cosmic_err("Feil på travel!!!!".to_string());
    }
}


fn handle_idle_state(
    self_container: &mut ElevatorContainer,
    e: Elevator,
    door_timer: &mut timer::Timer,
) {
    if self_container.behaviour == ElevatorBehaviour::Idle {
        let DBPair = request::choose_direction(&self_container.clone());

        if DBPair.behaviour != ElevatorBehaviour::Idle {
            print::err(format!("Skal nå være: {:?}", DBPair.behaviour));
            self_container.dirn = DBPair.dirn;
            self_container.behaviour = DBPair.behaviour;
            door_timer.timer_start();
            e.motor_direction(Dirn::Stop as u8);
        }
    }
}
