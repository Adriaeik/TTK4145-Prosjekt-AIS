use std::time::Duration;

use elevator_pro::{network::{local_network, tcp_network, udp_broadcast}, utils, world_view::{world_view, world_view_update}};
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

    let mut worldview = world_view::WorldView::default();
    let mut elev_container = world_view::ElevatorContainer::default();
    let ip = match local_ip() {
        Ok(ip) => {
            ip
        }
        Err(e) => {
            utils::print_err(format!("Fant ikke IP i starten av main: {}", e));
            panic!();
        }
    }; 
    let self_id = utils::ip2id(ip);
    elev_container.elevator_id = self_id;
    worldview.master_id = self_id;
    worldview.add_elev(elev_container);
    let mut worldview_serialised = world_view::serialize_worldview(&worldview);

    
/* START ----------- Init av lokale channels ---------------------- */
    //Kun bruk mpsc-rxene fra main_local_chs
    let mut main_local_chs = local_network::LocalChannels::new();
/* SLUTT ----------- Init av lokale channels ---------------------- */

/* START ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */
    let chs_udp_listen = main_local_chs.clone();
    let chs_udp_bc = main_local_chs.clone();
    let chs_tcp = main_local_chs.clone();
/* SLUTT ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */                                                     


/* START ----------- Starte Eksterne Nettverkstasks ---------------------- */
    let _broadcast_task = tokio::spawn(async move {
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
/* SLUTT ----------- Starte Eksterne Nettverkstasks ---------------------- */


    
    loop {
        match main_local_chs.mpscs.rxs.udp_wv.try_recv() {
            Ok(master_wv) => {
                worldview_serialised = world_view_update::join_wv(worldview_serialised, master_wv);
            },
            Err(_) => {},
        }

        let _ = main_local_chs.broadcasts.txs.wv.send(worldview_serialised.clone());
    }


    // let tcp_task = tcp_listener(); //TCP connection mellom master eller slaver


    // let heis_logikk_task = start_process(); //Selve kjøre-mekanismene


}


// Kanskje lage is_master = min_id == lavest_id_i_wv i en global multithread vareabel
// Så kan worldview-thread oppdatere den (låse + skrive) og alle kan lese av den uten å låse (så wv ikke blir låst hver gang den skal beregnes)







// fn start_process() {
//     loop {
//         let is_master = min_id == wv_lavest_id;

//         if is_master {
//             //Finn ut hvilken oppgaver som må gjøres
//             //Deleger oppgaver til heiser
//             //Send på kanal hvilken heis som skal gjøre hvilken task
//         }
//         else {
//             //Vent på Task fra kanal ansvarlig for egen heis
//             //Si fra når Task er gjort
//         }
//     }
// }



