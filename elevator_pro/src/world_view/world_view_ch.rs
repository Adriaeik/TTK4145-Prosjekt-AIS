use std::sync::atomic::Ordering;
use std::u16;
use crate::world_view::world_view;
use crate::world_view::world_view_update;
use crate::network::local_network;
use crate::utils::{self, print_err, print_info, print_ok};

use super::world_view::Task;


pub async fn update_wv(mut main_local_chs: local_network::LocalChannels, mut worldview_serialised: Vec<u8>) {
    println!("Starter update_wv");
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
    let mut wv_des = world_view::deserialize_worldview(&worldview_serialised);
    let init_task = Task{
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
                        print_info(format!("Callbutton: {:?}", msg.call_button));

                    }
                    local_network::ElevMsgType::FSENS => {
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
        if wv_edited_I {
            let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
            // println!("WV er endra");
            //println!("Worldview ble endra");
            wv_edited_I = false;
        }
    }
}