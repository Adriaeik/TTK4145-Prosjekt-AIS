use std::task;

use crate::world_view::{Dirn, ElevatorBehaviour, ElevatorContainer};

#[derive(Debug, Clone, Copy)]
struct DirnBehaviourPair {
    dirn: Dirn,
    behaviour: ElevatorBehaviour,
}



/////Requests
fn requests_above(elevator: &ElevatorContainer) -> bool {
    for floor in (elevator.last_floor_sensor as usize + 1)..elevator.tasks.len() {
        for btn in 0..2 {
            if elevator.tasks[floor][btn] {
                return true;
            }
        }
    }
    false
}

fn requests_below(elevator: &ElevatorContainer) -> bool {
    for floor in 0..elevator.last_floor_sensor as usize {
        for btn in 0..2 {
            if elevator.tasks[floor][btn] {
                return true;
            }
        }
    }
    false
}

fn requests_here(elevator: &ElevatorContainer) -> bool {
    for btn in 0..2 {
        if elevator.tasks[elevator.last_floor_sensor as usize][btn] {
            return true;
        }
    }
    false
}

fn requests_choose_direction(elevator: &ElevatorContainer) -> DirnBehaviourPair {
    match elevator.dirn {
        Dirn::Up => {
            if requests_above(elevator) {
                DirnBehaviourPair { dirn: Dirn::Up, behaviour: ElevatorBehaviour::Moving }
            } else if requests_here(elevator) {
                DirnBehaviourPair { dirn: Dirn::Down, behaviour: ElevatorBehaviour::DoorOpen }
            } else if requests_below(elevator) {
                DirnBehaviourPair { dirn: Dirn::Down, behaviour: ElevatorBehaviour::Moving }
            } else {
                DirnBehaviourPair { dirn: Dirn::Stop, behaviour: ElevatorBehaviour::Idle }
            }
        }
        Dirn::Down => {
            if requests_below(elevator) {
                DirnBehaviourPair { dirn: Dirn::Down, behaviour: ElevatorBehaviour::Moving }
            } else if requests_here(elevator) {
                DirnBehaviourPair { dirn: Dirn::Up, behaviour: ElevatorBehaviour::DoorOpen }
            } else if requests_above(elevator) {
                DirnBehaviourPair { dirn: Dirn::Up, behaviour: ElevatorBehaviour::Moving }
            } else {
                DirnBehaviourPair { dirn: Dirn::Stop, behaviour: ElevatorBehaviour::Idle }
            }
        }
        Dirn::Stop => {
            if requests_here(elevator) {
                DirnBehaviourPair { dirn: Dirn::Stop, behaviour: ElevatorBehaviour::DoorOpen }
            } else if requests_above(elevator) {
                DirnBehaviourPair { dirn: Dirn::Up, behaviour: ElevatorBehaviour::Moving }
            } else if requests_below(elevator) {
                DirnBehaviourPair { dirn: Dirn::Down, behaviour: ElevatorBehaviour::Moving }
            } else {
                DirnBehaviourPair { dirn: Dirn::Stop, behaviour: ElevatorBehaviour::Idle }
            }
        }
    }
}

fn requests_should_stop(elevator: &ElevatorContainer) -> bool {
    match elevator.dirn {
        Dirn::Down => {
            elevator.tasks[elevator.last_floor_sensor as usize][1] || !requests_below(elevator)
        }
        Dirn::Up => {
            elevator.tasks[elevator.last_floor_sensor as usize][0] || !requests_above(elevator)
        }
        Dirn::Stop => true,
    }
}

fn requests_clear_at_current_floor(mut elevator: ElevatorContainer) -> ElevatorContainer {
    elevator.tasks[elevator.last_floor_sensor as usize] = [false, false];
    elevator
}
