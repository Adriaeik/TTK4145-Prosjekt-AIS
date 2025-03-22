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
use crate::print;
use crate::world_view;
use crate::world_view::Dirn;
use crate::world_view::ElevatorBehaviour;
use crate::world_view::ElevatorContainer;

/// Initializes and runs the local elevator logic as a set of async tasks.
///
/// This function performs the following:
/// - Initializes the local elevator instance and communication channels.
/// - Spawns one async task to handle elevator state and behavior (`handle_elevator`).
/// - Spawns another task to continuously update the hall request lights based on world view state.
/// - Keeps the main task alive indefinitely via an infinite `yield_now` loop.
///
/// # Parameters
/// - `wv_watch_rx`: A `watch::Receiver` that provides the latest serialized world view.
/// - `elevator_states_tx`: A `mpsc::Sender` used to send the local elevator state back to the system.
///
/// # Behavior
/// - Runs all logic asynchronously and non-blocking.
/// - Continues operation until externally cancelled or interrupted.
/// - Each spawned task operates independently of the main loop.
///
/// # Note
/// The hall light updater task continuously reads the world view and sets the hall lights based on
/// the current state of the local elevator. Failure to extract the local container results in a warning.
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
                match world_view::extract_self_elevator_container(wv.clone()) {
                    Some(self_elevator) => {
                        lights::set_hall_lights(wv.clone(), e.clone());
                    }
                    None => {
                        print::warn(format!("Failed to extract self elevator container"));
                    }
                }
                sleep(config::POLL_PERIOD).await;
            }
        });
    }  

    loop {
        yield_now().await;
    }
    
}


/// Main event loop for handling local elevator logic, state transitions, and communication.
///
/// This function implements the core elevator state machine and handles:
/// - Receiving updates from local hardware (buttons, floor sensors, etc.)
/// - Executing FSM transitions based on current state and events
/// - Managing timers for door state, cab call delays, and error detection
/// - Updating direction and motor control
/// - Sending updated elevator state to the rest of the system
/// - Applying updates from the world view (task assignments, shared state)
///
/// # Parameters
/// - `wv_watch_rx`: A `watch::Receiver` used to access the latest global world view.
/// - `elevator_states_tx`: A `mpsc::Sender` used to transmit updated local elevator state.
/// - `local_elev_rx`: A `mpsc::Receiver` that receives elevator hardware messages.
/// - `e`: Handle representing the elevator hardware interface (for lights, motor, etc.)
///
/// # Behavior
/// - Blocks in a loop, continuously reacting to inputs and updating state.
/// - Relies on helper functions for modular FSM logic and safety mechanisms.
/// - Polls the world view and local state at a fixed interval (`config::POLL_PERIOD`).
///
/// # Notes
/// - The function will attempt to initialize the elevator state by waiting for it
///   to reach the closest floor in downward direction (via `fsm::onInit`).
/// - If the elevator starts on floor 0, special care must be taken (known crash case).
/// - Errors are handled internally via timers and behavior transitions.
async fn handle_elevator(wv_watch_rx: watch::Receiver<Vec<u8>>, elevator_states_tx: mpsc::Sender<Vec<u8>>, mut local_elev_rx: mpsc::Receiver<elevio::ElevMessage>, e: Elevator) {
    
    let mut wv = world_view::get_wv(wv_watch_rx.clone());
    let mut self_container = await_valid_self_container(wv_watch_rx.clone()).await;

    
    let mut timers = timer::ElevatorTimers::new(
        Duration::from_secs(3),   // door timer
        Duration::from_secs(10),  // cab call priority
        Duration::from_secs(7),   // error timer
    );


    //init the state. this is blocking until we reach the closest foor in down direction
    fsm::on_init(&mut self_container, e.clone(), &mut local_elev_rx, &mut timers).await;


    // self_container.dirn = Dirn::Stop;
    let mut prev_behavior:ElevatorBehaviour = self_container.behaviour;
    let mut prev_floor: u8 = self_container.last_floor_sensor;
    
    loop {
        /*OBS OBS!! krasjer når vi starter i 0 etasje..... uff da */
        //Les nye data fra heisen, putt de inn i self_container
        
        self_elevator::update_elev_container_from_msgs(&mut local_elev_rx, &mut self_container, &mut timers.cab_priority , &mut timers.error ).await;
        
        /*======================================================================*/
        /*                           START: FSM Events                          */
        /*======================================================================*/
        fsm::handle_floor_sensor_update(
            &mut self_container,
            e.clone(),
            &mut prev_floor,
            &mut timers,
        ).await;        

        
        fsm::handle_door_timeout_and_lights(
            &mut self_container,
            e.clone(),
            &timers.door,
            &mut timers.cab_priority,
        ).await;
        
        fsm::handle_error_timeout(
            &self_container,
            &timers.cab_priority,
            &mut timers.error,
            timers.prev_cab_priority_timeout,
        );
        
        // fsm::onIdle ?
        fsm::handle_idle_state(&mut self_container, e.clone(), &mut timers.door);
        /*======================================================================*/
        /*                           END: FSM Events                            */
        /*======================================================================*/

        /*============================================================================================================================================*/
        
        update_motor_direction_if_needed(&self_container, &e);

        update_error_state(&mut self_container, &timers.error, &mut timers.prev_cab_priority_timeout);

        let last_behavior: ElevatorBehaviour = track_behavior_change(&self_container, &mut prev_behavior);
        stop_motor_on_dooropen_to_error(&mut self_container, last_behavior, prev_behavior);

        
        
        //Send til update_wv -> nye self_container
        let _ = elevator_states_tx.send(world_view::serial::serialize_elev_container(&self_container)).await;    
        
        //Hent nyeste worldview
        if world_view::update_wv(wv_watch_rx.clone(), &mut wv).await{
            update_tasks_and_hall_requests(&mut self_container, wv.clone()).await;
        }
        yield_now().await;
        sleep(config::POLL_PERIOD).await;

        
        
    }
}

