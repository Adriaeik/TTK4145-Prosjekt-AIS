use crossbeam_channel as cbc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time;
use serde::{Serialize, Deserialize};
use std::hash::{Hash, Hasher};

use crate::config;
use crate::network::local_network;
use crate::print;
use crate::ip_help_functions;

use super::elev::{self/*, DIRN_STOP, DIRN_DOWN, DIRN_UP*/};

/// Represents the type of call for an elevator.
///
/// This enum is used to differentiate between different types of elevator requests.
/// 
/// ## Variants
/// - `UP`: A request to go up.
/// - `DOWN`: A request to go down.
/// - `INSIDE`: A request made from inside the elevator.
/// - `COSMIC_ERROR`: An invalid call type (used as an error fallback).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Ensures the enum is stored as a single byte.
#[allow(non_camel_case_types)]
pub enum CallType {
    /// Call to go up.
    UP = 0,
    
    /// Call to go down.
    DOWN = 1,
    
    /// Call from inside the elevator.
    INSIDE = 2,
    
    /// Represents an invalid call type.
    COSMIC_ERROR = 255,
}
impl From<u8> for CallType {
    /// Converts a `u8` value into a `CallType`.
    ///
    /// If the value does not match a valid `CallType`, it logs an error and returns `COSMIC_ERROR`.
    ///
    /// # Examples
    /// ```
    /// # use elevatorpro::elevio::poll::CallType;
    ///
    /// let call_type = CallType::from(0);
    /// assert_eq!(call_type, CallType::UP);
    ///
    /// let invalid_call = CallType::from(10);
    /// assert_eq!(invalid_call, CallType::COSMIC_ERROR);
    /// ```
    fn from(value: u8) -> Self {
        match value {
            0 => CallType::UP,
            1 => CallType::DOWN,
            2 => CallType::INSIDE,
            _ => {
                print::cosmic_err("Call type does not exist".to_string());
                CallType::COSMIC_ERROR
            },
        }
    }
}

/// Represents a button press in an elevator system.
///
/// Each button press consists of:
/// - `floor`: The floor where the button was pressed.
/// - `call`: The type of call (up, down, inside).
/// - `elev_id`: The ID of the elevator (relevant for `INSIDE` calls).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq)]
pub struct CallButton {
    /// The floor where the call was made.
    pub floor: u8,

    /// The type of call (UP, DOWN, or INSIDE).
    pub call_type: CallType,

    /// The ID of the elevator making the call (only relevant for `INSIDE` calls).
    pub elev_id: u8,
}
impl Default for CallButton {
    fn default() -> Self {
        CallButton{floor: 1, call_type: CallType::INSIDE, elev_id: config::ERROR_ID}
    }
}


impl PartialEq for CallButton {
    /// Custom equality comparison for `CallButton`.
    ///
    /// Two call buttons are considered equal if they have the same floor and call type.
    /// However, for `INSIDE` calls, the `elev_id` must also match.
    ///
    /// # Examples
    /// ```
    /// # use elevatorpro::elevio::poll::{CallType, CallButton};
    ///
    /// let button1 = CallButton { floor: 3, call: CallType::UP, elev_id: 1 };
    /// let button2 = CallButton { floor: 3, call: CallType::UP, elev_id: 2 };
    ///
    /// assert_eq!(button1, button2); // Same floor & call type
    ///
    /// let inside_button1 = CallButton { floor: 2, call: CallType::INSIDE, elev_id: 1 };
    /// let inside_button2 = CallButton { floor: 2, call: CallType::INSIDE, elev_id: 2 };
    ///
    /// assert_ne!(inside_button1, inside_button2); // Different elevators
    /// ```
    fn eq(&self, other: &Self) -> bool {
        // Hvis call er INSIDE, sammenligner vi også elev_id
        if self.call_type == CallType::INSIDE {
            self.floor == other.floor && self.call_type == other.call_type && self.elev_id == other.elev_id
        } else {
            // For andre CallType er det tilstrekkelig å sammenligne floor og call
            self.floor == other.floor && self.call_type == other.call_type
        }
    }
}
impl Hash for CallButton {
    /// Custom hashing function to ensure consistency with `PartialEq`.
    ///
    /// This ensures that buttons with the same floor and call type have the same hash.
    /// For `INSIDE` calls, the elevator ID is also included in the hash.
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Sørger for at hash er konsistent med eq
        self.floor.hash(state);
        self.call_type.hash(state);
        if self.call_type == CallType::INSIDE {
            self.elev_id.hash(state);
        }
    }
}

#[doc(hidden)]
pub fn call_buttons(elev: elev::Elevator, ch: cbc::Sender<CallButton>, period: time::Duration) {
    let mut prev = vec![[false; 3]; elev.num_floors.into()];
    loop {
        for f in 0..elev.num_floors {
            for c in 0..3 {
                let v = elev.call_button(f, c);
                if v && prev[f as usize][c as usize] != v {
                    ch.send(CallButton { floor: f, call_type: CallType::from(c), elev_id: local_network::SELF_ID.load(Ordering::SeqCst)}).unwrap();
                }
                prev[f as usize][c as usize] = v;
            }
        }
        thread::sleep(period)
    }
}

#[doc(hidden)]
pub fn floor_sensor(elev: elev::Elevator, ch: cbc::Sender<u8>, period: time::Duration) {
    let mut prev = u8::MAX;
    loop {
        if let Some(f) = elev.floor_sensor() {
            if f != prev {
                ch.send(f).unwrap();
                prev = f;
            }
        }
        thread::sleep(period)
    }
}

#[doc(hidden)]
pub fn stop_button(elev: elev::Elevator, ch: cbc::Sender<bool>, period: time::Duration) {
    let mut prev = false;
    loop {
        let v = elev.obstruction();
        if prev != v {
            ch.send(v).unwrap();
            prev = v;
        }
        thread::sleep(period)
    }
}

#[doc(hidden)]
pub fn obstruction(elev: elev::Elevator, ch: cbc::Sender<bool>, period: time::Duration) {
    let mut prev = false;
    loop {
        let v = elev.stop_button();
        if prev != v {
            ch.send(v).unwrap();
            prev = v;
        }
        thread::sleep(period)
    }
}