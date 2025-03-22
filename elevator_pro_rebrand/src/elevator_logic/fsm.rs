

use tokio::time::sleep;
use tokio::sync::mpsc;
use crate::{elevio::{self, elev::Elevator}, world_view, print};
use crate::world_view::{Dirn, ElevatorBehaviour, ElevatorContainer};
use crate::elevator_logic::self_elevator;
use crate::elevator_logic::request;
use crate::config;
use super::{lights, timer::{Timer, ElevatorTimers}};


/// Initializes the elevator by moving downward until a valid floor is reached.
/// 
/// This function sets the elevator in motion and waits until the floor sensor detects
/// a valid floor. During this process, it updates the local elevator state with incoming messages,
/// while tracking timeout conditions through the shared timer structure.
///
/// # Parameters
/// - `self_container`: Mutable reference to the elevator's internal state.
/// - `e`: The hardware-facing elevator handle (for motor control).
/// - `local_elev_rx`: Channel receiver for elevator sensor and button messages.
/// - `timers`: Mutable reference to the shared `ElevatorTimers` instance.
pub async fn on_init(
    self_container: &mut ElevatorContainer,
    e: Elevator,
    local_elev_rx: &mut mpsc::Receiver<elevio::ElevMessage>,
    timers: &mut ElevatorTimers,
) {
    e.motor_direction(Dirn::Down as u8);
    self_container.behaviour = ElevatorBehaviour::Moving;
    self_container.dirn = Dirn::Down;

    while self_container.last_floor_sensor == u8::MAX {
        self_elevator::update_elev_container_from_msgs(
            local_elev_rx,
            self_container,
            &mut timers.cab_priority,
            &mut timers.error,
        ).await;

        sleep(config::POLL_PERIOD).await;
    }

    on_floor_arrival(
        self_container,
        e.clone(),
        &mut timers.door,
        &mut timers.cab_priority,
    ).await;
}

/// Handles elevator behavior upon arrival at a new floor.
///
/// If the elevator is currently moving or in an error state, this function checks
/// whether it should stop at the current floor (e.g., due to a hall or cab request).
/// If a stop is needed, it performs the following actions:
/// - Stops the motor
/// - Clears any requests at the current floor
/// - Opens the door and turns on the door light
/// - Starts both the door timer and the cab call priority timer
/// - Sets the elevator's behavior to `DoorOpen`
///
/// This function is typically called from the main FSM loop or after initialization.
///
/// # Parameters
/// - `elevator`: Mutable reference to the elevator's internal state.
/// - `e`: Elevator hardware interface, used to control motor and lights.
/// - `door_timer`: Timer tracking how long the door should stay open.
/// - `cab_priority_timer`: Timer giving priority to inside cab requests after door opens.
pub async fn on_floor_arrival(
    elevator: &mut ElevatorContainer,
    e: Elevator,
    door_timer: &mut Timer,
    cab_priority_timer: &mut Timer,
) {
    // Fix startup case: sensor value is 255 when between floors → set to top floor
    if elevator.last_floor_sensor > elevator.num_floors {
        elevator.last_floor_sensor = elevator.num_floors - 1;
    }

    // Turn on light for active cab request (if any)
    lights::set_cab_light(e.clone(), elevator.last_floor_sensor);

    match elevator.behaviour {
        ElevatorBehaviour::Moving | ElevatorBehaviour::Error => {
            if request::should_stop(&elevator.clone()) {
                e.motor_direction(Dirn::Stop as u8);
                request::clear_at_current_floor(elevator);
                lights::set_door_open_light(e);
                door_timer.timer_start();
                cab_priority_timer.timer_start();
                elevator.behaviour = ElevatorBehaviour::DoorOpen;
            }
        }
        _ => {}
    }
}

/// Handles the event when the door timeout has expired.
///
/// This function is called after the door has been open for a certain time.
/// It decides what the elevator should do next by:
/// - Choosing a new direction and behaviour using the request logic
/// - Updating the elevator's direction and behaviour accordingly
///
/// If the elevator decides to stay in `DoorOpen`, it means there is still
/// a request at the current floor and the door should remain open.
/// Otherwise, the door light is cleared and the elevator starts moving in the chosen direction.
///
/// # Parameters
/// - `elevator`: Mutable reference to the elevator's internal state.
/// - `e`: Elevator hardware interface, used to control lights and motor.
pub async fn on_door_timeout(elevator: &mut ElevatorContainer, e: Elevator) {
    match elevator.behaviour {
        ElevatorBehaviour::DoorOpen => {
            let state_pair = request::choose_direction(&elevator.clone());

            
            elevator.behaviour = state_pair.behaviour;
            elevator.dirn = state_pair.dirn;

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




