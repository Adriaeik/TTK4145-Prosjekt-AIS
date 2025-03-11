// * NOTE: Some networking libraries requiring root privileges (for eks for pinging to router to read if we are still on the same network)
// * NOTE: Every new manager needs its own unique MANAGER_ID
// *
// * Because of these factors we must run this process as follows:
// * $ sudo -E MANAGER_ID=<ID> ELEVATOR_NETWORK_ID_LIST="[<ID 1>,<ID 2>,...,<ID N>]" cargo run --bin manager

// Library that allows us to use environment variables or command-line arguments to pass variables from terminal to the program directly
use std::env;

// Libraries for multithreading in cooperative mode
use std::sync::Arc;
use tokio::sync::RwLock; // For optimization RwLock for data that is read more than written to. // Mutex for 1-1 ratio of read write to
use tokio::time::{sleep, Duration};

// Libraries for highly customizable distributed network mode setup
use std::fs;
use std::path::Path;
use zenoh::Config;

// Libraries for distributed network
use zenoh::open;

// Libraries for network status and diagnostics
// * NOTE: Because of some functions in utils library that uses ICMP sockets, witch are ONLY available for root user, we must run our program as sudo cargo run
use elevator_system::distributed_systems::utils::{get_router_ip, network_status, parse_message_to_string, wait_with_timeout, NetworkStatus};

// Libraries for constructing data structures that are thread safe
use once_cell::sync::Lazy;

// Library for the cost function algorithm
use elevator_system::elevator_algorithm::cost_algorithm::run_cost_algorithm;
use elevator_system::elevator_algorithm::utils::AlgoInput;

// Global Variables ----------
const SYNC_INTERVAL: u64 = 1000; // ms
const NETWORK_CHECK_INTERVAL: u64 = 5000; // ms
const HEARTBEAT_INTERVAL: u64 = 1000; // ms (*Taken from elevator.rs node)

const LEADER_TOPIC: &str = "sync/manager/leader";

const ELEVATOR_DATA_SYNC_TOPIC: &str = "sync/elevator/data/synchronized";
const MANAGER_TOPIC: &str = "temp/manager/request";

// Set up environment variables ----------
// Get the MANAGER_ID from the environment variable, defaulting to 0 if not set
// !NOTE: Every new Rust process needs their own unique MANAGER_ID
static MANAGER_ID: Lazy<i64> = Lazy::new(|| {
    env::var("MANAGER_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .expect("MANAGER_ID must be a valid integer")
});

// Get the ELEVATOR_NETWORK_ID_LIST from the environment variable
// Defaulting to [0] if not set
static ELEVATOR_NETWORK_ID_LIST: Lazy<Vec<i64>> = Lazy::new(|| {
    // Expect the variable in the form "[1,2,3]"
    let list_str = env::var("ELEVATOR_NETWORK_ID_LIST").unwrap_or_else(|_| "[0]".to_string());
    list_str
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|s| s.trim().parse().expect("Invalid elevator id in list"))
        .collect()
});

// Distributed Network Topics of interest ----------
static DATA_STREAMS_ELEVATOR_HEARTBEATS: Lazy<Vec<String>> = Lazy::new(|| ELEVATOR_NETWORK_ID_LIST.iter().map(|&id| format!("stor/elevator{}/heartbeat", id)).collect());

