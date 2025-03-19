pub mod world_view_update;
pub mod serial;

use serde::{Serialize, Deserialize};
use tokio::sync::watch;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use crate::config;
use crate::print;
use crate::network::local_network;
use crate::elevio;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dirn {
    Down = -1,
    Stop = 0,
    Up = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalElevatorState {
    behaviour: ElevatorBehaviour,
    floor: i32,
    direction: Dirn,
    cab_requests: Vec<bool>,
}




/// Represents the state of an elevator, including tasks, status indicators, and movement.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ElevatorContainer {
    /// Unique identifier for the elevator.
    pub elevator_id: u8, // Default: ERROR_ID

    pub num_floors: u8,

    /// List of external call requests.
    // pub calls: Vec<elevio::CallButton>, // Default: empty vector
    pub unsent_hall_request: Vec<[bool; 2]>,

    /// List of assigned tasks for the elevator.
    pub cab_requests: Vec<bool>,

    pub tasks: Vec<[bool; 2]>, 

    pub dirn: Dirn,

    pub behaviour: ElevatorBehaviour,

    /// Indicates whether the elevator detects an obstruction.
    pub obstruction: bool, // Default: false

    /// The last detected floor sensor position.
    pub last_floor_sensor: u8, // Default: 255 (undefined)
}


impl Default for ElevatorContainer {
    fn default() -> Self {
        Self {
            elevator_id: config::ERROR_ID,
            num_floors: config::DEFAULT_NUM_FLOORS,
            // calls: Vec::new(),
            unsent_hall_request: vec![[false; 2]; config::DEFAULT_NUM_FLOORS as usize],
            cab_requests: vec![false; config::DEFAULT_NUM_FLOORS as usize],
            tasks: vec![[false, false]; config::DEFAULT_NUM_FLOORS as usize],
            // task: None,
            // status: ElevatorStatus::IDLE,
            dirn: Dirn::Stop,
            behaviour: ElevatorBehaviour::Idle,
            obstruction: false,
            last_floor_sensor: 255, // Spesifikk verdi for sensor
        }
    }
}


/// Represents the system's current state (WorldView).
///
/// `WorldView` contains an overview of all elevators in the system, 
/// the master elevator's ID, and the call buttons pressed outside the elevators.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorldView {
    /// - `n`: Number of elevators in the system.
    n: u8, 
    /// - `master_id`: The ID of the master elevator.
    pub master_id: u8, 
    /// - `pending_tasks`: A list of call buttons pressed outside elevators.
    // pub pending_tasks: Vec<Task>, 
    pub hall_request: Vec<[bool; 2]>,

    /// - `elevator_containers`: A list of `ElevatorContainer` structures containing
    ///   individual elevator information.
    pub elevator_containers: Vec<ElevatorContainer>, 

    pub cab_requests_backup: HashMap<u8, Vec<bool>>,
}


impl Default for WorldView {
    /// Creates a default `WorldView` instance with no elevators and an invalid master ID.
    fn default() -> Self {
        Self {
            n: 0,
            master_id: config::ERROR_ID,
            // pending_tasks: Vec::new(),
            hall_request: vec![[false; 2]; config::DEFAULT_NUM_FLOORS as usize],
            elevator_containers: Vec::new(),
            cab_requests_backup: HashMap::new(),
        }
    }
}


impl WorldView {
    /// Adds an elevator to the system.
    ///
    /// Updates the number of elevators (`n`) accordingly.
    ///
    /// ## Parameters
    /// - `elevator`: The `ElevatorContainer` to be added.
    pub fn add_elev(&mut self, elevator: ElevatorContainer) {
        self.elevator_containers.push(elevator);
        self.n = self.elevator_containers.len() as u8;
    }
    
