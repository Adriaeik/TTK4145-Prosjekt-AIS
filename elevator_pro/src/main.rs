use std::time::Duration;

use elevator_pro::{network::{local_network, tcp_network, udp_broadcast}, utils, world_view::{world_view, world_view_update}};
use elevator_pro::init;
use termcolor::HyperlinkSpec;
use tokio::{sync::broadcast, time::sleep};
use local_ip_address::local_ip;


#[tokio::main]
async fn main() {
/* START ----------- Task for å overvake Nettverksstatus ---------------------- */
    /* oppdaterer ein atomicbool der true er onlie, false er då offline */
    let _network_status_watcher_task = tokio::spawn(async move {
        utils::print_info("Starter å passe på nettverket".to_string());
        let _ = world_view_update::watch_ethernet().await;
    });
/* SLUTT ----------- Task for å overvake Nettverksstatus ---------------------- */



/*Skaper oss eit verdensbildet ved fødselen, vi tar vår første pust */
    let (mut worldview_serialised, self_id) = init::initialize_worldview();


    
/* START ----------- Init av lokale channels ---------------------- */
    //Kun bruk mpsc-rxene fra main_local_chs
    let mut main_local_chs = local_network::LocalChannels::new();
/* SLUTT ----------- Init av lokale channels ---------------------- */



/* START ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */
    let chs_udp_listen = main_local_chs.clone();
    let chs_udp_bc = main_local_chs.clone();
    let chs_tcp = main_local_chs.clone();
    let chs_udp_wd = main_local_chs.clone();
/* SLUTT ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */                                                     



/* START ----------- Starte Eksterne Nettverkstasks ---------------------- */
    let _listen_task = tokio::spawn(async move {
        utils::print_info("Starter å høre etter UDP-broadcast".to_string());
        let _ = udp_broadcast::start_udp_listener(chs_udp_listen).await;
    });

    let _broadcast_task = tokio::spawn(async move {
        utils::print_info("Starter UDP-broadcaster".to_string());
        let _ = udp_broadcast::start_udp_broadcaster(chs_udp_bc, self_id).await;
    });

    let _tcp_task = tokio::spawn(async move {
        utils::print_info("Starter å TCPe".to_string());
        let _ = tcp_network::tcp_listener(self_id, chs_tcp).await;
    });

    let udp_watchdog = tokio::spawn(async move {
        utils::print_info("Starter udp watchdog".to_string());
        let _ = udp_broadcast::udp_watchdog(chs_udp_wd).await;
    });
/* SLUTT ----------- Starte Eksterne Nettverkstasks ---------------------- */






    
    loop {
        //Ops. mister internett -> du må bli master (single elevator mode)
        match main_local_chs.mpscs.rxs.udp_wv.try_recv() {
            Ok(master_wv) => {
                worldview_serialised = world_view_update::join_wv(worldview_serialised, master_wv);
            },
            Err(_) => {},
        }
        match main_local_chs.mpscs.rxs.tcp_to_master_failed.try_recv() {
            Ok(_) => {
                //fikse wv
                let mut deserialized_wv = world_view::deserialize_worldview(&worldview_serialised);
                deserialized_wv.elevator_containers.retain(|elevator| elevator.elevator_id == self_id);
                deserialized_wv.master_id = self_id;
                worldview_serialised = world_view::serialize_worldview(&deserialized_wv);
            },
            Err(_) => {},
        }

        let _ = main_local_chs.broadcasts.txs.wv.send(worldview_serialised.clone());
    }




    // let heis_logikk_task = start_process(); //Selve kjøre-mekanismene


}




