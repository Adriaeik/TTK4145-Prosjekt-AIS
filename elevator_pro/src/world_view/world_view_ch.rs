use std::sync::atomic::Ordering;
use std::u16;
use crate::world_view::world_view;
use crate::world_view::world_view::TaskStatus;
use crate::world_view::world_view_update;
use crate::network::local_network;
use crate::utils::{self, print_err, print_info, print_ok};
use crate::elevator_logic::master;

use super::world_view::Task;


pub async fn update_wv(mut main_local_chs: local_network::LocalChannels, mut worldview_serialised: Vec<u8>) {
    println!("Starter update_wv");
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
    

    

    let mut wv_edited_I = false;
    loop {
        //Ops. mister internett -> du må bli master (single elevator mode)
        /*_____Oppdater WV fra UDP-melding_____ */
        match main_local_chs.mpscs.rxs.udp_wv.try_recv() {
            Ok(master_wv) => {
                wv_edited_I = join_wv_from_udp(&mut worldview_serialised, master_wv);
            },
            Err(_) => {
                // utils::print_cosmic_err("Recieving from mpscs.rxs.udp_wv".to_string());
            }, 
        }
        /*_____Signal om at tilkobling til master har feila_____ */
        match main_local_chs.mpscs.rxs.tcp_to_master_failed.try_recv() {
            Ok(_) => {
                wv_edited_I = abort_network(&mut worldview_serialised);
            },
            Err(_) => {
                // utils::print_cosmic_err("Recieving from mmpscs.rxs.tcp_to_master_failed".to_string());
            },
        }
        /*_____Melding til master fra slaven (elevator-containeren til slaven)_____*/
        match main_local_chs.mpscs.rxs.container.try_recv() {
            Ok(container) => {
                let deser_container = world_view::deserialize_elev_container(&container);
                let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
                if None == deserialized_wv.elevator_containers.iter().position(|x| x.elevator_id == deser_container.elevator_id) {
                    deserialized_wv.add_elev(deser_container.clone());
                }

                let self_idx = world_view::get_index_to_container(deser_container.elevator_id, world_view::serialize_worldview(&deserialized_wv));
                
                if let Some(i) = self_idx {
                    master::wv_from_slaves::update_statuses(&mut deserialized_wv, &deser_container, i).await;
                    master::wv_from_slaves::update_call_buttons(&mut deserialized_wv, &deser_container, i).await;
                } else {
                    utils::print_cosmic_err("The elevator does not exist mpscs.rxs.container".to_string());
                }

                worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                wv_edited_I = true;
            },
            Err(_) => {},
        }
        /*_____ID til slave som er død (ikke kontakt med slave)_____ */
        match main_local_chs.mpscs.rxs.remove_container.try_recv() {
            Ok(id) => {
                let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
                deserialized_wv.remove_elev(id);
                worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                wv_edited_I = true; 
            },
            Err(_) => {},
        }
        /*_____Knapper trykket på lokal heis_____ */
        match main_local_chs.mpscs.rxs.local_elev.try_recv() {
            Ok(msg) => {
                let is_master = utils::is_master(worldview_serialised.clone());
                let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
                let self_idx = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , worldview_serialised);
                match msg.msg_type {
                    local_network::ElevMsgType::CBTN => {
                        print_info(format!("Callbutton: {:?}", msg.call_button));
                        if let (Some(i), Some(call_btn)) = (self_idx, msg.call_button) {
                            deserialized_wv.elevator_containers[i].calls.push(call_btn); 

                            if is_master {
                                let container = deserialized_wv.elevator_containers[i].clone();
                                master::wv_from_slaves::update_call_buttons(&mut deserialized_wv, &container, i).await;
                                deserialized_wv.elevator_containers[i].calls.clear();
                            }
                        }
                    }
                    local_network::ElevMsgType::FSENS => {
                        print_info(format!("Floor: {:?}", msg.floor_sensor));
                        if let (Some(i), Some(floor)) = (self_idx, msg.floor_sensor) {
                            deserialized_wv.elevator_containers[i].last_floor_sensor = floor;
                        }
                        
                    }
                    local_network::ElevMsgType::SBTN => {
                        print_info(format!("Stop button: {:?}", msg.stop_button));
                        
                    }
                    local_network::ElevMsgType::OBSTRX => {
                        print_info(format!("Obstruction: {:?}", msg.obstruction));
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
        /*_____Hvis worldview er endra, oppdater kanalen_____ */
        if wv_edited_I {
            let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
            wv_edited_I = false;
        }
    }
}

pub fn join_wv_from_udp(wv: &mut Vec<u8>, master_wv: Vec<u8>) -> bool {
    *wv = world_view_update::join_wv(wv.clone(), master_wv);
    true
}

pub fn abort_network(wv: &mut Vec<u8>) -> bool {
    let mut deserialized_wv = world_view::deserialize_worldview(wv);
    deserialized_wv.elevator_containers.retain(|elevator| elevator.elevator_id == utils::SELF_ID.load(Ordering::SeqCst));
    deserialized_wv.set_num_elev(deserialized_wv.elevator_containers.len() as u8);
    deserialized_wv.master_id = utils::SELF_ID.load(Ordering::SeqCst);
    *wv = world_view::serialize_worldview(&deserialized_wv);
    true
}