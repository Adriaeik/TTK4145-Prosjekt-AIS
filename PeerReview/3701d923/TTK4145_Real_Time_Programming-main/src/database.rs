// * NOTE: Some networking libraries requiring root privileges (for eks for pinging to router to read if we are still on the same network)
// * NOTE: Every new database needs its own unique DATABASE_ID
// *
// * Because of these factors we must run this process as follows:
// * $ sudo -E DATABASE_NETWORK_ID=<ID> ELEVATOR_NETWORK_ID_LIST="[<ID 1>,<ID 2>,...,<ID N>]" NUMBER_FLOORS=<NUMBER FLOORS> cargo run --bin database

// Library that allows us to use environment variables or command-line arguments to pass variables from terminal to the program directly
use std::env;

// Libraries for multithreading in cooperative mode
use std::sync::Arc;
use tokio::sync::{watch, Mutex, RwLock}; // For optimization RwLock for data that is read more than written to. // Mutex for 1-1 ratio of read write to
use tokio::time::{sleep, Duration};

// Libraries for highly customizable distributed network mode setup
use std::fs;
use std::path::Path;
use zenoh::Config;

// Libraries for distributed network
use zenoh::open;

// Libraries for network status and diagnostics
// * NOTE: Because of some functions in utils library that uses ICMP sockets, witch are ONLY available for root user, we must run our program as sudo cargo run
use elevator_system::distributed_systems::utils::{get_router_ip, network_status, parse_message_to_hashset_u8, parse_message_to_string, wait_with_timeout, NetworkStatus};

// Libraries for constructing data structures that are thread safe
use elevator_system::elevator_algorithm::utils::{AlgoInput, ElevState};
use elevator_system::elevator_logic::utils::ElevHallRequests;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

// Global Variable ----------
const SYNC_INTERVAL: u64 = 1000; // ms
const NETWORK_CHECK_INTERVAL: u64 = 5000; // ms

const LEADER_TOPIC: &str = "sync/database/leader";

