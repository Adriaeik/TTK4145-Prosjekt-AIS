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
use crate::elevio;
use crate::elevio::elev::Elevator;
use crate::elevio::ElevMessage;
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
                lights::set_hall_lights(wv.clone(), e.clone());

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
    let mut error_timer = timer::new(Duration::from_secs(10));
    let mut prev_cab_call_timer_stat:bool = false;

    loop {
        //Les nye data fra heisen, putt de inn i self_container
        let prev_floor = self_container.last_floor_sensor;
        self_elevator::update_elev_container_from_msgs(&mut local_elev_rx, &mut self_container, &mut cab_call_timer).await;
        


        /*______ START: FSM Events ______ */
        // Hvis du er på ny etasje, 
        if prev_floor != self_container.last_floor_sensor {
            fsm::onFloorArrival(&mut self_container, e.clone(), &mut door_timer, &mut cab_call_timer).await;
            error_timer.timer_start();
        }

        if door_timer.timer_timeouted()  && !self_container.obstruction{
            lights::clear_door_open_light(e.clone());
            if  cab_call_timer.timer_timeouted() {

                fsm::onDoorTimeout(&mut self_container, e.clone(), &mut cab_call_timer).await;
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
            
            if DBPair.behaviour != ElevatorBehaviour::Idle {
                self_container.dirn = DBPair.dirn;
                self_container.behaviour = DBPair.behaviour;
                e.motor_direction(self_container.dirn as u8);
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
        
        // println!("Motor dir: {:?}, Elev behaviour: {:?}", self_container.dirn, self_container.behaviour);
        
        //Send til update_wv -> nye self_container
        let _ = elevator_states_tx.send(world_view::serial::serialize_elev_container(&self_container)).await;    
        
        //Hent nyeste worldview
        sleep(config::POLL_PERIOD).await;
        if world_view::update_wv(wv_watch_rx.clone(), &mut wv).await{
            self_container = world_view::extract_self_elevator_container(wv.clone());
        }
    }
}