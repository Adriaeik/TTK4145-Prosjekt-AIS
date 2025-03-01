use crate::world_view::world_view::{self, ElevatorContainer, TaskStatus};

pub fn remove_completed_tasks(tasks: &mut Vec<world_view::Task>, completed_tasks: Vec<u16>) {
    tasks.retain(|task| !completed_tasks.contains(&task.id));
}

pub async fn update_tasks(container_deser: world_view::ElevatorContainer, wv: &mut Vec<u8>, container_idx: usize) {
    let mut wv_deser = world_view::deserialize_worldview(&wv);

    let mut completed_tasks: Vec<u16> = Vec::new();
    
    for task in container_deser.tasks_status {     
        match task.status {
            TaskStatus::DONE => {completed_tasks.push(task.id);},
            TaskStatus::UNABLE => {
                //TODO: oppdater funksjonen som sorterer 
            },
            TaskStatus::PENDING => {},
        }
    }
    remove_completed_tasks(&mut wv_deser.elevator_containers[container_idx].tasks, completed_tasks);  

    *wv = world_view::serialize_worldview(&wv_deser);
}