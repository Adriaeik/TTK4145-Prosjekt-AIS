use crate::{elevio::elev, world_view::{Dirn, ElevatorBehaviour, ElevatorContainer}};

#[derive(Debug, Clone, Copy)]
pub struct DirnBehaviourPair {
    pub dirn: Dirn,
    pub behaviour: ElevatorBehaviour,
}

/////Requests
fn above(elevator: &ElevatorContainer) -> bool {
    for floor in (elevator.last_floor_sensor as usize + 1)..elevator.tasks.len() {
        for btn in 0..1 {
            if elevator.tasks[floor][btn] {
                return true;
            }
        }
        if elevator.cab_requests[floor] {
            return true;
        }
    }
    false
}

fn below(elevator: &ElevatorContainer) -> bool {
    for floor in 0..(elevator.last_floor_sensor) as usize {
        for btn in 0..1 {
            if elevator.tasks[floor][btn] {
                return true;
            }
        }
        if elevator.cab_requests[floor] {
            return true;
        }
    }
    false
}

fn here(elevator: &ElevatorContainer) -> bool {
    for btn in 0..1 {
        if elevator.tasks[elevator.last_floor_sensor as usize][btn] {
            return true;
        }
    }
    if elevator.cab_requests[elevator.last_floor_sensor as usize] {
        return true;
    }
    false
}

pub fn choose_direction(elevator: &ElevatorContainer) -> DirnBehaviourPair {
    match elevator.dirn {
        Dirn::Up => {
            if above(elevator) {
                DirnBehaviourPair { dirn: Dirn::Up, behaviour: ElevatorBehaviour::Moving }
            } else if here(elevator) {
                DirnBehaviourPair { dirn: Dirn::Down, behaviour: ElevatorBehaviour::DoorOpen }
            } else if below(elevator) {
                DirnBehaviourPair { dirn: Dirn::Down, behaviour: ElevatorBehaviour::Moving }
            } else {
                DirnBehaviourPair { dirn: Dirn::Stop, behaviour: ElevatorBehaviour::Idle }
            }
        }
        Dirn::Down => {
            if below(elevator) {
                DirnBehaviourPair { dirn: Dirn::Down, behaviour: ElevatorBehaviour::Moving }
            } else if here(elevator) {
                DirnBehaviourPair { dirn: Dirn::Up, behaviour: ElevatorBehaviour::DoorOpen }
            } else if above(elevator) {
                DirnBehaviourPair { dirn: Dirn::Up, behaviour: ElevatorBehaviour::Moving }
            } else {
                DirnBehaviourPair { dirn: Dirn::Stop, behaviour: ElevatorBehaviour::Idle }
            }
        }
        Dirn::Stop => {
            if here(elevator) {
                DirnBehaviourPair { dirn: Dirn::Stop, behaviour: ElevatorBehaviour::DoorOpen }
            } else if above(elevator) {
                DirnBehaviourPair { dirn: Dirn::Up, behaviour: ElevatorBehaviour::Moving }
            } else if below(elevator) {
                DirnBehaviourPair { dirn: Dirn::Down, behaviour: ElevatorBehaviour::Moving }
            } else {
                DirnBehaviourPair { dirn: Dirn::Stop, behaviour: ElevatorBehaviour::Idle }
            }
        }
    }
}

pub fn should_stop(elevator: &ElevatorContainer) -> bool {
    if elevator.cab_requests[elevator.last_floor_sensor as usize] {
        return true;
    }
    match elevator.dirn {
        Dirn::Down => {
            elevator.tasks[elevator.last_floor_sensor as usize][1] || !below(elevator)
        }
        Dirn::Up => {
            elevator.tasks[elevator.last_floor_sensor as usize][0] || !above(elevator)
        }
        Dirn::Stop => true,
    }
}


pub fn clear_at_current_floor(elevator: &mut ElevatorContainer) {
    match elevator.dirn {
        Dirn::Up => {
            elevator.cab_requests[elevator.last_floor_sensor as usize] = false;
            // Master clearer hall_request
        },
        Dirn::Down => {
            elevator.cab_requests[elevator.last_floor_sensor as usize] = false;
            // Master clearer hall_request
        },
        _ => {},        
    }
}
