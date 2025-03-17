use std::time::Duration;
use tokio::sync::mpsc;
use tokio::net::TcpStream;
use std::net::SocketAddr;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::sleep;

use elevatorpro::{backup, manager, network::{local_network, tcp_network, tcp_self_elevator, udp_broadcast}, ip_help_functions, world_view::{self, world_view_update}};
use elevatorpro::init;
use elevatorpro::print;






#[tokio::main]
async fn main() {
    // Sjekk om programmet startes som backup, retunerer true visst den blei det
    // vi starter i bacup med å skrive "cargo r -- backup"
    let mut is_backup = init::parse_args();
    
    let mut self_container: Option< world_view::ElevatorContainer> = None;
    if is_backup {
        println!("Starter backup-prosess...");
        self_container = Some(backup::run_as_backup().await);

        is_backup = false;
        //TODO: Visst vi er backup. så skal vi subscribe på TCP porten til år hovukode og den sender oss wv over TCP. 
        //TODO: vi skal så monitore connection, og printe WV med 
        /*
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

         */
        //TODO: (*TIL slutt*) Dersom vi tapar connection til master skal vi ta vare på ferdige oppgåver og starte med desse i WV slik at dei ikkje går tapt
        // Når det er fullført så brytes denne løkka og vi vil automatisk 
    }

    // 🚀 Hvis vi ikke er backup, starter vi som master!


    // Vanlig hovedprosess starter her:
    print::info("Starter hovedprosess...".to_string());

/* START ----------- Task for å overvake Nettverksstatus ---------------------- */
    /* oppdaterer ein atomicbool der true er online, false er då offline */
    let _network_status_watcher_task = tokio::spawn(async move {
        print::info("Starter å passe på nettverket".to_string());
        let _ = world_view_update::watch_ethernet().await;
    });
/* SLUTT ----------- Task for å overvake Nettverksstatus ---------------------- */



/*Skaper oss eit verdensbildet ved fødselen, vi tar vår første pust */
    let worldview_serialised = init::initialize_worldview(self_container).await;

    
/* START ----------- Init av lokale channels ---------------------- */
    //Kun bruk mpsc-rxene fra main_local_chs
    let main_local_chs = local_network::LocalChannels::new();
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
/* SLUTT ----------- Init av lokale channels ---------------------- */

/* START ----------- Init av diverse channels ---------------------- */
    //Kun bruk mpsc-rxene fra main_local_chs
    let (mut task_dellecator_tx, mut task_dellecator_rx) = mpsc::channel::<Vec<u8>>(1000);
/* SLUTT ----------- Init av diverse channels ---------------------- */



/* START ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */
    let chs_udp_listen = main_local_chs.clone();
    let chs_udp_bc = main_local_chs.clone();
    let chs_tcp = main_local_chs.clone();
    let chs_udp_wd = main_local_chs.clone();
    let chs_print = main_local_chs.clone();
    let chs_listener = main_local_chs.clone();
    let chs_local_elev = main_local_chs.clone();
    let chs_task_allocater = main_local_chs.clone();
    let chs_backup = main_local_chs.clone();
    let chs_task_dellecator = main_local_chs.clone();
    let mut chs_loop = main_local_chs.clone();
    let (socket_tx, socket_rx) = mpsc::channel::<(TcpStream, SocketAddr)>(100);
/* SLUTT ----------- Kloning av lokale channels til Tokio Tasks ---------------------- */                                                     

/* START ----------- Starte kritiske tasks ----------- */
    //Task som kontinuerlig oppdaterer lokale worldview
    let _update_wv_task = tokio::spawn(async move {
        print::info("Starter å oppdatere wv".to_string());
        let _ = local_network::update_wv_watch(main_local_chs, worldview_serialised, task_dellecator_tx).await;
    });
    //Task som håndterer den lokale heisen
    //TODO: Få den til å signalisere at vi er i known state.
    let _local_elev_task = tokio::spawn(async move {
        let _ = tcp_self_elevator::run_local_elevator(chs_local_elev).await;
    });
    let _task_allocater_task = tokio::spawn(async move {
        print::info("Staring task delegator".to_string());
        let _ = manager::task_allocator::delegate_tasks(chs_task_dellecator, task_dellecator_rx).await;
    });
/* SLUTT ----------- Starte kritiske tasks ----------- */

    // Start backup server i en egen task
    
    let _backup_task = tokio::spawn(async move {
        print::info("Starter backup".to_string());
        tokio::spawn(backup::start_backup_server(/*subscribe på wv chanel */chs_backup ));
    });
        
/* START ----------- Starte Eksterne Nettverkstasks ---------------------- */
    //Task som hører etter UDP-broadcasts
    let _listen_task = tokio::spawn(async move {
        print::info("Starter å høre etter UDP-broadcast".to_string());
        let _ = udp_broadcast::start_udp_listener(chs_udp_listen).await;
    });
    //Task som starter egen UDP-broadcaster
    let _broadcast_task = tokio::spawn(async move {
        print::info("Starter UDP-broadcaster".to_string());
        let _ = udp_broadcast::start_udp_broadcaster(chs_udp_bc).await;
    });
    //Task som håndterer TCP-koblinger
    let _tcp_task = tokio::spawn(async move {
        print::info("Starter å TCPe".to_string());
        let _ = tcp_network::tcp_handler(chs_tcp, socket_rx).await;
    });
    //UDP Watchdog
    let _udp_watchdog = tokio::spawn(async move {
        print::info("Starter udp watchdog".to_string());
        let _ = udp_broadcast::udp_watchdog(chs_udp_wd).await;
    });
    //Task som starter TCP-listener
    let _listener_handle = tokio::spawn(async move {
        print::info("Starter tcp listener".to_string());
        let _ = tcp_network::listener_task(chs_listener, socket_tx).await;
    });
    // Lag prat med egen heis thread her 
/* SLUTT ----------- Starte Eksterne Nettverkstasks ---------------------- */

/*
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
*/
    //Vent med å avslutte programmet
    let _ = chs_loop.broadcasts.rxs.shutdown.recv().await;
}


