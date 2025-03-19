use tokio::sync::watch;

use crate::{elevio::elev::Elevator, world_view::{self, ElevatorContainer}};


/// Sets all hall lights
/// 
/// ## Parameters
/// `wv`: Serialized worldview
/// `e`: Elevator instance
/// 
/// ## Behavior:
/// The function goes through all hall requests in the worldview, and sets hall lights if the corresponding lights on/off based on the boolean value in the worldview.   
/// The function skips any hall lights on floors grater than the elevators num_floors, as well as down on floor nr. 0 and up on floor nr. e.num_floors 
/// 
/// ## Note
/// The function only sets the lights once per call, and needs to be recalled every time the lights needs to be updated
pub fn set_hall_lights(wv: Vec<u8>, e: Elevator, container: &ElevatorContainer) {
    let wv_deser = world_view::serial::deserialize_worldview(&wv);

    for (i, on) in container.cab_requests.iter().enumerate() {
        e.floor_indicator(i as u8, *on);
    }

    for (i, [up, down]) in wv_deser.hall_request.iter().enumerate() {
        let floor = i as u8;
        if floor > e.num_floors {
            break;
        }
    
        if floor != 0 {
            e.call_button_light(floor, 1, *down);
        }
        if floor != e.num_floors {
            e.call_button_light(floor, 0, *up);
        }
    }
}

/// The function sets the door open light on
pub fn set_door_open_light(e: Elevator) {
    e.door_light(true);
}

/// The function sets the door open light off
pub fn clear_door_open_light(e: Elevator) {
    e.door_light(false);
}

/// The function sets the stop button light on
pub fn set_stop_button_light(e: Elevator) {
    e.stop_button_light(true);
}

/// The function sets the stop button light off
pub fn clear_stop_button_light(e: Elevator) {
    e.stop_button_light(false);
}