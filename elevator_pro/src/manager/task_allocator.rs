//! Module to calculate cost based on elevator-states, and delegate task to available elevator with lowest cost


// Regn med vi får inn array med (ID, State, Floor, Task), og et array med udelegerte tasks, `ID`: elevID, `State`: UP/DOWN/IDLE/DOOR_OPEN/ERROR, `Floor`: Floor, `Task`: Some(Task). Bør vite hvor mange etasjer heisen kan gå til. 

use std::collections::HashMap;
use std::time::Instant;
use crate::elevio::poll::CallButton;
use crate::network::local_network::{LocalChannels};
use crate::world_view::world_view::{deserialize_elev_container, ElevatorContainer, ElevatorStatus};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use crate::config;

#[derive(Debug, Clone)]
pub struct ElevatorState {
    id: u8,
    floor: u8,
    max_floors: u8,
    state: ElevatorStatus,
    current_task: Option<Task>,
    last_updated: Instant,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash)]
pub struct Task {
    pub id: u16,
    pub call: CallButton,
}

type CostMap = HashMap<u8, Vec<(Task, u32)>>; // ElevatorID -> List of (Task, Cost)



pub async fn delegate_tasks(chs: LocalChannels, mut container_ch: mpsc::Receiver::<Vec<u8>>) {
    let mut elevators: HashMap<u8, ElevatorState> = HashMap::new();
    let mut tasks: Vec<Task> = Vec::new();
    let mut cost_map: CostMap = CostMap::new();
    
    loop {
        match container_ch.try_recv() {
            Ok(cont_ser) => {
                update_elevator(&mut elevators, deserialize_elev_container(&cont_ser)); // Oppdater states
            },
            Err(_) => {},
        }
        let mut dead_tasks = detect_dead_elevators(&mut elevators, config::TASK_TIMEOUT);
        tasks.append(&mut dead_tasks);
        update_cost_map(&mut cost_map, elevators.clone(), tasks.clone());

        for (id, elevator) in elevators.clone() {
            //Send task med lavest cost i cost_map tilhørende id på kanal til update_wv. Marker her at den har tasken
        }
    }
}

/// Får inn hashmap med alle elevatorstates, oppdaterer statuser basert på elevcontainer mottat på TCP
/// Oppdaterer også tiden vi sist hørte fra den
fn update_elevator(elevators: &mut HashMap<u8, ElevatorState>, elevator_container: ElevatorContainer) {
    let entry = elevators.entry(elevator_container.elevator_id).or_insert(ElevatorState {
        id: elevator_container.elevator_id,
        floor: elevator_container.last_floor_sensor,
        max_floors: elevator_container.num_floors,
        state: elevator_container.status,
        current_task: elevator_container.task.clone(),
        last_updated: Instant::now(),
    });

    entry.floor = elevator_container.last_floor_sensor;
    entry.state = elevator_container.status;
    entry.current_task = elevator_container.task.clone();

    // Denne skal ta tiden på en task. Så oppdater den hvis status er Up/Down og den nye staten er ulik
    entry.last_updated = Instant::now();
}


fn detect_dead_elevators(elevators: &mut HashMap<u8, ElevatorState>, timeout: u64) -> Vec<Task> {
    let now = Instant::now();
    let mut to_reassign: Vec<Task> = Vec::new();

    for (&id, elevator) in &*elevators {
        // Hvis det er timeout siden forrige oppdatering: Anse tasken som feila, legg den til i tasks som skal omdirigeres
        if now.duration_since(elevator.last_updated).as_secs() > timeout {
            println!("⚠️ Heis {} anses som død. Omfordeler oppgaver!", id);
            if let Some(task) = &elevator.current_task {
                to_reassign.push(task.clone());
            }
        }
    }

    // Fjern heisen fra lokal liste med heiser om den timeouter
    elevators.retain(|_, e| now.duration_since(e.last_updated).as_secs() <= timeout);

    to_reassign
}



fn update_cost_map(cost_map: &mut HashMap<u8, Vec<(Task, u32)>>, elevators: HashMap<u8, ElevatorState>, tasks: Vec<Task>) {

}
