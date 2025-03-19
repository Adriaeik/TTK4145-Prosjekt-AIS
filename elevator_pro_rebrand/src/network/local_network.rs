//! Handles messages on internal channels regarding changes in worldview

use crate::{elevio::ElevMessage, manager, world_view::{Dirn, ElevatorBehaviour}};
use crate::print;
use crate::config;
// use crate::manager::task_allocator::Task;
use crate::world_view::world_view_update::{ 
    join_wv_from_udp, 
    abort_network, 
    join_wv_from_tcp_container, 
    remove_container,
    clear_from_sent_tcp,
    distribute_tasks,
    update_elev_states,
    // push_task,
    // publish_tasks,
};
use crate::world_view::{self, serial};

use tokio::{sync::{mpsc, watch}, time::sleep};
use local_ip_address::local_ip;
use std::{net::IpAddr, time::Duration};
use std::sync::atomic::AtomicU8;
use std::collections::HashMap;

/// Atomic bool storing self ID, standard inited as config::ERROR_ID
pub static SELF_ID: AtomicU8 = AtomicU8::new(config::ERROR_ID); // Startverdi 255

/// Returns the local IPv4 address of the machine as `IpAddr`.
///
/// If no local IPv4 address is found, returns `local_ip_address::Error`.
///
/// # Example
/// ```
/// use elevatorpro::utils::get_self_ip;
///
/// match get_self_ip() {
///     Ok(ip) => println!("Local IP: {}", ip), // IP retrieval successful
///     Err(e) => println!("Failed to get IP: {:?}", e), // No local IP available
/// }
/// ```
pub fn get_self_ip() -> Result<IpAddr, local_ip_address::Error> {
    let ip = match local_ip() {
        Ok(ip) => {
            ip
        }
        Err(e) => {
            // print::warn(format!("Fant ikke IP i get_self_ip() -> Vi er offline: {}", e));
            return Err(e);
        }
    };
    Ok(ip)
}


/// ### Oppdatering av lokal worldview
/// 
/// Funksjonen leser nye meldinger fra andre tasks som indikerer endring i systemet, og endrer og oppdaterer det lokale worldviewen basert på dette.
#[allow(non_snake_case)]
pub async fn update_wv_watch(mut mpsc_rxs: MpscRxs, worldview_watch_tx: watch::Sender<Vec<u8>>, mut worldview_serialised: Vec<u8>) {
    println!("Starter update_wv");
    let _ = worldview_watch_tx.send(worldview_serialised.clone());
    
    let mut wv_edited_I = false;
    let mut master_container_updated_I = false;

    let (master_container_tx, mut master_container_rx) = mpsc::channel::<Vec<u8>>(100);    
    let mut i = 0;
    loop {
        //OBS: Error kommer når kanal er tom. ikke print der uten å eksplisitt eksludere channel_empty error type

/* KANALER SLAVE HOVEDSAKLIG MOTTAR PÅ */
        /*_____Fjerne knappar som vart sendt på TCP_____ */
        match mpsc_rxs.sent_tcp_container.try_recv() {
            Ok(msg) => {
                // println!("{}", 1);
                wv_edited_I = clear_from_sent_tcp(&mut worldview_serialised, msg);
                // println!("{}", 2);
            },
            Err(_) => {},
        }
        /*_____Oppdater WV fra UDP-melding_____ */
        match mpsc_rxs.udp_wv.try_recv() {
            Ok(master_wv) => {
                // println!("udp wv len {}", mpsc_rxs.udp_wv.len());
                wv_edited_I = join_wv_from_udp(&mut worldview_serialised, master_wv);
            },
            Err(_) => {}, 
        }
        /*_____Signal om at tilkobling til master har feila_____ */
        match mpsc_rxs.tcp_to_master_failed.try_recv() {
            Ok(_) => {
                // println!("tcp fail len {}", mpsc_rxs.tcp_to_master_failed.len());
                wv_edited_I = abort_network(&mut worldview_serialised);
            },
            Err(_) => {},
        }
        /*_____Melding til master fra master (elevator-containeren til master)_____*/
        match master_container_rx.try_recv() {
            Ok(container) => {
                // println!("master cont len {}", master_container_rx.len());
                wv_edited_I = join_wv_from_tcp_container(&mut worldview_serialised, container.clone()).await;
                // let _ = to_task_alloc_tx.send(container.clone()).await;
            },
            Err(_) => {},
        }
        
        
/* KANALER MASTER HOVEDSAKLIG MOTTAR PÅ */
        /*_____Melding til master fra slaven (elevator-containeren til slaven)_____*/
        match mpsc_rxs.container.try_recv() {
            Ok(container) => {
                wv_edited_I = join_wv_from_tcp_container(&mut worldview_serialised, container.clone()).await;
                // let _ = to_task_alloc_tx.send(container.clone()).await;
            },
            Err(_) => {},
        }
        /*_____ID til slave som er død (ikke kontakt med slave)_____ */
        match mpsc_rxs.remove_container.try_recv() {
            Ok(id) => {
                wv_edited_I = remove_container(&mut worldview_serialised, id); 
            },
            Err(_) => {},
        }
        match mpsc_rxs.delegated_tasks.try_recv() {
            Ok(map) => {
                wv_edited_I = distribute_tasks(&mut worldview_serialised, map);
            },
            Err(_) => {},
        }
        // match mpsc_rxs.new_task.try_recv() {
        //     Ok((id, sometask)) => {
        //         // utils::print_master(format!("Fikk task: {:?}", task));
        //         wv_edited_I = push_task(&mut worldview_serialised, id, sometask);
        //     },
        //     Err(_) => {},
        // }
        // match mpsc_rxs.pending_tasks.try_recv() {
        //     Ok(tasks) => {
        //         wv_edited_I = publish_tasks(&mut worldview_serialised, tasks);
        //     },
        //     Err(_) => {},
        // }
        


/* KANALER MASTER OG SLAVE MOTTAR PÅ */
        /*____Får signal når lokal heis container er endra_____ */
        match mpsc_rxs.elevator_states.try_recv() {
            Ok(container) => {
                wv_edited_I = update_elev_states(&mut worldview_serialised, container);
                master_container_updated_I = world_view::is_master(worldview_serialised.clone());
            },
            Err(_) => {},
        }

        match mpsc_rxs.new_wv_after_offline.try_recv() {
            Ok(wv) => {
                worldview_serialised = wv;
                let _ = worldview_watch_tx.send(worldview_serialised.clone());
            },
            Err(_) => {},
        }
        
        
        
        /* KANALER ALLE SENDER LOKAL WV PÅ */
        /*_____Hvis worldview er endra, oppdater kanalen_____ */
        if master_container_updated_I {
            let container = world_view::extract_self_elevator_container(worldview_serialised.clone());
            let _ = master_container_tx.send(serial::serialize_elev_container(&container)).await;
            master_container_updated_I = false;
        }
        
        if wv_edited_I {
            
            let _ = worldview_watch_tx.send(worldview_serialised.clone());
            // println!("Sendte worldview lokalt {}", worldview_serialised[1]);
            
            wv_edited_I = false;
            // sleep(Duration::from_secs(1)).await;
        }
    }
}























