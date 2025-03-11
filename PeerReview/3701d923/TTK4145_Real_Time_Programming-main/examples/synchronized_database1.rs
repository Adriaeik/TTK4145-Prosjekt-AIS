// Library to keep time
use chrono::Utc;

// Libraries for multithreading in cooperative mode
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tokio::time::{sleep, Duration};

// Libraries for distributed network
use std::fs;
use std::path::Path;
use zenoh::handlers::FifoChannelHandler;
use zenoh::pubsub::{Publisher, Subscriber};
use zenoh::sample::Sample;
use zenoh::{open, Config};

// Global node identifier
// !NOTE: Every new Rust process needs their own unique NODE_ID
const NODE_ID: i64 = 1;

// Global TIMEOUT for topic so that we don't get stuck, after this amount of time we stop waiting for a new message
const TIMEOUT: u64 = 5000; // ms

// Global Broadcast pause length between syncing attempts
const BROADCAST_INTERVAL: u64 = 1000; // ms

// Shared topics for leader election and heartbeat
const HEARTBEAT_TOPIC: &str = "sync/leader/heartbeat";
const LEADER_TOPIC: &str = "sync/leader/announce";
const TOPIC_TEMP_NUM: &str = "temp/number";
const TOPIC_STOR_NUM: &str = "stor/number";

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

    // Declare publishers and subscribers
    let heartbeat_publisher = session.declare_publisher(HEARTBEAT_TOPIC).await.expect("Failed to declare heartbeat publisher");

    let heartbeat_subscriber = session.declare_subscriber(HEARTBEAT_TOPIC).await.expect("Failed to declare heartbeat subscriber");

    let leader_publisher = session.declare_publisher(LEADER_TOPIC).await.expect("Failed to declare leader publisher");

    let leader_subscriber = session.declare_subscriber(LEADER_TOPIC).await.expect("Failed to declare leader subscriber");

    let number_subscriber = session.declare_subscriber(TOPIC_TEMP_NUM).await.expect("Failed to declare subscription topic");

    let number_publisher = session.declare_publisher(TOPIC_STOR_NUM).await.expect("Failed to declare publisher");

    // Shared resources for leader status and heartbeat tracking
    let is_leader = Arc::new(RwLock::new(false));
    let last_heartbeat = Arc::new(RwLock::new(Utc::now().timestamp()));

    // Shared resource for storing subscribed messages
    let (shared_number_tx, shared_number_rx) = watch::channel(0i64);

    // Heartbeat monitoring task
    {
        let is_leader = is_leader.clone();
        let last_heartbeat = last_heartbeat.clone();
        tokio::spawn(heartbeat_monitor_task(heartbeat_subscriber, is_leader.clone(), last_heartbeat.clone()));
    }

    // Heartbeat broadcasting task
    {
        let is_leader = is_leader.clone();
        tokio::spawn(heartbeat_broadcast_task(heartbeat_publisher, is_leader.clone()));
    }

    // Leader monitoring task
    {
        let is_leader = is_leader.clone();
        tokio::spawn(leader_monitor_task(leader_subscriber, is_leader.clone()));
    }

    // Leader broadcasting task
    {
        let is_leader = is_leader.clone();
        tokio::spawn(leader_broadcast_task(leader_publisher, is_leader.clone()));
    }

    // Number monitor task
    {
        let shared_number_tx = shared_number_tx.clone();
        tokio::spawn(message_monitor_task(number_subscriber, shared_number_tx));
    }

    // Number broadcast task (only if leader)
    {
        let is_leader = is_leader.clone();
        let shared_number_rx = shared_number_rx.clone();
        tokio::spawn(number_broadcast_task(number_publisher, is_leader.clone(), shared_number_rx));
    }

    // Keep the program running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

