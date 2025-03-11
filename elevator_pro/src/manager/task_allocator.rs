//! Module to calculate cost based on elevator-states, and delegate task to available elevator with lowest cost



// Regn med vi får inn array med (ID, State, Floor, Task), og et array med udelegerte tasks, `ID`: elevID, `State`: UP/DOWN/IDLE/DOOR_OPEN/ERROR, `Floor`: Floor, `Task`: Some(Task). Bør vite hvor mange etasjer heisen kan gå til. 

use std::collections::HashMap;
use std::time::Instant;

use crate::network::local_network::LocalChannels;
use crate::world_view::world_view::ElevatorContainer;

struct ElevatorState {
    id: u8,
    floor: i16,
    max_floors: i16,
    state: ElevatorStatus,
    current_task: Option<Task>,
    last_updated: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ElevatorStatus {
    Up,
    Down,
    Idle,
    DoorOpen,
    Error,
}

#[derive(Debug, Clone, Copy)]
struct Task {
    floor: i16,
    direction: CallDirection,
}

#[derive(Debug, Clone, Copy)]
enum CallDirection {
    Up,
    Down,
}

type CostMap = HashMap<u8, Vec<(Task, u32)>>; // ElevatorID -> List of (Task, Cost)



pub fn delegate_tasks(chs: LocalChannels) {
    let mut elevators: HashMap<u32, ElevatorState> = HashMap::new();
    let mut tasks: Vec<Task> = Vec::new();
    let cost_map: CostMap;

    loop {
        let elev_msg: ElevatorContainer; // Elevatorcontainer den får på kanal fra update_wv
        update_elevator(elevators, elev_msg); // Oppdater states
        let dead_tasks = detect_dead_elevators(&mut elevators, config::TASK_TIMEOUT);
        tasks.append(&mut dead_tasks).await;
        update_cost_map(&mut cost_map, elevators, tasks);

        for (&id, elevator) in elevators.iter().filter(|(_, e)| e.state == ElevatorStatus::Idle) {
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
        state: elevator_container.state,
        current_task: elevator_container.task,
        last_updated: Instant::now(),
    });

    entry.floor = elevator_container.last_floor_sensor;
    entry.state = elevator_container.state;
    entry.current_task = elevator_container.task;

    // Denne skal ta tiden på en task. Så oppdater den hvis status er Up/Down og den nye staten er ulik
    entry.last_updated = Instant::now();
}


fn detect_dead_elevators(elevators: &mut HashMap<u32, ElevatorState>, timeout: u64) -> Vec<Task> {
    let now = Instant::now();
    let mut to_reassign = Vec::new();

    for (&id, elevator) in &*elevators {
        // Hvis det er timeout siden forrige oppdatering: Anse tasken som feila, legg den til i tasks som skal omdirigeres
        if now.duration_since(elevator.last_updated).as_secs() > timeout {
            println!("⚠️ Heis {} anses som død. Omfordeler oppgaver!", id);
            if let Some(task) = &elevator.current_task {
                to_reassign.push(task);
            }
        }
    }

    // Fjern heisen fra lokal liste med heiser om den timeouter
    elevators.retain(|_, e| now.duration_since(e.last_updated).as_secs() <= timeout);

    to_reassign
}