#[tokio::main]
async fn main() {
    // Distributed Network Initialization (START) ====================================================================================================
    println!("MANAGER_ID: {}", *MANAGER_ID);
    println!("ELEVATOR_NETWORK_ID_LIST: {:#?}", *ELEVATOR_NETWORK_ID_LIST);

    // Specify path to highly customable network modes for distributed networks
    // Most important settings: peer-2-peer and scouting to alow multicast and robust network connectivity
    // Then Load configuration from JSON5 file
    // Finally initialize networking session
    let networking_config_path = Path::new("network_config.json5");

    let networking_config_data = fs::read_to_string(networking_config_path).expect("Failed to read the network_config.json5 file");
    let config: Config = Config::from_json5(&networking_config_data).expect("Failed to parse the network_config.json5 file");

    let network_session = open(config).await.expect("Failed to open Zenoh session");
    // Distributed Network Initialization (STOP) ====================================================================================================

    // Network Monitoring (START) ====================================================================================================
    // Spawn a separate task to check network status every so often
    // If we detect we have been disconnected from the network we kill ourselves
    tokio::spawn(async move {
        let router_ip = match get_router_ip().await {
            Some(ip) => ip,
            None => {
                println!("#========================================#");
                println!("ERROR: Failed to retrieve the router IP");
                println!("Killing myself...");
                println!("Gugu gaga *O*");
                println!("#========================================#");
                std::process::exit(1);
            }
        };

        loop {
            match network_status(router_ip).await {
                NetworkStatus::Connected => {
                    // Do nothing
                }
                NetworkStatus::Disconnected => {
                    println!("#========================================#");
                    println!("ERROR: Disconnected from the network!");
                    println!("Killing myself...");
                    println!("Shiding and crying T_T");
                    println!("#========================================#");
                    std::process::exit(1);
                }
            }

            sleep(Duration::from_millis(NETWORK_CHECK_INTERVAL)).await;
        }
    });
    // Network Monitoring (STOP) ====================================================================================================

    // Synchronization (START) ====================================================================================================
    let leader_publisher = network_session.declare_publisher(LEADER_TOPIC).await.expect("Failed to declare leader publisher");

    let leader_subscriber = network_session.declare_subscriber(LEADER_TOPIC).await.expect("Failed to declare leader subscriber");

    let leader = Arc::new(RwLock::new(false));

    // Leader monitoring task ----------
    {
        let leader = leader.clone();
        let leader_elect_interval = SYNC_INTERVAL * 5; // 5x sync because we want to make sure everyone who wants to be a leader has broadcasted it at least once

        tokio::spawn(async move {
            loop {
                // Wait for a leader broadcast within the election interval
                let result = wait_with_timeout(leader_elect_interval, leader_subscriber.recv_async()).await;

                // Check the results
                // If we got a time-out, that means no one else on the network wants to be a leader
                // => become default leader
                // If there is someone else trying to become the leader
                // => Chose leader with lowest ID
                if let Some(message) = result {
                    // Parse leader ID from the announcement
                    let id = parse_message_to_string(message);

                    if let Ok(leader_id) = id.parse::<i64>() {
                        let mut are_we_leader = leader.write().await;

                        if leader_id < *MANAGER_ID {
                            *are_we_leader = false; // Step down from leadership
                        } else {
                            *are_we_leader = true; // Become leader
                        }
                    }
                } else {
                    // No leader broadcast received within the timeout
                    let mut is_leader_lock = leader.write().await;
                    *is_leader_lock = true; // Default to becoming the leader
                }
            }
        });
    }

    // Leader broadcasting task ----------
    {
        let leader = leader.clone();
        let leader_broadcast_interval = SYNC_INTERVAL;

        tokio::spawn(async move {
            loop {
                if *leader.read().await {
                    leader_publisher
                        .put((*MANAGER_ID).to_string().as_bytes())
                        .await
                        .expect("Failed to announce leadership");
                }

                sleep(Duration::from_millis(leader_broadcast_interval)).await;
            }
        });
    }
    // Synchronization (STOP) ====================================================================================================

    // Elevator Heartbeat Monitoring (START) ====================================================================================================
    // Listen to each heartbeat
    // If heartbeat stopped after a while updated shared resource
    // Once it starts up again it will update shared resource again
    let elevators_alive = Arc::new(RwLock::new(vec![true; ELEVATOR_NETWORK_ID_LIST.len()])); // Assume all elevators start alive

    // Loop through the whole Elevator Heartbeat list
    // Each elevator gets its own dedicated thread for listening at its heartbeat
    // If any anomalies or to timeout occurs, assume elevator dead
    // Otherwise keep holding the elevator alive
    for elevator_heartbeat_index in 0..ELEVATOR_NETWORK_ID_LIST.len() {
        // Set up resources for the local elevator thread
        let topic = DATA_STREAMS_ELEVATOR_HEARTBEATS
            .get(elevator_heartbeat_index)
            .expect("Invalid heartbeat index")
            .clone(); // Clone to avoid moving the String

        let elevator_heartbeat_subscriber = network_session
            .declare_subscriber(&topic) // Use &topic to avoid moving the String
            .await
            .expect("Failed to declare Elevator Heartbeat subscriber");

        let elevators_alive_clone = elevators_alive.clone();

        let heartbeat_dead_interval = HEARTBEAT_INTERVAL * 5; // 5x heartbeat interval because we want to make sure everyone who wants to be a heartbeat has broadcasted it at least once

        tokio::spawn(async move {
            loop {
                // Wait for a leader broadcast within the election interval
                let result = wait_with_timeout(heartbeat_dead_interval, elevator_heartbeat_subscriber.recv_async()).await;

                // Check the results
                if let Some(message) = result {
                    let parsed_message = parse_message_to_string(message); // Convert Zenoh message to string

                    if !parsed_message.trim().is_empty() {
                        // Message is valid (not empty and not NaN)
                        // Elevator is alive
                        {
                            let mut elevators_alive = elevators_alive_clone.write().await;
                            elevators_alive[elevator_heartbeat_index] = true;
                        }
                    } else {
                        {
                            let mut elevators_alive = elevators_alive_clone.write().await;
                            elevators_alive[elevator_heartbeat_index] = false;
                        }
                    }
                } else {
                    // No heartbeat
                    {
                        let mut elevators_alive = elevators_alive_clone.write().await;
                        elevators_alive[elevator_heartbeat_index] = false;
                    }
                }
            }
        });
    }
    // Elevator Heartbeat Monitoring (STOP) ====================================================================================================

    // Manager (START) ====================================================================================================
    let elevator_data_sync_subscriber = network_session
        .declare_subscriber(ELEVATOR_DATA_SYNC_TOPIC)
        .await
        .expect("Failed to declare Elevator Data Synchronization subscriber");

    let manager_publisher = network_session.declare_publisher(MANAGER_TOPIC).await.expect("Failed to declare Manager publisher");

    let leader_clone = leader.clone();
    let elevators_alive_clone = elevators_alive.clone();

    tokio::spawn(async move {
        // Wait for new messages
        while let Ok(message) = elevator_data_sync_subscriber.recv_async().await {
            // NOTE: Only run cost function if we are the leader
            // Otherwise, just wait for new messages
            if *leader_clone.read().await {
                let json_str = parse_message_to_string(message);

                // Parse JSON into a struct
                let mut parsed_data: AlgoInput = serde_json::from_str(&json_str).expect("Failed to parse elevator state JSON");

                // Filter out dead elevators based on `elevators_alive` index
                let elevators_alive = elevators_alive_clone.read().await;
                for (index, &is_alive) in elevators_alive.iter().enumerate() {
                    if !is_alive {
                        if let Some(elevator_id) = ELEVATOR_NETWORK_ID_LIST.get(index) {
                            parsed_data.states.remove(&elevator_id.to_string());
                        }
                    }
                }

                // println!("DEBUGGING: Alive?: {:#?}", elevators_alive);

                // Serialize updated JSON
                let filtered_json = serde_json::to_string(&parsed_data).expect("Failed to reserialize JSON");

                // println!("DEBUGGING: Input: {:#?}", filtered_json);

                // Run cost function with filtered JSON
                let result = run_cost_algorithm(filtered_json).await;

                // println!("DEBUGGING: Output: {:#?}", result);

                // Publish the result
                manager_publisher.put(result.as_bytes()).await.expect("Failed to publish result");
            }
        }
    });

    // Manager (STOP) ====================================================================================================

    // Keep the program running
    loop {
        tokio::task::yield_now().await; // Yield to other tasks
    }
}
