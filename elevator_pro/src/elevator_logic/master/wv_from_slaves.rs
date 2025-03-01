use crate::world_view::world_view::{self, ElevatorContainer};
use std::collections::HashSet;


pub async fn update_statuses(deser_wv: &mut world_view::WorldView, container: &ElevatorContainer, i: usize) {
    deser_wv.elevator_containers[i].door_open = container.door_open;
    deser_wv.elevator_containers[i].last_floor_sensor = container.last_floor_sensor;
    deser_wv.elevator_containers[i].obstruction = container.obstruction;
    deser_wv.elevator_containers[i].motor_dir = container.motor_dir;
    deser_wv.elevator_containers[i].calls = container.calls.clone(); 
}


pub async fn update_call_buttons(deser_wv: &mut world_view::WorldView, container: &ElevatorContainer, i: usize) {
    // Sett opp et HashSet for å sjekke for duplikater
    let mut seen = HashSet::new();
    
    // Legg til eksisterende elementer i HashSet
    for &elem in &deser_wv.outside_button.clone() {
        seen.insert(elem);
    }

    // Utvid outside_button med elementer som ikke er i HashSet
    for &call in &container.calls {
        if !seen.contains(&call) {
            deser_wv.outside_button.push(call);
            seen.insert(call.clone());
        }
    }
}


pub async fn update_tasks() {

}