use std::sync::atomic::Ordering;
use std::u16;
use tokio::time::sleep;

use crate::config;
use crate::elevio::poll::CallButton;
use crate::world_view::world_view;
use crate::world_view::world_view::TaskStatus;
use crate::network::tcp_network;
use crate::world_view::world_view_update;
use crate::network::local_network::{self, ElevMessage};
use std::collections::HashSet;
use crate::utils::{self, print_err, print_info, print_ok};
use crate::elevator_logic::master;

use super::world_view::Task;


/// ### Oppdatering av lokal worldview
/// 
/// Funksjonen leser nye meldinger fra andre tasks som indikerer endring i systemet, og endrer og oppdaterer det lokale worldviewen basert på dette.
pub async fn update_wv(mut main_local_chs: local_network::LocalChannels, mut worldview_serialised: Vec<u8>) {
    println!("Starter update_wv");
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
    
    let mut wv_edited_I = false;
    loop {
        //OBS: Error kommer når kanal er tom. ikke print der uten å eksplisitt eksludere channel_empty error type

/* KANALER SLAVE HOVEDSAKLIG MOTTAR PÅ */
        /*_____Fjerne knappar som vart sendt på TCP_____ */
        match main_local_chs.mpscs.rxs.sent_tcp_container.try_recv() {
            Ok(msg) => {
                wv_edited_I = clear_from_sent_tcp(&mut worldview_serialised, msg);
            },
            Err(_) => {},
        }
        /*_____Oppdater WV fra UDP-melding_____ */
        match main_local_chs.mpscs.rxs.udp_wv.try_recv() {
            Ok(master_wv) => {
                wv_edited_I = join_wv_from_udp(&mut worldview_serialised, master_wv);
            },
            Err(_) => {}, 
        }
        /*_____Signal om at tilkobling til master har feila_____ */
        match main_local_chs.mpscs.rxs.tcp_to_master_failed.try_recv() {
            Ok(_) => {
                wv_edited_I = abort_network(&mut worldview_serialised);
            },
            Err(_) => {},
        }
        
        
/* KANALER MASTER HOVEDSAKLIG MOTTAR PÅ */
        /*_____Melding til master fra slaven (elevator-containeren til slaven)_____*/
        match main_local_chs.mpscs.rxs.container.try_recv() {
            Ok(container) => {
                wv_edited_I = join_wv_from_tcp_container(&mut worldview_serialised, container).await;
            },
            Err(_) => {},
        }
        /*_____ID til slave som er død (ikke kontakt med slave)_____ */
        match main_local_chs.mpscs.rxs.remove_container.try_recv() {
            Ok(id) => {
                wv_edited_I = remove_container(&mut worldview_serialised, id); 
            },
            Err(_) => {},
        }
        match main_local_chs.mpscs.rxs.new_task.try_recv() {
            Ok((task ,id, button)) => {
                // utils::print_master(format!("Fikk task: {:?}", task));
                wv_edited_I = push_task(&mut worldview_serialised, task, id, button);
            },
            Err(_) => {},
        }
        


/* KANALER MASTER OG SLAVE MOTTAR PÅ */
        /*_____Knapper trykket på lokal heis_____ */
        match main_local_chs.mpscs.rxs.local_elev.try_recv() {
            Ok(msg) => {
                wv_edited_I = recieve_local_elevator_msg(&mut worldview_serialised, msg).await;
            },
            Err(_) => {},
        }
        /*____Får signal når en task er ferdig_____ */
        match main_local_chs.mpscs.rxs.update_task_status.try_recv() {
            Ok((id, status)) => {
                println!("Skal sette status {:?} på task id: {}", status, id);
                wv_edited_I = update_task_status(&mut worldview_serialised, id, status);
            },
            Err(_) => {},
        }

        

/* KANALER ALLE SENDER LOKAL WV PÅ */
        /*_____Hvis worldview er endra, oppdater kanalen_____ */
        if wv_edited_I {
            let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
            // println!("Sendte worldview lokalt {}", worldview_serialised[1]);
    
            wv_edited_I = false;
        }
    }
}

