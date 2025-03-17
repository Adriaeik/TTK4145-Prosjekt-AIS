//! Handles messages on internal channels regarding changes in worldview

use crate::{elevio::poll::CallButton, world_view::ElevatorStatus};
use crate::print;
use crate::config;
use crate::manager::task_allocator::Task;
use tokio::sync::{mpsc, broadcast, watch, Semaphore};
use std::sync::Arc;
use crate::world_view::world_view_update::{ join_wv_from_udp, 
    abort_network, 
    join_wv_from_tcp_container, 
    remove_container, 
    recieve_local_elevator_msg, 
    clear_from_sent_tcp,
    update_elev_state,
    push_task,
    publish_tasks,
};
use crate::world_view::{self, serial};

use local_ip_address::local_ip;
use std::net::IpAddr;
use std::sync::atomic::AtomicU8;

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
            print::warn(format!("Fant ikke IP i get_self_ip() -> Vi er offline: {}", e));
            return Err(e);
        }
    };
    Ok(ip)
}


/// ### Oppdatering av lokal worldview
/// 
/// Funksjonen leser nye meldinger fra andre tasks som indikerer endring i systemet, og endrer og oppdaterer det lokale worldviewen basert på dette.
#[allow(non_snake_case)]
pub async fn update_wv_watch(mut main_local_chs: LocalChannels, mut worldview_serialised: Vec<u8>, to_task_alloc_tx: mpsc::Sender<Vec<u8>>) {
    println!("Starter update_wv");
    let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
    
    let mut wv_edited_I = false;
    let mut master_container_updated_I = false;
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
                wv_edited_I = join_wv_from_tcp_container(&mut worldview_serialised, container.clone()).await;
                let _ = to_task_alloc_tx.send(container.clone()).await;
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
            Ok((id, sometask)) => {
                // utils::print_master(format!("Fikk task: {:?}", task));
                wv_edited_I = push_task(&mut worldview_serialised, id, sometask);
            },
            Err(_) => {},
        }
        match main_local_chs.mpscs.rxs.pending_tasks.try_recv() {
            Ok(tasks) => {
                wv_edited_I = publish_tasks(&mut worldview_serialised, tasks);
            },
            Err(_) => {},
        }
        


/* KANALER MASTER OG SLAVE MOTTAR PÅ */
        /*____Får signal når en task er ferdig_____ */
        match main_local_chs.mpscs.rxs.update_elev_state.try_recv() {
            Ok(status) => {
                wv_edited_I = update_elev_state(&mut worldview_serialised, status);
                master_container_updated_I = world_view::is_master(worldview_serialised.clone());
            },
            Err(_) => {},
        }
        /*_____Knapper trykket på lokal heis_____ */
        match main_local_chs.mpscs.rxs.local_elev.try_recv() {
            Ok(msg) => {
                wv_edited_I = recieve_local_elevator_msg(main_local_chs.clone(), &mut worldview_serialised, msg).await;
                master_container_updated_I = world_view::is_master(worldview_serialised.clone());
            },
            Err(_) => {},
        }
        
        
        
        /* KANALER ALLE SENDER LOKAL WV PÅ */
        /*_____Hvis worldview er endra, oppdater kanalen_____ */
        if master_container_updated_I {
            let container = world_view::extract_self_elevator_container(worldview_serialised.clone());
            let _ = main_local_chs.mpscs.txs.container.send(serial::serialize_elev_container(&container)).await;
            master_container_updated_I = false;
        }

        if wv_edited_I {

            let _ = main_local_chs.watches.txs.wv.send(worldview_serialised.clone());
            // println!("Sendte worldview lokalt {}", worldview_serialised[1]);
    
            wv_edited_I = false;
        }
    }
}



















/// Represents different types of elevator messages.
#[derive(Debug)]
pub enum ElevMsgType {
    /// Call button press event.
    CALLBTN,
    /// Floor sensor event.
    FLOORSENS,
    /// Stop button press event.
    STOPBTN,
    /// Obstruction detected event.
    OBSTRX,
}

