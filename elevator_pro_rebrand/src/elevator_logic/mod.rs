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

    let mut timer = timer::new(Duration::from_secs(3));

    loop {
        //Hent nyeste worldview
        
        //Les nye data fra heisen, putt de inn i self_container
        let prev_floor = self_container.last_floor_sensor;
        self_elevator::update_elev_container_from_msgs(&mut local_elev_rx, &mut self_container).await;
        
        if prev_floor != self_container.last_floor_sensor {
            fsm::onFloorArrival(&mut self_container, e.clone(), &mut timer).await;
        }
        fsm::onDoorTimeout(&mut self_container, e.clone(), &mut timer).await;

        if self_container.behaviour != ElevatorBehaviour::DoorOpen {
            e.motor_direction(self_container.dirn as u8);  
        }
        
        // println!("Motor dir: {:?}, Elev behaviour: {:?}", self_container.dirn, self_container.behaviour);
        
        //Send til update_wv -> nye self_container
        let _ = elevator_states_tx.send(world_view::serial::serialize_elev_container(&self_container)).await;    
        
        sleep(config::POLL_PERIOD).await;
        if world_view::update_wv(wv_watch_rx.clone(), &mut wv).await{
            self_container = world_view::extract_self_elevator_container(wv.clone());
        }
    }
}