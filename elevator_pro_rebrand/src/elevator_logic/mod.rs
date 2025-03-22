pub mod fsm;
pub mod request;
pub mod timer;
mod lights;
mod self_elevator;

use std::time::Duration;

use tokio::task::yield_now;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::sleep;
use crate::config;
use crate::config::PRINT_INFO_ON;
use crate::elevio;
use crate::elevio::elev::Elevator;
use crate::elevio::ElevMessage;
use crate::world_view::ElevatorContainer;
use crate::print;
use crate::world_view;
use crate::world_view::Dirn;
use crate::world_view::ElevatorBehaviour;


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
                let self_elevator = world_view::extract_self_elevator_container(wv.clone());
                lights::set_hall_lights(wv.clone(), e.clone(), &self_elevator);

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
    let mut self_container = world_view::extract_self_elevator_container(wv.clone());

    e.motor_direction(Dirn::Down as u8);
    self_container.behaviour = ElevatorBehaviour::Moving;
    self_container.dirn = Dirn::Down;

    let mut door_timer = timer::new(Duration::from_secs(3));
    let mut cab_call_timer = timer::new(Duration::from_secs(10));
    let mut error_timer = timer::new(Duration::from_secs(5));
    let mut prev_cab_call_timer_stat:bool = false;
    // let mut prev_behavior:ElevatorBehaviour = self_container.behaviour;
    let mut prev_behavior:ElevatorBehaviour = ElevatorBehaviour::Moving;
    let mut prev_floor: u8 = 0;
    let mut prev_prev_floor = 0;

    loop {
        /*OBS OBS!! krasjer når vi starter i 0 etasje..... uff da */
        //Les nye data fra heisen, putt de inn i self_container
        prev_floor = self_container.last_floor_sensor;
        
        self_elevator::update_elev_container_from_msgs(&mut local_elev_rx, &mut self_container, &mut cab_call_timer , &mut error_timer ).await;

        /*______ START: FSM Events ______ */
        // Hvis du er på ny etasje, 
        // if prev_floor != self_container.last_floor_sensor {
        //     fsm::onFloorArrival(&mut self_container, e.clone(), &mut door_timer, &mut cab_call_timer).await;
        //     println!("linje 94:: last_floor_sensor:: {}",self_container.last_floor_sensor);
        //     error_timer.timer_start();
        //     //skal ignorere cab_call_timer visst oppdraget kom fra ein insidebtn
        //     if !request::was_outside(&self_container){
        //         cab_call_timer.release_timer();
        //     }
        // }
        handle_floor_arrival_if_needed(
            prev_floor,
            &mut self_container,
            e.clone(),
            &mut door_timer,
            &mut cab_call_timer,
            &mut error_timer,
        ).await;

        if door_timer.timer_timeouted()  && !self_container.obstruction {
            lights::clear_door_open_light(e.clone());
            // if inside_call og vi moving_towards -> tving cab_call_timer til timout
            if request::moving_towards_cab_call(&self_container.clone()) {
                cab_call_timer.release_timer();
            }
            if prev_floor != self_container.last_floor_sensor {println!("linje 111:: last_floor_sensor:: {}",self_container.last_floor_sensor);}


            if  cab_call_timer.timer_timeouted() {

                fsm::onDoorTimeout(&mut self_container, e.clone(), &mut cab_call_timer).await;
                if prev_floor != self_container.last_floor_sensor {println!("linje 117:: last_floor_sensor:: {}",self_container.last_floor_sensor)};

            }
        }
        if !cab_call_timer.timer_timeouted()|| self_container.behaviour == ElevatorBehaviour::Idle {
            error_timer.timer_start();
        }
        if error_timer.timer_timeouted() && !prev_cab_call_timer_stat {
            print::cosmic_err("Feil på travel!!!!".to_string());
            // error_timer.timer_start();

        }
        
        
        // fsm::onIdle ?
        if self_container.behaviour == ElevatorBehaviour::Idle {
            let DBPair = request::choose_direction(&self_container.clone());
            if prev_floor != self_container.last_floor_sensor {println!("linje 134:: last_floor_sensor:: {}",self_container.last_floor_sensor)};
            
            if DBPair.behaviour != ElevatorBehaviour::Idle {
                print::err(format!("Skal nå være: {:?}", DBPair.behaviour));
                self_container.dirn = DBPair.dirn;
                self_container.behaviour = DBPair.behaviour;
                door_timer.timer_start();
                e.motor_direction(Dirn::Stop as u8);
            }
        }
        /*______ SLUTT: FSM Events ______ */
        
        
        
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
            print::info(format!("Endra status: {:?} -> {:?}", last_behavior, prev_behavior));
        }

        // Sett motor til stopp når vi går frå DoorOpen til Error
        if last_behavior == ElevatorBehaviour::DoorOpen && prev_behavior == ElevatorBehaviour::Error {
            self_container.dirn = Dirn::Stop;
        }
        // println!("Motor dir: {:?}, Elev behaviour: {:?}", self_container.dirn, self_container.behaviour);
        
        //Send til update_wv -> nye self_container
        let _ = elevator_states_tx.send(world_view::serial::serialize_elev_container(&self_container)).await;    
        if prev_floor != self_container.last_floor_sensor {println!("linje 175:: last_floor_sensor:: {}",self_container.last_floor_sensor);}
        
        //Hent nyeste worldview
        if world_view::update_wv(wv_watch_rx.clone(), &mut wv).await{

            self_container = world_view::extract_self_elevator_container(wv.clone()); //her det blir overskrive
            if prev_floor != self_container.last_floor_sensor {
                println!("linje 181:: last_floor_sensor:: {}",self_container.last_floor_sensor); 
                self_container.last_floor_sensor = prev_floor}

        }
        sleep(config::POLL_PERIOD).await;

        
    }
}

/// Hjelpefunksjon som bestemmer om me har ankomme ei ny, gyldig etasje
fn is_new_valid_floor(prev: u8, current: u8, num_floors: u8) -> bool {
    current < num_floors && current != prev
}

/// Kall `onFloorArrival` og relaterte FSM-event, dersom me er i ny gyldig etasje
async fn handle_floor_arrival_if_needed(
    prev_floor: u8,
    self_container: &mut ElevatorContainer,
    e: Elevator,
    door_timer: &mut timer::Timer,
    cab_call_timer: &mut timer::Timer,
    error_timer: &mut timer::Timer,
) {
    let current = self_container.last_floor_sensor;

    if is_new_valid_floor(prev_floor, current, self_container.num_floors) {
        println!("[onFloorArrival] last: {}, prev: {}", current, prev_floor);

        fsm::onFloorArrival(self_container, e.clone(), door_timer, cab_call_timer).await;
        error_timer.timer_start();

        if !request::was_outside(self_container) {
            cab_call_timer.release_timer();
        }
    }
}