    /// Removes an elevator with the given ID from the system.
    ///
    /// If no elevator with the specified ID is found, a warning is printed.
    ///
    /// ## Parameters
    /// - `id`: The ID of the elevator to remove.
    pub fn remove_elev(&mut self, id: u8) {
        let initial_len = self.elevator_containers.len();

        self.elevator_containers.retain(|elevator| elevator.elevator_id != id);
    
        if self.elevator_containers.len() == initial_len {
            print::warn(format!("No elevator with ID {} was found. (remove_elev())", id));
        } else {
            print::ok(format!("Elevator with ID {} was removed. (remove_elev())", id));
        }
        self.n = self.elevator_containers.len() as u8;
    }

    /// Returns the number of elevators in the system.
    pub fn get_num_elev(&self) -> u8 {
        return self.n;
    }


    /// Sets the number of elevators manually.
    ///
    /// **Note:** This does not affect the `elevator_containers` list. 
    /// Use `add_elev()` or `remove_elev()` to modify the actual elevators.
    ///
    /// ## Parameters
    /// - `n`: The new number of elevators.
    // TODO: Burde være veldig mulig å gjøre denne privat
    pub fn set_num_elev(&mut self, n: u8)  {
        self.n = n;
    }
}





/// Retrieves the index of an `ElevatorContainer` with the specified `id` in the deserialized `WorldView`.
///
/// This function deserializes the provided `WorldView` data and iterates through the elevator containers
/// to find the one that matches the given `id`. If found, it returns the index of the container; otherwise, it returns `None`.
///
/// ## Parameters
/// - `id`: The ID of the elevator whose index is to be retrieved.
/// - `wv`: A serialized `WorldView` as a `Vec<u8>`.
///
/// ## Returns
/// - `Some(usize)`: The index of the `ElevatorContainer` in the `WorldView` if found.
/// - `None`: If no elevator with the given `id` exists.
pub fn get_index_to_container(id: u8, wv: Vec<u8>) -> Option<usize> {
    let wv_deser = serial::deserialize_worldview(&wv);
    for i in 0..wv_deser.get_num_elev() {
        if wv_deser.elevator_containers[i as usize].elevator_id == id {
            return Some(i as usize);
        }
    }
    return None;
}


/// Fetches a clone of the latest local worldview (wv) from the system.
///
/// This function retrieves the most recent worldview stored in the provided `LocalChannels` object.
/// It returns a cloned vector of bytes representing the current serialized worldview.
///
/// # Parameters
/// - `chs`: The `LocalChannels` object, which contains the latest worldview data in `wv`.
///
/// # Return Value
/// Returns a vector of `u8` containing the cloned serialized worldview.
///
/// # Example
/// ```
/// use elevatorpro::utils::get_wv;
/// use elevatorpro::network::local_network::LocalChannels;
/// 
/// let local_chs = LocalChannels::new();
/// let _ = local_chs.watches.txs.wv.send(vec![1, 2, 3, 4]);
/// 
/// let fetched_wv = get_wv(local_chs.clone());
/// assert_eq!(fetched_wv, vec![1, 2, 3, 4]);
/// ```
///
/// **Note:** This function clones the current state of `wv`, so any future changes to `wv` will not affect the returned vector.
pub fn get_wv(wv_watch_rx: watch::Receiver<Vec<u8>>) -> Vec<u8> {
    wv_watch_rx.borrow().clone()
}

