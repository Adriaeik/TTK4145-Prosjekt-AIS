use serde_json::json;
use std::env;

use tokio::process::Command;

#[tokio::main]
async fn main() {
    run_algortihm().await;
    loop {}
}

async fn run_algortihm() {
    let json = json!({
        "hallRequests" :
            [[false,false],[true,false],[false,false],[false,true]],
        "states" : {
            "one" : {
                "behaviour":"moving",
                "floor":2,
                "direction":"up",
                "cabRequests":[false,false,true,true]
            },
            "two" : {
                "behaviour":"idle",
                "floor":0,
                "direction":"stop",
                "cabRequests":[false,false,false,false]
            }
        }
    });

    let cur_dir = env::current_dir().unwrap();
    let algorithm_path = cur_dir
        .join("Project-resources")
        .join("cost_fns")
        .join("hall_request_assigner")
        .join("hall_request_assigner");

    let output = Command::new(algorithm_path)
        .arg("--input")
        .arg(json.to_string())
        .output()
        .await
        .expect("Could not start algorithm");

    println!("{:?}", String::from_utf8_lossy(&output.stdout));
}
