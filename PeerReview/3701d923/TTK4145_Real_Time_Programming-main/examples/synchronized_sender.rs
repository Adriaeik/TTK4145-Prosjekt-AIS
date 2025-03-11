// Library for generating random numbers
use rand::Rng;

// Libraries for multithreading and time management
use tokio::time::{sleep, Duration};

// Libraries for distributed network
use std::fs;
use std::path::Path;
use std::sync::Arc;
use zenoh::pubsub::Publisher;
use zenoh::{open, Config};

// Topic to publish random numbers
const RANDOM_NUMBER_TOPIC: &str = "temp/number";

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

    // Declare publisher
    let random_number_publisher = Arc::new(
        session
            .declare_publisher(RANDOM_NUMBER_TOPIC)
            .await
            .expect("Failed to declare random number publisher"),
    );

    // Spawn the random number publishing task
    let publisher = random_number_publisher.clone();
    tokio::spawn(random_number_publishing_task(publisher));

    // Keep the program running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

// Random number publishing task: Publishes random numbers to a topic
async fn random_number_publishing_task(random_number_publisher: Arc<Publisher<'_>>) {
    loop {
        // Generate a random number
        let random_number: i32 = rand::thread_rng().gen_range(1..=100);

        // Publish the random number
        random_number_publisher
            .put(random_number.to_string().as_bytes())
            .await
            .expect("Failed to publish random number");

        println!("Published random number: {}", random_number);

        // Wait for a second (1000 ms) before publishing the next number
        sleep(Duration::from_millis(1000)).await;
    }
}