/// ### Oppdater WorldView fra master sin UDP melding
pub fn join_wv_from_udp(wv: &mut Vec<u8>, master_wv: Vec<u8>) -> bool {
    *wv = world_view_update::join_wv(wv.clone(), master_wv);
    true
}

/// ### 'Forlater' nettverket, fjerner alle heiser som ikke er seg selv
pub fn abort_network(wv: &mut Vec<u8>) -> bool {
    let mut deserialized_wv = world_view::deserialize_worldview(wv);
    deserialized_wv.elevator_containers.retain(|elevator| elevator.elevator_id == utils::SELF_ID.load(Ordering::SeqCst));
    deserialized_wv.set_num_elev(deserialized_wv.elevator_containers.len() as u8);
    deserialized_wv.master_id = utils::SELF_ID.load(Ordering::SeqCst);
    *wv = world_view::serialize_worldview(&deserialized_wv);
    true
}

/// ### Oppdaterer worldview basert på TCP melding fra slave 
pub async fn join_wv_from_tcp_container(wv: &mut Vec<u8>, container: Vec<u8>) -> bool {
    let deser_container = world_view::deserialize_elev_container(&container);
    let mut deserialized_wv = world_view::deserialize_worldview(&wv);

    // Hvis slaven ikke eksisterer, legg den til som den er
    if None == deserialized_wv.elevator_containers.iter().position(|x| x.elevator_id == deser_container.elevator_id) {
        deserialized_wv.add_elev(deser_container.clone());
    }

    let self_idx = world_view::get_index_to_container(deser_container.elevator_id, world_view::serialize_worldview(&deserialized_wv));
    
    if let Some(i) = self_idx {
        //Oppdater statuser + fjerner tasks som er TaskStatus::DONE
        master::wv_from_slaves::update_statuses(&mut deserialized_wv, &deser_container, i).await;
        //Oppdater call_buttons
        master::wv_from_slaves::update_call_buttons(&mut deserialized_wv, &deser_container, i).await;
        *wv = world_view::serialize_worldview(&deserialized_wv);
        return true;
    } else {
        //Hvis dette printes, finnes ikke slaven i worldview. I teorien umulig, ettersom slaven blir lagt til over hvis den ikke allerede eksisterte
        utils::print_cosmic_err("The elevator does not exist join_wv_from_tcp_conatiner()".to_string());
        return false;
    }
}

/// ### Fjerner slave basert på ID
pub fn remove_container(wv: &mut Vec<u8>, id: u8) -> bool {
    let mut deserialized_wv = world_view::deserialize_worldview(&wv);
    deserialized_wv.remove_elev(id);
    *wv = world_view::serialize_worldview(&deserialized_wv);
    true
}

/// ### Behandler meldinger fra egen heis
pub async fn recieve_local_elevator_msg(wv: &mut Vec<u8>, msg: ElevMessage) -> bool {
    let is_master = utils::is_master(wv.clone());
    let mut deserialized_wv = world_view::deserialize_worldview(&wv);
    let self_idx = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , wv.clone());

    // Matcher hvilken knapp-type som er mottat
    match msg.msg_type {
        // Callbutton -> Legg den til i calls under egen heis-container
        local_network::ElevMsgType::CBTN => {
            print_info(format!("Callbutton: {:?}", msg.call_button));
            if let (Some(i), Some(call_btn)) = (self_idx, msg.call_button) {
                deserialized_wv.elevator_containers[i].calls.push(call_btn); 

                //Om du er master i nettverket, oppdater call_buttons (Samme funksjon som kjøres i join_wv_from_tcp_container(). Behandler altså egen heis som en slave i nettverket) 
                if is_master {
                    let container = deserialized_wv.elevator_containers[i].clone();
                    master::wv_from_slaves::update_call_buttons(&mut deserialized_wv, &container, i).await;
                    deserialized_wv.elevator_containers[i].calls.clear();
                }
            }
        }

        // Floor_sensor -> oppdater last_floor_sensor i egen heis-container
        local_network::ElevMsgType::FSENS => {
            print_info(format!("Floor: {:?}", msg.floor_sensor));
            if let (Some(i), Some(floor)) = (self_idx, msg.floor_sensor) {
                deserialized_wv.elevator_containers[i].last_floor_sensor = floor;
            }
            
        }

        // Stop_button -> funksjon kommer
        local_network::ElevMsgType::SBTN => {
            print_info(format!("Stop button: {:?}", msg.stop_button));
            
        }

        // Obstruction -> Sett obstruction lik melding fra heis i egen heis-container
        local_network::ElevMsgType::OBSTRX => {
            print_info(format!("Obstruction: {:?}", msg.obstruction));
            if let (Some(i), Some(obs)) = (self_idx, msg.obstruction) {
                deserialized_wv.elevator_containers[i].obstruction = obs;
            }
        }
    }
    *wv = world_view::serialize_worldview(&deserialized_wv);
    true
}

