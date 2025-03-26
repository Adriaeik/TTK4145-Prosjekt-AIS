use tokio::sync::mpsc;
use tokio::net::TcpStream;
use std::net::SocketAddr;
use tokio::sync::watch;

use elevatorpro::{backup, elevator_logic, manager, network::{self, local_network, tcp_network, udp_network}, world_view};
use elevatorpro::init;
use elevatorpro::print;






#[tokio::main]
async fn main() {
    // Check if the program started as backup ("cargo r -- backup")
    let is_backup = init::parse_args();
    
    let mut self_container: Option<world_view::ElevatorContainer> = None;
    if is_backup {
        println!("Starting backup-process...");
        self_container = backup::run_as_backup().await;
    }    
    
    //init::build_cost_fn().await;
    print::info("Starting master process...".to_string());

    
    /* Initialize a worldview */
    let mut worldview = init::initialize_worldview(self_container.as_ref()).await;
    print::worldview(&worldview, Some(network::ConnectionStatus::new()));
    
    
    /* START ----------- Initializing of channels used for the worldview updater ---------------------- */
    let main_mpscs = local_network::Mpscs::new();
    let (wv_watch_tx, wv_watch_rx) = watch::channel(worldview.clone());
    /* END ----------- Initializing of channels used for the worldview updater ---------------------- */

    /* START ----------- Initializing of channels used for the networkstus update ---------------------- */
    let (network_watch_tx, network_watch_rx) = watch::channel(network::ConnectionStatus::new());
    let packetloss_rx = network_watch_rx.clone();
    /* END ----------- Initializing of channels used for the worldview updater ---------------------- */
    
    // // Send the initialized worldview on the worldview watch, so its not empty when rx tries to borrow it
    let _ = wv_watch_tx.send(worldview.clone());



    /* START ----------- Seperate the mpsc Rx's so they can be sent to the worldview updater ---------------------- */
    let mpsc_rxs = main_mpscs.rxs;
    /* END ----------- Seperate the mpsc Rx's so they can be sent to the worldview updater ---------------------- */





    /* START ----------- Seperate the mpsc Tx's so they can be sent to their designated tasks ---------------------- */
    let elevator_states_tx = main_mpscs.txs.elevator_states;
    let delegated_tasks_tx = main_mpscs.txs.delegated_tasks;
    let udp_wv_tx = main_mpscs.txs.udp_wv;
    let remove_container_tx = main_mpscs.txs.remove_container;
    let container_tx = main_mpscs.txs.container;
    let connection_to_master_failed_tx_clone = main_mpscs.txs.connection_to_master_failed.clone();
    let sent_tcp_container_tx = main_mpscs.txs.sent_tcp_container;
    let connection_to_master_failed_tx = main_mpscs.txs.connection_to_master_failed;
    let new_wv_after_offline_tx = main_mpscs.txs.new_wv_after_offline;
    /* END ----------- Seperate the mpsc Tx's so they can be sent to their designated tasks ---------------------- */
    




    /* START ----------- Task to watch over the internet connection ---------------------- */
    {
        let wv_watch_rx = wv_watch_rx.clone();
        let _network_status_watcher_task = tokio::spawn(async move {
            print::info("Starting to monitor internet".to_string());
            let _ = network::watch_ethernet(wv_watch_rx, network_watch_tx, new_wv_after_offline_tx).await;
        });
    }
    /* END ----------- Task to watch over the internet connection ---------------------- */
    
    



    /* START ----------- Init of channel to send sockets from new TCP-connections on ---------------------- */ 
    let (socket_tx, socket_rx) = mpsc::channel::<(TcpStream, SocketAddr)>(100);
    /* START ----------- Init of channel to send sockets from new TCP-connections on ---------------------- */





    /* START ----------- Critical tasks tasks ----------- */
    {
        //Continously updates the local worldview
        let _update_wv_task = tokio::spawn(async move {
            print::info("Starting to update worldview".to_string());
            let _ = local_network::update_wv_watch(mpsc_rxs, wv_watch_tx, &mut worldview).await;
        });
    }
    // {
    //     //Task handling the elevator
    //     let wv_watch_rx = wv_watch_rx.clone();
    //     let _local_elev_task = tokio::spawn(async move {
    //         let _ = elevator_logic::run_local_elevator(wv_watch_rx, elevator_states_tx).await;
    //     });
    // }
    // {
    //     //Starting the task manager, responsible for delegating tasks
    //     let wv_watch_rx = wv_watch_rx.clone();
    //     let _manager_task = tokio::spawn(async move {
    //         print::info("Staring task manager".to_string());
    //         let _ = manager::start_manager(wv_watch_rx, delegated_tasks_tx).await;
    //     });
    // }
    /* END ----------- Critical tasks tasks ----------- */





    /* START ----------- Backup server ----------- */
    // {
    //     let wv_watch_rx = wv_watch_rx.clone();
    //     let _backup_task = tokio::spawn(async move {
    //         print::info("Starting backup".to_string());
    //         tokio::spawn(backup::start_backup_server(wv_watch_rx, network_watch_rx));
    //     });
    // }
    /* END ----------- Backup server ----------- */
        




    /* START ----------- Network related tasks ---------------------- */
    {
        //Task listening for UDP broadcasts
        let wv_watch_rx = wv_watch_rx.clone();
        let _listen_task = tokio::spawn(async move {
            print::info("Starting to listen for UDP-broadcast".to_string());
            let _ = udp_network::start_udp_listener(wv_watch_rx, udp_wv_tx).await;
        });
    }

    {
        //Task sending UDP broadcasts
        let wv_watch_rx = wv_watch_rx.clone();
        let _broadcast_task = tokio::spawn(async move {
            print::info("Starting UDP-broadcaster".to_string());
            let _ = udp_network::start_udp_broadcaster(wv_watch_rx).await;
        });
    }


    {
        let wv_watch_rx = wv_watch_rx.clone();
        let _tcp_task = tokio::spawn(async move {
            print::info("Starting UDP direct network".to_string());
            let _ = network::udp_net::start_udp_network(
                wv_watch_rx,
                container_tx,
                packetloss_rx,
            ).await;
        });
    }

    // {
    //     //Task handling TCP connections
    //     let wv_watch_rx = wv_watch_rx.clone();
    //     let _tcp_task = tokio::spawn(async move {
    //         print::info("Starting TCP handler".to_string());
    //         let _ = tcp_network::tcp_handler(wv_watch_rx, remove_container_tx, container_tx, connection_to_master_failed_tx, sent_tcp_container_tx, socket_rx).await;
    //     });
    // }
    
    // {
    //     //Task handling the TCP-listener
    //     let _listener_handle = tokio::spawn(async move {
    //         print::info("Starting tcp listener".to_string());
    //         let _ = tcp_network::listener_task(socket_tx).await;
    //     });
    // }

    {
        //UDP Watchdog
        let _udp_watchdog = tokio::spawn(async move {
            print::info("Starting udp watchdog".to_string());
            let _ = udp_network::udp_watchdog(connection_to_master_failed_tx_clone).await;
        });
    }
    /* START ----------- Network related tasks ---------------------- */


    //Wait before exiting the program
    loop{
        tokio::task::yield_now().await;
    }
}


