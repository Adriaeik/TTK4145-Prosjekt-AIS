
use std::{thread::sleep, time::Duration};

use crate::{elevio::poll::{CallButton, CallType}, network::local_network, utils, world_view::world_view::{self, ElevatorContainer, Task, TaskStatus}};


struct Orders {
    task: Vec<Task>,
}


pub async fn distribute_task(chs: local_network::LocalChannels) {
    let mut i: u16 = 0;
    let mut wv = utils::get_wv(chs.clone());
    let mut wv_deser = world_view::deserialize_worldview(&wv);
    let mut prev_button_0 = CallButton{call: CallType::from(69), floor: 255, elev_id: 255}; 

    loop {
        utils::update_wv(chs.clone(), &mut wv).await;
        
        while utils::is_master(wv.clone()) {
            utils::update_wv(chs.clone(), &mut wv).await;
            wv_deser = world_view::deserialize_worldview(&wv);
            let buttons = wv_deser.outside_button;
            
            if !buttons.is_empty() && buttons[0] != prev_button_0 {
                let task = create_task(buttons[0], i);
                i = (i % (u16::MAX - 1000)) + 1;
                let (mut lowest_cost, mut id) = (i32::MAX, 0);
    
                for elev in wv_deser.elevator_containers.iter() {
                    let cost = calculate_cost(task.clone(), elev.clone());
                    if cost < lowest_cost {
                        lowest_cost = cost;
                        id = elev.elevator_id;
                    }
                }
                let _ = chs.mpscs.txs.new_task.send((task, id, buttons[0])).await;
                println!("Antall knapper: {}", buttons.len());
                prev_button_0 = buttons[0];   
            }
        }
        
        sleep(Duration::from_millis(100));
    }
}


fn create_task(button: CallButton, task_id: u16) -> Task {
    Task { id: task_id, to_do: button.floor, status: TaskStatus::PENDING, is_inside: false }
}

fn calculate_cost(task: Task, elev: ElevatorContainer) -> i32 {
    elev.tasks.len() as i32
}


// fn optimze_active_tasks()






















// -----------------------------------------------------------------------------
// Kalkulerer ein "kostnad" for kor godt ein heis kan ta imot eit eksternt kall
// -----------------------------------------------------------------------------
/* 
fn kalkuler_kostnad(elev: &ElevatorStatus, call: &CallButton) -> u32 {
    // Basiskostnad er avstanden i etasjar
    let diff = if elev.current_floor > call.floor {
        elev.current_floor - call.floor
    } else {
        call.floor - elev.current_floor
    } as u32;
    let mut kostnad = diff;
    
    // Legg til ekstra kostnad dersom heisens retning ikkje stemmer med kallretninga
    match (elev.direction, call.call) {
        // Om heisen køyrer opp og kall er UP, og heisen er under kall-etasjen
        (Direction::Up, CallType::UP) if elev.current_floor <= call.floor => { }
        // Om heisen køyrer ned og kall er DOWN, og heisen er over kall-etasjen
        (Direction::Down, CallType::DOWN) if elev.current_floor >= call.floor => { }
        // Om heisen er idle er det optimalt
        (Direction::Idle, _) => { }
        // I alle andre tilfelle legg til ein straff
        _ => {
            kostnad += 100;
        }
    }
    
    // Legg til kostnad basert på talet på allereie tildelte oppgåver
    kostnad += (elev.tasks.len() as u32) * 10;
    
    kostnad
}

// -----------------------------------------------------------------------------
// Funksjon som tildeler ein oppgåve til rett heis
//
// - For INSIDE kall: finn heisen med samsvarande elev_id (forutsatt at han ikkje er offline).
// - For eksterne kall (UP/DOWN): vel heisen med lågaste kostnad.
// -----------------------------------------------------------------------------
pub fn tildele_oppgave(elevators: &[ElevatorStatus], call: CallButton) -> Option<u8> {
    // Dersom kalltypen er INSIDE, skal oppgåva gå til den spesifikke heisen
    if call.call == CallType::INSIDE {
        return elevators.iter()
        .find(|e| e.elevator_id == call.elev_id && !e.offline)
        .map(|e| e.elevator_id);
}

// For eksterne kall: iterer gjennom alle heisar som ikkje er offline
let mut beste_id = None;
let mut beste_kostnad = u32::MAX;

for elev in elevators.iter().filter(|e| !e.offline) {
    let kost = kalkuler_kostnad(elev, &call);
    if kost < beste_kostnad {
        beste_kostnad = kost;
        beste_id = Some(elev.elevator_id);
    }
}

beste_id
}

*/