// --- MPSC-KANALAR ---
/// Struct containing multiple MPSC (multi-producer, single-consumer) sender channels.
/// These channels are primarely used to send data to the task updating the local worldview.
#[allow(missing_docs)]
#[derive(Clone)]
pub struct MpscTxs {
    /// Sends a UDP worldview packet.
    pub udp_wv: mpsc::Sender<Vec<u8>>,
    /// Notifies if the TCP connection to the master has failed.
    pub tcp_to_master_failed: mpsc::Sender<bool>,
    /// Sends elevator containers recieved from slaves on TCP.
    pub container: mpsc::Sender<Vec<u8>>,
    /// Requests the removal of a container by ID.
    pub remove_container: mpsc::Sender<u8>,
    /// Sends a TCP container message that has been transmitted to the master.
    pub sent_tcp_container: mpsc::Sender<Vec<u8>>,
    /// Additional buffered channels for various data streams.
    // pub pending_tasks: mpsc::Sender<Vec<Task>>,
    pub delegated_tasks: mpsc::Sender<HashMap<u8, Vec<[bool; 2]>>>,
    pub elevator_states: mpsc::Sender<Vec<u8>>,
    pub new_wv_after_offline: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch6: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch7: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch8: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch9: mpsc::Sender<Vec<u8>>,
}

/// Struct containing multiple MPSC (multi-producer, single-consumer) receiver channels.
/// These channels are used to receive data from different parts of the system.
#[allow(missing_docs)]
pub struct MpscRxs {
    /// Receives a UDP worldview packet.
    pub udp_wv: mpsc::Receiver<Vec<u8>>,
    /// Receives a notification if the TCP connection to the master has failed.
    pub tcp_to_master_failed: mpsc::Receiver<bool>,
    /// Receives elevator containers recieved from slaves on TCP.
    pub container: mpsc::Receiver<Vec<u8>>,
    /// Receives requests to remove a container by ID.
    pub remove_container: mpsc::Receiver<u8>,
    /// Receives TCP container messages that have been transmitted.
    pub sent_tcp_container: mpsc::Receiver<Vec<u8>>,
    /// Additional buffered channels for various data streams.
    // pub pending_tasks: mpsc::Receiver<Vec<Task>>,
    pub delegated_tasks: mpsc::Receiver<HashMap<u8, Vec<[bool; 2]>>>,
    pub elevator_states: mpsc::Receiver<Vec<u8>>,
    pub new_wv_after_offline: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch6: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch7: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch8: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch9: mpsc::Receiver<Vec<u8>>,
}

