use tokio::sync::{mpsc, broadcast};

use crate::world_view::world_view::ElevatorContainer;

// --- MPSC-KANALAR ---

pub struct MpscTxs {
    pub udp_wv: mpsc::Sender<Vec<u8>>,
    pub tcp_to_master_failed: mpsc::Sender<bool>,
    pub mpsc_buffer_ch2: mpsc::Sender<bool>,
    pub mpsc_buffer_ch3: mpsc::Sender<bool>,
    pub mpsc_buffer_ch4: mpsc::Sender<bool>,
    pub mpsc_buffer_ch5: mpsc::Sender<bool>,
}

pub struct MpscRxs {
    pub udp_wv: mpsc::Receiver<Vec<u8>>,
    pub tcp_to_master_failed: mpsc::Receiver<bool>,
    pub mpsc_buffer_ch2: mpsc::Receiver<bool>,
    pub mpsc_buffer_ch3: mpsc::Receiver<bool>,
    pub mpsc_buffer_ch4: mpsc::Receiver<bool>,
    pub mpsc_buffer_ch5: mpsc::Receiver<bool>,
}

impl Clone for MpscTxs {
    fn clone(&self) -> MpscTxs {
        MpscTxs {
            udp_wv: self.udp_wv.clone(),
            tcp_to_master_failed: self.tcp_to_master_failed.clone(),
            mpsc_buffer_ch2: self.mpsc_buffer_ch2.clone(),
            mpsc_buffer_ch3: self.mpsc_buffer_ch3.clone(),
            mpsc_buffer_ch4: self.mpsc_buffer_ch4.clone(),
            mpsc_buffer_ch5: self.mpsc_buffer_ch5.clone(),
        }
    }
}

pub struct Mpscs {
    pub txs: MpscTxs,
    pub rxs: MpscRxs,
}

impl Mpscs {
    pub fn new() -> Self {
        let (tx_udp, rx_udp) = mpsc::channel(32);
        let (tx1, rx1) = mpsc::channel(1);
        let (tx2, rx2) = mpsc::channel(1);
        let (tx3, rx3) = mpsc::channel(1);
        let (tx4, rx4) = mpsc::channel(1);
        let (tx5, rx5) = mpsc::channel(1);

        Mpscs { 
            txs: MpscTxs { 
                udp_wv: tx_udp,
                tcp_to_master_failed: tx1,
                mpsc_buffer_ch2: tx2,
                mpsc_buffer_ch3: tx3,
                mpsc_buffer_ch4: tx4,
                mpsc_buffer_ch5: tx5,
            }, 
            rxs: MpscRxs { 
                udp_wv: rx_udp,
                tcp_to_master_failed: rx1,
                mpsc_buffer_ch2: rx2,
                mpsc_buffer_ch3: rx3,
                mpsc_buffer_ch4: rx4,
                mpsc_buffer_ch5: rx5,
            }
        }
    }
}

impl Clone for Mpscs {
    fn clone(&self) -> Mpscs {
        let (_, rx_udp) = mpsc::channel(32);
        let (_, rx1) = mpsc::channel(1);
        let (_, rx2) = mpsc::channel(1);
        let (_, rx3) = mpsc::channel(1);
        let (_, rx4) = mpsc::channel(1);
        let (_, rx5) = mpsc::channel(1);

        Mpscs {
            txs: self.txs.clone(),
            rxs: MpscRxs { 
                udp_wv: rx_udp,
                tcp_to_master_failed: rx1,
                mpsc_buffer_ch2: rx2,
                mpsc_buffer_ch3: rx3,
                mpsc_buffer_ch4: rx4,
                mpsc_buffer_ch5: rx5,
            }
        }
    }
}

// --- BROADCAST-KANALAR ---

pub struct BroadcastTxs {
    pub shutdown: broadcast::Sender<()>,
    pub self_elevator_container: broadcast::Sender<ElevatorContainer>,
    pub broadcast_buffer_ch2: broadcast::Sender<bool>,
    pub broadcast_buffer_ch3: broadcast::Sender<bool>,
    pub broadcast_buffer_ch4: broadcast::Sender<bool>,
    pub broadcast_buffer_ch5: broadcast::Sender<bool>,
    pub wv: broadcast::Sender<Vec<u8>>,
}

pub struct BroadcastRxs {
    pub shutdown: broadcast::Receiver<()>,
    pub self_elevator_container: broadcast::Receiver<ElevatorContainer>,
    pub broadcast_buffer_ch2: broadcast::Receiver<bool>,
    pub broadcast_buffer_ch3: broadcast::Receiver<bool>,
    pub broadcast_buffer_ch4: broadcast::Receiver<bool>,
    pub broadcast_buffer_ch5: broadcast::Receiver<bool>,
    pub wv: broadcast::Receiver<Vec<u8>>,
}