// Set up environment variables ----------
// Get the DATABASE_NETWORK_ID from the environment variable, defaulting to 0 if not set
// !NOTE: Every new Rust process needs their own unique DATABASE_NETWORK_ID
static DATABASE_NETWORK_ID: Lazy<i64> = Lazy::new(|| {
    env::var("DATABASE_NETWORK_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .expect("DATABASE_NETWORK_ID must be a valid integer")
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

// Create a static parameter for number of floors this specific elevator serves
// If none => Default to NUMBER FLOORS: 4
static NUMBER_FLOORS: Lazy<u8> = Lazy::new(|| {
    env::var("NUMBER_FLOORS")
        .unwrap_or_else(|_| "4".to_string())
        .parse()
        .expect("NUMBER_FLOORS must be a valid integer")
});

// Data Structure Construction ----------
// NOTE: Box::leak() is a powerful yet dangerous command
// NOTE: Used inappropriately in a dynamic continuous running process, it will clog up the memory as it is never deallocated, causing memory leaks and overflows
// NOTE: However for use in startup where the values will never be manipulated afterwards this is a safe way to us it in
// NOTE: Since we only manipulate memory on startup for Topics, we don't have to worry about memory leaks and overflows :)
type SharedData = Arc<Mutex<String>>; // All shared data is stored as a String at the end of the day

#[derive(Clone)]
struct DataStreamConfig {
    temp_topic: &'static str,
    stor_topic: &'static str,
    shared_data: SharedData,
    rebroadcast_interval: u64, // * [ms] TIPS: Setting this value to 0 disables rebroadcasting ability for that specific datastream
}

static DATA_STREAMS_ELEVATOR: Lazy<Vec<DataStreamConfig>> = Lazy::new(|| {
    ELEVATOR_NETWORK_ID_LIST
        .iter()
        .flat_map(|&id| {
            let id_str = id.to_string();
            vec![
                // This topic MUST be broadcasted
                // Reason being is that database itself uses it as backup
                // If a database dies and then rejoins,
                // by rebroadcasting this data, we get to read the states of the elevators on initialization again
                // This way we hold everything synchronized and backed up
                DataStreamConfig {
                    temp_topic: Box::leak(format!("temp/elevator{}/states", id_str).into_boxed_str()),
                    stor_topic: Box::leak(format!("stor/elevator{}/states", id_str).into_boxed_str()),
                    shared_data: Arc::new(Mutex::new(String::new())),
                    rebroadcast_interval: SYNC_INTERVAL, // ms
                },
                // Heartbeat topic
                DataStreamConfig {
                    temp_topic: Box::leak(format!("temp/elevator{}/heartbeat", id_str).into_boxed_str()),
                    stor_topic: Box::leak(format!("stor/elevator{}/heartbeat", id_str).into_boxed_str()),
                    shared_data: Arc::new(Mutex::new(String::new())),
                    rebroadcast_interval: 0, // DISABLED
                },
                // These topics must be rebroadcasted
                // Reason being is that node initialization depend on data that is backed up
                // The way nodes receive backup data is listen to stor/ topics for next rebroadcast
                DataStreamConfig {
                    temp_topic: Box::leak(format!("temp/elevator{}/backup", id_str).into_boxed_str()),
                    stor_topic: Box::leak(format!("stor/elevator{}/backup", id_str).into_boxed_str()),
                    shared_data: Arc::new(Mutex::new(String::new())),
                    rebroadcast_interval: 500, // ms (NOTE: Should be the same as BACKUP_INTERVAL in "elevator.rs")
                },
            ]
        })
        .collect()
});

static DATA_STREAMS_MANAGER: Lazy<Vec<DataStreamConfig>> = Lazy::new(|| {
    vec![DataStreamConfig {
        temp_topic: Box::leak(format!("temp/manager/request").into_boxed_str()),
        stor_topic: Box::leak(format!("stor/manager/request").into_boxed_str()),
        shared_data: Arc::new(Mutex::new(String::new())),
        rebroadcast_interval: 0, // DISABLED
    }]
});

const HALL_REQUESTS_SYNC: &str = "sync/elevator/hall/requests";
const HALL_REQUESTS_UP_STOR: &str = "stor/elevator/hall/requests/up";
const HALL_REQUESTS_DOWN_STOR: &str = "stor/elevator/hall/requests/down";
const HALL_REQUESTS_BACKUP_INTERVAL: u64 = 500; // ms

const ELEVATOR_DATA_SYNC: &str = "sync/elevator/data/synchronized";

#[tokio::main]
async fn main() {
    // Distributed Network Initialization (START) ====================================================================================================
    println!("DATABASE_NETWORK_ID: {}", *DATABASE_NETWORK_ID);
    println!("ELEVATOR_NETWORK_ID_LIST: {:#?}", *ELEVATOR_NETWORK_ID_LIST);
    println!("NUMBER_FLOORS: {}", *NUMBER_FLOORS);
    println!();

    // Specify path to highly customable network modes for distributed networks
    // Most important settings: peer-2-peer and scouting to alow multicast and robust network connectivity
    // Then Load configuration from JSON5 file
    // Finally initialize networking session
    let networking_config_path = Path::new("network_config.json5");

    let networking_config_data = fs::read_to_string(networking_config_path).expect("Failed to read the network_config.json5 file");
    let config: Config = Config::from_json5(&networking_config_data).expect("Failed to parse the network_config.json5 file");

    let network_session = open(config).await.expect("Failed to open Zenoh session");
    // Distributed Network Initialization (STOP) ====================================================================================================

    // Database Initialization (START) ====================================================================================================
    // Initialization step to check if stored messages were updated while this database node was gone
    // If we detect new stored data we update our internal data to match outside world
    let mut tasks = Vec::new();

    // Add together all streaming topics into 1 big vector array
    let mut all_data_streams: Vec<DataStreamConfig> = Vec::new();
    all_data_streams.extend(DATA_STREAMS_ELEVATOR.iter().cloned()); // Clone the data from Lazy
    all_data_streams.extend(DATA_STREAMS_MANAGER.iter().cloned()); // Clone the data from Lazy

    for stream in all_data_streams {
        let stor_topic = stream.stor_topic;
        let shared_data = stream.shared_data.clone();
        let networking_session_clone = network_session.clone();

        tasks.push(tokio::spawn(async move {
            let subscriber_stor = networking_session_clone
                .declare_subscriber(stor_topic)
                .await
                .expect("Failed to declare stor subscription topic");

            // Wait for some stored data
            // If we don't get any in a certain amount of time we assume there is no stored data, so we pass
            // If we find stored data being published we save it internally in our data base
            // 10 000 ms because it takes some time for network config to configure our networking protocol, thus we need to compensate for it
            // + wait a bit for a given broadcast interval just to be sure
            let init_timeout = 10000 + stream.rebroadcast_interval;
            let result = wait_with_timeout(init_timeout, subscriber_stor.recv_async()).await;

            if let Some(message) = result {
                let data_new = parse_message_to_string(message);
                println!("New data for storage: {}: {}", stor_topic, data_new);

                let mut data = shared_data.lock().await;
                *data = data_new.to_string();
            } else {
                println!("No new data for storage: {}", stor_topic);
            }
        }));
    }

    for task in tasks {
        let _ = task.await;
    }
    // Database Initialization (STOP) ====================================================================================================

    // Elevator Data Synchronization Initialization (START) ====================================================================================================
    // Elevator State Backup ----------
    // Stores elevator states in a **shared HashMap** (`elevator_states`)
    // Uses **RwLock** since reads will be more frequent than writes

    // Shared resource for storing elevator states
    // - `RwLock` allows multiple readers and a single writer (better performance for read-heavy workloads)
    let elevator_states: Arc<RwLock<HashMap<i64, String>>> = Arc::new(RwLock::new(HashMap::new()));

    // Before subscribing to updates, we initialize the HashMap with existing state data
    // This ensures that the database starts with **correct values**
    // Extracts initial state from the `DATA_STREAMS_ELEVATOR` list and maps it to each elevator ID
    let elevator_states_clone = elevator_states.clone();
    {
        let mut elevator_states = elevator_states_clone.write().await;
        let mut index = 0; // Tracks valid elevator IDs

        for stream in DATA_STREAMS_ELEVATOR.iter() {
            if stream.temp_topic.contains("/states") {
                if let Some(&id) = ELEVATOR_NETWORK_ID_LIST.get(index) {
                    let initial_state = stream.shared_data.lock().await.clone(); // Extract initial state
                    elevator_states.insert(id, initial_state);
                    index += 1; // Only increment if we successfully mapped an elevator
                }
            }
        }
    }

    // Hall Requests Backup ----------
    // The only thing we need to update now are hall requests
    // We subscribe to hall requests backup topics and listen to them for a moment
    // If no new hall calls we just continue with no new data
    // If there are responses, we back that data up

    // Shared resources: Separate HashSets for UP and DOWN hall requests
    let hall_requests_up: Arc<RwLock<HashSet<u8>>> = Arc::new(RwLock::new(HashSet::new()));
    let hall_requests_down: Arc<RwLock<HashSet<u8>>> = Arc::new(RwLock::new(HashSet::new()));

    // Create backup storage subscribers
    let backup_hall_requests_up_subscriber = network_session
        .declare_subscriber(HALL_REQUESTS_UP_STOR)
        .await
        .expect("Failed to declare UP requests publisher");
    let backup_hall_requests_down_subscriber = network_session
        .declare_subscriber(HALL_REQUESTS_DOWN_STOR)
        .await
        .expect("Failed to declare DOWN requests publisher");

    // Create tasks to get backup data if it exists
    // Wait for some stored data
    // If we don't get any in a certain amount of time we assume there is no stored data, so we pass
    // If we find stored data being published we save it internally in our data base
    // 5 000 ms because it takes some time for network config to configure our networking protocol, thus we need to compensate for it
    // + wait a bit for a given broadcast interval just to be sure
    let mut tasks = Vec::new();
    let backup_init_timeout = 5000 + HALL_REQUESTS_BACKUP_INTERVAL;

    let hall_requests_up_clone = hall_requests_up.clone();
    tasks.push(tokio::spawn(async move {
        let result = wait_with_timeout(backup_init_timeout, backup_hall_requests_up_subscriber.recv_async()).await;

        if let Some(message) = result {
            let data_new: HashSet<u8> = parse_message_to_hashset_u8(message);
            println!("New data from: {}: {:#?}", HALL_REQUESTS_UP_STOR, data_new);

            let mut data = hall_requests_up_clone.write().await;
            *data = data_new;
        } else {
            println!("No new data from: {}", HALL_REQUESTS_UP_STOR);
        }
    }));

    let hall_requests_down_clone = hall_requests_down.clone();
    tasks.push(tokio::spawn(async move {
        let result = wait_with_timeout(backup_init_timeout, backup_hall_requests_down_subscriber.recv_async()).await;

        if let Some(message) = result {
            let data_new: HashSet<u8> = parse_message_to_hashset_u8(message);
            println!("New data from: {}: {:#?}", HALL_REQUESTS_DOWN_STOR, data_new);

            let mut data = hall_requests_down_clone.write().await;
            *data = data_new;
        } else {
            println!("No new data from: {}", HALL_REQUESTS_DOWN_STOR);
        }
    }));

    for task in tasks {
        let _ = task.await;
    }
    // Elevator Data Synchronization Initialization (STOP) ====================================================================================================

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

                        if leader_id < *DATABASE_NETWORK_ID {
                            *are_we_leader = false; // Step down from leadership
                        } else {
                            *are_we_leader = true; // Become leader
                        }
                    }
                } else {
                    // No leader broadcast received within the timeout
                    let mut is_leader_lock = leader.write().await;
                    *is_leader_lock = true; // Default to becoming the leader

                    println!("No leader detected, becoming the leader (o0o)");
                }
            }
        });
    }

    // Leader broadcasting task ----------
    {
        let leader = leader.clone();

        tokio::spawn(async move {
            loop {
                if *leader.read().await {
                    leader_publisher
                        .put(DATABASE_NETWORK_ID.to_string().as_bytes())
                        .await
                        .expect("Failed to announce leadership");
                }

                sleep(Duration::from_millis(SYNC_INTERVAL)).await;
            }
        });
    }
    // Synchronization (STOP) ====================================================================================================

    // Database (START) ====================================================================================================
    // Add together all streaming topics into 1 big vector array
    let mut all_data_streams: Vec<DataStreamConfig> = Vec::new();
    all_data_streams.extend(DATA_STREAMS_ELEVATOR.iter().cloned()); // Clone the data from Lazy
    all_data_streams.extend(DATA_STREAMS_MANAGER.iter().cloned()); // Clone the data from Lazy

    // Data Monitor, Store and Broadcast
    for stream in all_data_streams {
        let temp_topic = stream.temp_topic;
        let stor_topic = stream.stor_topic;
        let shared_data = stream.shared_data.clone();
        let leader = leader.clone();

        let subscriber_temp = network_session
            .declare_subscriber(temp_topic)
            .await
            .expect("Failed to declare temp subscription topic");

        let publisher_stor = network_session.declare_publisher(stor_topic).await.expect("Failed to declare stor publisher");

        tokio::spawn(async move {
            loop {
                // Wait for a new message or timeout depending on the data stream settings
                if stream.rebroadcast_interval != 0 {
                    let result = wait_with_timeout(stream.rebroadcast_interval, subscriber_temp.recv_async()).await;

                    // Process received data
                    if let Some(message) = result {
                        let data_new = parse_message_to_string(message);

                        // Update shared data
                        let mut data = shared_data.lock().await;
                        *data = data_new.to_string();
                    } else {
                        // No new data received or timeout occurred
                        // Do nothing
                    }
                } else {
                    match subscriber_temp.recv_async().await {
                        Ok(message) => {
                            let data_new = parse_message_to_string(message);

                            // Update shared data
                            let mut data = shared_data.lock().await;
                            *data = data_new.to_string();
                        }
                        Err(e) => {
                            // Log an error if receiving a message fails
                            println!("#========================================#");
                            println!("ERROR: Failed to receive data from {}", temp_topic);
                            println!("Error code: {}", e);
                            println!("Killing myself...");
                            println!("ReeeEEEEeeee!");
                            println!("#========================================#");
                            std::process::exit(1);
                        }
                    }
                }

                // Publish the value stored in shared data if this node is the leader
                if *leader.read().await {
                    let data = shared_data.lock().await.clone();
                    if publisher_stor.put(data.as_bytes()).await.is_ok() {
                        // Successfully sent data
                        // Do Nothing

                        // println!("DEBUG: {}: {}", stor_topic, data);
                    } else {
                        // Log an error if sending a message fails
                        println!("#========================================#");
                        println!("ERROR: Failed to send data to {}", stor_topic);
                        println!("Killing myself...");
                        println!("Bruuhhhhhh =-=");
                        println!("#========================================#");
                        std::process::exit(1);
                    }
                }
            }
        });
    }
    // Database (STOP) ====================================================================================================

    // Elevator Data Synchronization (START) ====================================================================================================
    // Set up notifications ----------
    // Notification channel (tx = sender, rx = receiver)
    // When any of the states update, we notify the main synchronization thread through this cannel
    // This way, the final synchronization thread for all the elevators data can do its job
    // It will combine and decide if we should send the data or not
    let (notify_tx, mut notify_rx) = watch::channel(false); // Initial value is `false`, meaning no updates yet

    // Elevator State Sync Threads ----------
    // Instead of polling, this section listens for real-time updates
    // Each elevator gets **its own async task** that waits for new messages
    // When a new state update arrives, it is **immediately** written to the HashMap
    let network_session_clone = network_session.clone();
    let mut index = 0; // Reset index for correct elevator ID mapping

    for stream in DATA_STREAMS_ELEVATOR.iter() {
        if stream.stor_topic.contains("/states") {
            if let Some(&id) = ELEVATOR_NETWORK_ID_LIST.get(index) {
                // Each thread listens to its designated `stor/elevator{id}/states` topic.
                let subscriber = network_session_clone
                    .declare_subscriber(stream.stor_topic)
                    .await
                    .expect("Failed to declare stor subscription topic");

                let elevator_states_clone = elevator_states.clone();
                let id_clone = id; // Copy ID (i64 is Copy, no need for .clone())
                let notify_tx_clone = notify_tx.clone();

                // Runs **forever**, listening for new messages.
                // Updates only the **correct elevator's** state when data arrives.
                tokio::spawn(async move {
                    while let Ok(message) = subscriber.recv_async().await {
                        let new_state = parse_message_to_string(message);

                        let mut elevator_states = elevator_states_clone.write().await;
                        elevator_states.insert(id_clone, new_state.clone());

                        // Notify listeners that an update happened
                        let _ = notify_tx_clone.send(true);

                        // println!("DEBUG: Updated Elevator {} State: {}", id_clone, new_state);
                    }
                });
            }

            index += 1; // Only increment if we successfully mapped an elevator
        }
    }

    // Elevator Hall Request Sync Threads ----------
    // Synchronize hall request data
    // Very similar to Elevator State Sync Threads
    // However here there is only one thread
    // Since any elevator can write to this topic it is a synchronization topic of its own
    // And the data goes to a HashSet for good data structure

    // Subscribe to HALL_REQUESTS_SYNC updates
    let hall_requests_up_clone = hall_requests_up.clone();
    let hall_requests_down_clone = hall_requests_down.clone();
    let notify_tx_clone = notify_tx.clone();

    let hall_requests_subscriber = network_session
        .declare_subscriber(HALL_REQUESTS_SYNC)
        .await
        .expect("Failed to subscribe to HALL_REQUESTS_SYNC");

    tokio::spawn(async move {
        while let Ok(message) = hall_requests_subscriber.recv_async().await {
            // Convert Zenoh message to a JSON string
            let json_str = parse_message_to_string(message);

            // Attempt to deserialize JSON into `ElevHallRequests`
            if let Ok(request) = serde_json::from_str::<ElevHallRequests>(&json_str) {
                // Add/Remove requests in HashSets based on received data
                if let Some(floor) = request.add_up {
                    let mut hall_set_up = hall_requests_up_clone.write().await;
                    hall_set_up.insert(floor);
                }
                if let Some(floor) = request.add_down {
                    let mut hall_set_down = hall_requests_down_clone.write().await;
                    hall_set_down.insert(floor);
                }
                if let Some(floor) = request.remove_up {
                    let mut hall_set_up = hall_requests_up_clone.write().await;
                    hall_set_up.remove(&floor);
                }
                if let Some(floor) = request.remove_down {
                    let mut hall_set_down = hall_requests_down_clone.write().await;
                    hall_set_down.remove(&floor);
                }
            } else {
                eprintln!("ERROR: Failed to deserialize Hall Request JSON: {:#?}", json_str);
            }

            // Notify listeners that an update happened
            let _ = notify_tx_clone.send(true);
        }
    });

    // Elevator Hall Request Backup Threads ----------
    // NOTE: Backup only happens if our node is the leader
    // We also want to back up Hall Requests that are currently pending
    // This is done so in case our database crashes, we can always recover from backup
    // Meaning we never lose our previous hall requests
    let backup_hall_requests_up_publisher = network_session
        .declare_publisher(HALL_REQUESTS_UP_STOR)
        .await
        .expect("Failed to declare UP requests publisher");
    let backup_hall_requests_down_publisher = network_session
        .declare_publisher(HALL_REQUESTS_DOWN_STOR)
        .await
        .expect("Failed to declare DOWN requests publisher");

    let hall_requests_up_clone = hall_requests_up.clone();
    let hall_requests_down_clone = hall_requests_down.clone();
    let leader_clone = leader.clone();

    tokio::spawn(async move {
        loop {
            // NOTE: Only backup data if you are the leader
            // If not we just sit and wait
            if *leader_clone.read().await {
                let hall_up = hall_requests_up_clone.read().await;
                let hall_down = hall_requests_down_clone.read().await;

                backup_hall_requests_up_publisher
                    .put(format!("{:?}", hall_up).to_string().as_bytes())
                    .await
                    .expect("Failed to backup Hall Requests UP");
                backup_hall_requests_down_publisher
                    .put(format!("{:?}", hall_down).to_string().as_bytes())
                    .await
                    .expect("Failed to backup Hall Requests DOWN");

                sleep(Duration::from_millis(HALL_REQUESTS_BACKUP_INTERVAL)).await;
            }
        }
    });

    // Elevator Data Synchronization Thread ----------
    // NOTE: Synchronization only happens if our node is the leader
    let elevator_data_sync_publisher = network_session
        .declare_publisher(ELEVATOR_DATA_SYNC)
        .await
        .expect("Failed to declare Elevator Data Synchronization publisher");

    let elevator_states_clone = elevator_states.clone();
    let hall_requests_up_clone = hall_requests_up.clone();
    let hall_requests_down_clone = hall_requests_down.clone();
    let leader_clone = leader.clone();

    tokio::spawn(async move {
        while notify_rx.changed().await.is_ok() {
            // NOTE: Only send synchronized data if we are the leader
            // Otherwise we just wait for new change
            if *leader_clone.read().await {
                // Since we are a leader and there was a change we get to combine the data into a single JSON string to output

                let elev_states = elevator_states_clone.read().await;
                let hall_up = hall_requests_up_clone.read().await;
                let hall_down = hall_requests_down_clone.read().await;

                // Format JSON -----
                // Read elevator states
                let mut formatted_elevators: HashMap<String, ElevState> = HashMap::new();

                for (&id, state_json) in elev_states.iter() {
                    if let Ok(state) = serde_json::from_str::<ElevState>(state_json) {
                        formatted_elevators.insert(id.to_string(), state);
                    }
                }

                // Read hall requests
                let mut hall_requests_2d = vec![vec![false, false]; (*NUMBER_FLOORS).into()];

                // UP
                for &floor in hall_up.iter() {
                    if floor < *NUMBER_FLOORS {
                        hall_requests_2d[floor as usize][1] = true;
                    }
                }

                // DOWN
                for &floor in hall_down.iter() {
                    if floor < *NUMBER_FLOORS {
                        hall_requests_2d[floor as usize][0] = true;
                    }
                }

                // Construct the final system state
                let system_state = AlgoInput { hallRequests: hall_requests_2d, states: formatted_elevators };

                // Convert it into JSON format
                let json_output = serde_json::to_string_pretty(&system_state).expect("Failed to serialize system state");

                // println!("DEBUG: Json: {:#?}", json_output);

                // Send JSON to manager node
                elevator_data_sync_publisher
                    .put(json_output.as_bytes())
                    .await
                    .expect("Failed to publish Elevator Data Synchronization");
            }
        }
    });
    // Elevator Data Synchronization (STOP) ====================================================================================================
    // Keep the program running
    loop {
        tokio::task::yield_now().await; // Yield to other tasks
    }
}
