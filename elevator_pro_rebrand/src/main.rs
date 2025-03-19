use tokio::sync::mpsc;
use tokio::net::TcpStream;
use std::{net::SocketAddr, time::Duration};

use elevatorpro::{backup, elevator_logic, manager, network::{local_network, tcp_network, udp_broadcast}, world_view::{self, world_view_update}};
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

    
    
    
    /*Skaper oss eit verdensbildet ved fødselen, vi tar vår første pust */
    let worldview_serialised = init::initialize_worldview(self_container).await;
    
    
    /* START ----------- Init av channels brukt til oppdatering av lokal worldview ---------------------- */
    let main_mpscs = local_network::Mpscs::new();
    let watches = local_network::Watches::new();
    
    // Send the initialized worldview on the worldview watch, so its not empty when rx tries to borrow it
    let _ = watches.txs.wv.send(worldview_serialised.clone());
    // Seperate the watch Tx's so they can be sent to theis designated tasks
    let wv_watch_tx = watches.txs.wv;
    // let elev_task_tx= watches.txs.elev_task;
    
    // Seperate the mpsc Rx's so they can be sent to [local_network::update_wv_watch]
    let mpsc_rxs = main_mpscs.rxs;
    // Seperate the mpsc Tx's so they can be sent to their designated tasks
    let elevator_states_tx = main_mpscs.txs.elevator_states;
    let delegated_tasks_tx = main_mpscs.txs.delegated_tasks;
    let udp_wv_tx = main_mpscs.txs.udp_wv;
    let remove_container_tx = main_mpscs.txs.remove_container;
    let container_tx = main_mpscs.txs.container;
    let tcp_to_master_failed_tx_clone = main_mpscs.txs.tcp_to_master_failed.clone();
    let sent_tcp_container_tx = main_mpscs.txs.sent_tcp_container;
    let tcp_to_master_failed_tx = main_mpscs.txs.tcp_to_master_failed;
    let new_wv_after_offline_tx = main_mpscs.txs.new_wv_after_offline;
    
    /* SLUTT ----------- Init av channels brukt til oppdatering av lokal worldview ---------------------- */

    /* START ----------- Task for å overvake Nettverksstatus ---------------------- */
    {
        let wv_watch_rx = watches.rxs.wv.clone();
        let _network_status_watcher_task = tokio::spawn(async move {
            print::info("Starter å passe på nettverket".to_string());
            let _ = world_view_update::watch_ethernet(wv_watch_rx, new_wv_after_offline_tx).await;
        });
    }
    /* SLUTT ----------- Task for å overvake Nettverksstatus ---------------------- */
    
    
    /* START ----------- Init av diverse channels ---------------------- */ 
    // Create other channels used for other things
    let (socket_tx, socket_rx) = mpsc::channel::<(TcpStream, SocketAddr)>(100);

/* SLUTT ----------- Init av diverse channels ---------------------- */

/* START ----------- Starte kritiske tasks ----------- */
    {
        //Task som kontinuerlig oppdaterer lokale worldview
        let _update_wv_task = tokio::spawn(async move {
            print::info("Starter å oppdatere wv".to_string());
            let _ = local_network::update_wv_watch(mpsc_rxs, wv_watch_tx, worldview_serialised).await;
        });
    }
    //Task som håndterer den lokale heisen
    //TODO: Få den til å signalisere at vi er i known state.
    {
        let wv_watch_rx = watches.rxs.wv.clone();
        let _local_elev_task = tokio::spawn(async move {
            let _ = elevator_logic::run_local_elevator(wv_watch_rx, elevator_states_tx).await;
        });
    }
    {
        let wv_watch_rx = watches.rxs.wv.clone();
        let _manager_task = tokio::spawn(async move {
            print::info("Staring task manager".to_string());
            let _ = manager::start_manager(wv_watch_rx, delegated_tasks_tx).await;
        });
    }
/* SLUTT ----------- Starte kritiske tasks ----------- */

    // Start backup server i en egen task
    {
        let wv_watch_rx = watches.rxs.wv.clone();
        let _backup_task = tokio::spawn(async move {
            print::info("Starter backup".to_string());
            tokio::spawn(backup::start_backup_server(wv_watch_rx));
        });
    }
        
/* START ----------- Starte Eksterne Nettverkstasks ---------------------- */
    //Task som hører etter UDP-broadcasts
    {
        let wv_watch_rx = watches.rxs.wv.clone();
        let _listen_task = tokio::spawn(async move {
            print::info("Starter å høre etter UDP-broadcast".to_string());
            let _ = udp_broadcast::start_udp_listener(wv_watch_rx, udp_wv_tx).await;
        });
    }

    //Task som starter egen UDP-broadcaster
    {
        let wv_watch_rx = watches.rxs.wv.clone();
        let _broadcast_task = tokio::spawn(async move {
            print::info("Starter UDP-broadcaster".to_string());
            let _ = udp_broadcast::start_udp_broadcaster(wv_watch_rx).await;
        });
    }

    //Task som håndterer TCP-koblinger
    {
        let wv_watch_rx = watches.rxs.wv.clone();
        let _tcp_task = tokio::spawn(async move {
            print::info("Starter å TCPe".to_string());
            let _ = tcp_network::tcp_handler(wv_watch_rx, remove_container_tx, container_tx, tcp_to_master_failed_tx, sent_tcp_container_tx, socket_rx).await;
        });
    }

    //UDP Watchdog
    {
        let _udp_watchdog = tokio::spawn(async move {
            print::info("Starter udp watchdog".to_string());
            let _ = udp_broadcast::udp_watchdog(tcp_to_master_failed_tx_clone).await;
        });
    }

    //Task som starter TCP-listener
    {
        let _listener_handle = tokio::spawn(async move {
            print::info("Starter tcp listener".to_string());
            let _ = tcp_network::listener_task(socket_tx).await;
        });
    }
    // Lag prat med egen heis thread her 
/* SLUTT ----------- Starte Eksterne Nettverkstasks ---------------------- */


    // Task som printer worldview
    // let _print_task = tokio::spawn(async move {
    //     let mut wv = world_view::get_wv(watches.rxs.wv.clone());
    //     loop {
    //         if world_view::update_wv(watches.rxs.wv.clone(), &mut wv).await {
    //             print::worldview(wv.clone());
    //             tokio::time::sleep(Duration::from_millis(500)).await;
    //         }
    //     }
    // });

    //Vent med å avslutte programmet
    loop{
        tokio::task::yield_now().await;
    }
}