/// Struct that combines MPSC senders and receivers into a single entity.
pub struct Mpscs {
    /// Contains all sender channels.
    pub txs: MpscTxs,
    /// Contains all receiver channels.
    pub rxs: MpscRxs,
}

impl Mpscs {
    /// Creates a new `Mpscs` instance with initialized channels.
    pub fn new() -> Self {
        let (tx_udp, rx_udp) = mpsc::channel(300);
        let (tx_tcp_to_master_failed, rx_tcp_to_master_failed) = mpsc::channel(300);
        let (tx_container, rx_container) = mpsc::channel(300);
        let (tx_remove_container, rx_remove_container) = mpsc::channel(300);
        let (tx_sent_tcp_container, rx_sent_tcp_container) = mpsc::channel(300);
        // let (tx_new_task, rx_new_task) = mpsc::channel(300);
        // let (tx_pending_tasks, rx_pending_tasks) = mpsc::channel(300);
        let (tx_buf3, rx_buf3) = mpsc::channel(300);
        let (tx_buf4, rx_buf4) = mpsc::channel(300);
        let (tx_buf5, rx_buf5) = mpsc::channel(300);
        let (tx_buf6, rx_buf6) = mpsc::channel(300);
        let (tx_buf7, rx_buf7) = mpsc::channel(300);
        let (tx_buf8, rx_buf8) = mpsc::channel(300);
        let (tx_buf9, rx_buf9) = mpsc::channel(300);

        Mpscs {
            txs: MpscTxs {
                udp_wv: tx_udp,
                tcp_to_master_failed: tx_tcp_to_master_failed,
                container: tx_container,
                remove_container: tx_remove_container,
                sent_tcp_container: tx_sent_tcp_container,
                delegated_tasks: tx_buf3,
                elevator_states: tx_buf4,
                new_wv_after_offline: tx_buf5,
                mpsc_buffer_ch6: tx_buf6,
                mpsc_buffer_ch7: tx_buf7,
                mpsc_buffer_ch8: tx_buf8,
                mpsc_buffer_ch9: tx_buf9,
            },
            rxs: MpscRxs {
                udp_wv: rx_udp,
                tcp_to_master_failed: rx_tcp_to_master_failed,
                container: rx_container,
                remove_container: rx_remove_container,
                sent_tcp_container: rx_sent_tcp_container,
                delegated_tasks: rx_buf3,
                elevator_states: rx_buf4,
                new_wv_after_offline: rx_buf5,
                mpsc_buffer_ch6: rx_buf6,
                mpsc_buffer_ch7: rx_buf7,
                mpsc_buffer_ch8: rx_buf8,
                mpsc_buffer_ch9: rx_buf9,
            },
        }
    }
}


// --- WATCH-KANALER ---
/// Struct containing watch senders for state updates.
#[derive(Clone)]
pub struct WatchTxs {
    /// Sender for the `wv` channel, transmitting a vector of bytes.
    pub wv: watch::Sender<Vec<u8>>,
    // Sender for the `elev_task` channel, transmitting a list of tasks.
    // pub elev_task: watch::Sender<Vec<Task>>,
}

/// Struct containing watch receivers for listening to state updates.
#[derive(Clone)]
pub struct WatchRxs {
    /// Receiver for the `wv` channel, listening to a vector of bytes.
    pub wv: watch::Receiver<Vec<u8>>,
    // Receiver for the `elev_task` channel, listening to a list of tasks.
    // pub elev_task: watch::Receiver<Vec<Task>>,
}


/// Struct encapsulating both watch senders (`WatchTxs`) and receivers (`WatchRxs`).
#[derive(Clone)]
pub struct Watches {
    /// Transmitters for watch channels.
    pub txs: WatchTxs,
    /// Receivers for watch channels.
    pub rxs: WatchRxs,
}

impl Watches {
    /// Creates a new `Watches` instance with initialized watch channels.
    ///
    /// # Returns
    /// A `Watches` instance containing both senders and receivers.
    pub fn new() -> Self {
        let (wv_tx, wv_rx) = watch::channel(Vec::<u8>::new());
        // let (tx1, rx1) = watch::channel(Vec::new());

        Watches {
            txs: WatchTxs {
                wv: wv_tx,
                // elev_task: tx1,
            },
            rxs: WatchRxs {
                wv: wv_rx,
                // elev_task: rx1,
            },
        }
    }
}
