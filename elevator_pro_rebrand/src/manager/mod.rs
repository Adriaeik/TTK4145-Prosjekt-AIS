use std::collections::HashMap;
use serde::{Serialize, Deserialize};

mod json_serial;


pub async fn get_elev_tasks(wv: Vec<u8>) -> HashMap<u8, Vec<[bool; 2]>> {
    let json_str = json_serial::run_cost_algorithm(json_serial::create_hall_request_json(wv).await).await;
    serde_json::from_str(&json_str).expect("Faild to deserialize_json_to_map")
}
