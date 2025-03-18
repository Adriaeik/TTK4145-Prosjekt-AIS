use std::task;

use crate::world_view::{Dirn, ElevatorBehaviour, ElevatorContainer};


#[derive(Debug)]
enum ElevatorEvent {
    FloorArrival(usize),
    DoorTimeout,
    Stop,
}
fn make_event(elevator: &mut ElevatorContainer) -> ElevatorEvent {
    let floor = elevator.last_floor_sensor as usize;

    if elevator.tasks[floor][0] || elevator.tasks[floor][1] {
        return ElevatorEvent::FloorArrival(floor);
    }

    let next_dir = determine_next_direction(elevator);
    if next_dir == Dirn::Stop {
        return ElevatorEvent::Stop;
    }

    elevator.dirn = next_dir;
    ElevatorEvent::FloorArrival(floor + match next_dir {
        Dirn::Up => 1,
        Dirn::Down => -1,
        Dirn::Stop => 0,
    } as usize)
}

fn handle_elevator_event(elevator: &mut ElevatorContainer, tx: &Sender<ElevatorEvent>) {
    let event = make_event(elevator);

    
}


fn determine_next_direction(elevator: &ElevatorContainer) -> Dirn {
    let current_floor = elevator.last_floor_sensor;

    match elevator.dirn {
        Dirn::Stop => {
            // request_here
            // Sjekk etter næraste oppdrag, prioriterer opp
            for floor in current_floor..elevator.tasks.len() as u8 {
                if elevator.tasks[floor as usize][0] || elevator.tasks[floor as usize][1] {
                    return if floor > current_floor { Dirn::Up } else { Dirn::Down };
                }
            }
            Dirn::Stop
        }
        Dirn::Up => {
            // Fortsett oppover dersom det finst fleire oppdrag
            for floor in (current_floor + 1)..elevator.tasks.len() as u8 {
                if elevator.tasks[floor as usize][0] {
                    return Dirn::Up;
                }
            }
            // Dersom ingen fleire oppdrag opp, sjekk nedover
            for floor in (0..=current_floor).rev() {
                if elevator.tasks[floor as usize][1] {
                    return Dirn::Down;
                }
            }
            Dirn::Stop
        }
        Dirn::Down => {
            // Fortsett nedover dersom det finst fleire oppdrag
            for floor in (0..=current_floor).rev() {
                if elevator.tasks[floor as usize][1] {
                    return Dirn::Down;
                }
            }
            // Dersom ingen fleire oppdrag ned, sjekk oppover
            for floor in current_floor..elevator.tasks.len() as u8 {
                if elevator.tasks[floor as usize][0] {
                    return Dirn::Up;
                }
            }
            Dirn::Stop
        }
    }
}