/// Updates the local elevator container's task-related fields based on the latest world view.
///
/// This function attempts to extract the elevator container corresponding to `SELF_ID` from
/// the given serialized world view. If found, it updates `tasks` and `unsent_hall_request`
/// in the local container. If extraction fails, the local values are left unchanged,
/// and a warning is printed.
///
/// # Parameters
/// - `self_container`: A mutable reference to the local elevator container to be updated.
/// - `wv`: A serialized world view (`Vec<u8>`) to extract the container from.
///
/// # Behavior
/// - Safe to call repeatedly.
/// - Only updates the two mentioned fields if a valid container is found.
/// - Prints a warning if no container is found.
///
/// # Example
/// ```ignore
/// update_tasks_and_hall_requests(&mut local_container, serialized_worldview).await;
/// ```
async fn update_tasks_and_hall_requests(self_container: &mut ElevatorContainer, wv: Vec<u8>){
    if let Some(task_container) = world_view::extract_self_elevator_container(wv) {
        self_container.tasks = task_container.tasks;
        self_container.unsent_hall_request = task_container.unsent_hall_request;
    } else {
        print::warn(format!("Failed to extract self elevator container – keeping previous value"));
    }
}

/// Continuously attempts to extract the local elevator container from the world view until successful.
///
/// This function loops until it successfully extracts the container for `SELF_ID` from the
/// current world view received over a `watch::Receiver`. It prints a warning for each failed
/// attempt and waits 100 milliseconds between retries.
///
/// # Parameters
/// - `wv_rx`: A watch channel receiver providing the latest serialized world view (`Vec<u8>`).
///
/// # Returns
/// - A fully initialized `ElevatorContainer` once it is successfully extracted.
///
/// # Notes
/// - This function does not return until a valid container is available.
/// - It is suitable for running inside a long-lived async task.
///
/// # Example
/// ```ignore
/// let container = await_valid_self_container(wv_rx).await;
/// ```
async fn await_valid_self_container(wv_rx: watch::Receiver<Vec<u8>>) -> ElevatorContainer {
    loop {
        let wv = world_view::get_wv(wv_rx.clone());
        if let Some(container) = world_view::extract_self_elevator_container(wv) {
            return container;
        } else {
            print::warn(format!("Failed to extract self elevator container, retrying..."));
            sleep(Duration::from_millis(100)).await;
        }
    }
}