/// Asynchronously updates the worldview (wv) in the system.
///
/// This function reads the latest worldview data from a specific channel and updates
/// the given `wv` vector with the new data if it has changed. The function operates asynchronously,
/// allowing it to run concurrently with other tasks without blocking.
///
/// ## Parameters
/// - `chs`: The `LocalChannels` object, which holds the channels used for receiving worldview data.
/// - `wv`: A mutable reference to the `Vec<u8>` that will be updated with the latest worldview data.
///
/// ## Returns
/// - `true` if wv was updated, `false` otherwise.
///
/// ## Example
/// ```
/// # use tokio::runtime::Runtime;
/// use elevatorpro::utils::update_wv;
/// use elevatorpro::network::local_network::LocalChannels;
/// 
/// let chs = LocalChannels::new();
/// let mut wv = vec![1, 2, 3, 4];
/// 
/// # let rt = Runtime::new().unwrap();
/// # rt.block_on(async {/// 
/// chs.watches.txs.wv.send(vec![4, 3, 2, 1]);
/// let result = update_wv(chs.clone(), &mut wv).await;
/// assert_eq!(result, true);
/// assert_eq!(wv, vec![4, 3, 2, 1]);
/// 
/// let result = update_wv(chs.clone(), &mut wv).await;
/// assert_eq!(result, false);
/// assert_eq!(wv, vec![4, 3, 2, 1]);
/// # });
/// ```
///
/// ## Notes
/// - This function is asynchronous and requires an async runtime, such as Tokio, to execute.
/// - The `LocalChannels` channels allow for thread-safe communication across threads.
pub async fn update_wv(wv_watch_rx: watch::Receiver<Vec<u8>>, wv: &mut Vec<u8>) -> bool {
    let new_wv = wv_watch_rx.borrow().clone();  // Clone the latest data
    if new_wv != *wv {  // Check if the data has changed compared to the current state
        *wv = new_wv;  // Update the worldview if it has changed
        return true;
    }
    false
}


/// Checks if the current system is the master based on the latest worldview data.
///
/// This function compares the system's `SELF_ID` with the value at `MASTER_IDX` in the provided worldview (`wv`).
///
/// ## Returns
/// - `true` if the current system's `SELF_ID` matches the value at `MASTER_IDX` in the worldview.
/// - `false` otherwise.
pub fn is_master(wv: Vec<u8>) -> bool {
    return local_network::SELF_ID.load(Ordering::SeqCst) == wv[config::MASTER_IDX];
}

// /// Retrieves the latest elevator tasks from the system.
// ///
// /// This function borrows the value from the `elev_task` channel and clones it, returning a copy of the tasks.
// /// It is used to fetch the current tasks for the local elevator.
// ///
// /// ## Parameters
// /// - `chs`: A `LocalChannels` struct that contains the communication channels for the system.
// ///
// /// ## Returns
// /// - A `Vec<Task>` containing the current elevator tasks.
// pub fn get_elev_tasks(elev_task_rx: watch::Receiver<Vec<Task>>) -> Vec<Task> {
//     elev_task_rx.borrow().clone()
// }

/// Retrieves a clone of the `ElevatorContainer` with the specified `id` from the provided worldview.
///
/// This function deserializes the provided worldview (`wv`), filters the elevator containers based on the given `id`,
/// and returns a clone of the matching `ElevatorContainer`. If no matching elevator is found, the behavior is undefined.
///
/// ## Parameters
/// - `wv`: The latest worldview in serialized state.
/// - `id`: The `id` of the elevator container to extract.
///
/// ## Returns
/// - A clone of the `ElevatorContainer` with the specified `id`, or the first match found.
///
/// **Note:** If no elevator container with the specified `id` is found, this function will panic due to indexing.
pub fn extract_elevator_container(wv: Vec<u8>, id: u8) -> ElevatorContainer {
    let mut deser_wv = serial::deserialize_worldview(&wv);

    deser_wv.elevator_containers.retain(|elevator| elevator.elevator_id == id);
    deser_wv.elevator_containers[0].clone()
}

/// Retrieves a clone of the `ElevatorContainer` with `SELF_ID` from the latest worldview.
///
/// This function calls `extract_elevator_container` with `SELF_ID` to fetch the elevator container that matches the
/// current `SELF_ID` from the provided worldview (`wv`). The `SELF_ID` is a static identifier loaded from memory,
/// which represents the current elevator's unique identifier.
///
/// ## Parameters
/// - `wv`: The latest worldview in serialized state.
///
/// ## Returns
/// - A clone of the `ElevatorContainer` associated with `SELF_ID`.
///
/// **Note:** This function internally calls `extract_elevator_container` to retrieve the correct elevator container.
pub fn extract_self_elevator_container(wv: Vec<u8>) -> ElevatorContainer {
    extract_elevator_container(wv, local_network::SELF_ID.load(Ordering::SeqCst))
}


