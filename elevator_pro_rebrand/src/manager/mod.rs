use std::{collections::HashMap, time::{Duration, Instant}};
use tokio::{sync::{mpsc, watch}, time::sleep};
use std::process::Command;
use crate::{config, world_view};

mod json_serial;



pub async fn start_manager(wv_watch_rx: watch::Receiver<Vec<u8>>, delegated_tasks_tx: mpsc::Sender<HashMap<u8, Vec<[bool; 2]>>>) {
    let mut wv = world_view::get_wv(wv_watch_rx.clone());
    loop {
        if world_view::update_wv(wv_watch_rx.clone(), &mut wv).await {
            if world_view::is_master(wv.clone()) {
                let _ = delegated_tasks_tx.send(get_elev_tasks(wv.clone()).await).await;
            }else {
                sleep(config::SLAVE_TIMEOUT).await;
            }
        } 

        sleep(config::POLL_PERIOD).await;
    }
}



// async fn get_elev_tasks(wv: Vec<u8>) -> HashMap<u8, Vec<[bool; 2]>> {
//     let json_str = json_serial::create_hall_request_json(wv).await;
//     // println!("json_str: {}", json_str.clone());
//     if let Some(str) = json_str {
//         let json_cost_str = json_serial::run_cost_algorithm(str).await;
//         return serde_json::from_str(&json_cost_str).expect("Faild to deserialize_json_to_map");
//     }
//     return HashMap::new()
//     // println!("json_cost_str: {}", json_cost_str.clone());
// }


//Ditta fjerna Panicken, men vi krasjer enda npr vi starter under 0 etasje
async fn get_elev_tasks(wv: Vec<u8>) -> HashMap<u8, Vec<[bool; 2]>> {
    let json_str = json_serial::create_hall_request_json(wv).await;

    if let Some(str) = json_str {
        let json_cost_str = json_serial::run_cost_algorithm(str.clone()).await;
        
        if json_cost_str.trim().is_empty() {
            eprintln!("⚠️ run_cost_algorithm returnerte tom streng!, vi sendte {}", str);
            return HashMap::new();
        }

        return serde_json::from_str(&json_cost_str)
            .unwrap_or_else(|e| {
                eprintln!("❌ JSON-parsing feila: {}", e);
                HashMap::new()
            });
    }

    eprintln!("⚠️ create_hall_request_json returnerte None! string inn:");
    HashMap::new()
}