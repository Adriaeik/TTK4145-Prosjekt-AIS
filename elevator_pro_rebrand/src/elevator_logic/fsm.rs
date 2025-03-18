use std::f32::consts::E;
use std::task;



use tokio::time::sleep;

use crate::{elevio::elev::Elevator, world_view};
use crate::world_view::{Dirn, ElevatorBehaviour, ElevatorContainer};

use crate::elevator_logic::request;

use super::{lights, timer};




pub async fn onFloorArrival(elevator: &mut ElevatorContainer, e: Elevator, door_timer: &mut timer::Timer) {
    if elevator.last_floor_sensor > elevator.num_floors {
        elevator.last_floor_sensor = elevator.num_floors-1;
    }

    lights::set_cab_light(e.clone(), elevator.last_floor_sensor);

    match elevator.behaviour {
        ElevatorBehaviour::Moving => {
            if request::should_stop(&elevator.clone()) {
                e.motor_direction(Dirn::Stop as u8);
                request::clear_at_current_floor(elevator);
                lights::set_door_open_light(e);
                door_timer.timer_start();
                elevator.behaviour = ElevatorBehaviour::DoorOpen;
            }
        }
        _ => {},
    }
}

pub async fn onDoorTimeout(elevator: &mut ElevatorContainer, e: Elevator) {
    match elevator.behaviour {
        ElevatorBehaviour::DoorOpen => {
            let DBPair = request::choose_direction(&elevator.clone());

            elevator.dirn = DBPair.dirn;
            elevator.behaviour = DBPair.behaviour;
        

            match elevator.behaviour {
                ElevatorBehaviour::DoorOpen => {
                    // TODO: timeren
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