// This function continuously monitors heartbeat messages from other nodes
// to detect potential leader failures. It uses a timeout mechanism to determine
// whether a heartbeat is received within the specified TIMEOUT period.
//
// If a heartbeat is received:
// - The function updates the last heartbeat timestamp, ensuring that the node
//   acknowledges the active leadership of another node.
//
// If no heartbeat is received within the timeout:
// - The node assumes leadership by setting its `is_leader` flag to `true`.
// - It also updates the last heartbeat timestamp to avoid immediate failover loops.
//
// The loop pauses briefly before each retry to avoid overloading the system.
// This mechanism ensures that nodes can dynamically adjust leadership roles
// in response to network changes or failures, maintaining decentralized control.
async fn heartbeat_monitor_task(heartbeat_subscriber: Subscriber<FifoChannelHandler<Sample>>, is_leader: Arc<RwLock<bool>>, last_heartbeat: Arc<RwLock<i64>>) {
    loop {
        // Timeout duration for receiving heartbeat
        let heartbeat_timeout = Duration::from_millis(TIMEOUT);

        // Wait for a heartbeat with a timeout
        let heartbeat_received = tokio::time::timeout(heartbeat_timeout, heartbeat_subscriber.recv_async()).await.is_ok();

        if heartbeat_received {
            // Update the last heartbeat time
            let mut heartbeat_time = last_heartbeat.write().await;
            *heartbeat_time = Utc::now().timestamp();
        } else {
            println!("No heartbeat received within timeout.");
            println!("Node {} is claiming leadership...", NODE_ID);

            // No heartbeat received within timeout
            let mut is_leader_lock = is_leader.write().await;
            *is_leader_lock = true;

            // Update last heartbeat to avoid immediate failover
            let mut heartbeat_time = last_heartbeat.write().await;
            *heartbeat_time = Utc::now().timestamp();
        }
    }
}

// This function periodically sends heartbeat messages if the node is currently the leader.
//
// The purpose of this task is to signal the node's active leadership to other nodes in the system.
// Heartbeats serve as a mechanism for other nodes to verify that a leader is still functioning,
// reducing unnecessary failovers and ensuring stable operation.
//
// Key operations:
// - The function checks if the node is the leader by reading the `is_leader` flag.
// - If the node is the leader, it publishes a heartbeat message to the shared heartbeat topic.
// - The loop includes a delay between iterations to regulate the frequency of heartbeat messages.
//
// This approach minimizes network overhead while maintaining leader status visibility across the system.
async fn heartbeat_broadcast_task(heartbeat_publisher: Publisher<'_>, is_leader: Arc<RwLock<bool>>) {
    loop {
        if *is_leader.read().await {
            heartbeat_publisher.put("alive".as_bytes()).await.expect("Failed to send heartbeat");
        }
        sleep(Duration::from_millis(BROADCAST_INTERVAL)).await;
    }
}

// This function monitors leadership announcements in the network and dynamically adjusts the node's leadership status.
//
// The task continuously listens to messages on the leader announcement topic to stay informed about the current leader.
// The primary goal is to ensure that the leadership hierarchy is respected, and no conflicting leaders exist simultaneously.
//
// Key operations:
// - It listens for incoming messages using the `leader_subscriber`.
// - The message payload is parsed to extract the `leader_id`, representing the announcing leader.
// - If the `leader_id` is smaller (higher priority) than the current node's `NODE_ID`, the node steps down by setting `is_leader` to `false`.
// - If the `leader_id` is equal to the current `NODE_ID`, the node confirms its leadership status by setting `is_leader` to `true`.
//
// This mechanism maintains consistency in the system by dynamically updating the node's leadership status
// based on network-wide announcements, ensuring proper coordination and hierarchy.
async fn leader_monitor_task(leader_subscriber: Subscriber<FifoChannelHandler<Sample>>, is_leader: Arc<RwLock<bool>>) {
    loop {
        if let Ok(sample) = leader_subscriber.recv_async().await {
            // Parse leader ID from the announcement
            let message = sample.payload().try_to_string().unwrap_or_else(|_| "Invalid UTF-8".into());

            if let Ok(leader_id) = message.parse::<i64>() {
                // Adjust leadership status based on the received ID
                let mut is_leader_lock = is_leader.write().await;
                if leader_id < NODE_ID {
                    *is_leader_lock = false; // Step down if another higher-ranked leader exists
                } else {
                    *is_leader_lock = true; // Confirm leadership
                }
            }
        }
    }
}

