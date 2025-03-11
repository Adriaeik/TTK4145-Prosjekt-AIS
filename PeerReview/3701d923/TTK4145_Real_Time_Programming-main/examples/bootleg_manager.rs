use serde_json::Value;
use tokio::time::{sleep, Duration};
use zenoh::{open, Config};

#[tokio::main]
async fn main() {
    // Use default config (or load your custom one if desired)
    let session = open(Config::default()).await.expect("Failed to open Zenoh session");

    // Topics:
    // Elevator publishes its request to "temp/elevator1/request"
    // Manager replies to "temp/manager/elevator1/request"
    let elevator_request_topic = "stor/elevator1/request";
    let manager_response_topic = "temp/manager/elevator1/request";

    // Declare a subscriber for elevator1's request topic
    let subscriber = session
        .declare_subscriber(elevator_request_topic)
        .await
        .expect("Failed to declare subscriber for elevator1 request");

    // Declare a publisher for manager's response topic
    let publisher = session
        .declare_publisher(manager_response_topic)
        .await
        .expect("Failed to declare publisher for manager response");

    println!("Bootleg Manager started.");
    println!("Listening on: {}", elevator_request_topic);
    println!("Publishing responses on: {}", manager_response_topic);

    loop {
        match subscriber.recv_async().await {
            Ok(sample) => {
                // Convert payload to string (may include a prefix)
                let request_str = sample.payload().try_to_string().unwrap_or_else(|_| "Invalid UTF-8".into());
                println!("Received elevator request: {}", request_str);

                // If there is a prefix (e.g., "DEBUG: stor/elevator1/request: "),
                // find the first '{' and extract from there.
                let json_str = if let Some(idx) = request_str.find('{') { &request_str[idx..] } else { &request_str[..] };

                // Parse the JSON and extract "hallRequests"
                match serde_json::from_str::<Value>(json_str) {
                    Ok(json_value) => {
                        if let Some(hall_requests) = json_value.get("hallRequests") {
                            let response_str = hall_requests.to_string();
                            publisher.put(response_str.as_bytes()).await.expect("Failed to publish manager response");
                            println!("Published manager response: {}", response_str);
                        } else {
                            eprintln!("'hallRequests' field not found in JSON.");
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse JSON: {}. Raw: {}", e, json_str);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error receiving elevator request: {}", e);
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}
