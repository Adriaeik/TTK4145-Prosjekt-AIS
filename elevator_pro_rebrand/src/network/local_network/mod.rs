//! This module handles messages on internal channels related to changes in the worldview. 
//! It is primarily responsible for updating the worldview based on information received from other parts of the program, 
//! such as the UDP-network, TCP-network, local elevator and network manager.
//! 
//! The module also includes a MPSC-struct, making it easier to initialize all the channels used for passing this information.
mod update_wv;

use crate::print;
use crate::world_view::{ElevatorContainer, WorldView};

use update_wv::{ 
    join_wv_from_udp, 
    abort_network, 
    join_wv_from_tcp_container, 
    remove_container,
    clear_from_sent_tcp,
    distribute_tasks,
    update_elev_states,
    merge_wv_after_offline,
};
use crate::world_view::{self};

use tokio::sync::{mpsc, watch};
use std::collections::HashMap;





/// The function that updates the worldview watch.
/// 
/// # Note
/// It is **critical** that this function is run. This is the "heart" of the local system, 
/// and is responsible in updating the worldview based on information recieved form other parts of the program.
#[allow(non_snake_case)]
pub async fn update_wv_watch(mut mpsc_rxs: MpscRxs, worldview_watch_tx: watch::Sender<WorldView>, mut worldview: &mut WorldView) {
    let _ = worldview_watch_tx.send(worldview.clone());
    
    let mut wv_edited_I = false;
    let mut master_container_updated_I = false;

    let (master_container_tx, mut master_container_rx) = mpsc::channel::<ElevatorContainer>(100);    
    loop {

/* CHANNELS SLAVE MAINLY RECIEVES ON */
        /*_____Update worldview based on information send on TCP_____ */
        match mpsc_rxs.sent_tcp_container.try_recv() {
            Ok(msg) => {
                wv_edited_I = clear_from_sent_tcp(&mut worldview, msg);
            },
            Err(_) => {},
        }
        /*_____Update worldview based on worldviews recieved on UDP_____ */
        match mpsc_rxs.udp_wv.try_recv() {
            Ok(mut master_wv) => {
                wv_edited_I = join_wv_from_udp(&mut worldview, &mut master_wv);
            },
            Err(_) => {}, 
        }
        /*_____Update worldview when tcp to master has failed_____ */
        match mpsc_rxs.connection_to_master_failed.try_recv() {
            Ok(_) => {
                wv_edited_I = abort_network(&mut worldview);
            },
            Err(_) => {},
        }
        
        
/* CHANNELS MASTER MAINLY RECIEVES ON */
        /*_____Update worldview based on message from master (simulated TCP message, so the master treats its own elevator as a slave)_____*/
        match master_container_rx.try_recv() {
            Ok(container) => {
                wv_edited_I = join_wv_from_tcp_container(&mut worldview, &container).await;
            },
            Err(_) => {},
        }
        /*_____Update worldview based on message from slave_____*/
        match mpsc_rxs.container.try_recv() {
            Ok(container) => {
                wv_edited_I = join_wv_from_tcp_container(&mut worldview, &container).await;
            },
            Err(_) => {},
        }
        /*_____Update worldview when a slave should be removed_____ */
        match mpsc_rxs.remove_container.try_recv() {
            Ok(id) => {
                wv_edited_I = remove_container(&mut worldview, id); 
            },
            Err(_) => {},
        }
        /*_____Update worldview when new tasks has been given_____ */
        match mpsc_rxs.delegated_tasks.try_recv() {
            Ok(map) => {
                wv_edited_I = distribute_tasks(&mut worldview, map);
            },
            Err(_) => {},
        }        


/* CHANNELS MASTER AND SLAVE RECIEVES ON */
        /*____Update worldview based on changes in the local elevator_____ */
        match mpsc_rxs.elevator_states.try_recv() {
            Ok(container) => {
                wv_edited_I = update_elev_states(&mut worldview, container);
                master_container_updated_I = world_view::is_master(&worldview);
            },
            Err(_) => {},
        }
        /*_____Update worldview after you reconeccted to internet  */
        match mpsc_rxs.new_wv_after_offline.try_recv() {
            Ok(mut read_wv) => {
                merge_wv_after_offline(&mut worldview, &mut read_wv);
                let _ = worldview_watch_tx.send(worldview.clone());
            },
            Err(_) => {},
        }
        
        
        
        /*_____If master container has changed, send the container on master_container_tx_____ */
        if master_container_updated_I {
            if let Some(container) = world_view::extract_self_elevator_container(&worldview) {
                let _ = master_container_tx.send(container.clone()).await;
            } else {
                print::warn(format!("Failed to extract self elevator container – skipping update"));
            }
            master_container_updated_I = false;
        }
        
        /* UPDATE WORLDVIEW WATCH */
        if wv_edited_I {
            let _ = worldview_watch_tx.send(worldview.clone());
            wv_edited_I = false;
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
    pub udp_wv: mpsc::Sender<WorldView>,

    /// Notifies if the TCP connection to the master has failed.
    pub connection_to_master_failed: mpsc::Sender<bool>,

    /// Sends elevator containers recieved from slaves on TCP.
    pub container: mpsc::Sender<ElevatorContainer>,

    /// Requests the removal of a container by ID.
    pub remove_container: mpsc::Sender<u8>,

    /// Sends a TCP container message that has been transmitted to the master.
    pub sent_tcp_container: mpsc::Sender<ElevatorContainer>,

    /// Sends delegated tasks from the manager
    pub delegated_tasks: mpsc::Sender<HashMap<u8, Vec<[bool; 2]>>>,

    /// Send ElevatorContainer from the local elevator handler
    pub elevator_states: mpsc::Sender<ElevatorContainer>,

    /// Sends the new worldview after reconnecting to the network
    pub new_wv_after_offline: mpsc::Sender<WorldView>,

    /// Additional buffer channels that can be used if necessary
    pub mpsc_buffer_ch6: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch7: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch8: mpsc::Sender<Vec<u8>>,
    pub mpsc_buffer_ch9: mpsc::Sender<Vec<u8>>,
}

/// Struct containing multiple MPSC (multi-producer, single-consumer) receiver channels.
/// These channels are used to receive data from different parts of the system.
#[allow(missing_docs)]
pub struct MpscRxs {
    /// Recieves a UDP worldview packet.
    pub udp_wv: mpsc::Receiver<WorldView>,

    /// Recieves a notification if the TCP connection to the master has failed.
    pub connection_to_master_failed: mpsc::Receiver<bool>,

    /// Recieves elevator containers recieved from slaves on TCP.
    pub container: mpsc::Receiver<ElevatorContainer>,

    /// Recieves requests to remove a container by ID.
    pub remove_container: mpsc::Receiver<u8>,

    /// Recieves TCP container messages that have been transmitted.
    pub sent_tcp_container: mpsc::Receiver<ElevatorContainer>,

    /// Recieves delegated tasks from the manager
    pub delegated_tasks: mpsc::Receiver<HashMap<u8, Vec<[bool; 2]>>>,

    /// Recieves ElevatorContainer from the local elevator handler
    pub elevator_states: mpsc::Receiver<ElevatorContainer>,

    /// Recieves new worldview after reconnecting to the network 
    pub new_wv_after_offline: mpsc::Receiver<WorldView>,

    /// Additional buffer channels which can be used if necessary
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
        let (tx_connection_to_master_failed, rx_connection_to_master_failed) = mpsc::channel(300);
        let (tx_container, rx_container) = mpsc::channel(300);
        let (tx_remove_container, rx_remove_container) = mpsc::channel(300);
        let (tx_sent_tcp_container, rx_sent_tcp_container) = mpsc::channel(300);
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
                connection_to_master_failed: tx_connection_to_master_failed,
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
                connection_to_master_failed: rx_connection_to_master_failed,
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

