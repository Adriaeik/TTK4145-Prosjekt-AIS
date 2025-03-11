//! Module to calculate cost based on elevator-states, and delegate task to available elevator with lowest cost


// Regn med vi får inn array med (ID, State, Floor, Task), og et array med udelegerte tasks, `ID`: elevID, `State`: UP/DOWN/IDLE/DOOR_OPEN/ERROR, `Floor`: Floor, `Task`: Some(Task). Bør vite hvor mange etasjer heisen kan gå til. 

use std::collections::HashMap;
use std::thread::sleep;
use std::time::{Duration, Instant};
use crate::elevio::poll::CallButton;
use crate::network::local_network::{LocalChannels};
use crate::world_view::world_view::{deserialize_elev_container, ElevatorContainer, ElevatorStatus};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use crate::{config, utils};
use crate::elevio::poll::CallType;
use tokio::sync::Mutex;
use std::sync::Arc;

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
    let elevators: Arc<RwLock<HashMap<u8, ElevatorState>>> = Arc::new(RwLock::new(HashMap::new()));
    let tasks: Arc<Mutex<Vec<Task>>> = Arc::new(Mutex::new(Vec::new()));
    let mut cost_map: CostMap = CostMap::new();
    
    let tasks_clone = tasks.clone(); 
    let elevator_clone = elevators.clone();

    // Lager egen task å lese container-meldinger, så den leser fort nok (cost fcn tar litt tid)
    let chs_clone = chs.clone();
    tokio::spawn(async move {
        let mut wv = utils::get_wv(chs_clone.clone());
        let mut task_id: u16 = 0;
        loop {
            sleep(Duration::from_millis(1));
            // println!("Meldinger i kø: {}", container_ch.len());
            match container_ch.try_recv() {
                Ok(cont_ser) => {
                    // println!("Fikk melding fra slave heis");
                    let mut elevators_unlocked = elevator_clone.write().await;
                    let mut new_tasks = update_elevator(&mut elevators_unlocked, deserialize_elev_container(&cont_ser), &mut task_id); // Oppdater states
                    
                    // Gå gjennom elevators. hvis de er idle, fjern tasken dems fra tasks
                    // Gå gjennom heiser. Hvis de er IDLE, fjern deres `current_task`
                    let mut tasks_to_remove = Vec::new();
                    for (_, elev) in elevators_unlocked.iter_mut().filter(|(_, e)| e.state == ElevatorStatus::IDLE) {
                        if let Some(task) = elev.current_task.clone() {
                            tasks_to_remove.push(task);
                            let _ = chs_clone.mpscs.txs.new_task.send((elev.id, None)).await;
                        }
                    }

                    
                    // Sjekk for heisar som har "timea ut" og samle opp oppgåver dei har
                    let mut dead_tasks = detect_dead_elevators(&mut elevators_unlocked, config::TASK_TIMEOUT);
                    new_tasks.append(&mut dead_tasks);
                    let mut tasks_locked = tasks_clone.lock().await;
                    tasks_locked.retain(|t| !tasks_to_remove.iter().any(|ot| ot.id == t.id));
                    tasks_locked.append(&mut new_tasks);

                    //Hvis vi er master, send tasks til worldview
                    utils::update_wv(chs_clone.clone(), &mut wv).await;
                    if utils::is_master(wv.clone()) {
                        let _ = chs_clone.mpscs.txs.pending_tasks.send(tasks_locked.clone()).await;
                    }
                },
                Err(_) => {},
            }
        }
    });


    let mut tasks_vec_copy: Vec<Task>;
    let mut elevator_copy: HashMap<u8, ElevatorState>;
    let mut active_task_ids: HashMap<u8, u16> = HashMap::new();
    loop {
        {
            tasks_vec_copy = tasks.lock().await.clone();
        }
        {
            elevator_copy = elevators.read().await.clone();
        }


         // Oppdater kostkartet med den noverande tilstanden til heisar og udelegerte oppgåver
        update_cost_map(&mut cost_map, elevator_copy.clone(), tasks_vec_copy.clone());
        // For kvar heis, finn oppgåva med lågast kostnad og deleger den
        for (id, elevator) in elevator_copy.iter_mut() {
            if let Some(task_costs) = cost_map.get(id) {
                if let Some((best_task, _best_cost)) = task_costs.iter()
                /*.filter(|(task, _)| {
                        // Behold task hvis den ikke er i active_task_ids,
                        // eller task.id er lik den nåværende heisens id
                        !active_task_ids.values().any(|assigned_task_id| *assigned_task_id == task.id) || active_task_ids.get(id) == Some(&task.id)
                })*/
                .min_by(|a, b| {
                    a.1.cmp(&b.1)
                }) {
                    // Her kan du sende oppgåva til heisa via ein kanal.
                    // Eksempel:
                    // send_task_to_elevator(*id, best_task.clone());
                    elevator.current_task = Some(best_task.clone());
                    // println!("Best task for ID {} is {:?}", *id, best_task.clone());
                    let _ = chs.mpscs.txs.new_task.send((*id, Some(best_task.clone()))).await;
                    active_task_ids.insert(*id, best_task.clone().id);
                        
                    // Viss nødvendig: Fjern oppgåva frå den globale lista for å unngå at den vert delegert fleire gonger.
                }
            }
        }
        sleep(Duration::from_millis(20));
    }
}

/// Får inn hashmap med alle elevatorstates, oppdaterer statuser basert på elevcontainer mottat på TCP
/// Oppdaterer også tiden vi sist hørte fra den
fn update_elevator(elevators: &mut HashMap<u8, ElevatorState>, elevator_container: ElevatorContainer, task_id: &mut u16) -> Vec<Task> {
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

    let mut new_tasks = Vec::new();
    for call in elevator_container.calls {
        *task_id = (*task_id % (u16::MAX - u8::MAX as u16)) + 1;
        // print!("Knapp: {:?}", call);
        let task = Task { id: *task_id, call: call};
        new_tasks.push(task.clone());
        // println!("    Tilsvarende task: {:?}", task.clone());
    }
    new_tasks
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
        cost += config::BUSY_PENALTY; // Straff for at heisa er opptatt
    }
    
    // For kall som kjem frå utsida, sjekk om heisa er på veg i rett retning.
    if task.call.call_type != CallType::INSIDE {
        if !is_moving_toward(elevator, &task.call) {
            cost += config::WRONG_DIRECTION_PENALTY; // Straff for feil retning
        }
    }
    // println!("{}", cost);
    cost
}

/// Hjelpefunksjon som avgjer om heisa er på veg mot kalla si etasje.
/// - For heisar som er på veg oppover, bør kalla ligge same eller over heisens etasje.
/// - For heisar på veg nedover, bør kalla ligge same eller under heisens etasje.
/// - Heisar i IDLE- eller DOOR_OPEN-status blir rekna som at dei kan endre retning utan ekstra straff.
fn is_moving_toward(elevator: &ElevatorState, call: &CallButton) -> bool {
    // println!("Status: {:?}", elevator.state);
    match elevator.state {
        ElevatorStatus::UP    => call.floor > elevator.floor,
        ElevatorStatus::DOWN  => call.floor < elevator.floor,
        ElevatorStatus::IDLE | ElevatorStatus::DOOR_OPEN => true,
        ElevatorStatus::ERROR => false,
        // Eventuelle andre statusar: vi tek ein konservativ tilnærming.
        _ => true,
    }
}


