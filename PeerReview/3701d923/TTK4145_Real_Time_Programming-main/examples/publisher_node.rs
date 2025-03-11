use std::fs;
use std::path::Path;
use tokio::task::yield_now; // Importing yield_now for cooperative multitasking
use tokio::time::{sleep, Duration}; // Importing Tokio's time module for asynchronous delays
use zenoh::{open, Config}; // Importing Zenoh for distributed communication

#[tokio::main] // Specifies the asynchronous entry point using Tokio
async fn main() {
    // Configure Zenoh session with reliability and persistence (QoS (Quality of Service) Settings)
    //
    // Zenoh default settings already come in with:
    // - Robust peer-to-peer communication
    // - Automatic peer discovery
    // - Time to live 1 second
    // OBS!: Zenoh does NOT have persistent data retention RUST natively (need extra server solution with syncing :P)
    // Specify path to highly customable network modes for distributed networks
    // Most important settings: peer-2-peer and scouting to alow multicast and robust network connectivity
    let config_path = Path::new("network_config.json5");

    // Load configuration from JSON5 file
    let config_data = fs::read_to_string(config_path).expect("Failed to read the network_config.json5 file");
    let config: Config = Config::from_json5(&config_data).expect("Failed to parse the network_config.json5 file");

    // Initialize Zenoh session
    let session = open(config).await.expect("Failed to open Zenoh session");

    // Declare a publisher for Topic 1
    // The publisher sends data to the specified topic ("topic_1")
    let publisher1 = session
        .declare_publisher("topic_1") // Remove leading slash
        .await
        .expect("Failed to declare publisher for topic 1");

    // Declare a publisher for Topic 2
    // Similar to Topic 1, this handles publishing data to "topic_2"
    let publisher2 = session
        .declare_publisher("topic_2") // Remove leading slash
        .await
        .expect("Failed to declare publisher for topic 2");

    // Spawn an asynchronous task to publish messages to Topic 1
    tokio::spawn(async move {
        let mut counter = 0; // Counter to keep track of the message sequence
        loop {
            let message = format!("Message to Topic 1: {}", counter); // Generate a message string
            publisher1
                .put(message.as_bytes()) // Publish the message as bytes
                .await
                .expect("Failed to publish to Topic 1");
            println!("Published: {}", message); // Log the message to the console
            counter += 1; // Increment the counter
            sleep(Duration::from_secs(1)).await; // Delay for 1 second before sending the next message
        }
    });

    // Spawn an asynchronous task to publish messages to Topic 2
    tokio::spawn(async move {
        let mut counter = 0; // Counter for Topic 2 messages
        loop {
            let message = format!("Message to Topic 2: {}", counter); // Generate the message
            publisher2
                .put(message.as_bytes()) // Publish the message to Topic 2
                .await
                .expect("Failed to publish to Topic 2");
            println!("Published: {}", message); // Log the message
            counter += 1; // Increment the counter
            sleep(Duration::from_secs(1)).await; // Delay for 1 second
        }
    });

    // Keep the program running
    // Without this loop, the main function would exit, and the spawned tasks would terminate
    // `yield_now()` ensures cooperative multitasking, allowing other tasks to run
    loop {
        yield_now().await;
    }
}
