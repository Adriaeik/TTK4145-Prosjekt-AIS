

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
    let mut my_wv_deserialised = world_view::deserialize_worldview(&my_wv);
    let mut master_wv_deserialised = world_view::deserialize_worldview(&master_wv);

    let my_self_index = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , my_wv);
    let master_self_index = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , master_wv);


    if let (Some(my_i), Some(master_i)) = (my_self_index, master_self_index) {
        master_wv_deserialised.elevator_containers[master_i].door_open = my_wv_deserialised.elevator_containers[my_i].door_open;
        master_wv_deserialised.elevator_containers[master_i].obstruction = my_wv_deserialised.elevator_containers[my_i].obstruction;
        master_wv_deserialised.elevator_containers[master_i].last_floor_sensor = my_wv_deserialised.elevator_containers[my_i].last_floor_sensor;
        master_wv_deserialised.elevator_containers[master_i].motor_dir = my_wv_deserialised.elevator_containers[my_i].motor_dir;

        //Oppdater callbuttons, når master har fått de med seg fjern dine egne
        // Bytter til at vi antar at TCP får frem alle meldinger, og at vi fjerner calls etter vi har sendt på TCP
        // let to_remove_set: HashSet<_> = master_wv_deserialised.outside_button.clone().into_iter().collect();
        // my_wv_deserialised.elevator_containers[my_i].calls.retain(|call| !to_remove_set.contains(call));
        // master_wv_deserialised.elevator_containers[master_i].calls = my_wv_deserialised.elevator_containers[my_i].calls.clone();
        if tcp_network::TCP_SENT.load(Ordering::SeqCst) {
            tcp_network::TCP_SENT.store(false, Ordering::SeqCst);
            master_wv_deserialised.elevator_containers[master_i].calls.clear(); 
        } else {
            master_wv_deserialised.elevator_containers[master_i].calls = my_wv_deserialised.elevator_containers[my_i].calls.clone();
        }

    } else if let Some(my_i) = my_self_index {
        master_wv_deserialised.add_elev(my_wv_deserialised.elevator_containers[my_i].clone());
    }


    my_wv = world_view::serialize_worldview(&master_wv_deserialised);
    //utils::print_info(format!("Oppdatert wv fra UDP: {:?}", my_wv));
    my_wv 
}


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



