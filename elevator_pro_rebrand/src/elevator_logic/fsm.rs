use std::f32::consts::E;
use std::task;



use tokio::time::sleep;

use crate::{elevio::elev::Elevator, world_view, print};
use crate::world_view::{Dirn, ElevatorBehaviour, ElevatorContainer};

use crate::elevator_logic::request;

use super::{lights, timer};




pub async fn onFloorArrival(elevator: &mut ElevatorContainer, e: Elevator, door_timer: &mut timer::Timer, cab_call_timer: &mut timer::Timer) {
    // Ved init between floors: last_floor = 255, sett den til høyeste etasje for å slippe index error
    if elevator.last_floor_sensor > elevator.num_floors {
        elevator.last_floor_sensor = elevator.num_floors-1;
    }

    // Hvis man vil ha bedre modus og lyse der heisen er
    // lights::set_cab_light(e.clone(), elevator.last_floor_sensor);

    match elevator.behaviour {
        ElevatorBehaviour::Moving | ElevatorBehaviour::Error => {
            if request::should_stop(&elevator.clone()) {
                e.motor_direction(Dirn::Stop as u8);
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




