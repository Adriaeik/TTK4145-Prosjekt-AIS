#![warn(missing_docs)]

use std::{fmt::format, sync::atomic::Ordering, time::Duration};

use elevator_pro::{config, elevator_logic::master::task_allocater, network::{local_network, tcp_network, tcp_self_elevator, udp_broadcast}, utils::{self, print_err, print_info, print_ok}, world_view::{world_view, world_view_ch, world_view_update}};
use elevator_pro::init;

use tokio::{sync::broadcast, time::sleep};
use tokio::sync::mpsc;
use tokio::net::TcpStream;
use std::net::SocketAddr;



#[tokio::main]
async fn main() {
    // Oppdater config-verdier basert på argumenter
    init::parse_args();

/* START ----------- Task for å overvake Nettverksstatus ---------------------- */
    /* oppdaterer ein atomicbool der true er online, false er då offline */
    let _network_status_watcher_task = tokio::spawn(async move {
        utils::print_info("Starter å passe på nettverket".to_string());
        let _ = world_view_update::watch_ethernet().await;
    });
/* SLUTT ----------- Task for å overvake Nettverksstatus ---------------------- */



/*Skaper oss eit verdensbildet ved fødselen, vi tar vår første pust */
    let worldview_serialised = init::initialize_worldview().await;
    
/* START ----------- Init av lokale channels ---------------------- */
    //Kun bruk mpsc-rxene fra main_local_chs
    let main_local_chs = local_network::LocalChannels::new();
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
    let chs_task_allocater = main_local_chs.clone();
    let mut chs_loop = main_local_chs.clone();
    let (socket_tx, socket_rx) = mpsc::channel::<(TcpStream, SocketAddr)>(100);
/* SLUTT ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */                                                     

/* START ----------- Starte kritiske tasks ----------- */
    //Task som kontinuerlig oppdaterer lokale worldview
    let _update_wv_task = tokio::spawn(async move {
        utils::print_info("Starter å oppdatere wv".to_string());
        let _ = world_view_ch::update_wv(main_local_chs, worldview_serialised).await;
    });
    //Task som håndterer den lokale heisen
    //TODO: Få den til å signalisere at vi er i known state.
    let _local_elev_task = tokio::spawn(async {
        let _ = tcp_self_elevator::run_local_elevator(chs_local_elev).await;
    });
/* SLUTT ----------- Starte kritiske tasks ----------- */


/* START ----------- Starte Eksterne Nettverkstasks ---------------------- */
    //Task som hører etter UDP-broadcasts
    let _listen_task = tokio::spawn(async move {
        utils::print_info("Starter å høre etter UDP-broadcast".to_string());
        let _ = udp_broadcast::start_udp_listener(chs_udp_listen).await;
    });
    //Task som starter egen UDP-broadcaster
    let _broadcast_task = tokio::spawn(async move {
        utils::print_info("Starter UDP-broadcaster".to_string());
        let _ = udp_broadcast::start_udp_broadcaster(chs_udp_bc).await;
    });
    //Task som håndterer TCP-koblinger
    let _tcp_task = tokio::spawn(async move {
        utils::print_info("Starter å TCPe".to_string());
        let _ = tcp_network::tcp_handler(chs_tcp, socket_rx).await;
    });
    //UDP Watchdog
    let _udp_watchdog = tokio::spawn(async move {
        utils::print_info("Starter udp watchdog".to_string());
        let _ = udp_broadcast::udp_watchdog(chs_udp_wd).await;
    });
    //Task som starter TCP-listener
    let _listener_handle = tokio::spawn(async move {
        utils::print_info("Starter tcp listener".to_string());
        let _ = tcp_network::listener_task(chs_listener, socket_tx).await;
    });
    //Task som fordeler heis-tasks
    let _allocater_handle = tokio::spawn(async move {
        utils::print_info("Starter task allocater listener".to_string());
        let _ = task_allocater::distribute_task(chs_task_allocater).await;
    });
    // Lag prat med egen heis thread her 
/* SLUTT ----------- Starte Eksterne Nettverkstasks ---------------------- */

    //Task som printer worldview
    let _print_task = tokio::spawn(async move {
        let mut wv = utils::get_wv(chs_print.clone());
        loop {
            let chs_clone = chs_print.clone();
            if utils::update_wv(chs_clone, &mut wv).await {
                world_view::print_wv(wv.clone());
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    });

    //Vent med å avslutte programmet
    let _ = chs_loop.broadcasts.rxs.shutdown.recv().await;
}