// This function broadcasts the node's leadership status to the network periodically if the node is the leader.
//
// The task operates in a continuous loop, where the node checks its leadership status using the `is_leader` flag.
// If the node identifies itself as the leader, it publishes its `NODE_ID` to the leader announcement topic, ensuring other nodes are aware of its leadership.
//
// Key operations:
// - Reads the `is_leader` status using an `RwLock` for efficient read access.
// - If the node is the leader, it publishes its `NODE_ID` as a message to the leader topic via the `leader_publisher`.
// - Introduces a delay of 1 second between successive announcements to avoid spamming the network.
//
// This mechanism allows the leader node to assert its presence and ensures that all nodes in the network are
// aware of the current leader, maintaining consistency and preventing conflicts.
async fn leader_broadcast_task(leader_publisher: Publisher<'_>, is_leader: Arc<RwLock<bool>>) {
    loop {
        if *is_leader.read().await {
            leader_publisher.put(NODE_ID.to_string().as_bytes()).await.expect("Failed to announce leadership");
        }
        sleep(Duration::from_millis(BROADCAST_INTERVAL)).await; // Publish leadership every 1 second
    }
}

// This function monitors a specific topic, processes incoming messages, and updates a shared value.
//
// The task continuously subscribes to a topic for new messages, parses them as integers, and updates a shared value
// using a `tokio::watch::Sender`. This ensures that all other threads or tasks monitoring the `watch` channel
// are immediately informed of the latest value.
//
// Key operations:
// - Subscribes to a topic and listens for incoming messages.
// - Parses the message payload into a UTF-8 string, providing a fallback for invalid data.
// - Attempts to parse the string into an `i64` value. If successful, the value is sent to the `watch` channel.
// - Logs any failures in parsing or updating the shared value for debugging purposes.
//
// This mechanism is ideal for maintaining a consistent shared state across threads or tasks,
// ensuring real-time updates and minimizing delays between message reception and shared value propagation.
async fn message_monitor_task(subscriber: Subscriber<FifoChannelHandler<Sample>>, shared_number_tx: watch::Sender<i64>) {
    loop {
        if let Ok(sample) = subscriber.recv_async().await {
            let message = sample.payload().try_to_string().unwrap_or_else(|_| "0".into());

            // Parse the message as an integer and save it
            if let Ok(number) = message.parse::<i64>() {
                if let Err(_) = shared_number_tx.send(number) {
                    println!("Failed to update shared value");
                } else {
                    println!("Received and saved number: {}", number);
                }
            }
        }
    }
}

// This function continuously broadcasts a shared value to a specific topic if the current node is the leader.
//
// The task is designed to:
// - Monitor a `watch` channel for changes in a shared value (`shared_number_rx`).
// - If a change is detected and this node is the leader, the new value is published to a specific topic.
// - Use `tokio::watch` for immediate updates to minimize latency between receiving and broadcasting.
//
// Key functionality:
// - Checks leadership status before publishing, ensuring only the leader node broadcasts the value.
// - Publishes the shared value as a UTF-8 encoded string.
// - Logs the published value for visibility and debugging.
// - Efficiently handles updates by reacting only when the shared value changes.
//
// This task ensures that shared values are always up-to-date on the target topic while maintaining leadership constraints.
async fn number_broadcast_task(number_publisher: Publisher<'_>, is_leader: Arc<RwLock<bool>>, mut shared_number_rx: watch::Receiver<i64>) {
    while shared_number_rx.changed().await.is_ok() {
        if *is_leader.read().await {
            let value_to_publish = *shared_number_rx.borrow();

            // Publish the current shared value
            number_publisher
                .put(value_to_publish.to_string().as_bytes())
                .await
                .expect("Failed to publish number");

            println!("Node {}: Published number: {}", NODE_ID, value_to_publish);
        }
    }
}
