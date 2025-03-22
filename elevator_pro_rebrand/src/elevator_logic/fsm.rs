

use tokio::time::sleep;
use tokio::sync::mpsc;
use crate::{elevio::{self, elev::Elevator}, world_view, print};
use crate::world_view::{Dirn, ElevatorBehaviour, ElevatorContainer};
use crate::elevator_logic::self_elevator;
use crate::elevator_logic::request;
use crate::config;
use super::{lights, timer};


pub async fn onInit(self_container: &mut ElevatorContainer, e: Elevator, local_elev_rx: &mut mpsc::Receiver<elevio::ElevMessage>, 
                    cab_call_timer: &mut timer::Timer, error_timer: &mut timer::Timer, door_timer: &mut timer::Timer){

    e.motor_direction(Dirn::Down as u8);
    self_container.behaviour = ElevatorBehaviour::Moving;
    self_container.dirn = Dirn::Down;

    while self_container.last_floor_sensor == u8::MAX {
        self_elevator::update_elev_container_from_msgs( local_elev_rx,  self_container,  cab_call_timer ,  error_timer ).await;
        sleep(config::POLL_PERIOD).await;
    }
    onFloorArrival( self_container, e.clone(), door_timer, cab_call_timer).await;
}

pub async fn onFloorArrival(elevator: &mut ElevatorContainer, e: Elevator, door_timer: &mut timer::Timer, cab_call_timer: &mut timer::Timer) {
    // Ved init between floors: last_floor = 255, sett den til høyeste etasje for å slippe index error
    if elevator.last_floor_sensor > elevator.num_floors {
        elevator.last_floor_sensor = elevator.num_floors-1;
    }

    lights::set_cab_light(e.clone(), elevator.last_floor_sensor);

    match elevator.behaviour {
        ElevatorBehaviour::Moving | ElevatorBehaviour::Error => {
            if request::should_stop(&elevator.clone()) {
                e.motor_direction(Dirn::Stop as u8);
                println!("floor: {}", elevator.last_floor_sensor);
                request::clear_at_current_floor(elevator);
                lights::set_door_open_light(e);
                door_timer.timer_start();
                cab_call_timer.timer_start();
                elevator.behaviour = ElevatorBehaviour::DoorOpen;
            }
        }
        _ => {},
    }
}

pub async fn onDoorTimeout(elevator: &mut ElevatorContainer, e: Elevator, cab_call_timer: &mut timer::Timer) {
    match elevator.behaviour {
        ElevatorBehaviour::DoorOpen => {
            let DBPair = request::choose_direction(&elevator.clone());

            
            elevator.behaviour = DBPair.behaviour;
            elevator.dirn = DBPair.dirn;

            match elevator.behaviour {
                ElevatorBehaviour::DoorOpen => {
                    request::clear_at_current_floor(elevator);
                }
                _ => {
                    lights::clear_door_open_light(e.clone());
                    e.motor_direction(elevator.dirn as u8);
                }
            }
        },
        _ => {},
    }
}




