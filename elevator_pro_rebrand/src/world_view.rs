//! ## WorldView Module
//!
//! This module defines central data structures and helper functions for managing the 
//! global state of the elevator network. It contains the `WorldView` struct, which serves as 
//! an information packet about the entire system, including all elevators, their tasks, and 
//! hall requests. Additionally, it provides serialization, deserialization, and retrieval 
//! utilities for handling worldview data efficiently.
//!
//! ### Key Responsibilities:
//! - **Defining Core Structs**: `WorldView` and `ElevatorContainer` store network-wide 
//!   and per-elevator state, respectively.
//! - **Handling Directions and States**: The `Dirn` and `ElevatorBehaviour` enums define 
//!   movement direction and current operational state of elevators.
//! - **Data Serialization and Deserialization**: Utility functions facilitate efficient 
//!   transmission and storage of worldview data.
//! - **Retrieving Elevator Information**: Functions allow querying and modifying elevator 
//!   state within the network.
//! - **Master Detection**: Determines whether the current system is the master node.
//!
//! ### Overview of Structs & Enums:
//! - [`Dirn`] – Represents the movement direction of an elevator.
//! - [`ElevatorBehaviour`] – Describes the current state of an elevator.
//! - [`ElevatorContainer`] – Holds information about an individual elevator's state, tasks, and requests.
//! - [`WorldView`] – Contains global network state, including all elevators and hall requests.
//!
//! ### Overview of Functions:
//! - [`serialize`] / [`deserialize`] – Convert worldview data to and from binary format.
//! - [`get_wv`] – Retrieves the latest local worldview.
//! - [`update_wv`] – Updates worldview asynchronously if changes are detected.
//! - [`is_master`] – Checks if the current elevator is the master.
//! - [`extract_elevator_container`] / [`extract_self_elevator_container`] – Retrieve elevator state from worldview.
//! - [`get_index_to_container`] – Finds the index of an elevator container by ID.
//!
//! This module is critical for ensuring a synchronized state across networked elevators.


use crate::config;
use crate::network;
use crate::print;

use bincode;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::collections::HashMap;
use tokio::sync::watch;


#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Struct describing direction an elevator is taking calls in.
pub enum Dirn {
    Down = -1,
    Stop = 0,
    Up = 1,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Struct describing the current behaviour of an elevator
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    TravelError,
    ObstructionError,
    CosmicError,
}


/// Represents the state of an elevator, including tasks, status indicators, and movement.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ElevatorContainer {
    /// Unique identifier for the elevator.  
    /// Default: [config::ERROR_ID]
    pub elevator_id: u8,

    /// The number of floors the elevator can access  
    /// Default: [config::DEFAULT_NUM_FLOORS]
    pub num_floors: u8,

    /// Vector of hall requests not yet sent to master over TCP  
    /// Default: full of \[false, false\], length [config::DEFAULT_NUM_FLOORS]
    pub unsent_hall_request: Vec<[bool; 2]>,

    /// Vector of cab_requests.  
    /// Default: full of false, length [config::DEFAULT_NUM_FLOORS]
    pub cab_requests: Vec<bool>,

    /// Vector of hall_requests given to this elevator from the manager.  
    /// Default: full of \[false, false\], length [config::DEFAULT_NUM_FLOORS]
    pub tasks: Vec<[bool; 2]>, 

    /// [Dirn]  
    ///  Default: [Dirn::Stop]
    pub dirn: Dirn, 

    /// The current behaviour of the elevator  
    /// Default: [ElevatorBehaviour::Idle]
    pub behaviour: ElevatorBehaviour, 

    /// The last behaviour of the elevator
    pub last_behaviour: ElevatorBehaviour,

    /// Indicates whether the elevator detects an obstruction.  
    /// Default: false
    pub obstruction: bool, 

    /// Indicates wether the stop button is being pressed.
    /// Default: false
    pub stop: bool,

    /// The last detected floor sensor position.  
    /// Default: 255
    pub last_floor_sensor: u8,
}

impl Default for ElevatorContainer {
    fn default() -> Self {
        Self {
            elevator_id: config::ERROR_ID,
            num_floors: config::DEFAULT_NUM_FLOORS,
            unsent_hall_request: vec![[false; 2]; config::DEFAULT_NUM_FLOORS as usize],
            cab_requests: vec![false; config::DEFAULT_NUM_FLOORS as usize],
            tasks: vec![[false, false]; config::DEFAULT_NUM_FLOORS as usize],
            dirn: Dirn::Stop,
            behaviour: ElevatorBehaviour::Idle,
            last_behaviour: ElevatorBehaviour::Idle,
            obstruction: false,
            stop: false,
            last_floor_sensor: 255, 
        }
    }
}


/// Represents the system's current state (WorldView).
///
/// `WorldView` contains an overview of all elevators in the system, 
/// the master elevator's ID, and the call buttons pressed outside the elevators.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WorldView {
    /// Number of elevators in the system.
    n: u8, 
    /// The ID of the master elevator.
    pub master_id: u8, 
    /// A vector contining statuses on all hall requests  
    pub hall_request: Vec<[bool; 2]>,

    /// A list of `ElevatorContainer` structures containing
    ///   individual elevator information.
    pub elevator_containers: Vec<ElevatorContainer>, 
    
    /// A HashMap backing up cab_call statuses for all elevators, mapping them to their IDs
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



/// Serialiserer kva som helst `T` til `Vec<u8>` via bincode
pub fn serialize<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).expect("Klarte ikkje serialisere verdi")
}

/// Deserialiserer `&[u8]` til `T` viss mogleg
pub fn deserialize<T: DeserializeOwned>(buf: &[u8]) -> Option<T> {
    bincode::deserialize(buf).ok()
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
pub fn get_index_to_container(id: u8, wv: &WorldView) -> Option<usize> {
    for i in 0..wv.get_num_elev() {
        if wv.elevator_containers[i as usize].elevator_id == id {
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
pub fn get_wv(wv_watch_rx: watch::Receiver<WorldView>) -> WorldView {
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
pub async fn update_wv(wv_watch_rx: watch::Receiver<WorldView>, wv: &mut WorldView) -> bool {
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
pub fn is_master(wv: &WorldView) -> bool {
    return network::read_self_id() == wv.master_id;
}


/// Extracts the elevator container with the specified `id` from the given serialized worldview.
///
/// This function deserializes the provided worldview, filters out elevator containers
/// that do not match the given `id`, and returns the first matching result if available.
///
/// ## Parameters
/// - `wv`: A `Vec<u8>` representing the serialized worldview.
/// - `id`: The elevator ID to search for.
///
/// ## Returns
/// - `Some(ElevatorContainer)` if a container with the given `id` is found.
/// - `None` if no matching elevator container exists in the worldview.
///
/// ## Note
/// If multiple containers have the same `id`, only the first match is returned.
pub fn extract_elevator_container(wv: &WorldView, id: u8) -> Option<&ElevatorContainer> {
    wv.elevator_containers.iter().find(|elevator| elevator.elevator_id == id)
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
pub fn extract_self_elevator_container(wv: &WorldView) -> Option<&ElevatorContainer> {
    let id = network::read_self_id();
    extract_elevator_container(wv, id)
}


