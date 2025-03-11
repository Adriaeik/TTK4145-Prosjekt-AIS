use std::{env, sync::Arc};

use tokio::process::Command;
use tokio::sync::RwLock;

use elevator_system::distributed_systems::utils::parse_message_to_string;
use elevator_system::elevator_algorithm::utils::*;

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(AlgoInput::new()));
    let update_flag = Arc::new(RwLock::new(false));

    let session = zenoh::open(zenoh::Config::default()).await.expect("Failed to open zenoh session.");

    for i in 0..5 {
        let state_clone = Arc::clone(&state);
        let flag_clone = Arc::clone(&update_flag);

        let id = i;
        let key = format!("stor/elevator{}/request", id);
        let subscriber = session.declare_subscriber(key).await.unwrap();

        tokio::spawn(async move {
            while let Ok(sample) = subscriber.recv_async().await {
                let msg = parse_message_to_string(sample);
                println!("Received: {}", msg);
                println!();
                let msg: ElevMsg = serde_json::from_str(&msg).expect("Could not convert to ElevMsg");
                let (hall_req, elev_state) = ElevState::from_elevmsg(msg);
                let hash_key = format!("elevator{}", id);

                {
                    let mut data = state_clone.write().await;
                    data.hallRequests = hall_req;
                    data.states.insert(hash_key, elev_state);
                }
                {
                    let mut flag = flag_clone.write().await;
                    *flag = true;
                }
            }
        });
    }

    loop {
        let run = {
            let flag = update_flag.read().await;
            *flag
        };

        if run {
            let temp_state = {
                let state_lock = state.read().await;
                state_lock.clone()
            };

            run_algorithm(&temp_state).await;

            {
                let mut flag = update_flag.write().await;
                *flag = false;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

async fn run_algorithm(input: &AlgoInput) -> Vec<Vec<Vec<bool>>> {
    let json_string = serde_json::to_string(input).expect("Could not convert AlgoInput to string");
    println!("AlgoInput to string:{}", json_string);
    println!();

    let algorithm_path = env::current_dir()
        .unwrap()
        .join("Project-resources")
        .join("cost_fns")
        .join("hall_request_assigner")
        .join("hall_request_assigner");

    let output = Command::new(algorithm_path)
        .arg("--input")
        .arg(json_string)
        .output()
        .await
        .expect("Could not start algorithm");

    let output = String::from_utf8_lossy(&output.stdout).into_owned();
    println!("Output: {}", output);
    println!();
    serde_json::from_str(&output).expect("Could not convert sting to json")
}
