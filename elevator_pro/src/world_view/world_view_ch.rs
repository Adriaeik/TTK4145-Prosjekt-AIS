use std::thread::sleep;
use std::time::Duration;

use tokio::sync::mpsc;
use std::sync::atomic::Ordering;

use crate::world_view::world_view_update::{ join_wv_from_udp, 
                                            abort_network, 
                                            join_wv_from_tcp_container, 
                                            remove_container, 
                                            recieve_local_elevator_msg, 
                                            clear_from_sent_tcp,
                                            update_elev_state,
                                        };
use crate::network::local_network::LocalChannels;
use crate::utils::{self, extract_self_elevator_container};
use crate::world_view::world_view::{self, deserialize_worldview, serialize_worldview};


/// ### Oppdatering av lokal worldview
/// 
/// Funksjonen leser nye meldinger fra andre tasks som indikerer endring i systemet, og endrer og oppdaterer det lokale worldviewen basert på dette.
#[allow(non_snake_case)]
pub async fn update_wv(mut main_local_chs: LocalChannels, mut worldview_serialised: Vec<u8>, to_task_alloc_tx: mpsc::Sender<Vec<u8>>) {
    println!("Starter update_wv");
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
    
    let mut wv_edited_I = false;
    let mut master_container_updated_I = false;
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
                wv_edited_I = join_wv_from_tcp_container(&mut worldview_serialised, container.clone()).await;
                let _ = to_task_alloc_tx.send(container.clone()).await;
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
        // match main_local_chs.mpscs.rxs.new_task.try_recv() {
        //     Ok((task ,id, button)) => {
        //         // utils::print_master(format!("Fikk task: {:?}", task));
        //         wv_edited_I = push_task(&mut worldview_serialised, task, id, button);
        //     },
        //     Err(_) => {},
        // }
        


/* KANALER MASTER OG SLAVE MOTTAR PÅ */
        /*____Får signal når en task er ferdig_____ */
        match main_local_chs.mpscs.rxs.update_elev_state.try_recv() {
            Ok(status) => {
                wv_edited_I = update_elev_state(&mut worldview_serialised, status);
                master_container_updated_I = utils::is_master(worldview_serialised.clone());
            },
            Err(_) => {},
        }
        /*_____Knapper trykket på lokal heis_____ */
        match main_local_chs.mpscs.rxs.local_elev.try_recv() {
            Ok(msg) => {
                wv_edited_I = recieve_local_elevator_msg(main_local_chs.clone(), &mut worldview_serialised, msg).await;
                master_container_updated_I = utils::is_master(worldview_serialised.clone());
            },
            Err(_) => {},
        }
        
        
        
        /* KANALER ALLE SENDER LOKAL WV PÅ */
        /*_____Hvis worldview er endra, oppdater kanalen_____ */
        if master_container_updated_I {
            let container = extract_self_elevator_container(worldview_serialised.clone());
            let _ = main_local_chs.mpscs.txs.container.send(world_view::serialize_elev_container(&container)).await;
            master_container_updated_I = false;
        }

        if wv_edited_I {

            let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
            // println!("Sendte worldview lokalt {}", worldview_serialised[1]);
    
            wv_edited_I = false;
        }
    }
}
