/// Represents a message related to elevator events.
#[derive(Debug)]
pub struct ElevMessage {
    /// The type of elevator message.
    pub msg_type: ElevMsgType,
    /// Optional call button information, if applicable.
    pub call_button: Option<CallButton>,
    /// Optional floor sensor reading, indicating the current floor.
    pub floor_sensor: Option<u8>,
    /// Optional stop button state (`true` if pressed).
    pub stop_button: Option<bool>,
    /// Optional obstruction status (`true` if obstruction detected).
    pub obstruction: Option<bool>,
}



// --- MPSC-KANALAR ---
/// Struct containing multiple MPSC (multi-producer, single-consumer) sender channels.
/// These channels are primarely used to send data to the task updating the local worldview.
#[allow(missing_docs)]
pub struct MpscTxs {
    /// Sends a UDP worldview packet.
    pub udp_wv: mpsc::Sender<Vec<u8>>,
    /// Notifies if the TCP connection to the master has failed.
    pub tcp_to_master_failed: mpsc::Sender<bool>,
    /// Sends elevator containers recieved from slaves on TCP.
    pub container: mpsc::Sender<Vec<u8>>,
    /// Requests the removal of a container by ID.
    pub remove_container: mpsc::Sender<u8>,
    /// Sends messages from the local elevator.
    pub local_elev: mpsc::Sender<ElevMessage>,
    /// Sends a TCP container message that has been transmitted to the master.
    pub sent_tcp_container: mpsc::Sender<Vec<u8>>,
    /// Sends a new task along with associated data.
    pub new_task: mpsc::Sender<(u8, Option<Task>)>,
    /// Updates the status of a task.
    pub update_elev_state: mpsc::Sender<ElevatorStatus>,
    /// Additional buffered channels for various data streams.
    pub pending_tasks: mpsc::Sender<Vec<Task>>,
    pub mpsc_buffer_ch3: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch4: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch5: mpsc::Sender<Vec<u8>>,
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
    /// Receives messages from the local elevator.
    pub local_elev: mpsc::Receiver<ElevMessage>,
    /// Receives TCP container messages that have been transmitted.
    pub sent_tcp_container: mpsc::Receiver<Vec<u8>>,
    /// Receives new tasks along with associated data.
    pub new_task: mpsc::Receiver<(u8, Option<Task>)>,
    /// Receives updates for the status of a task.
    pub update_elev_state: mpsc::Receiver<ElevatorStatus>,
    /// Additional buffered channels for various data streams.
    pub pending_tasks: mpsc::Receiver<Vec<Task>>,
    pub mpsc_buffer_ch3: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch4: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch5: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch6: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch7: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch8: mpsc::Receiver<Vec<u8>>,
    pub mpsc_buffer_ch9: mpsc::Receiver<Vec<u8>>,
}

impl Clone for MpscTxs {
    fn clone(&self) -> MpscTxs {
        MpscTxs {
            udp_wv: self.udp_wv.clone(),
            tcp_to_master_failed: self.tcp_to_master_failed.clone(),
            container: self.container.clone(),
            remove_container: self.remove_container.clone(),
            local_elev: self.local_elev.clone(),
            sent_tcp_container: self.sent_tcp_container.clone(),

            // Klonar buffer-kanalane
            new_task: self.new_task.clone(),
            update_elev_state: self.update_elev_state.clone(),
            pending_tasks: self.pending_tasks.clone(),
            mpsc_buffer_ch3: self.mpsc_buffer_ch3.clone(),
            mpsc_buffer_ch4: self.mpsc_buffer_ch4.clone(),
            mpsc_buffer_ch5: self.mpsc_buffer_ch5.clone(),
            mpsc_buffer_ch6: self.mpsc_buffer_ch6.clone(),
            mpsc_buffer_ch7: self.mpsc_buffer_ch7.clone(),
            mpsc_buffer_ch8: self.mpsc_buffer_ch8.clone(),
            mpsc_buffer_ch9: self.mpsc_buffer_ch9.clone(),
        }
    }
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
        let (tx1, rx1) = mpsc::channel(300);
        let (tx2, rx2) = mpsc::channel(300);
        let (tx3, rx3) = mpsc::channel(300);
        let (tx4, rx4) = mpsc::channel(300);
        let (tx5, rx5) = mpsc::channel(300);

