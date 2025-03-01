use std::sync::atomic::Ordering;
use std::u16;
use crate::elevio::poll::CallType;
use crate::world_view::world_view::{self, serialize_worldview, TaskStatus};
use crate::world_view::world_view_update;
use crate::network::local_network;
use crate::utils::{self, print_err, print_info, print_ok};
use crate::elevator_logic::{self, master};
use super::world_view::Task;


pub async fn update_wv(mut main_local_chs: local_network::LocalChannels, mut worldview_serialised: Vec<u8>) {
    println!("Starter update_wv");
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
    let mut wv_des = world_view::deserialize_worldview(&worldview_serialised);
    let init_task0 = Task{
        id: u16::MAX,
        to_do: 0,
        status: TaskStatus::PENDING,
        is_inside: true,
    };
    let init_task1 = Task{
        id: u16::MAX,
        to_do: 1,
        status: TaskStatus::PENDING,
        is_inside: true,
    };
    if let Some(i) = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst), worldview_serialised) {
        wv_des.elevator_containers[i].tasks.push(init_task0); //Antar at vi er eneste heisen i systemet mikromillisekundet vi starter
        wv_des.elevator_containers[i].tasks.push(init_task1); //Antar at vi er eneste heisen i systemet mikromillisekundet vi starter
    }
    
    worldview_serialised = world_view::serialize_worldview(&wv_des);
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());

    

    let mut wv_edited_I = false;
    loop {
        //Ops. mister internett -> du må bli master (single elevator mode)
        match main_local_chs.mpscs.rxs.udp_wv.try_recv() {
            Ok(master_wv) => {
                worldview_serialised = world_view_update::join_wv(worldview_serialised, master_wv);
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
                    master::update_from_slave::update_tasks(deser_container.clone(), &mut worldview_serialised, index).await;

                    /* Kommer til å bli en egen funksjon */
                    let mut deser_wv = world_view::deserialize_worldview(&worldview_serialised);
                    for call in deser_container.calls {
                        if call.call == CallType::INSIDE {
                            let new_task = Task { id: 69, to_do: call.floor, status: TaskStatus::PENDING, is_inside: true};
                            deser_wv.elevator_containers[index].tasks.push(new_task);
                            
                        } else {
                            deser_wv.outside_button.push(call);
                        }
                    }
                    worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                    /* Til hit */

                } else {
                    deserialized_wv.add_elev(deser_container);
                    worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                } 
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
                if let Some(i) = self_idx {
                    match msg.msg_type {
                        local_network::ElevMsgType::CBTN => {
                            print_info(format!("Callbutton: {:?}", msg.call_button));
                            if let Some(call_button) = msg.call_button {
                                deserialized_wv.elevator_containers[i].calls.push(call_button);
                            }

                        }
                        local_network::ElevMsgType::FSENS => {
                            if let Some(floor) =  msg.floor_sensor {
                                deserialized_wv.elevator_containers[i].last_floor_sensor = floor;
                            }
                            
                        }
                        local_network::ElevMsgType::SBTN => {
                            print_info(format!("Stop button: {:?}", msg.stop_button));
                            
                        }
                        local_network::ElevMsgType::OBSTRX => {
                            //print_info(format!("Obstruction: {:?}", msg.obstruction));
                            if let Some(obs) = msg.obstruction {
                                deserialized_wv.elevator_containers[i].obstruction = obs;
                            }
                        }
                    }
                }
                worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
                wv_edited_I = true;
            },
            Err(_) => {},
        }
        
        // match main_local_chs.mpscs.rxs.first_task_done.try_recv() {
        //     Ok(()) => {
        //         let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
        //         let self_idx = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , worldview_serialised);
        //         print_ok("Fist task done".to_string());
        //         if let Some(i) = self_idx {
        //             deserialized_wv.elevator_containers[i].tasks_status[0].status = TaskStatus::DONE;
        //         } else {
        //             utils::print_cosmic_err();
        //         }
        //         worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
        //         wv_edited_I = true; 
        //     },
        //     Err(_) => {},
        // }
        
        
        
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