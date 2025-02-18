use tokio::sync::{mpsc, broadcast, watch};
use std::sync::Arc;

// #[derive(Clone, Copy, PartialEq, Debug)]
// pub struct RxChs{
//     // Watchdog-kanal (ein-til-ein, høg frekvens, rask sjekk)
//     pub watchdog_rx: mpsc::Receiver<()>,

//     // Shutdown-kanal (broadcast, fleire må lytte etter shutdown-signal)
//     pub shutdown_rx: broadcast::Receiver<()>,

//     // TCP-meldingar (ein-til-ein, asynkrone)
//     pub tcp_rx: mpsc::Receiver<String>,

//     // UDP-meldingar (broadcast, fleire kan abonnere på UDP-meldingar)
//     pub udp_rx: broadcast::Receiver<Vec<u8>>,

//     // Worldview deling (watch, berre den siste tilstanden er viktig)
//     pub wv_rx: watch::Receiver<String>,
// }

// #[derive(Debug, Clone)]
// pub struct TxChs {
//     pub watchdog_tx: mpsc::Sender<()>,
//     pub shutdown_tx: broadcast::Sender<()>,
//     pub tcp_tx: mpsc::Sender<String>,
//     pub udp_tx: broadcast::Sender<Vec<u8>>,
//     pub wv_tx: watch::Sender<String>,
// }

// pub fn create_channels() -> (TxChs, RxChs) {
//     let (watchdog_tx, watchdog_rx) = mpsc::channel(100);
//     let (shutdown_tx, shutdown_rx) = broadcast::channel(10);
//     let (tcp_tx, tcp_rx) = mpsc::channel(100);
//     let (udp_tx, udp_rx) = broadcast::channel(100);
//     let (wv_tx, wv_rx) = watch::channel("Initial Worldview".to_string());

//     let tx_channels = TxChs {
//         watchdog_tx,
//         shutdown_tx,
//         tcp_tx,
//         udp_tx,
//         wv_tx,
//     };

//     let rx_channels = RxChs {
//         watchdog_rx,
//         shutdown_rx,
//         tcp_rx,
//         udp_rx,
//         wv_rx,
//     };

//     (tx_channels, rx_channels)
// }


//     // let (tx_channels, mut rx_channels) = create_channels();




pub struct BroadcastRxChs{
    // Shutdown-kanal (broadcast, fleire må lytte etter shutdown-signal)
    pub shutdown_rx: broadcast::Receiver<()>,
    pub wv_rx: broadcast::Receiver<(Vec<u8>)>,
}

impl Clone for BroadcastRxChs {
    fn clone(&self) -> BroadcastRxChs {
        BroadcastRxChs(
            shutdown_rx = self.shutdown_rx.clone().resubscribe(),
            wv_rx = self.wv_rx.clone().resubscribe(), 
        )
    }
}