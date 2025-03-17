
// Regn med vi får inn array med (ID, State, Floor, Task), og et array med udelegerte tasks, `ID`: elevID, `State`: UP/DOWN/IDLE/DOOR_OPEN/ERROR, `Floor`: Floor, `Task`: Some(Task). Bør vite hvor mange etasjer heisen kan gå til. 

use std::collections::HashMap;
use std::thread::sleep;
use std::time::{Duration, Instant};
use crate::elevio::poll::CallButton;
use crate::network::local_network::LocalChannels;
use crate::world_view::world_view::{self, deserialize_elev_container, deserialize_worldview, ElevatorContainer, ElevatorStatus};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use crate::{config, print, ip_help_functions};
use crate::elevio::poll::CallType;
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
/// Represents an elevators and its state in the allocator
pub struct ElevatorState {
    /// The elevators ID
    id: u8,

    /// The last floor the elevator was at
    floor: u8,

    /// The highest floor the elevator can go to
    max_floors: u8,

    /// The state the elevator can be in, see [ElevatorStatus]
    state: ElevatorStatus,

    /// Option of the current task the elevator is handling. None if the elevator is idle
    current_task: Option<Task>,

    /// Last instant the elevators task was updated. Used to detect timeouts 
    last_updated: Instant,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash)]
/// The struct for a task
pub struct Task {
    /// The ID of the task
    pub id: u16,
    /// The call leading to the task, see [CallButton]
    pub call: CallButton,
}

/// Mapping of an elevator ID to a vector of tasks and their cost value
type CostMap = HashMap<u8, Vec<(Task, u32)>>; 



