pub mod fsm;
mod lights;
mod self_elevator;

use tokio::task::yield_now;
use tokio::sync::mpsc;
use tokio::sync::watch;
use crate::elevio;


pub async fn run_local_elevator(wv_watch_rx: watch::Receiver<Vec<u8>>, local_elev_tx: mpsc::Sender<elevio::ElevMessage>) {

    let elevator = self_elevator::init(local_elev_tx).await;



    // Task som utfører deligerte tasks (ikke implementert korrekt enda)
    {
        let _handle_task = tokio::spawn(async move {
            // let _ = task_handler::execute_tasks(wv_watch_rx, update_elev_state_tx, elevator).await;
        });
    }  

    {
        let e = elevator.clone();
        let _lights_task = tokio::spawn(async move {
            let _ = lights::set_lights(wv_watch_rx, e);
        });
    }  





    loop {
        yield_now().await;
    }
    
}