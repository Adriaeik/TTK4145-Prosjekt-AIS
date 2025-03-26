//! Elevator request evaluation and direction decision logic.
//!
//! This module provides helper functions for determining the next action of an elevator,
//! based on its current direction, pending requests, and behavioural state.
//! 
//! It is used as part of the elevator finite state machine (FSM) and ensures deterministic,
//! stateless logic for evaluating what the elevator should do at any given point in time.
//!
//! # Overview
//! The core functionality includes:
//! - Checking for cab or hall requests above or below the current floor.
//! - Determining whether to stop at the current floor.
//! - Choosing direction and behaviour based on task layout.
//! - Inferring whether the elevator is moving towards a cab request.
//!
//! # Primary Structs
//! - [`DirnBehaviourPair`]: Return value combining direction and behaviour (e.g., Moving Up).
//!
//! # Behaviour
//! The logic is stateless and purely functional, based on snapshot data of the elevator's state.
//! Each function expects a reference to the full elevator state (`ElevatorContainer`),
//! and returns either a boolean, a direction, or a `DirnBehaviourPair`.
//!
//! This ensures consistency, testability, and reusability of logic across different parts of the system.
//!
//! # Example
//! ```rust,no_run
//! use elevator_logic::request::{choose_direction, should_stop};
//! use world_view::{ElevatorContainer, Dirn};
//! 
//! let direction_and_behaviour = choose_direction(&elevator);
//! if should_stop(&elevator) {
//!     // Open doors, reset timers
//! }
//! ```

use crate::world_view::{Dirn, ElevatorBehaviour, ElevatorContainer};

/// Represents a combination of a direction and an elevator behaviour state.
///
/// Typically used as the return type for direction decision functions,
/// such as in the elevator finite state machine.
#[derive(Debug, Clone, Copy)]
pub struct DirnBehaviourPair {
    /// direction of the elevator
    pub dirn: Dirn,

    /// the behavior of the elevator
    pub behaviour: ElevatorBehaviour,
}

/// Checks if there are any hall or cab requests above the elevator's current floor.
///
/// Returns `true` if any requests exist on floors higher than the current one, otherwise `false`.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
fn above(elevator: &ElevatorContainer) -> bool {
    for floor in (elevator.last_floor_sensor as usize + 1)..elevator.tasks.len() {
        for btn in 0..2 {
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

/// Checks if there are any **cab requests** (inside elevator) above the current floor.
///
/// Returns `true` if any cab calls exist above, otherwise `false`.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
fn inside_above(elevator: &ElevatorContainer) -> bool {
    for floor in (elevator.last_floor_sensor as usize + 1)..elevator.tasks.len() {
        if elevator.cab_requests[floor] {
            return true;
        }
    }
    false
}

/// Checks if there are any hall or cab requests below the elevator's current floor.
///
/// Returns `true` if any requests exist on floors lower than the current one, otherwise `false`.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
fn below(elevator: &ElevatorContainer) -> bool {
    for floor in 0..elevator.last_floor_sensor as usize {
        for btn in 0..2 {
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


/// Checks if there are any **cab requests** (inside elevator) below the current floor.
///
/// Returns `true` if any cab calls exist below, otherwise `false`.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
fn insie_below(elevator: &ElevatorContainer) -> bool {
    for floor in 0..elevator.last_floor_sensor as usize {
        if elevator.cab_requests[floor] {
            return true;
        }
    }
    false
}

/// Checks for any pending tasks or cab requests at the elevator's current floor.
///
/// Returns `true` if there is a request at the current floor, otherwise `false`.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
fn here(elevator: &ElevatorContainer) -> bool {
    // if elevator.last_floor_sensor >= elevator.num_floors{
    //     return false; // retuner ved feil 
    // }
    for btn in 0..2 {
        if elevator.tasks[elevator.last_floor_sensor as usize][btn] {
            return true;
        }
    }
    if elevator.cab_requests[elevator.last_floor_sensor as usize] {
        return true;
    }
    false
}

/// Determines the intended direction of travel based on current tasks at the elevator's floor.
///
/// Returns:
/// - `Dirn::Up` if there's an up request
/// - `Dirn::Down` if there's a down request
/// - `Dirn::Stop` if no direction is requested
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
fn get_here_dirn(elevator: &ElevatorContainer) -> Dirn {
    if elevator.tasks[elevator.last_floor_sensor as usize][0] {
        return Dirn::Up;
    } else if elevator.tasks[elevator.last_floor_sensor as usize][1] {
        return Dirn::Down;
    } else {
        return Dirn::Stop;
    }

}

/// Determines whether the elevator is moving towards any cab request (inside call).
///
/// This is used to decide whether to stop even if there are no hall requests.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
///
/// # Returns
/// `true` if there is a cab call in the current direction of travel, otherwise `false`.
pub fn moving_towards_cab_call(elevator: &ElevatorContainer) -> bool {
    if elevator.last_floor_sensor == elevator.num_floors-1 || elevator.last_floor_sensor == 0 {
        return true;
    }
    match elevator.dirn {
        Dirn::Up => {
            return inside_above(&elevator.clone());
        },
        Dirn::Down => {
            return insie_below(&elevator.clone());
        },
        Dirn::Stop => {
            return false;
        }
    }
}

/// Main decision logic to determine the elevator's next direction and behaviour.
///
/// Uses the elevator's current direction, position, and requests above/below to return a
/// `DirnBehaviourPair` representing whether to move, open the door, or stay idle.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
///
/// # Returns
/// A `DirnBehaviourPair` representing the chosen direction and behaviour state.
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
                DirnBehaviourPair { dirn: get_here_dirn(elevator), behaviour: ElevatorBehaviour::DoorOpen }
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

/// Determines whether the elevator should stop at the current floor.
///
/// This decision depends on cab calls and hall calls at the floor,
/// and whether there are pending requests in the current direction.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
///
/// # Returns
/// `true` if the elevator should stop, otherwise `false`.
pub fn should_stop(elevator: &ElevatorContainer) -> bool {
    let floor = elevator.last_floor_sensor as usize;
    
    if elevator.cab_requests[floor] {
        return true;
    }

    match elevator.dirn {
        Dirn::Down => {
            elevator.tasks[floor][1] || !below(elevator)
        }
        Dirn::Up => {
            elevator.tasks[floor][0] || !above(elevator)
        }
        Dirn::Stop => true,
    }
}


/// Evaluates whether the elevator was previously outside its designated service range.
///
/// This function mirrors `should_stop()` logic and may be used for debugging or fallback decisions.
///
/// # Parameters
/// - `elevator`: Reference to the elevator's internal state.
///
/// # Returns
/// `true` if considered outside or should stop, otherwise `false`.
pub fn was_outside(elevator: &ElevatorContainer) -> bool {
    let floor = elevator.last_floor_sensor as usize;
    
    match elevator.dirn {
        Dirn::Down => {
            elevator.tasks[floor][1] || !below(elevator)
        }
        Dirn::Up => {
            elevator.tasks[floor][0] || !above(elevator)
        }
        Dirn::Stop => true,
    }
}

/// Clears any active cab request at the elevator's current floor.
///
/// This function assumes the elevator has stopped at the correct floor
/// and has fulfilled the passenger's request.
///
/// # Parameters
/// - `elevator`: Mutable reference to the elevator's internal state.
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
        Dirn::Stop => {
            elevator.cab_requests[elevator.last_floor_sensor as usize] = false;
        },        
    }
}