/// Updates the motor direction if the elevator is not in the DoorOpen state.
///
/// This function sends the current direction (`dirn`) to the motor controller
/// only if the elevator is not in the `DoorOpen` state.
///
/// # Parameters
/// - `self_container`: Reference to the current elevator state.
/// - `e`: Elevator interface used to send motor direction commands.
///
/// # Behavior
/// - Prevents motor updates while the door is open.
/// - Useful for ensuring motor is only active during appropriate states.
fn update_motor_direction_if_needed(self_container: &ElevatorContainer, e: &Elevator) {
    if self_container.behaviour != ElevatorBehaviour::DoorOpen {
        e.motor_direction(self_container.dirn as u8);
    }
}

/// Updates the elevator state based on the error timer's status.
///
/// If the error timer has expired, the elevator transitions into the `Error` state
/// and the `prev_cab_priority_timer_stat` flag is set. Otherwise, the flag is cleared.
///
/// # Parameters
/// - `self_container`: Mutable reference to the elevator state.
/// - `error_timer`: Timer that tracks potential error conditions.
/// - `prev_cab_priority_timer_stat`: Mutable flag to track whether the system was previously in a faultable state.
///
/// # Behavior
/// - Sets elevator to `Error` if timer has expired.
/// - Updates a boolean tracking previous timer state.
fn update_error_state(
    self_container: &mut ElevatorContainer,
    error_timer: &timer::Timer,
    prev_cab_priority_timer_stat: &mut bool,
) {
    if error_timer.timer_timeouted() {
        *prev_cab_priority_timer_stat = true;
        self_container.behaviour = ElevatorBehaviour::Error;
    } else {
        *prev_cab_priority_timer_stat = false;
    }
}

/// Tracks and logs changes to the elevator's behavior state.
///
/// Compares the current elevator behavior to a previously stored value.
/// If the state has changed, logs the transition and updates `prev_behavior`.
///
/// # Parameters
/// - `self_container`: Reference to the current elevator state.
/// - `prev_behavior`: Mutable reference to the last recorded behavior state.
///
/// # Returns
/// - The previous behavior before the update (if any).
///
/// # Behavior
/// - Detects and logs behavior transitions for debugging or system monitoring.
fn track_behavior_change(
    self_container: &ElevatorContainer,
    prev_behavior: &mut ElevatorBehaviour,
) -> ElevatorBehaviour {
    let last_behavior = *prev_behavior;

    if *prev_behavior != self_container.behaviour {
        *prev_behavior = self_container.behaviour;
        println!("Endra status: {:?} -> {:?}", last_behavior, self_container.behaviour);
    }

    last_behavior
}

/// Forces the elevator to stop the motor when transitioning from DoorOpen to Error state.
///
/// If the behavior transition is specifically from `DoorOpen` to `Error`, the elevator
/// direction is set to `Stop` to ensure the motor halts immediately.
///
/// # Parameters
/// - `self_container`: Mutable reference to the elevator state.
/// - `last_behavior`: The previous elevator behavior before the transition.
/// - `current_behavior`: The current elevator behavior after the transition.
///
/// # Behavior
/// - Stops the motor only for the specific transition from `DoorOpen` → `Error`.
fn stop_motor_on_dooropen_to_error(
    self_container: &mut ElevatorContainer,
    last_behavior: ElevatorBehaviour,
    current_behavior: ElevatorBehaviour,
) {
    if last_behavior == ElevatorBehaviour::DoorOpen && current_behavior == ElevatorBehaviour::Error {
        self_container.dirn = Dirn::Stop;
    }
}
