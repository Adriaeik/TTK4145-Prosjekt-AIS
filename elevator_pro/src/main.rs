use std::{fmt::format, sync::atomic::Ordering, time::Duration};

use elevator_pro::{network::{local_network, tcp_network, tcp_self_elevator, udp_broadcast}, utils::{self, print_err, print_info, print_ok}, world_view::{world_view, world_view_ch, world_view_update}};
use elevator_pro::init;

use tokio::{sync::broadcast, time::sleep};
use tokio::sync::mpsc;
use tokio::net::TcpStream;
use std::net::SocketAddr;


#[tokio::main]
async fn main() {
/* START ----------- Task for å overvake Nettverksstatus ---------------------- */
    /* oppdaterer ein atomicbool der true er online, false er då offline */
    let _network_status_watcher_task = tokio::spawn(async move {
        utils::print_info("Starter å passe på nettverket".to_string());
        let _ = world_view_update::watch_ethernet().await;
    });
/* SLUTT ----------- Task for å overvake Nettverksstatus ---------------------- */



/*Skaper oss eit verdensbildet ved fødselen, vi tar vår første pust */
    let mut worldview_serialised = init::initialize_worldview();
    
/* START ----------- Init av lokale channels ---------------------- */
    //Kun bruk mpsc-rxene fra main_local_chs
    let mut main_local_chs = local_network::LocalChannels::new();
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
/* SLUTT ----------- Init av lokale channels ---------------------- */



/* START ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */
    let chs_udp_listen = main_local_chs.clone();
    let chs_udp_bc = main_local_chs.clone();
    let chs_tcp = main_local_chs.clone();
    let chs_udp_wd = main_local_chs.clone();
    let chs_print = main_local_chs.clone();
    let chs_listener = main_local_chs.clone();
    let chs_local_elev = main_local_chs.clone();
    let (socket_tx, socket_rx) = mpsc::channel::<(TcpStream, SocketAddr)>(8);
/* SLUTT ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */                                                     


    // let _update_wv_task = tokio::spawn(async move {
    //     utils::print_info("Starter å oppdatere wv".to_string());
    //     let _ = world_view_ch::update_wv(main_local_chs, worldview_serialised).await;
    // });
    //TODO: Få den til å signalisere at vi er i known state. Den kommer ikke til å returnere etterhvert
    let _local_elev_task = tokio::spawn(async {
        let _ = tcp_self_elevator::run_local_elevator(chs_local_elev).await;
    });


/* START ----------- Starte Eksterne Nettverkstasks ---------------------- */
    let _listen_task = tokio::spawn(async move {
        utils::print_info("Starter å høre etter UDP-broadcast".to_string());
        let _ = udp_broadcast::start_udp_listener(chs_udp_listen).await;
    });

    let _broadcast_task = tokio::spawn(async move {
        utils::print_info("Starter UDP-broadcaster".to_string());
        let _ = udp_broadcast::start_udp_broadcaster(chs_udp_bc).await;
    });

    let _tcp_task = tokio::spawn(async move {
        utils::print_info("Starter å TCPe".to_string());
        let _ = tcp_network::tcp_handler(chs_tcp, socket_rx).await;
    });

    // let _udp_watchdog = tokio::spawn(async move {
    //     utils::print_info("Starter udp watchdog".to_string());
    //     let _ = udp_broadcast::udp_watchdog(chs_udp_wd).await;
    // });
    
    let _listener_handle = tokio::spawn(async move {
        utils::print_info("Starter tcp listener".to_string());
        let _ = tcp_network::listener_task(chs_listener, socket_tx).await;
    });
    // Lag prat med egen heis thread her 
/* SLUTT ----------- Starte Eksterne Nettverkstasks ---------------------- */

    let _print_task = tokio::spawn(async move {
        let mut wv = utils::get_wv(chs_print.clone());
        loop {
            let chs_clone = chs_print.clone();
            utils::update_wv(chs_clone, &mut wv).await;
            world_view::print_wv(wv.clone());
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });



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
    
    




    // let heis_logikk_task = start_process(); //Selve kjøre-mekanismene


}