/// ### Oppdaterer egne call-buttons og task_statuses etter de er sent over TCP til master
fn clear_from_sent_tcp(wv: &mut Vec<u8>, tcp_container: Vec<u8>) -> bool {
    let mut deserialized_wv = world_view::deserialize_worldview(&wv);
    let self_idx = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , wv.clone());
    let tcp_container_des = world_view::deserialize_elev_container(&tcp_container);

    // Lagre task-IDen til alle sendte tasks. 
    let tasks_ids: HashSet<u16> = tcp_container_des
        .tasks_status
        .iter()
        .map(|t| t.id)
        .collect();
    
    if let Some(i) = self_idx {
        /*_____ Fjern Tasks som master har oppdatert _____ */
        deserialized_wv.elevator_containers[i].tasks_status.retain(|t| tasks_ids.contains(&t.id));
        /*_____ Fjern sendte CallButtons _____ */
        deserialized_wv.elevator_containers[i].calls.retain(|call| !tcp_container_des.calls.contains(call));
        *wv = world_view::serialize_worldview(&deserialized_wv);
        return true;
    } else {
        utils::print_cosmic_err("The elevator does not exist clear_sent_container_stuff()".to_string());
        return false;
    }
}

/// ### Gir `task` til slave med `id`
/// 
/// Ikke ferdig implementert
fn push_task(wv: &mut Vec<u8>, task: Task, id: u8, button: CallButton) -> bool {
    let mut deser_wv = world_view::deserialize_worldview(&wv);

    // Fjern `button` frå `outside_button` om han finst
    if let Some(index) = deser_wv.outside_button.iter().position(|b| *b == button) {
        deser_wv.outside_button.swap_remove(index);
    }
    
    let self_idx = world_view::get_index_to_container(id, wv.clone());

    if let Some(i) = self_idx {
        // **Hindrar duplikatar: sjekk om task.id allereie finst i `tasks`**
        // NB: skal i teorien være unødvendig å sjekke dette
        if !deser_wv.elevator_containers[i].tasks.iter().any(|t| t.id == task.id) {
            deser_wv.elevator_containers[i].tasks.push(task);
            *wv = world_view::serialize_worldview(&deser_wv);
            return true;
        }
    }
    
    false
}

/// ### Oppdaterer status til `new_status` til task med `id` i egen heis_container.tasks_status
fn update_task_status(wv: &mut Vec<u8>, task_id: u16, new_status: TaskStatus) -> bool {
    let mut wv_deser = world_view::deserialize_worldview(&wv);
    let self_idx = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst), wv.clone());

    if let Some(i) = self_idx {
        // Finner `task` i tasks_status og setter status til `new_status`
        if let Some(task) = wv_deser.elevator_containers[i]
            .tasks_status
            .iter_mut()
            .find(|t| t.id == task_id) 
            {
                task.status = new_status.clone();
            }
    }
    // println!("Satt {:?} på id: {}", new_status, task_id);
    *wv = world_view::serialize_worldview(&wv_deser);
    true
}




