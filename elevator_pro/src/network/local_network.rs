use tokio::sync::{mpsc, broadcast, watch};




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


