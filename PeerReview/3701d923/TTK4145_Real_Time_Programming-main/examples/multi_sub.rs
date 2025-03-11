use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};
use zenoh::handlers::FifoChannelHandler;
use zenoh::pubsub::Subscriber;
use zenoh::sample::Sample;
use zenoh::{open, Config};

// Topics for subscribing
const INT_TOPIC: &str = "stor/int";
const STRING_TOPIC: &str = "stor/string";
const ARRAY_TOPIC: &str = "stor/array";

#[tokio::main]
async fn main() {
    // Load configuration
    let config_path = Path::new("network_config.json5");
    let config_data = fs::read_to_string(config_path).expect("Failed to read the network_config.json5 file");
    let config: Config = Config::from_json5(&config_data).expect("Failed to parse the network_config.json5 file");

    // Initialize Zenoh session
    let session = open(config).await.expect("Failed to open Zenoh session");

    // Declare subscribers
    let int_subscriber = session.declare_subscriber(INT_TOPIC).await.expect("Failed to declare int subscriber");
    let string_subscriber = session.declare_subscriber(STRING_TOPIC).await.expect("Failed to declare string subscriber");
    let array_subscriber = session.declare_subscriber(ARRAY_TOPIC).await.expect("Failed to declare array subscriber");

    // Spawn subscription tasks
    tokio::spawn(subscription_task(int_subscriber, "int"));
    tokio::spawn(subscription_task(string_subscriber, "string"));
    tokio::spawn(subscription_task(array_subscriber, "array"));

    // Keep program running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn subscription_task(subscriber: Subscriber<FifoChannelHandler<Sample>>, topic: &str) {
    loop {
        if let Ok(sample) = subscriber.recv_async().await {
            let message = sample.payload().try_to_string().unwrap_or_else(|_| "Invalid UTF-8".into());
            println!("Received on {}: {}", topic, message);
        }
    }
}
