//! Elevator Finite State Machine (FSM) Module
//!
//! This module contains the core logic for the elevator's finite state machine (FSM).
//! It is responsible for reacting to sensor inputs, handling requests, updating the internal
//! elevator state, and controlling motor and lights accordingly.
//!
//! The FSM implements transitions between key elevator states, such as:
//! - Moving
//! - DoorOpen
//! - Idle
//! - Error
//!
//! # Main Responsibilities
//! - Handling initialization from unknown position (`on_init`)
//! - Managing floor arrivals and door timeout logic (`on_floor_arrival`, `on_door_timeout`)
//! - Monitoring inactivity or fault conditions (`handle_error_timeout`)
//! - Executing transitions from Idle state (`handle_idle_state`)
//!
//! # Timers
//! The FSM relies on three coordinated timers:
//! - `door`: Tracks how long the door has been open.
//! - `cab_priority`: Gives passengers time to press cab buttons after door opens.
//! - `error`: Tracks how long the system has been inactive or blocked.
//!
//! These timers are grouped into the [`ElevatorTimers`] struct and passed where needed.
//!
//! # Integration
//! This module is called from the elevator runtime loop and reacts to:
//! - New floor sensor values
//! - Cab and hall call requests
//! - Timer expirations
//!
//!
//! # Related Modules
//! - [`request`]: Direction and behaviour decision logic.
//! - [`self_elevator`]: Local state updates from hardware events.
//! - [`lights`]: Door and button light controls.
//!
//! # Note
//! All function names follow snake_case naming for consistency.


use tokio::time::sleep;
use tokio::sync::mpsc;
use crate::{elevio::{self, elev::Elevator}, print};
use crate::world_view::{Dirn, ElevatorBehaviour, ElevatorContainer};
use crate::elevator_logic::self_elevator;
use crate::elevator_logic::request;
use crate::config;
use super::{lights, request::should_stop, timer::{ElevatorTimers, Timer}};


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
async fn on_floor_arrival(
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
        ElevatorBehaviour::Moving | ElevatorBehaviour::ObstructionError | ElevatorBehaviour::TravelError => {
            if request::should_stop(&elevator.clone()) {
                e.motor_direction(Dirn::Stop as u8);
                request::clear_at_current_floor(elevator);
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
async fn on_door_timeout(elevator: &mut ElevatorContainer, e: Elevator) {
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
                    e.motor_direction(elevator.dirn as u8);
                }
            }
        },
        _ => {},
    }
}

/// Handles floor arrival updates when a new floor sensor reading is detected.
///
/// If the current floor (`last_floor_sensor`) is different from the previously known floor,
/// this function triggers arrival-handling logic, restarts the error timer,
/// and optionally releases the cab call timer if the stop was due to an inside request.
///
/// # Parameters
/// - `self_container`: Mutable reference to the local elevator state.
/// - `e`: Elevator hardware interface for controlling motor and lights.
/// - `prev_floor`: Mutable reference to the previous floor value (used for change detection).
/// - `timers`: Mutable reference to the shared `ElevatorTimers` instance.
///
/// # Behavior
/// - Calls `on_floor_arrival()` if floor changed.
/// - Starts the error timer on valid floor detection.
/// - Releases the cab call timer if the stop was due to an inside (cab) request.
pub async fn handle_floor_sensor_update(
    self_container: &mut ElevatorContainer,
    e: Elevator,
    prev_floor: &mut u8,
    timers: &mut ElevatorTimers,
) {
    if *prev_floor != self_container.last_floor_sensor {
        on_floor_arrival(self_container, e, &mut timers.door, &mut timers.cab_priority).await;
        timers.error.timer_start();

        // Ignore cab call timeout if request came from inside button
        if !request::was_outside(self_container) {
            timers.cab_priority.release_timer();
        }

        *prev_floor = self_container.last_floor_sensor;
    }
}