        // Initialisering av 10 nye buffer-kanalar
        let (tx_buf0, rx_buf0) = mpsc::channel(300);
        let (tx_buf1, rx_buf1) = mpsc::channel(300);
        let (tx_buf2, rx_buf2) = mpsc::channel(300);
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
                tcp_to_master_failed: tx1,
                container: tx2,
                remove_container: tx3,
                local_elev: tx4,
                sent_tcp_container: tx5,

                // Legg til dei nye buffer-kanalane
                new_task: tx_buf0,
                update_elev_state: tx_buf1,
                pending_tasks: tx_buf2,
                mpsc_buffer_ch3: tx_buf3,
                mpsc_buffer_ch4: tx_buf4,
                mpsc_buffer_ch5: tx_buf5,
                mpsc_buffer_ch6: tx_buf6,
                mpsc_buffer_ch7: tx_buf7,
                mpsc_buffer_ch8: tx_buf8,
                mpsc_buffer_ch9: tx_buf9,
            },
            rxs: MpscRxs {
                udp_wv: rx_udp,
                tcp_to_master_failed: rx1,
                container: rx2,
                remove_container: rx3,
                local_elev: rx4,
                sent_tcp_container: rx5,

                // Legg til dei nye buffer-kanalane
                new_task: rx_buf0,
                update_elev_state: rx_buf1,
                pending_tasks: rx_buf2,
                mpsc_buffer_ch3: rx_buf3,
                mpsc_buffer_ch4: rx_buf4,
                mpsc_buffer_ch5: rx_buf5,
                mpsc_buffer_ch6: rx_buf6,
                mpsc_buffer_ch7: rx_buf7,
                mpsc_buffer_ch8: rx_buf8,
                mpsc_buffer_ch9: rx_buf9,
            },
        }
    }
}

impl Clone for Mpscs {
    fn clone(&self) -> Mpscs {
        let (_, rx_udp) = mpsc::channel(300);
        let (_, rx1) = mpsc::channel(300);
        let (_, rx2) = mpsc::channel(300);
        let (_, rx3) = mpsc::channel(300);
        let (_, rx4) = mpsc::channel(300);
        let (_, rx5) = mpsc::channel(300);

        // Initialiser mottakar-kanalane ved cloning
        let (_, rx_buf0) = mpsc::channel(300);
        let (_, rx_buf1) = mpsc::channel(300);
        let (_, rx_buf2) = mpsc::channel(300);
        let (_, rx_buf3) = mpsc::channel(300);
        let (_, rx_buf4) = mpsc::channel(300);
        let (_, rx_buf5) = mpsc::channel(300);
        let (_, rx_buf6) = mpsc::channel(300);
        let (_, rx_buf7) = mpsc::channel(300);
        let (_, rx_buf8) = mpsc::channel(300);
        let (_, rx_buf9) = mpsc::channel(300);

        Mpscs {
            txs: self.txs.clone(),
            rxs: MpscRxs {
                udp_wv: rx_udp,
                tcp_to_master_failed: rx1,
                container: rx2,
                remove_container: rx3,
                local_elev: rx4,
                sent_tcp_container: rx5,

                // Klonar buffer-kanalane
                new_task: rx_buf0,
                update_elev_state: rx_buf1,
                pending_tasks: rx_buf2,
                mpsc_buffer_ch3: rx_buf3,
                mpsc_buffer_ch4: rx_buf4,
                mpsc_buffer_ch5: rx_buf5,
                mpsc_buffer_ch6: rx_buf6,
                mpsc_buffer_ch7: rx_buf7,
                mpsc_buffer_ch8: rx_buf8,
                mpsc_buffer_ch9: rx_buf9,
            },
        }
    }
}


// --- BROADCAST-KANALAR ---

/// Contains broadcast senders for various events and channels.
pub struct BroadcastTxs {
    /// Sender for signaling system shutdown.
    pub shutdown: broadcast::Sender<()>,
    /// Sender for broadcasting messages on buffer channel 1.
    pub broadcast_buffer_ch1: broadcast::Sender<bool>,
    /// Sender for broadcasting messages on buffer channel 2.
    pub broadcast_buffer_ch2: broadcast::Sender<bool>,
    /// Sender for broadcasting messages on buffer channel 3.
    pub broadcast_buffer_ch3: broadcast::Sender<bool>,
    /// Sender for broadcasting messages on buffer channel 4.
    pub broadcast_buffer_ch4: broadcast::Sender<bool>,
    /// Sender for broadcasting messages on buffer channel 5.
    pub broadcast_buffer_ch5: broadcast::Sender<bool>,
}