impl Clone for BroadcastTxs {
    fn clone(&self) -> BroadcastTxs {
        BroadcastTxs {
            shutdown: self.shutdown.clone(),
            self_elevator_container: self.self_elevator_container.clone(),
            broadcast_buffer_ch2: self.broadcast_buffer_ch2.clone(),
            broadcast_buffer_ch3: self.broadcast_buffer_ch3.clone(),
            broadcast_buffer_ch4: self.broadcast_buffer_ch4.clone(),
            broadcast_buffer_ch5: self.broadcast_buffer_ch5.clone(),
            wv: self.wv.clone(),
        }
    }
}

impl BroadcastTxs {
    pub fn subscribe(&self) -> BroadcastRxs {
        BroadcastRxs {
            shutdown: self.shutdown.subscribe(),
            self_elevator_container: self.self_elevator_container.subscribe(),
            broadcast_buffer_ch2: self.broadcast_buffer_ch2.subscribe(),
            broadcast_buffer_ch3: self.broadcast_buffer_ch3.subscribe(),
            broadcast_buffer_ch4: self.broadcast_buffer_ch4.subscribe(),
            broadcast_buffer_ch5: self.broadcast_buffer_ch5.subscribe(),
            wv: self.wv.subscribe(),
        }
    }
}

impl BroadcastRxs {
    pub fn resubscribe(&self) -> BroadcastRxs {
        BroadcastRxs {
            shutdown: self.shutdown.resubscribe(),
            self_elevator_container: self.self_elevator_container.resubscribe(),
            broadcast_buffer_ch2: self.broadcast_buffer_ch2.resubscribe(),
            broadcast_buffer_ch3: self.broadcast_buffer_ch3.resubscribe(),
            broadcast_buffer_ch4: self.broadcast_buffer_ch4.resubscribe(),
            broadcast_buffer_ch5: self.broadcast_buffer_ch5.resubscribe(),
            wv: self.wv.resubscribe(),
        }
    }
}

pub struct Broadcasts {
    pub txs: BroadcastTxs,
    pub rxs: BroadcastRxs,
}

impl Broadcasts {
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (tx1, rx1) = broadcast::channel(1);
        let (tx2, rx2) = broadcast::channel(1);
        let (tx3, rx3) = broadcast::channel(1);
        let (tx4, rx4) = broadcast::channel(1);
        let (tx5, rx5) = broadcast::channel(1);
        let (wv_tx, wv_rx) = broadcast::channel(1);

        Broadcasts {
            txs: BroadcastTxs {
                shutdown: shutdown_tx,
                self_elevator_container: tx1,
                broadcast_buffer_ch2: tx2,
                broadcast_buffer_ch3: tx3,
                broadcast_buffer_ch4: tx4,
                broadcast_buffer_ch5: tx5,
                wv: wv_tx,
            },
            rxs: BroadcastRxs {
                shutdown: shutdown_rx,
                self_elevator_container: rx1,
                broadcast_buffer_ch2: rx2,
                broadcast_buffer_ch3: rx3,
                broadcast_buffer_ch4: rx4,
                broadcast_buffer_ch5: rx5,
                wv: wv_rx,
            },
        }
    }

    pub fn subscribe(&self) -> BroadcastRxs {
        self.txs.subscribe()
    }
}

impl Clone for Broadcasts {
    fn clone(&self) -> Broadcasts {
        Broadcasts {
            txs: self.txs.clone(),
            rxs: self.subscribe(),
        }
    }
}

// --- OVERKLASSE FOR ALLE KANALAR ---

pub struct LocalChannels {
    pub mpscs: Mpscs,
    pub broadcasts: Broadcasts,
}

impl LocalChannels {
    pub fn new() -> Self {
        LocalChannels {
            mpscs: Mpscs::new(),
            broadcasts: Broadcasts::new(),
        }
    }

    pub fn subscribe_broadcast(&mut self) {
        self.broadcasts.rxs = self.broadcasts.subscribe();
    }

    pub fn resubscribe_broadcast(&mut self) {
        self.broadcasts.rxs = self.broadcasts.rxs.resubscribe();
    }
}

impl Clone for LocalChannels {
    fn clone(&self) -> LocalChannels {
        LocalChannels {
            mpscs: self.mpscs.clone(),
            broadcasts: self.broadcasts.clone(),
        }
    }
}
