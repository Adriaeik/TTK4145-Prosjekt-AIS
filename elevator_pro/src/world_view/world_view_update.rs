use crate::network::local_network;
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

    let my_self_index = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , my_wv);
    let master_self_index = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , master_wv);

    if let (Some(my_i), Some(master_i)) = (my_self_index, master_self_index) {
        master_wv_deserialised.elevator_containers[master_i].door_open = my_wv_deserialised.elevator_containers[my_i].door_open;
        master_wv_deserialised.elevator_containers[master_i].obstruction = my_wv_deserialised.elevator_containers[my_i].obstruction;
        master_wv_deserialised.elevator_containers[master_i].last_floor_sensor = my_wv_deserialised.elevator_containers[my_i].last_floor_sensor;
        master_wv_deserialised.elevator_containers[master_i].motor_dir = my_wv_deserialised.elevator_containers[my_i].motor_dir;
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


pub async fn update_wv(mut main_local_chs: local_network::LocalChannels, wv_init: Vec<u8>) {
    let mut worldview_serialised = wv_init.clone();  

    println!("Starter update_wv");
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
    let mut wv_des = world_view::deserialize_worldview(&worldview_serialised);
    let init_task = world_view::Task{
        id: u16::MAX,
        to_do: 0,
    };
    if let Some(i) = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst), worldview_serialised) {
        wv_des.elevator_containers[i].tasks.push(init_task); //Antar at vi er eneste heisen i systemet mikromillisekundet vi starter
    }
    
    worldview_serialised = world_view::serialize_worldview(&wv_des);
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());  

    let mut wv_edited_I = false;
    loop {
        //Ops. mister internett -> du må bli master (single elevator mode)
        match main_local_chs.mpscs.rxs.udp_wv.try_recv() {
            Ok(master_wv) => {
                worldview_serialised = join_wv(worldview_serialised, master_wv);
                wv_edited_I = true;
            },
            Err(_) => {}, 
        }
        match main_local_chs.mpscs.rxs.tcp_to_master_failed.try_recv() {
            Ok(_) => {
                //fikse wv
                let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
                deserialized_wv.elevator_containers.retain(|elevator| elevator.elevator_id == utils::SELF_ID.load(Ordering::SeqCst));
                deserialized_wv.set_num_elev(deserialized_wv.elevator_containers.len() as u8);
                deserialized_wv.master_id = utils::SELF_ID.load(Ordering::SeqCst);
                worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                wv_edited_I = true;
            },
            Err(_) => {},
        }
        match main_local_chs.mpscs.rxs.container.try_recv() {
            Ok(container) => {
                let deser_container = world_view::deserialize_elev_container(&container);
                let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
                if let Some(index) = deserialized_wv.elevator_containers.iter().position(|x| x.elevator_id == deser_container.elevator_id) {
                    //TODO: sjekk at den er riktig / som forventa?
                    deserialized_wv.elevator_containers[index] = deser_container;
                } else {
                    deserialized_wv.add_elev(deser_container);
                } 
                worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                wv_edited_I = true;
            },
            Err(_) => {},
        }
        match main_local_chs.mpscs.rxs.remove_container.try_recv() {
            Ok(id) => {
                let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
                deserialized_wv.remove_elev(id);
                worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                wv_edited_I = true; 
            },
            Err(_) => {},
        }
        match main_local_chs.mpscs.rxs.local_elev.try_recv() {
            Ok(msg) => {
                let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
                let self_idx = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , worldview_serialised);
                match msg.msg_type {
                    local_network::ElevMsgType::CBTN => {
                        utils::print_info(format!("Callbutton: {:?}", msg.call_button));

                    }
                    local_network::ElevMsgType::FSENS => {
                        if let (Some(i), Some(floor)) = (self_idx, msg.floor_sensor) {
                            deserialized_wv.elevator_containers[i].last_floor_sensor = floor;
                        }
                        
                    }
                    local_network::ElevMsgType::SBTN => {
                        utils::print_info(format!("Stop button: {:?}", msg.stop_button));
                        
                    }
                    local_network::ElevMsgType::OBSTRX => {
                        utils::print_info(format!("Obstruction: {:?}", msg.obstruction));
                        if let (Some(i), Some(obs)) = (self_idx, msg.obstruction) {
                            deserialized_wv.elevator_containers[i].obstruction = obs;
                        }
                    }
                }
                worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                wv_edited_I = true;
            },
            Err(_) => {},
        }
        // let mut ww_des = world_view::deserialize_worldview(&worldview_serialised);
        // ww_des.elevator_containers[0].last_floor_sensor = (ww_des.elevator_containers[0].last_floor_sensor %255) + 1;
        // worldview_serialised = world_view::serialize_worldview(&ww_des);
        if wv_edited_I {
            let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
            // println!("WV er endra");
            //println!("Worldview ble endra");
            wv_edited_I = false;
        }
    }
    
}