/// Contains broadcast receivers for various events and channels.
pub struct BroadcastRxs {
    /// Receiver for system shutdown signals.
    pub shutdown: broadcast::Receiver<()>,
    /// Receiver for messages on buffer channel 1.
    pub broadcast_buffer_ch1: broadcast::Receiver<bool>,
    /// Receiver for messages on buffer channel 2.
    pub broadcast_buffer_ch2: broadcast::Receiver<bool>,
    /// Receiver for messages on buffer channel 3.
    pub broadcast_buffer_ch3: broadcast::Receiver<bool>,
    /// Receiver for messages on buffer channel 4.
    pub broadcast_buffer_ch4: broadcast::Receiver<bool>,
    /// Receiver for messages on buffer channel 5.
    pub broadcast_buffer_ch5: broadcast::Receiver<bool>,
}

impl Clone for BroadcastTxs {
    fn clone(&self) -> BroadcastTxs {
        BroadcastTxs {
            shutdown: self.shutdown.clone(),
            broadcast_buffer_ch1: self.broadcast_buffer_ch1.clone(),
            broadcast_buffer_ch2: self.broadcast_buffer_ch2.clone(),
            broadcast_buffer_ch3: self.broadcast_buffer_ch3.clone(),
            broadcast_buffer_ch4: self.broadcast_buffer_ch4.clone(),
            broadcast_buffer_ch5: self.broadcast_buffer_ch5.clone(),
        }
    }
}

impl BroadcastTxs {
    /// Creates a new set of receivers (`BroadcastRxs`) subscribing to the current senders.
    ///
    /// # Returns
    /// A `BroadcastRxs` instance that listens to all broadcast channels.
    pub fn subscribe(&self) -> BroadcastRxs {
        BroadcastRxs {
            shutdown: self.shutdown.subscribe(),
            broadcast_buffer_ch1: self.broadcast_buffer_ch1.subscribe(),
            broadcast_buffer_ch2: self.broadcast_buffer_ch2.subscribe(),
            broadcast_buffer_ch3: self.broadcast_buffer_ch3.subscribe(),
            broadcast_buffer_ch4: self.broadcast_buffer_ch4.subscribe(),
            broadcast_buffer_ch5: self.broadcast_buffer_ch5.subscribe(),
        }
    }
}

impl BroadcastRxs {
    /// Resubscribes to all broadcast channels, creating new receivers.
    ///
    /// # Returns
    /// A fresh `BroadcastRxs` instance with new subscriptions.
    pub fn resubscribe(&self) -> BroadcastRxs {
        BroadcastRxs {
            shutdown: self.shutdown.resubscribe(),
            broadcast_buffer_ch1: self.broadcast_buffer_ch1.resubscribe(),
            broadcast_buffer_ch2: self.broadcast_buffer_ch2.resubscribe(),
            broadcast_buffer_ch3: self.broadcast_buffer_ch3.resubscribe(),
            broadcast_buffer_ch4: self.broadcast_buffer_ch4.resubscribe(),
            broadcast_buffer_ch5: self.broadcast_buffer_ch5.resubscribe(),
        }
    }
}


/// Encapsulates both broadcast senders (`BroadcastTxs`) and receivers (`BroadcastRxs`).
pub struct Broadcasts {
    /// Transmitters for broadcasting messages.
    pub txs: BroadcastTxs,
    /// Receivers for listening to broadcasted messages.
    pub rxs: BroadcastRxs,
}

impl Broadcasts {
    /// Creates a new `Broadcasts` instance with initialized channels.
    ///
    /// # Returns
    /// A `Broadcasts` instance containing senders and receivers.
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (tx1, rx1) = broadcast::channel(1);
        let (tx2, rx2) = broadcast::channel(1);
        let (tx3, rx3) = broadcast::channel(1);
        let (tx4, rx4) = broadcast::channel(1);
        let (tx5, rx5) = broadcast::channel(1);