pub async fn handle_stop_button(
    self_container: &mut ElevatorContainer,
    e: Elevator,
    prev_stop_btn: &mut bool,
) {
    if *prev_stop_btn != self_container.stop {
        if self_container.stop {
            self_container.behaviour = ElevatorBehaviour::CosmicError; 
            e.motor_direction(Dirn::Stop as u8);
        } else {
            self_container.behaviour = ElevatorBehaviour::Idle;
        }
        *prev_stop_btn = self_container.stop;
    }
}

/// Handles door timeout logic when appropriate.
///
/// If the door timer has expired and no obstruction is detected. 
/// If the elevator is moving toward a cab call, the cab call timer
/// is released. If the cab call timer has also expired, the system proceeds to handle
/// the door timeout state transition.
///
/// # Parameters
/// - `self_container`: Mutable reference to the elevator's internal state.
/// - `e`: Elevator identifier or hardware handle used to control lights and motors.
/// - `door_timer`: Timer that tracks how long the door has been open.
///
/// # Behavior
/// - Handles door-close logic via finite state machine if cab call timer is also expired.
pub async fn handle_door_timeout(
    self_container: &mut ElevatorContainer,
    e: Elevator,
    door_timer: &Timer,
    cab_priority_timer: &mut Timer,
) {
    if door_timer.timer_timeouted() && !self_container.obstruction {
        if request::moving_towards_cab_call(&self_container.clone()) {
            cab_priority_timer.release_timer();
        }

        if cab_priority_timer.timer_timeouted() {
            on_door_timeout(self_container, e.clone()).await;
        }
    }
}

/// Monitors elevator activity and triggers error behavior after a timeout period.
///
/// If no cab call has timed out or the elevator is idle, the error timer is restarted.
/// If the error timer itself has expired and a cab call was previously active,
/// the elevator enters an error state and logs a critical error message.
///
/// # Parameters
/// - `self_container`: Reference to the elevator state being monitored.
/// - `cab_priority_timer`: Timer tracking how long a cab call has been pending.
/// - `error_timer`: Mutable timer for detecting inactivity or system faults.
/// - `prev_cab_priority_timer_stat`: Whether the cab call timer had previously expired.
///
/// # Behavior
/// - Triggers an error state if prolonged inactivity or failure is detected.
pub fn handle_error_timeout(
    self_container: &ElevatorContainer,
    cab_priority_timer: &Timer,
    error_timer: &mut Timer,
    prev_cab_priority_timer_stat: bool,
) {
    if !cab_priority_timer.timer_timeouted() || self_container.behaviour == ElevatorBehaviour::Idle {
        error_timer.timer_start();
    }


    if error_timer.timer_timeouted() && !prev_cab_priority_timer_stat {
        if !self_container.obstruction && (self_container.behaviour == ElevatorBehaviour::DoorOpen) {
            error_timer.timer_start();
        } else {
            print::err("Elevator entered error".to_string());
        }
    }
}

/// Attempts to transition the elevator from idle to active movement if a request is pending.
///
/// If the elevator is currently idle, the system chooses a new direction and behavior
/// using the request logic. If a non-idle state is chosen, the elevator's direction
/// and behavior are updated, the door timer is started, and the motor is stopped
/// in preparation for movement or door logic.
///
/// # Parameters
/// - `self_container`: Mutable reference to the elevator's current state.
/// - `e`: Elevator handle or control interface.
/// - `door_timer`: Timer used to delay transitions or prepare door actions.
///
/// # Behavior
/// - Only operates when the elevator is in an idle state.
/// - Initializes direction and behavior when transitioning out of idle.
/// - Starts door timer and stops the motor to stabilize before further action.
pub fn handle_idle_state(
    self_container: &mut ElevatorContainer,
    e: Elevator,
    door_timer: &mut Timer,
) {
    if self_container.behaviour == ElevatorBehaviour::Idle {
        let status_pair = request::choose_direction(&self_container.clone());

        if status_pair.behaviour != ElevatorBehaviour::Idle {
            print::err(format!("Skal nå være: {:?}", status_pair.behaviour));
            self_container.dirn = status_pair.dirn;
            self_container.behaviour = status_pair.behaviour;
            door_timer.timer_start();
            e.motor_direction(Dirn::Stop as u8);
        }
    }
}


