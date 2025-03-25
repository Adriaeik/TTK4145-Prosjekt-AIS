//! ## Elevator I/O module for the local elevator
//! 
//! This module is mostly consisting of handed out resources, but some functionality were added.
//! The handed out functionality is placed in the submodules [`elev`] and [`poll`].
//! 
//! Additional functionality includes message handling for elevator events, 
//! call button state management, and conversion utilities for call types.
//! 
//! ## Overview
//! This module provides data structures and utilities for handling elevator 
//! input/output operations. It includes:
//! 
//! - `ElevMsgType`: Enum representing different elevator events.
//! - `ElevMessage`: Struct for wrapping elevator messages.
//! - `CallType`: Enum for representing call button types.
//! - `CallButton`: Struct for representing call button presses, including 
//!    floor, call type, and elevator ID.
//! 
//! These components allow structured handling of elevator input events, ensuring 
//! that different types of messages (such as button presses and sensor activations) 
//! are processed in a uniform manner.

#[doc(hidden)]
pub mod elev;
pub mod poll;

use crate::print;
use crate::config;

use serde::{Serialize, Deserialize};
use std::hash::{Hash, Hasher};


/// Represents different types of elevator messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElevMsgType {
    /// Call button press event.
    CALLBTN,
    /// Floor sensor event.
    FLOORSENS,
    /// Stop button press event.
    STOPBTN,
    /// Obstruction detected event.
    OBSTRX,
}

/// Represents a message related to elevator events.
#[derive(Debug, Clone)]
pub struct ElevMessage {
    /// The type of elevator message.
    pub msg_type: ElevMsgType,
    /// Optional call button information, if applicable.
    pub call_button: Option<CallButton>,
    /// Optional floor sensor reading, indicating the current floor.
    pub floor_sensor: Option<u8>,
    /// Optional stop button state (`true` if pressed).
    pub stop_button: Option<bool>,
    /// Optional obstruction status (`true` if obstruction detected).
    pub obstruction: Option<bool>,
}

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