        Broadcasts {
            txs: BroadcastTxs {
                shutdown: shutdown_tx,
                broadcast_buffer_ch1: tx1,
                broadcast_buffer_ch2: tx2,
                broadcast_buffer_ch3: tx3,
                broadcast_buffer_ch4: tx4,
                broadcast_buffer_ch5: tx5,
            },
            rxs: BroadcastRxs {
                shutdown: shutdown_rx,
                broadcast_buffer_ch1: rx1,
                broadcast_buffer_ch2: rx2,
                broadcast_buffer_ch3: rx3,
                broadcast_buffer_ch4: rx4,
                broadcast_buffer_ch5: rx5,
            },
        }
    }

    /// Subscribes to all broadcast channels.
    ///
    /// # Returns
    /// A new `BroadcastRxs` instance listening to all channels.
    pub fn subscribe(&self) -> BroadcastRxs {
        self.txs.subscribe()
    }
}

impl Clone for Broadcasts {
    fn clone(&self) -> Broadcasts {
        Broadcasts {
            txs: self.txs.clone(),
            rxs: self.rxs.resubscribe(),
        }
    }
}

// --- WATCH-KANALER ---
/// Struct containing watch senders for broadcasting state updates.
pub struct WatchTxs {
    /// Sender for the `wv` channel, transmitting a vector of bytes.
    pub wv: watch::Sender<Vec<u8>>,
    /// Sender for the `elev_task` channel, transmitting a list of tasks.
    pub elev_task: watch::Sender<Vec<Task>>,
    /// Boolean sender for `watch_buffer_ch2`.
    pub watch_buffer_ch2: watch::Sender<bool>,
    /// Boolean sender for `watch_buffer_ch3`.
    pub watch_buffer_ch3: watch::Sender<bool>,
    /// Boolean sender for `watch_buffer_ch4`.
    pub watch_buffer_ch4: watch::Sender<bool>,
    /// Boolean sender for `watch_buffer_ch5`.
    pub watch_buffer_ch5: watch::Sender<bool>,
}

impl Clone for WatchTxs {
    /// Clones the `WatchTxs` instance, creating new handles to the same watch channels.
    fn clone(&self) -> WatchTxs {
        WatchTxs {
            wv: self.wv.clone(),
            elev_task: self.elev_task.clone(),
            watch_buffer_ch2: self.watch_buffer_ch2.clone(),
            watch_buffer_ch3: self.watch_buffer_ch3.clone(),
            watch_buffer_ch4: self.watch_buffer_ch4.clone(),
            watch_buffer_ch5: self.watch_buffer_ch5.clone(),
        }
    }
}

/// Struct containing watch receivers for listening to state updates.
pub struct WatchRxs {
    /// Receiver for the `wv` channel, listening to a vector of bytes.
    pub wv: watch::Receiver<Vec<u8>>,
    /// Receiver for the `elev_task` channel, listening to a list of tasks.
    pub elev_task: watch::Receiver<Vec<Task>>,
    /// Boolean receiver for `watch_buffer_ch2`.
    pub watch_buffer_ch2: watch::Receiver<bool>,
    /// Boolean receiver for `watch_buffer_ch3`.
    pub watch_buffer_ch3: watch::Receiver<bool>,
    /// Boolean receiver for `watch_buffer_ch4`.
    pub watch_buffer_ch4: watch::Receiver<bool>,
    /// Boolean receiver for `watch_buffer_ch5`.
    pub watch_buffer_ch5: watch::Receiver<bool>,
}

impl Clone for WatchRxs {
    /// Clones the `WatchRxs` instance, creating new handles to the same watch channels.
    fn clone(&self) -> WatchRxs {
        WatchRxs {
            wv: self.wv.clone(),
            elev_task: self.elev_task.clone(),
            watch_buffer_ch2: self.watch_buffer_ch2.clone(),
            watch_buffer_ch3: self.watch_buffer_ch3.clone(),
            watch_buffer_ch4: self.watch_buffer_ch4.clone(),
            watch_buffer_ch5: self.watch_buffer_ch5.clone(),
        }
    }
}


