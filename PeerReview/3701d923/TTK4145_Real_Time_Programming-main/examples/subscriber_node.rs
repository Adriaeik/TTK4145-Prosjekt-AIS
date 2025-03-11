use std::fs;
use std::path::Path;
use tokio::task::yield_now; // Importing yield_now for cooperative multitasking
use tokio::time::{sleep, Duration}; // Importing sleep for task delays
use zenoh::{open, Config}; // Importing Zenoh essentials for communication

#[tokio::main] // Specifies the asynchronous entry point
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

    // Declare a subscriber for Topic 1
    // This subscribes to messages published on "topic_1"
    let subscriber1 = session.declare_subscriber("topic_1").await.expect("Failed to declare subscriber for topic 1");

    // Declare a subscriber for Topic 2
    // Subscribes to messages on "topic_2"
    let subscriber2 = session.declare_subscriber("topic_2").await.expect("Failed to declare subscriber for topic 2");

    // Spawn a task to listen to messages from Topic 1
    tokio::spawn(async move {
        loop {
            // Wait for a message to be received on Topic 1
            match subscriber1.recv_async().await {
                Ok(sample) => {
                    // Attempt to convert the payload of the received sample to a UTF-8 string.
                    // - `sample.payload()` retrieves the data from the received Zenoh message.
                    // - `try_to_string()` tries to interpret the raw bytes as a valid UTF-8 string.
                    // - If the conversion succeeds, it returns the string.
                    // - If the conversion fails (e.g., due to invalid UTF-8 data), the `unwrap_or_else`
                    //   handles the error gracefully by providing a fallback string ("Invalid UTF-8").
                    //   This ensures the program does not panic and continues running even if the payload
                    //   contains unexpected or corrupt data.
                    let message = sample.payload().try_to_string().unwrap_or_else(|_| {
                        // Fallback string used when the payload is not valid UTF-8.
                        "Invalid UTF-8".into()
                    });

                    println!("Received from Topic 1: {}", message);
                }
                Err(e) => {
                    // Log an error if receiving a message fails
                    eprintln!("Error receiving from Topic 1: {}", e);
                }
            }
            // Add a small delay to simulate processing time
            sleep(Duration::from_millis(100)).await;
        }
    });

    // Spawn a task to listen to messages from Topic 2
    tokio::spawn(async move {
        loop {
            // Wait for a message to be received on Topic 2
            match subscriber2.recv_async().await {
                Ok(sample) => {
                    // Attempt to convert the payload of the received sample to a UTF-8 string.
                    // - `sample.payload()` retrieves the data from the received Zenoh message.
                    // - `try_to_string()` tries to interpret the raw bytes as a valid UTF-8 string.
                    // - If the conversion succeeds, it returns the string.
                    // - If the conversion fails (e.g., due to invalid UTF-8 data), the `unwrap_or_else`
                    //   handles the error gracefully by providing a fallback string ("Invalid UTF-8").
                    //   This ensures the program does not panic and continues running even if the payload
                    //   contains unexpected or corrupt data.
                    let message = sample.payload().try_to_string().unwrap_or_else(|_| {
                        // Fallback string used when the payload is not valid UTF-8.
                        "Invalid UTF-8".into()
                    });

                    println!("Received from Topic 2: {}", message);
                }
                Err(e) => {
                    // Log any errors encountered while receiving messages
                    eprintln!("Error receiving from Topic 2: {}", e);
                }
            }
            // Delay for 100ms to simulate message processing
            sleep(Duration::from_millis(100)).await;
        }
    });

    // Keep the program running
    // This ensures the main function doesn't exit prematurely,
    // keeping the Tokio runtime active so spawned tasks can execute
    loop {
        yield_now().await; // Cooperative multitasking: allows other tasks to run
    }
}
