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
use crate::elevio::poll::CallType;

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
        // Sjekk for heisar som har "timea ut" og samle opp oppgåver dei har
        let mut dead_tasks = detect_dead_elevators(&mut elevators, config::TASK_TIMEOUT);
        tasks.append(&mut dead_tasks);

         // Oppdater kostkartet med den noverande tilstanden til heisar og udelegerte oppgåver
        update_cost_map(&mut cost_map, elevators.clone(), tasks.clone());

        // For kvar heis, finn oppgåva med lågast kostnad og deleger den
        for (id, elevator) in elevators.iter_mut() {
            if let Some(task_costs) = cost_map.get(id) {
                if let Some((best_task, _best_cost)) = task_costs.iter().min_by(|a, b| {
                    a.1.cmp(&b.1)
                }) {
                    // Her kan du sende oppgåva til heisa via ein kanal.
                    // Eksempel:
                    // send_task_to_elevator(*id, best_task.clone());
                    elevator.current_task = Some(best_task.clone());
                    
                    // Viss nødvendig: Fjern oppgåva frå den globale lista for å unngå at den vert delegert fleire gonger.
                }
            }
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


/// Oppdaterer kostkartet med kostnader for kvar kombinasjon av heis og oppgåve.
fn update_cost_map(cost_map: &mut CostMap, elevators: HashMap<u8, ElevatorState>, tasks: Vec<Task>) {
    cost_map.clear();
    for (id, elevator) in elevators.iter() {
         let mut task_costs = Vec::new();
         for task in tasks.iter() {
              let cost = compute_cost(elevator, task);
              task_costs.push((task.clone(), cost));
         }
         cost_map.insert(*id, task_costs);
    }
}

/// Reknar ut kostnaden for ein oppgåve basert på heisens tilstand og retning.
/// Dersom oppgåva er eit INSIDE-kall og heis-ID ikkje stemmer, returner u32::MAX.
fn compute_cost(elevator: &ElevatorState, task: &Task) -> u32 {
    // INSIDE-kall skal berre behandlast av den heisa som sende kalla.
    if task.call.call_type == CallType::INSIDE && task.call.elev_id != elevator.id {
        return u32::MAX;
    }
    
    // Dersom heisa er i ein ERROR-status, skal den ikkje bli brukt.
    if let ElevatorStatus::ERROR = elevator.state {
        return u32::MAX;
    }
    
    // Grunnkostnaden er basert på avstanden mellom heisens noverande etasje og kalla si etasje.
    let distance = if elevator.floor > task.call.floor {
        elevator.floor - task.call.floor
    } else {
        task.call.floor - elevator.floor
    };
    let mut cost = distance as u32;
    
    // Dersom heisa allereie har ein oppgåve, legg på ein ekstra straff.
    if elevator.current_task.is_some() {
        cost += 5; // Straff for at heisa er opptatt
    }
    
    // For kall som kjem frå utsida, sjekk om heisa er på veg i rett retning.
    if task.call.call_type != CallType::INSIDE {
        if !is_moving_toward(elevator, &task.call) {
            cost += 10; // Straff for feil retning
        }
    }
    
    cost
}

/// Hjelpefunksjon som avgjer om heisa er på veg mot kalla si etasje.
/// - For heisar som er på veg oppover, bør kalla ligge same eller over heisens etasje.
/// - For heisar på veg nedover, bør kalla ligge same eller under heisens etasje.
/// - Heisar i IDLE- eller DOOR_OPEN-status blir rekna som at dei kan endre retning utan ekstra straff.
fn is_moving_toward(elevator: &ElevatorState, call: &CallButton) -> bool {
    match elevator.state {
        ElevatorStatus::UP    => call.floor >= elevator.floor,
        ElevatorStatus::DOWN  => call.floor <= elevator.floor,
        ElevatorStatus::IDLE | ElevatorStatus::DOOR_OPEN => true,
        ElevatorStatus::ERROR => false,
        // Eventuelle andre statusar: vi tek ein konservativ tilnærming.
        _ => true,
    }
}