/// Struct encapsulating both watch senders (`WatchTxs`) and receivers (`WatchRxs`).
pub struct Watches {
    /// Transmitters for watch channels.
    pub txs: WatchTxs,
    /// Receivers for watch channels.
    pub rxs: WatchRxs,
}

impl Clone for Watches {
    /// Clones the `Watches` instance, ensuring the new instance subscribes to the channels.
    fn clone(&self) -> Watches {
        Watches {
            txs: self.txs.clone(),
            rxs: self.rxs.clone(),
        }
    }
}

impl Watches {
    /// Creates a new `Watches` instance with initialized watch channels.
    ///
    /// # Returns
    /// A `Watches` instance containing both senders and receivers.
    pub fn new() -> Self {
        let (wv_tx, wv_rx) = watch::channel(Vec::<u8>::new());
        let (tx1, rx1) = watch::channel(Vec::new());
        let (tx2, rx2) = watch::channel(false);
        let (tx3, rx3) = watch::channel(false);
        let (tx4, rx4) = watch::channel(false);
        let (tx5, rx5) = watch::channel(false);

        Watches {
            txs: WatchTxs {
                wv: wv_tx,
                elev_task: tx1,
                watch_buffer_ch2: tx2,
                watch_buffer_ch3: tx3,
                watch_buffer_ch4: tx4,
                watch_buffer_ch5: tx5,
            },
            rxs: WatchRxs {
                wv: wv_rx,
                elev_task: rx1,
                watch_buffer_ch2: rx2,
                watch_buffer_ch3: rx3,
                watch_buffer_ch4: rx4,
                watch_buffer_ch5: rx5,
            },
        }
    }
}

// --- SEMAPHORE-KANALAR ---
pub struct Semaphores {
    pub tcp_sent: Arc<Semaphore>,
    pub sem_buffer: Arc<Semaphore>,
}

impl Semaphores {
    pub fn new() -> Self {
        Semaphores {
            tcp_sent: Arc::new(Semaphore::new(10)),
            sem_buffer: Arc::new(Semaphore::new(5)),
        }
    }
}

impl Clone for Semaphores {
    fn clone(&self) -> Semaphores {
        Semaphores {
            tcp_sent: self.tcp_sent.clone(),
            sem_buffer: self.sem_buffer.clone(),
        }
    }
}


// --- OVERKLASSE FOR ALLE KANALAR ---


/// Struct containing various communication mechanisms for local inter-thread messaging.
pub struct LocalChannels {
    /// Multi-producer, single-consumer channels.
    pub mpscs: Mpscs,
    /// Broadcast channels for multi-receiver communication.
    pub broadcasts: Broadcasts,
    /// Watch channels for state tracking.
    pub watches: Watches,
    /// Semaphores for synchronization.
    pub semaphores: Semaphores,
}

impl LocalChannels {
    /// Creates a new instance of `LocalChannels` with all channels initialized.
    ///
    /// # Returns
    /// A `LocalChannels` instance with `Mpscs`, `Broadcasts`, `Watches`, and `Semaphores`.
    pub fn new() -> Self {
        LocalChannels {
            mpscs: Mpscs::new(),
            broadcasts: Broadcasts::new(),
            watches: Watches::new(),
            semaphores: Semaphores::new(),
        }
    }

    /// Subscribes to the broadcast channels, updating the receiver set.
    ///
    /// This function should be called when a new receiver needs to listen to broadcasts.
    pub fn subscribe_broadcast(&mut self) {
        self.broadcasts.rxs = self.broadcasts.subscribe();
    }

    /// Resubscribes to the broadcast channels, refreshing the receiver set.
    ///
    /// This function should be called when existing broadcast receivers need to be updated.
    pub fn resubscribe_broadcast(&mut self) {
        self.broadcasts.rxs = self.broadcasts.rxs.resubscribe();
    }
}

impl Clone for LocalChannels {
    fn clone(&self) -> LocalChannels {
        LocalChannels {
            mpscs: self.mpscs.clone(),
            broadcasts: self.broadcasts.clone(),
            watches: self.watches.clone(),
            semaphores: self.semaphores.clone(),
        }
    }
}
