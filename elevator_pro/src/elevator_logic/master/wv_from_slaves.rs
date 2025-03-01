use crate::world_view::world_view::{self, ElevatorContainer};



pub async fn update_statuses(deser_wv: &mut world_view::WorldView, container: &ElevatorContainer, i: usize) {
    deser_wv.elevator_containers[i].door_open = container.door_open;
    deser_wv.elevator_containers[i].last_floor_sensor = container.last_floor_sensor;
    deser_wv.elevator_containers[i].obstruction = container.obstruction;
    deser_wv.elevator_containers[i].motor_dir = container.motor_dir;
}


pub async fn update_call_buttons(deser_wv: &mut world_view::WorldView, container: &ElevatorContainer, i: usize) {
    deser_wv.outside_button.extend(container.calls.iter().cloned());
}


pub async fn update_tasks() {

}