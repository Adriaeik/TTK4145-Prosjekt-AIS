use tokio::sync::watch;

use crate::{elevio::elev::Elevator, world_view};


pub fn set_lights(wv_watch_rx: watch::Receiver<Vec<u8>>, e: Elevator) {

}



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
fn set_hall_lights(wv: Vec<u8>, e: Elevator) {
    let wv_deser = world_view::serial::deserialize_worldview(&wv);

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

/// The function sets the cab light on last_floor_sensor
fn set_cab_light(e: Elevator, last_floor: u8) {
    e.floor_indicator(last_floor);
}

/// The function sets the door open light on
fn set_door_open_light(e: Elevator) {
    e.door_light(true);
}

/// The function sets the door open light off
fn clear_door_open_light(e: Elevator) {
    e.door_light(false);
}

/// The function sets the stop button light on
fn set_stop_button_light(e: Elevator) {
    e.stop_button_light(true);
}

/// The function sets the stop button light off
fn clear_stop_button_light(e: Elevator) {
    e.stop_button_light(false);
}