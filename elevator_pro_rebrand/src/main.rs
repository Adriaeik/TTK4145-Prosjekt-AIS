//! Entry point for the distributed elevator system.
//!
//! This async function initializes and launches all major tasks for controlling,
//! synchronizing, and communicating between elevators in the system.
//!
//! Key responsibilities:
//! - Starts in either master or backup mode based on CLI arguments
//! - Initializes a shared `WorldView` containing elevator states and requests
//! - Spawns background tasks for:
//!   - Monitoring internet connection
//!   - Updating and broadcasting the worldview over UDP
//!   - Running the local elevator logic
//!   - Managing task delegation
//!   - Synchronizing state with other nodes via UDP
//! - Sets up watch and mpsc channels for internal communication between components
//!
//! Note:
//! - TCP-based communication is deprecated and currently inactive
//! - This function never returns; it enters an infinite loop after initializing all tasks

use tokio::sync::watch;

use elevatorpro::{backup, elevator_logic, manager, network::{self, local_network, udp_network}, world_view};
use elevatorpro::init;
use elevatorpro::print;



#[tokio::main]
async fn main() {
    // Determine if this instance should run in backup mode (via CLI argument)
    let is_backup = init::parse_args();
    
    let mut self_container: Option<world_view::ElevatorContainer> = None;
    if is_backup {
        println!("Starting backup-process...");
        self_container = backup::run_as_backup().await;
    }    
    
    // Initializes the cost function used for task delegation between elevators.
    // This step is necessary before any scheduling decisions are made.
    init::build_cost_fn().await;
    print::info("Starting master process...".to_string());

    
    /* Initialize a worldview */
    // Initializes the global shared elevator state (`WorldView`).
    // If started as backup, uses data from the previous master if available

    // ⚠️ Note:
    // This restoration behavior is only relevant when starting the node offline (in backup mode).
    // In all other cases, the active network of elevators maintains and synchronizes your state.
    // If you crash and restart normally, your previous tasks will be remembered and reassigned by others.

    let mut worldview = init::initialize_worldview(self_container.as_ref()).await;
    print::worldview(&worldview, Some(network::ConnectionStatus::new()));
    
    
    /* START ----------- Initializing of channels used for the worldview updater ---------------------- */
    let main_mpscs = local_network::Mpscs::new();
    let (wv_watch_tx, wv_watch_rx) = watch::channel(worldview.clone());
    /* END ----------- Initializing of channels used for the worldview updater ---------------------- */
    
    /* START ----------- Initializing of channels used for the networkstus update ---------------------- */
    // Creates watch channels for sharing the current `WorldView` and `ConnectionStatus`.
    // These channels are used to propagate updates to all active tasks.  
    let (network_watch_tx, network_watch_rx) = watch::channel(network::ConnectionStatus::new());
    let packetloss_rx = network_watch_rx.clone();
    /* END ----------- Initializing of channels used for the worldview updater ---------------------- */

    // Push the initial worldview into the watch channel to ensure consumers receive a valid state immediately
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
    let sent_container_tx = main_mpscs.txs.sent_container;
    let connection_to_master_failed_tx = main_mpscs.txs.connection_to_master_failed;
    let new_wv_after_offline_tx = main_mpscs.txs.new_wv_after_offline;
    /* END ----------- Seperate the mpsc Tx's so they can be sent to their designated tasks ---------------------- */
    




    /* START ----------- Task to watch over the internet connection ---------------------- */
    {
        // Monitors internet connectivity and updates the `ConnectionStatus`.
        // 
        // This allows the system to detect network failures and trigger operation mode
        // when network conditions change.

        let wv_watch_rx = wv_watch_rx.clone();
        let _network_status_watcher_task = tokio::spawn(async move {
            print::info("Starting to monitor internet".to_string());
            let _ = network::watch_ethernet(wv_watch_rx, network_watch_tx, new_wv_after_offline_tx).await;
        });
    }
    /* END ----------- Task to watch over the internet connection ---------------------- */



    /* START ----------- Critical tasks tasks ----------- */
    {
        // Continously updates the local worldview
        let _update_wv_task = tokio::spawn(async move {
            print::info("Starting to update worldview".to_string());
            let _ = local_network::update_wv_watch(mpsc_rxs, wv_watch_tx, &mut worldview).await;
        });
    }
    {
        // Task handling the elevator
        let wv_watch_rx = wv_watch_rx.clone();
        let _local_elev_task = tokio::spawn(async move {
            print::info("Starting to run local elevator".to_string());
            let _ = elevator_logic::run_local_elevator(wv_watch_rx, elevator_states_tx).await;
        });
    }
    {
        // Starting the task manager, responsible for delegating tasks
        let wv_watch_rx = wv_watch_rx.clone();
        let _manager_task = tokio::spawn(async move {
            print::info("Staring task manager".to_string());
            let _ = manager::start_manager(wv_watch_rx, delegated_tasks_tx).await;
        });
    }
    /* END ----------- Critical tasks tasks ----------- */





    /* START ----------- Backup server ----------- */
    {
        // Starts the backup server task, which listens to the current `WorldView`
        // and maintains a live copy of the system state.
        //
        // Originally, this was part of a local failover concept, now deprecated.
        // It is currently used only as a GUI visualizer and debugging tool
        //
        // For more, see `mod backup`: `//! # ⚠️ NOT part of the final solution – Legacy backup module`
        let wv_watch_rx = wv_watch_rx.clone();
        let _backup_task = tokio::spawn(async move {
            print::info("Starting backup".to_string());
            tokio::spawn(backup::start_backup_server(wv_watch_rx, network_watch_rx));
        });
    }
    /* END ----------- Backup server ----------- */
        




    /* START ----------- Network related tasks ---------------------- */
    {
        // Listens for incoming UDP broadcasts from other nodes containing their `WorldView`.
        //
        // Received data is forwarded to the worldview updater via mpsc.
        let wv_watch_rx = wv_watch_rx.clone();
        let _listen_task = tokio::spawn(async move {
            print::info("Starting to listen for UDP-broadcast".to_string());
            let _ = udp_network::start_udp_listener(wv_watch_rx, udp_wv_tx).await;
        });
    }

    {
        // If master, Periodically broadcasts the `WorldView` to all over UDP.
        let wv_watch_rx = wv_watch_rx.clone();
        let _broadcast_task = tokio::spawn(async move {
            print::info("Starting UDP-broadcaster".to_string());
            let _ = udp_network::start_udp_broadcaster(wv_watch_rx).await;
        });
    }


    { 
        // Handles direct UDP-based communication between nodes.
        //
        // This includes:
        // - Syncing container states
        // - Detecting dropped slaves/master
        // - Reacting to master loss
        // - Handling connection failover
        let wv_watch_rx = wv_watch_rx.clone();
        tokio::spawn(async move {
            print::info("Starting UDP direct network".to_string());
            let _ = network::udp_net::start_direct_udp_network(
                wv_watch_rx,
                container_tx,
                packetloss_rx,
                connection_to_master_failed_tx,
                remove_container_tx,
                sent_container_tx,
            ).await;
        });
    }

    /* END ----------- Network related tasks ---------------------- */


    // Prevents the main task from exiting by yielding continuously.
    // 
    // All runtime logic happens in spawned background tasks.
    loop{
        tokio::task::yield_now().await;
    }
}


