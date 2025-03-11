// Libraries for distributed network
use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};
use zenoh::handlers::FifoChannelHandler;
use zenoh::pubsub::Subscriber;
use zenoh::sample::Sample;
use zenoh::{open, Config};

// Shared topic to subscribe to
const NUMBER_TOPIC: &str = "stor/number";

#[tokio::main]
async fn main() {
    // Specify path to highly customable network modes for distributed networks
    // Most important settings: peer-2-peer and scouting to alow multicast and robust network connectivity
    let config_path = Path::new("network_config.json5");

    // Load configuration from JSON5 file
    let config_data = fs::read_to_string(config_path).expect("Failed to read the network_config.json5 file");
    let config: Config = Config::from_json5(&config_data).expect("Failed to parse the network_config.json5 file");

    // Initialize Zenoh session
    let session = open(config).await.expect("Failed to open Zenoh session");

    // Declare subscriber for the random topic
    let random_subscriber = session.declare_subscriber(NUMBER_TOPIC).await.expect("Failed to declare random subscriber");

    println!("Subscribed to topic: {}", NUMBER_TOPIC);

    // Run the subscription task
    tokio::spawn(random_subscription_task(random_subscriber));

    // Keep the program running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

// Random subscription task: Listens to messages on the random topic
async fn random_subscription_task(random_subscriber: Subscriber<FifoChannelHandler<Sample>>) {
    loop {
        if let Ok(sample) = random_subscriber.recv_async().await {
            let message = sample.payload().try_to_string().unwrap_or_else(|_| "Invalid UTF-8".into());

            println!("Received message: {}", message);
        }
    }
}