pub async fn delegate_tasks(chs: LocalChannels, mut container_ch: mpsc::Receiver::<Vec<u8>>) {
    let elevators: Arc<RwLock<HashMap<u8, ElevatorState>>> = Arc::new(RwLock::new(HashMap::new()));
    let tasks: Arc<Mutex<Vec<Task>>> = Arc::new(Mutex::new(Vec::new()));
    let mut cost_map: CostMap = CostMap::new();
    
    let tasks_clone = tasks.clone(); 
    let elevator_clone = elevators.clone();

    // Lager egen task å lese container-meldinger, så den leser fort nok (cost fcn tar litt tid)
    let chs_clone = chs.clone();
    tokio::spawn(async move {
        let mut wv = world_view::get_wv(chs_clone.clone());
        let mut task_id: u16 = 0;
        let mut read_slave = false;
        loop {
            world_view::update_wv(chs_clone.clone(), &mut wv).await;
            if world_view::is_master(wv.clone()) && read_slave {
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
                                if task.call.floor == elev.floor {
                                    tasks_to_remove.push(task);
                                    let _ = chs_clone.mpscs.txs.new_task.send((elev.id, None)).await;
                                }
                            }
                        }

                        
                        // Sjekk for heisar som har "timea ut" og samle opp oppgåver dei har
                        let mut dead_tasks = detect_dead_elevators(&mut elevators_unlocked, config::TASK_TIMEOUT);
                        new_tasks.append(&mut dead_tasks);
                        let mut tasks_locked = tasks_clone.lock().await;
                        tasks_locked.retain(|t| !tasks_to_remove.iter().any(|ot| ot.id == t.id));
                        tasks_locked.append(&mut new_tasks);

                        let _ = chs_clone.mpscs.txs.pending_tasks.send(tasks_locked.clone()).await;
                    },
                    Err(_) => {},
                }
            } else {
                let wv_deser = deserialize_worldview(&wv.clone());
                let mut tasks_locked = tasks_clone.lock().await;
                *tasks_locked = wv_deser.pending_tasks;
                read_slave = true;
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
                .filter(|(task, _)| {
                        // Behold task hvis den ikke er i active_task_ids,
                        // eller task.id er lik den nåværende heisens id
                        !active_task_ids.values().any(|assigned_task_id| *assigned_task_id == task.id) || active_task_ids.get(id) == Some(&task.id)
                })
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


/// **Updates the elevator states in the allocator based on an elevator container**
/// 
/// ## Parameters:
/// `elevators`: Mutable reference to the hashmap of <elevator IDs, ElevatorStates>  
/// `elevator_container`: The new elevator container recieved (from slave on TCP)  
/// `task_id`: Mutable reference to the last used task_id, used to generate a unique task ID for any new tasks generated
/// 
/// ## Steps:
/// - Looks for any elevator in the hashmap with matching ID to the elevator container, if none is found, it inserts a new one
/// - Update floor, state and current task of the elevator
/// - **TODO:** Updates time if the task has changed
/// - Generates new tasks based on buttoncalls in the elevator container
/// 
/// ## Returns:
/// - A vector of any generated tasks 
/// 
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


/// **Detects 'dead' elevators and returns their current Task**
/// 
/// ## Parameters:
/// `elevators`: Mutable reference to the hashmap of <elevator IDs, ElevatorStates>
/// `timeout`: An u64 representing the duration before timeout in seconds
/// 
/// ## Steps:
/// - Iterates through elevators in the hashmap
/// - If any elevator has reached a timeout, push its current_task to a new vector of tasks which is returned after all elevators has been itered
/// - Removes any elevators wich reached timeout from the hashmap
/// 
/// ## Returns:
/// - A vector containing the ongoing tasks of 'dead' elevators
///   
fn detect_dead_elevators(elevators: &mut HashMap<u8, ElevatorState>, timeout: u64) -> Vec<Task> {
    let now = Instant::now();
    let mut to_reassign: Vec<Task> = Vec::new();

    for (&id, elevator) in &*elevators {
        // Hvis det er timeout siden forrige oppdatering: Anse tasken som feila, legg den til i tasks som skal omdirigeres
        if now.duration_since(elevator.last_updated).as_secs() > timeout {
            print::warn(format!("Elevator {} took too long, reallocating its task!", id));
            if let Some(task) = &elevator.current_task {
                to_reassign.push(task.clone());
            }
        }
    }

    // Fjern heisen fra lokal liste med heiser om den timeouter
    elevators.retain(|_, e| now.duration_since(e.last_updated).as_secs() <= timeout);

    to_reassign
}


/// Updates the costmap with costs for every combination of elevator and task
/// 
/// ## Parameters:
/// `cost_map`: A mutable reference to the costmap to be updated  
/// `elevators`: Mutable reference to the hashmap of <elevator IDs, ElevatorStates>  
/// `tasks`: Vector of pending tasks in the system
/// 
/// ## Steps:
/// - Clear the cost_map
/// - Iterate through every elevator
/// - For each elevator, iterate through every task, calculate its cost and insert it in the costmap
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


/// **Help function to decide if an elevator is moving towards a call's floor**
/// - For elevators going upwards, the call should be abowe the elevators last floor.
/// - For elevators going downwards, the call should be bellow the elevators last floor.
/// - Elevators in IDLE- or DOOR_OPEN-status returns true: could go any direction without extra cost
/// 
/// ## Parameters:
/// `elevator`: A reference to an [ElevatorState]  
/// `call`: A reference to a [CallButton]
/// 
/// ## Steps:
/// - match the elevators [ElevatorStatus]
/// - decide if the elevator is moving towards the call (see source code for clearer documentation) 
/// 
/// ## Returns:
/// A bool. True if the elevator is moving towards the call, false otherwise 
fn is_moving_toward(elevator: &ElevatorState, call: &CallButton) -> bool {
    match elevator.state {
        ElevatorStatus::UP    => call.floor > elevator.floor,
        ElevatorStatus::DOWN  => call.floor < elevator.floor,
        ElevatorStatus::IDLE | ElevatorStatus::DOOR_OPEN => true,
        ElevatorStatus::ERROR => false,
        // Eventuelle andre statusar: vi tek ein konservativ tilnærming.
        _ => true,
    }
}


