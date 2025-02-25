use crate::world_view::world_view;
use crate::{config, utils};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;


static ONLINE: OnceLock<AtomicBool> = OnceLock::new(); // worldview_channel_request
pub fn get_network_status() -> &'static AtomicBool {
    ONLINE.get_or_init(|| AtomicBool::new(false))
}





pub fn join_wv(mut my_wv: Vec<u8>, master_wv: Vec<u8>) -> Vec<u8> {
    //TODO: Lag copy funkjon for worldview structen
    let my_wv_deserialised = world_view::deserialize_worldview(&my_wv);
    let mut master_wv_deserialised = world_view::deserialize_worldview(&master_wv);


    let mut my_elev_exists = false;


    for mut elevator in master_wv_deserialised.clone().elevator_containers {
        if elevator.elevator_id == utils::SELF_ID.load(Ordering::SeqCst) {
            my_elev_exists = true;

            //Vi har bedre styr på interne var. enn master. Hent egt bare ut tasks'
            // for my_elevator in my_wv_deserialised.clone().elevator_containers {
            //     if my_elevator.elevator_id == utils::SELF_ID.load(Ordering::SeqCst) {
            //         elevator.door_open = my_elevator.door_open;
            //         elevator.obstruction = my_elevator.obstruction;
            //         elevator.last_floor_sensor = my_elevator.last_floor_sensor;
            //         elevator.motor_dir = my_elevator.motor_dir;
            //     }
            // }
        }
    }
    if !my_elev_exists {
        if let Some(index) = my_wv_deserialised.clone().elevator_containers.iter().position(|x| x.elevator_id == utils::SELF_ID.load(Ordering::SeqCst)) {
            master_wv_deserialised.add_elev(my_wv_deserialised.elevator_containers[index].clone());   
        } 
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




