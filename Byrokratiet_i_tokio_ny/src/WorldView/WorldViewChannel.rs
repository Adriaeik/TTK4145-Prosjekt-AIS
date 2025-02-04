use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use crossbeam_channel::Receiver;
use tokio::sync::mpsc;
use tokio::task;
use std::task::{Context, Poll};
use std::pin::Pin;
use futures::future::poll_fn;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use std::sync::Arc;


static worldview_channel_request: OnceLock<AtomicBool> = OnceLock::new();
static worldview_channel_pending: OnceLock<AtomicBool> = OnceLock::new();

pub fn get_worldview_request_flag() -> &'static AtomicBool {
    worldview_channel_request.get_or_init(|| AtomicBool::new(false))
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

impl WorldViewChannel {
    pub async fn send_worldview(&self, worldview: Arc<Mutex<Vec<u8>>>) {

        loop {
            while !get_worldview_request_flag().load(Ordering::SeqCst) {};
            //Send worldview på tx             
            let wv = worldview.lock().await;
            let wv_cloned = wv.clone();
            if let Err(e) = self.tx.send(wv_cloned) {eprintln!("Feil ved sending av worldview: {} (worldviewchannel.rs, send_worldview())", e);}
            get_worldview_request_flag().store(false, Ordering::SeqCst);
        }
    }

}

