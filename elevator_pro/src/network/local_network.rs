use tokio::sync::{mpsc, broadcast, watch};




pub struct MpscTxs {
    pub udp_wv: mpsc::Sender<Vec<u8>>,

}

pub struct MpscRxs {
    pub udp_wv: mpsc::Receiver<Vec<u8>>,
    
}

impl Clone for MpscTxs {
    fn clone(&self) -> MpscTxs {
        return MpscTxs{
            udp_wv: self.udp_wv.clone(), 
        }
    }
}

pub struct Mpscs {
    pub txs: MpscTxs,
    pub rxs: MpscRxs,
}

impl Mpscs {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(32);
        Mpscs { 
            txs: MpscTxs { udp_wv: (tx) }, 
            rxs: MpscRxs { udp_wv: (rx) } 
        }
    }
}

/// NB!
/// Vil gjøre Rx ubrukelig
/// Ikke clon om du skal bruke RX, bruk RX til den originale!
impl Clone for Mpscs {
    fn clone(&self) -> Mpscs {
        let (_, rx) = mpsc::channel(32); // Ny kanal for klonen
        Mpscs {
            txs: self.txs.clone(), // Behaldar same sender
            rxs: MpscRxs { udp_wv: rx }, // Ny mottakar
        }
    }
}



pub struct BroadcastTxs {
    // Shutdown-kanal
    pub shutdown: broadcast::Sender<()>,

    // Kanal for å sende wv som `Vec<u8>`-meldingar
    pub wv: broadcast::Sender<Vec<u8>>,
}

pub struct BroadcastRxs {
    // Shutdown-kanal
    pub shutdown: broadcast::Receiver<()>,

    // Kanal for å sende wv som `Vec<u8>`-meldingar
    pub wv: broadcast::Receiver<Vec<u8>>,
}


impl Clone for BroadcastTxs {
    fn clone(&self) -> BroadcastTxs {
        return BroadcastTxs{
            shutdown: self.shutdown.clone(),
            wv: self.wv.clone(), 
        }
    }
}

impl BroadcastTxs {
    pub fn subscribe(&self) -> BroadcastRxs {
        BroadcastRxs {
            shutdown: self.shutdown.subscribe(),
            wv: self.wv.subscribe(),
        }
    }

}

impl BroadcastRxs {
    pub fn resubscribe(&self) -> BroadcastRxs {
        BroadcastRxs {
            shutdown: self.shutdown.resubscribe(),
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
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1); // Buffer på 10
        let (wv_tx, wv_rx) = broadcast::channel(1); // Buffer på 32

        Broadcasts {
            txs: BroadcastTxs {
                shutdown: shutdown_tx,
                wv: wv_tx,
            },
            rxs: BroadcastRxs {
                shutdown: shutdown_rx,
                wv: wv_rx,
            },
        }
    }

    /// Opprettar ein ny mottakar basert på `txs`
    pub fn subscribe(&self) -> BroadcastRxs {
        self.txs.subscribe()
    }
}

impl Clone for Broadcasts {
    fn clone(&self) -> Broadcasts {
        return Broadcasts{
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

    /// Opprettar ein ny `BroadcastRxs` abonnent
    pub fn subscribe_broadcast(&self) -> BroadcastRxs {
        self.broadcasts.subscribe()
    }
}

impl Clone for LocalChannels {
    fn clone(&self) -> LocalChannels {
        return LocalChannels{
            mpscs: self.mpscs.clone(),
            broadcasts: self.broadcasts.clone(), 
        }
    }
}


