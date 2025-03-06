

use crate::network::local_network;
use crate::world_view::world_view;
use crate::network::tcp_network;
use crate::{config, utils};

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;


static ONLINE: OnceLock<AtomicBool> = OnceLock::new(); // worldview_channel_request
pub fn get_network_status() -> &'static AtomicBool {
    ONLINE.get_or_init(|| AtomicBool::new(false))
}




pub fn join_wv(mut my_wv: Vec<u8>, master_wv: Vec<u8>) -> Vec<u8> {
    //TODO: Lag copy funkjon for worldview structen
    let my_wv_deserialised = world_view::deserialize_worldview(&my_wv);
    let mut master_wv_deserialised = world_view::deserialize_worldview(&master_wv);

    let my_self_index = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , my_wv);
    let master_self_index = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , master_wv);


    if let (Some(i_org), Some(i_new)) = (my_self_index, master_self_index) {
        master_wv_deserialised.elevator_containers[i_new].door_open = my_wv_deserialised.elevator_containers[i_org].door_open;
        master_wv_deserialised.elevator_containers[i_new].obstruction = my_wv_deserialised.elevator_containers[i_org].obstruction;
        master_wv_deserialised.elevator_containers[i_new].last_floor_sensor = my_wv_deserialised.elevator_containers[i_org].last_floor_sensor;
        master_wv_deserialised.elevator_containers[i_new].motor_dir = my_wv_deserialised.elevator_containers[i_org].motor_dir;

        master_wv_deserialised.elevator_containers[i_new].calls = my_wv_deserialised.elevator_containers[i_org].calls.clone();

        master_wv_deserialised.elevator_containers[i_new].tasks_status = my_wv_deserialised.elevator_containers[i_org].tasks_status.clone();




        /*Oppdater task_statuses. putt i funksjon hvis det funker?*/
        let new_ids: HashSet<u16> = master_wv_deserialised.elevator_containers[i_new].tasks.iter().map(|t| t.id).collect();
        let old_ids: HashSet<u16> = master_wv_deserialised.elevator_containers[i_new].tasks_status.iter().map(|t| t.id).collect();

        // Legg til taskar frå masters task som ikkje allereie finst i task_status
        for task in master_wv_deserialised.elevator_containers[i_new].tasks.clone().iter() {
            if !old_ids.contains(&task.id) {
                master_wv_deserialised.elevator_containers[i_new].tasks_status.push(task.clone());
            }
        }
        // Fjern taskar frå task_status som ikkje fins lenger i masters tasks
        master_wv_deserialised.elevator_containers[i_org]
        .tasks_status
        .retain(|t| new_ids.contains(&t.id));


        //Oppdater callbuttons, når master har fått de med seg fjern dine egne
        // Bytter til at vi antar at TCP får frem alle meldinger, og at vi fjerner calls etter vi har sendt på TCP    
    } else if let Some(i_org) = my_self_index {
        master_wv_deserialised.add_elev(my_wv_deserialised.elevator_containers[i_org].clone());
    }

    my_wv = world_view::serialize_worldview(&master_wv_deserialised);
    //utils::print_info(format!("Oppdatert wv fra UDP: {:?}", my_wv));
    my_wv 
}

/// ### Sjekker om vi har internett-tilkobling
pub async fn watch_ethernet() {
    let mut last_net_status = false;
    let mut net_status = false;
    loop {
        let ip = utils::get_self_ip();

        match ip {
            Ok(ip) => {
                if utils::get_root_ip(ip) == config::NETWORK_PREFIX {
                    net_status = true;
                }
                else {
                    net_status = false   
                }
            }
            Err(_) => {
                net_status = false
            }
        }

        if last_net_status != net_status {  
            get_network_status().store(net_status, Ordering::SeqCst);
            if net_status {utils::print_ok("Vi er online".to_string());}
            else {utils::print_warn("Vi er offline".to_string());}
            last_net_status = net_status;
        }
    }
}



