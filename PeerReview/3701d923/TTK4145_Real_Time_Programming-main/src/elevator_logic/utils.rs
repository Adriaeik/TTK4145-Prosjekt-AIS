// Libraries for data structures
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Idle,
    Up,
    Down,
    Door,
    EmergencyStop,
    EmergencyStopIdle,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Stop,
    Up,
    Down,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElevHallRequests {
    pub add_up: Option<u8>,
    pub add_down: Option<u8>,
    pub remove_up: Option<u8>,
    pub remove_down: Option<u8>,
}

// Function to create an `ElevHallRequests` and return its JSON representation
pub fn create_hall_request_json(add_up: Option<u8>, add_down: Option<u8>, remove_up: Option<u8>, remove_down: Option<u8>) -> String {
    let request = ElevHallRequests { add_up, add_down, remove_up, remove_down };

    serde_json::to_string(&request).expect("Failed to serialize ElevHallRequests")
}
