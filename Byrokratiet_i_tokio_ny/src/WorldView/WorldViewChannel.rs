use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use std::sync::Arc;


static WV_CH_REQ: OnceLock<AtomicBool> = OnceLock::new(); // worldview_channel_request

pub fn get_worldview_request_flag() -> &'static AtomicBool {
    WV_CH_REQ.get_or_init(|| AtomicBool::new(false))
}




pub async fn request_worldview() {
    /*skru på atomic bool fra hvor som helst jippi */

    if get_worldview_request_flag().load(Ordering::SeqCst) {
        return;
    };
    
    get_worldview_request_flag().store(true, Ordering::SeqCst);


}


pub struct WorldViewChannel {
    pub tx: Arc<broadcast::Sender<Vec<u8>>>,
}

impl Clone for WorldViewChannel {
    fn clone(&self) -> Self {
        WorldViewChannel {tx: self.tx.clone()}
    }
}


impl WorldViewChannel {
    pub async fn send_worldview(
        &self,
        worldview: Arc<Mutex<Vec<u8>>>,
        mut shutdown_rx: broadcast::Receiver<()>, // 🔹 Legg til shutdown-kanal
    ) {
        loop {
            tokio::select! {
                // 🔹 Vent på at flagget blir `true`
                _ = async {
                    while !get_worldview_request_flag().load(Ordering::SeqCst) {
                        tokio::task::yield_now().await; // 🔹 Gjer CPU tilbake til Tokio
                    }
                } => {},
    
                // 🔹 Shutdown-melding
                _ = shutdown_rx.recv() => {
                    println!("send_worldview() mottok shutdown-signal!");
                    break;
                }
            }
    
            // 🔹 Send worldview på tx
            let wv = worldview.lock().await;
            let wv_cloned = wv.clone();
            if let Err(e) = self.tx.send(wv_cloned) {
                eprintln!(
                    "Feil ved sending av worldview: {} (worldviewchannel.rs, send_worldview())",
                    e
                );
            }
    
            get_worldview_request_flag().store(false, Ordering::SeqCst);
        }
    }

    pub async fn spawn_send_worldview( &self, worldview: Arc<Mutex<Vec<u8>>>, shutdown_tx: broadcast::Sender<()>){
        let self_clone = self.clone();
        tokio::spawn(async move {
            // Denne koden kjører i den asynkrone oppgaven (tasken)
            println!("Starter å sende wolrdview");
            self_clone.send_worldview(worldview.clone(), shutdown_tx.clone().subscribe()).await;
        });
    }
}

