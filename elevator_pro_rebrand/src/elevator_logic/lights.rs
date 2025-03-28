//! Elevator Lights Module
//!
//! This module provides utility functions for controlling the elevator's indicator lights.
//!
//! It includes functionality to set:
//! - Hall request lights (`set_hall_lights`)
//! - Cab floor indicator (`set_cab_light`)
//! - Door open light (`set_door_open_light`, `clear_door_open_light`)
//! - Stop button light (`set_stop_button_light`, `clear_stop_button_light`)
//!
//! These lights are updated based on the current elevator state and serialized worldview,
//! and must be explicitly set on each update cycle.
//!
//! # Example
//! ```rust,no_run
//! let e: Elevator = ...;
//! let wv: Vec<u8> = get_serialized_worldview();
//! lights::set_hall_lights(wv, e.clone());
//! lights::set_cab_light(e.clone(), 2);
//! ```


use crate::elevator_logic::ElevatorBehaviour;
use crate::elevio::elev::Elevator; 
use crate::world_view::ElevatorContainer;
use crate::world_view::WorldView;


/// Sets all hall lights (this includes the door light)
/// 
/// ## Parameters
/// `wv`: Serialized worldview
/// `e`: Elevator instance
/// 
/// ## Behavior:
/// The function goes through all hall requests in the worldview, and sets hall lights if the corresponding lights on/off based on the boolean value in the worldview.   
/// The function skips any hall lights on floors grater than the elevators num_floors, as well as down on floor nr. 0 and up on floor nr. e.num_floors 
/// The function sets/clears the doorlight based on the elevators behaviour
/// 
/// ## Note
/// The function only sets the lights once per call, and should therefore be called continiously
pub fn set_hall_lights(
    wv: &WorldView, 
    e: Elevator, 
    self_container: &ElevatorContainer
) 
{
    for (i, [up, down]) in wv.hall_request.iter().enumerate() 
    {
        let floor = i as u8;
        if floor > e.num_floors {break;}
    
        e.call_button_light(floor, 2, self_container.cab_requests[i]);
        if floor != 0 
        {
            e.call_button_light(floor, 1, *down);
        }
        if floor != e.num_floors 
        {
            e.call_button_light(floor, 0, *up);
        }
    }

    if self_container.behaviour == ElevatorBehaviour::DoorOpen || self_container.behaviour == ElevatorBehaviour::ObstructionError 
    {
        set_door_open_light(e.clone());
    } else 
    {
        clear_door_open_light(e.clone());
    }
}

/// The function sets the cab light on last_floor_sensor
pub fn set_cab_light(
    e: Elevator, 
    last_floor: u8
) 
{
    e.floor_indicator(last_floor);
}

/// The function sets the door open light on
pub fn set_door_open_light(
    e: Elevator
) 
{
    e.door_light(true);
}

/// The function sets the door open light off
pub fn clear_door_open_light(
    e: Elevator
) 
{
    e.door_light(false);
}

/// The function sets the stop button light on
pub fn set_stop_button_light(
    e: Elevator
) 
{
    e.stop_button_light(true);
}

/// The function sets the stop button light off
pub fn clear_stop_button_light(
    e: Elevator
) 
{
    e.stop_button_light(false);
}