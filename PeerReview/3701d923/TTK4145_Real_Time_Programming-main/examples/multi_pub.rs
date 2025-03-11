use rand::Rng;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use zenoh::pubsub::Publisher;
use zenoh::{open, Config};

// Topics for publishing
const INT_TOPIC: &str = "temp/int";
const STRING_TOPIC: &str = "temp/string";
const ARRAY_TOPIC: &str = "temp/array";

#[tokio::main]
async fn main() {
    // Load configuration
    let config_path = Path::new("network_config.json5");
    let config_data = fs::read_to_string(config_path).expect("Failed to read the network_config.json5 file");
    let config: Config = Config::from_json5(&config_data).expect("Failed to parse the network_config.json5 file");

    // Initialize Zenoh session
    let session = open(config).await.expect("Failed to open Zenoh session");

    // Declare publishers
    let int_publisher = Arc::new(session.declare_publisher(INT_TOPIC).await.expect("Failed to declare int publisher"));
    let string_publisher = Arc::new(session.declare_publisher(STRING_TOPIC).await.expect("Failed to declare string publisher"));
    let array_publisher = Arc::new(session.declare_publisher(ARRAY_TOPIC).await.expect("Failed to declare array publisher"));

    // Spawn publishing threads
    tokio::spawn(publish_int_task(int_publisher.clone()));
    tokio::spawn(publish_string_task(string_publisher.clone()));
    tokio::spawn(publish_array_task(array_publisher.clone()));

    // Keep program running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn publish_int_task(publisher: Arc<Publisher<'_>>) {
    loop {
        let random_number: i32 = rand::thread_rng().gen_range(1..=100);
        publisher.put(random_number.to_string().as_bytes()).await.expect("Failed to publish int");
        println!("Published int: {}", random_number);
        sleep(Duration::from_millis(1000)).await;
    }
}

async fn publish_string_task(publisher: Arc<Publisher<'_>>) {
    loop {
        let random_string = format!("Hello {}", rand::thread_rng().gen_range(1..=100));
        publisher.put(random_string.as_bytes()).await.expect("Failed to publish string");
        println!("Published string: {}", random_string);
        sleep(Duration::from_millis(1000)).await;
    }
}

async fn publish_array_task(publisher: Arc<Publisher<'_>>) {
    loop {
        let array: [i32; 3] = [rand::thread_rng().gen_range(1..=100), rand::thread_rng().gen_range(1..=100), rand::thread_rng().gen_range(1..=100)];
        let array_str = format!("{:?}", array);
        publisher.put(array_str.as_bytes()).await.expect("Failed to publish array");
        println!("Published array: {:?}", array);
        sleep(Duration::from_millis(1000)).await;
    }
}
