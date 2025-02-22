use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use termcolor::Color;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::Byrokrati::konsulent;


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
    pub tx: broadcast::Sender<Vec<u8>>,
}

impl Clone for WorldViewChannel {
    fn clone(&self) -> Self {
        WorldViewChannel {tx: self.tx.clone()}
    }
}


impl WorldViewChannel {
    pub async fn send_worldview(&self, worldview: Arc<Mutex<Vec<u8>>>){
        loop{
            while !get_worldview_request_flag().load(Ordering::SeqCst){};
            let wv = worldview.lock().await;
            let wv_clone = wv.clone();
            if let Err(e) = self.tx.send(wv_clone){eprintln!("Feil ved sending av worldview: {} (worldviewchannel.rs, send_worldview())",e)};           
            get_worldview_request_flag().store(false, Ordering::SeqCst);
        }
    }

    pub async fn spawn_send_worldview( &self, worldview: Arc<Mutex<Vec<u8>>>, _shutdown_tx: broadcast::Sender<u8>){
        let self_clone = self.clone();
        tokio::spawn(async move {
            // Denne koden kjører i den asynkrone oppgaven (tasken)
            konsulent::print_farge("Starter å sende \"intern\" worldview på bestilling".to_string(), Color::Green);
            self_clone.send_worldview(worldview.clone()/*, shutdown_tx.clone().subscribe()*/).await;
        });
    }
}

