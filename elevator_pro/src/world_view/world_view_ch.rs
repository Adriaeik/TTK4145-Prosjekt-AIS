use crate::world_view::world_view_update::{ join_wv_from_udp, 
                                            abort_network, 
                                            join_wv_from_tcp_container, 
                                            remove_container, 
                                            recieve_local_elevator_msg, 
                                            clear_from_sent_tcp, 
                                            push_task, 
                                            update_task_status
                                        };
use crate::network::local_network::LocalChannels;



// TODO: prøv å bruk tokio::select! istedenfor lang match for mer optimal cpu-bruk: eks fra chat:
// pub async fn update_wv(mut main_local_chs: local_network::LocalChannels, mut worldview_serialised: Vec<u8>) {
//     println!("Starter update_wv");
//     let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());

//     let mut wv_edited = false;

//     loop {
//         select! {
//             /* KANALER SLAVE MOTTAR PÅ */
//             Some(msg) = main_local_chs.mpscs.rxs.sent_tcp_container.recv() => {
//                 wv_edited = clear_from_sent_tcp(&mut worldview_serialised, msg);
//             }
//             Some(master_wv) = main_local_chs.mpscs.rxs.udp_wv.recv() => {
//                 wv_edited = join_wv_from_udp(&mut worldview_serialised, master_wv);
//             }
//             Some(_) = main_local_chs.mpscs.rxs.tcp_to_master_failed.recv() => {
//                 wv_edited = abort_network(&mut worldview_serialised);
//             }

//             /* KANALER MASTER MOTTAR PÅ */
//             Some(container) = main_local_chs.mpscs.rxs.container.recv() => {
//                 wv_edited = join_wv_from_tcp_container(&mut worldview_serialised, container).await;
//             }
//             Some(id) = main_local_chs.mpscs.rxs.remove_container.recv() => {
//                 wv_edited = remove_container(&mut worldview_serialised, id);
//             }
//             Some((task, id, button)) = main_local_chs.mpscs.rxs.new_task.recv() => {
//                 wv_edited = push_task(&mut worldview_serialised, task, id, button);
//             }

//             /* KANALER MASTER OG SLAVE MOTTAR PÅ */
//             Some(msg) = main_local_chs.mpscs.rxs.local_elev.recv() => {
//                 wv_edited = recieve_local_elevator_msg(&mut worldview_serialised, msg).await;
//             }
//             Some((id, status)) = main_local_chs.mpscs.rxs.update_task_status.recv() => {
//                 println!("Skal sette status {:?} på task id: {}", status, id);
//                 wv_edited = update_task_status(&mut worldview_serialised, id, status);
//             }

//             /* Timeout for å unngå 100% CPU-bruk */
//             _ = sleep(Duration::from_millis(1)) => {}
//         }

//         /* Hvis worldview er oppdatert, send til andre */
//         if wv_edited {
//             let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
//             wv_edited = false;
//         }
//     }
// }


/// ### Oppdatering av lokal worldview
/// 
/// Funksjonen leser nye meldinger fra andre tasks som indikerer endring i systemet, og endrer og oppdaterer det lokale worldviewen basert på dette.
#[allow(non_snake_case)]
pub async fn update_wv(mut main_local_chs: LocalChannels, mut worldview_serialised: Vec<u8>) {
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
















