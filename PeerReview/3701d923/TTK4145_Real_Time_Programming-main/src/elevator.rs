// * NOTE: We don't actually need to run the node in "sudo", however for consistency with other nodes that require "sudo" we have made this node also run in "sudo"
// * NOTE: Every new elevator needs its own unique ELEVATOR_NETWORK_ID and ELEVATOR_HARDWARE_PORT
// *
// * Because of these factors we must run this process as follows:
// * $ sudo -E ELEVATOR_NETWORK_ID=<ID> ELEVATOR_HARDWARE_PORT=<PORT> NUMBER_FLOORS=<NUMBER FLOORS> cargo run --bin elevator

// Library that allows us to use environment variables or command-line arguments to pass variables from terminal to the program directly
use std::env;

// Libraries for real time systems
use std::collections::HashSet;
use std::sync::Arc;

use tokio::spawn;
use tokio::sync::{mpsc, RwLock};
use tokio::task::yield_now;
use tokio::time::{self, Duration};

// Libraries for highly customizable distributed network mode setup
use std::fs;
use std::path::Path;
use zenoh::Config;

// Libraries for distributed network
use zenoh::open;

// Libraries for network status and diagnostics
// * NOTE: Because of some functions in utils library that uses ICMP sockets, witch are ONLY available for root user, we must run our program as sudo cargo run
use elevator_system::distributed_systems::utils::{get_router_ip, network_status, parse_message_to_elevator_backup, parse_message_to_elevator_requests, wait_with_timeout, ElevatorBackup, NetworkStatus};

// Import necessary drivers for controlling the elevator
use elevator_system::elevator_io::{data, driver};
use elevator_system::elevator_logic::state_machine;
use elevator_system::elevator_logic::utils::{create_hall_request_json, Direction, State};

// Import elevator manager algorithm library because when sending data we have to format our data in a way that other manager algorithm nodes can understand it
use elevator_system::elevator_algorithm::utils::ElevState;

// Library for constructing data structures that are thread safe
use once_cell::sync::Lazy;

// Global Variable ----------
const NETWORK_CHECK_INTERVAL: u64 = 5000; // ms
const POLL_INTERVAL: u64 = 200; // ms
const HEARTBEAT_INTERVAL: u64 = 1000; // ms
const BACKUP_INTERVAL: u64 = 500; // ms

// Topics for datastream ----------
const HALL_REQUESTS_SYNC_TOPIC: &str = "sync/elevator/hall/requests";

const MANAGER_TOPIC: &str = "stor/manager/request";

struct Topics {
    heartbeat: &'static str,

    elevator_states: &'static str,

    backup_temp: &'static str,
    backup_stor: &'static str,
}

// NOTE: Box::leak() is a powerful yet dangerous command
// NOTE: Used inappropriately in a dynamic continuous running process, it will clog up the memory as it is never deallocated, causing memory leaks and overflows
// NOTE: However for use in startup where the values will never be manipulated afterwards this is a safe way to us it in
// NOTE: Since we only manipulate memory on startup for Topics, we don't have to worry about memory leaks and overflows :)
impl Topics {
    fn new(elevator_id: i64) -> Self {
        let id = elevator_id.to_string();
        Topics {
            heartbeat: Box::leak(format!("temp/elevator{}/heartbeat", id).into_boxed_str()),

            elevator_states: Box::leak(format!("temp/elevator{}/states", id).into_boxed_str()),

            backup_temp: Box::leak(format!("temp/elevator{}/backup", id).into_boxed_str()),
            backup_stor: Box::leak(format!("stor/elevator{}/backup", id).into_boxed_str()),
        }
    }
}

// Set up environment variables ----------
// Create a static parameter for number of floors this specific elevator serves
// If none => Default to NUMBER FLOORS: 4
static NUMBER_FLOORS: Lazy<u8> = Lazy::new(|| {
    env::var("NUMBER_FLOORS")
        .unwrap_or_else(|_| "4".to_string())
        .parse()
        .expect("NUMBER_FLOORS must be a valid integer")
});

// Create a static parameter for the hardware address using the port from the environment
// If none => Default to PORT: localhost:15657
// !NOTE: Every new Rust process needs their own unique ELEVATOR_HARDWARE_PORT
static ELEVATOR_HARDWARE_PORT: Lazy<&'static str> = Lazy::new(|| {
    // Read the port from env, defaulting to "15657"
    let port = env::var("ELEVATOR_HARDWARE_PORT").unwrap_or_else(|_| "15657".to_string());
    // Build the address and leak it to get a &'static str.
    Box::leak(format!("localhost:{}", port).into_boxed_str())
});

// Existing topics static block remains, now printing the hardware address as well
// If none => Default to ID: 0
// !NOTE: Every new Rust process needs their own unique ELEVATOR_NETWORK_ID
static ELEVATOR_NETWORK_ID: Lazy<i64> = Lazy::new(|| {
    env::var("ELEVATOR_NETWORK_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .expect("ELEVATOR_NETWORK_ID must be a valid integer")
});

// Build topics with our ELEVATOR_NETWORK_ID
static TOPICS: Lazy<Topics> = Lazy::new(|| Topics::new(*ELEVATOR_NETWORK_ID));

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    // Elevator Initialization (START) ====================================================================================================
    println!("ELEVATOR_NETWORK_ID: {}", *ELEVATOR_NETWORK_ID);
    println!("ELEVATOR_HARDWARE_PORT: {}", *ELEVATOR_HARDWARE_PORT);
    println!("NUMBER_FLOORS: {}", *NUMBER_FLOORS);
    println!();

    // Start elevator
    let elevator = driver::Elevator::init(*ELEVATOR_HARDWARE_PORT, *NUMBER_FLOORS).await;
    println!("Elevator initialized:\n{:#?}", elevator);
    println!("Jipppyyyyyy!");
    println!();

    // Start by turn off all the lights
    // The lights that should be on will turn on eventually after the backup data kicks in
    let elevator_clone = elevator.clone();
    spawn(async move {
        for floor in 0..*NUMBER_FLOORS {
            elevator_clone.call_button_light(floor, 0, false).await; // Turn OFF UP light
            elevator_clone.call_button_light(floor, 1, false).await; // Turn OFF DOWN light
            elevator_clone.call_button_light(floor, 2, false).await; // Turn OFF CAB light
        }
    });
    // Elevator Initialization (STOP) ====================================================================================================

    // Distributed Network Initialization (START) ====================================================================================================
    // Specify path to highly customable network modes for distributed networks
    // Most important settings: peer-2-peer and scouting to alow multicast and robust network connectivity
    // Then Load configuration from JSON5 file
    // Finally initialize networking session
    let networking_config_path = Path::new("network_config.json5");

    let networking_config_data = fs::read_to_string(networking_config_path).expect("Failed to read the network_config.json5 file");
    let config: Config = Config::from_json5(&networking_config_data).expect("Failed to parse the network_config.json5 file");

    let network_session = open(config).await.expect("Failed to open Zenoh session");
    // Distributed Network Initialization (STOP) ====================================================================================================

    // GET - NETWORK: Data backup Initialization (START) ====================================================================================================
    // Shared states initialization
    let state = Arc::new(RwLock::new(State::Idle));
    let direction = Arc::new(RwLock::new(Direction::Stop));
    let current_floor = Arc::new(RwLock::new(None::<u8>));
    let cab_queue = Arc::new(RwLock::new(HashSet::new()));
    let hall_up_queue = Arc::new(RwLock::new(HashSet::new()));
    let hall_down_queue = Arc::new(RwLock::new(HashSet::new()));

    // Create backup storage subscribers
    let backup_subscriber = network_session
        .declare_subscriber(TOPICS.backup_stor)
        .await
        .expect("Failed to declare Backup subscriber");

    // Create tasks to get backup data if it exists
    // Wait for some stored data
    // If we don't get any in a certain amount of time we assume there is no stored data, so we pass
    // If we find stored data being published we save it internally in our data base
    // 5 000 ms because it takes some time for network config to configure our networking protocol, thus we need to compensate for it
    // + wait a bit for a given broadcast interval just to be sure
    let mut tasks = Vec::new();
    let backup_init_timeout = 10000 + BACKUP_INTERVAL;

    let state_clone = state.clone();
    let direction_clone = direction.clone();
    let current_floor_clone = current_floor.clone();
    let cab_queue_clone = cab_queue.clone();
    let hall_up_queue_clone = hall_up_queue.clone();
    let hall_down_queue_clone = hall_down_queue.clone();
    tasks.push(tokio::spawn(async move {
        let result = wait_with_timeout(backup_init_timeout, backup_subscriber.recv_async()).await;

        if let Some(message) = result {
            if let Some(backup_data) = parse_message_to_elevator_backup(message) {
                println!("New data from: {}: {:#?}", TOPICS.backup_stor, backup_data);

                // Save data to the correct location
                {
                    let mut data = state_clone.write().await;
                    *data = backup_data.state;
                }
                {
                    let mut data = direction_clone.write().await;
                    *data = backup_data.direction;
                }
                {
                    let mut data = current_floor_clone.write().await;
                    *data = backup_data.current_floor;
                }
                {
                    let mut data = cab_queue_clone.write().await;
                    *data = backup_data.cab_queue;
                }
                {
                    let mut data = hall_up_queue_clone.write().await;
                    *data = backup_data.hall_up_queue;
                }
                {
                    let mut data = hall_down_queue_clone.write().await;
                    *data = backup_data.hall_down_queue;
                }
            } else {
                println!("Failed to parse data from: {}", TOPICS.backup_stor);
            }
        } else {
            println!("No new data from: {}", TOPICS.backup_stor);
        }
    }));

    for task in tasks {
        let _ = task.await;
    }
    // GET - NETWORK: Data backup Initialization (START) ====================================================================================================

    // Network Monitoring (START) ====================================================================================================
    // Spawn a separate task to check network status every so often
    // If we detect we have been disconnected from the network we signal it by changing the shared resource for network state
    let on_the_network = Arc::new(RwLock::new(true));

    let on_the_network_clone = on_the_network.clone();
    tokio::spawn(async move {
        let router_ip = match get_router_ip().await {
            Some(ip) => ip,
            None => {
                println!("#========================================#");
                println!("ERROR: Failed to retrieve the router IP");
                println!("Killing myself...");
                println!("Jinkies (=o.o=)");
                println!("#========================================#");
                std::process::exit(1);
            }
        };

        loop {
            match network_status(router_ip).await {
                NetworkStatus::Connected => {
                    let mut on_the_network = on_the_network_clone.write().await;
                    *on_the_network = true;
                }
                NetworkStatus::Disconnected => {
                    println!("WARNING: Disconnected from the network!");

                    let mut on_the_network = on_the_network_clone.write().await;
                    *on_the_network = false;
                }
            }

            tokio::time::sleep(Duration::from_millis(NETWORK_CHECK_INTERVAL)).await;
        }
    });
    // Network Monitoring (STOP) ====================================================================================================

    // 6 - READING: Button orders (START) ====================================================================================================
    // Create publisher that send hall button requests UP/DOWN to add them to the manager later
    let hall_requests_publisher = network_session
        .declare_publisher(HALL_REQUESTS_SYNC_TOPIC)
        .await
        .expect("Failed to declare Hall Requests publisher");

    // Create a channel for button call updates
    let (button_tx, mut button_rx) = mpsc::channel(32);

    // Poll button calls and send updates
    let elevator_clone = elevator.clone();
    spawn(async move {
        data::call_buttons(elevator_clone, button_tx, Duration::from_millis(POLL_INTERVAL)).await;
    });

    // Process button call updates and update states
    // If its Floor call, we instead send it to the distributed network for manager node decide
    let cab_queue_clone = cab_queue.clone();
    spawn(async move {
        while let Some(button) = button_rx.recv().await {
            match button.call {
                2 => {
                    // CAB button
                    {
                        let mut cab = cab_queue_clone.write().await;
                        cab.insert(button.floor);
                    }
                }
                1 => {
                    // DOWN button
                    {
                        let request = create_hall_request_json(None, Some(button.floor), None, None);

                        hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button DOWN");
                    }
                }
                0 => {
                    // UP button
                    {
                        let request = create_hall_request_json(Some(button.floor), None, None, None);

                        hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button UP");
                    }
                }
                _ => {
                    // Ignore invalid types
                }
            }
        }
    });
    // 6 - READING: Button orders (STOP) ====================================================================================================

    // 7 - READING: Floor sensor (START) ====================================================================================================
    // Create a channel for hall sensor updates
    let (floor_tx, mut floor_rx) = mpsc::channel(32);

    // Poll hall sensor and send updates
    let elevator_clone = elevator.clone();
    spawn(async move {
        data::floor_sensor(elevator_clone, floor_tx, Duration::from_millis(50)).await;
    });

    // Process hall updates and print state
    let current_floor_clone = current_floor.clone();
    spawn(async move {
        while let Some(floor) = floor_rx.recv().await {
            // Check if the new value is different or if the current value is None
            let current_floor = *current_floor_clone.read().await;

            if current_floor.is_none() || current_floor != Some(floor) {
                let mut current_floor = current_floor_clone.write().await;
                *current_floor = Some(floor);
            }
        }
    });
    // 7 - READING: Floor sensor (STOP) ====================================================================================================

    // 8 - READING: Stop button (START) ==================================================================================================
    // Shared state for stop button light
    let stop_button_state = Arc::new(RwLock::new(false)); // Initially, stop button is NOT pressed
    let stop_button_state_clone = Arc::clone(&stop_button_state);

    // Control stop button light
    let elevator_clone = elevator.clone();
    let (stop_tx, mut stop_rx) = mpsc::channel(32);

    // Spawn a task to poll the stop button state
    spawn(async move {
        data::stop_button(elevator_clone.clone(), stop_tx, Duration::from_millis(POLL_INTERVAL)).await;
    });

    // Spawn a task to update the stop button state
    spawn(async move {
        while let Some(stop_button) = stop_rx.recv().await {
            {
                let mut stop_state = stop_button_state_clone.write().await;
                *stop_state = stop_button;
            }
        }
    });
    // 8 - READING: Stop button (STOP) ==================================================================================================

    // 9 - READING: Obstruction switch (START) ==================================================================================================
    // Control obstruction switch state
    let obstruction_state = Arc::new(RwLock::new(false)); // Shared state for obstruction switch
    let obstruction_state_clone = Arc::clone(&obstruction_state);
    let elevator_clone = elevator.clone();
    let (obstruction_tx, mut obstruction_rx) = mpsc::channel(32);

    // Spawn a task to poll the obstruction switch state
    spawn(async move {
        data::obstruction(elevator_clone.clone(), obstruction_tx, Duration::from_millis(POLL_INTERVAL)).await;
    });

    // Spawn a task to update the obstruction state
    spawn(async move {
        while let Some(is_active) = obstruction_rx.recv().await {
            {
                let mut obstruction = obstruction_state_clone.write().await;
                *obstruction = is_active; // Update the obstruction switch state
            }
        }
    });
    // 9 - READING: Obstruction switch (STOP) ==================================================================================================

    // GET - NETWORK: Listen to manager (START) ==================================================================================================
    // We listen to manager hall delegation
    // Once we get a hall delegated to us
    // We save the order to a temp buffer
    // Then we save requests to hall UP/DOWN queue
    // This way if there are any more orders pending we only have to use shared resources only once
    //
    // In addition, if someone on the elevator pressed STOP button
    // This means we are in a emergency, witch in turn means we should stop listening to the outside world as well
    // The only thing that matters in a emergency situation is people inside the elevator cab
    // Because of this, in case of emergency stop, we also stop listening to the manager requests
    // We still receive manager data and keep track of whats going on in the network
    // However we simply disobey manager commands as this is an emergency
    // We also stop heartbeat, meaning manager will sooner or later realize something went wrong and divert requests to other elevators
    // Leaving our emergency stop elevator in piece until we have sorted stuff out
    //
    // In addition we will check all the manager requests, not just only ours
    // This way we can display all the active hall call through button LEDS later on in the process
    // We check this no matter the state, even in emergency state we update LEDs for hall
    // NOTE: In case of network disconnect, global LEDs will get set to 0

    // Create global hall LED display
    let global_leds_up: Arc<RwLock<HashSet<u8>>> = Arc::new(RwLock::new(HashSet::new()));
    let global_leds_down: Arc<RwLock<HashSet<u8>>> = Arc::new(RwLock::new(HashSet::new()));

    // Manager Hall Requests Thread ----------
    let manager_request_subscriber = network_session
        .declare_subscriber(MANAGER_TOPIC)
        .await
        .expect("Failed to declare subscriber for Manager Request Up");

    let state_clone = state.clone();
    let hall_up_queue_clone = hall_up_queue.clone();
    let hall_down_queue_clone = hall_down_queue.clone();
    spawn(async move {
        loop {
            match manager_request_subscriber.recv_async().await {
                Ok(message) => {
                    if let Some(elevator_requests) = parse_message_to_elevator_requests(message) {
                        // Check if the received data contains our elevator ID
                        if let Some(hall_requests) = elevator_requests.requests.get(&*ELEVATOR_NETWORK_ID.to_string()) {
                            // Use scoped locks to prevent holding lock for to long
                            let state = {
                                let state = state_clone.read().await;
                                state.clone() // Copy the state, avoiding unnecessary clones
                            };

                            // Finally ensure that our elevator is NOT in emergency (ie, no STOP button has been pressed)
                            // If elevator is in good state, we listen to the manager
                            // If elevator is in any emergency state, then we disobey manager orders by never reading them
                            if state != State::EmergencyStop && state != State::EmergencyStopIdle {
                                // Temporary buffers for up/down hall requests
                                let mut temp_up = HashSet::new();
                                let mut temp_down = HashSet::new();

                                // Iterate over our elevator's assigned hall requests
                                for (floor, hall) in hall_requests.iter().enumerate() {
                                    if hall[0] {
                                        temp_down.insert(floor as u8);
                                    }
                                    if hall[1] {
                                        temp_up.insert(floor as u8);
                                    }
                                }

                                // println!("DEBUG: Hall UP: {:#?}", temp_up);
                                // println!("DEBUG: Hall DOWN: {:#?}", temp_down);

                                // Efficiently update the shared HashSets in one go
                                // NOTE: We only update it if we read the difference between the current and received hall requests
                                // The READ lock comes in clutch by letting us read without sacrificing concurrency
                                // And enclosed in if statement we only use the actual lock in Write for a split second
                                // Combined with only writing when necessary this is super fast
                                // (The magic of rust compiler never cease to amaze me, WoooOOoowWw... *o*)
                                {
                                    let down_queue = {
                                        let down_queue = hall_down_queue_clone.read().await;
                                        down_queue.clone() // Clone the HashSet into a separate variable
                                    }; // Lock is released here

                                    if down_queue != temp_down {
                                        let mut down_queue = hall_down_queue_clone.write().await;
                                        *down_queue = temp_down;
                                    }
                                }
                                {
                                    let up_queue = {
                                        let up_queue = hall_up_queue_clone.read().await;
                                        up_queue.clone() // Clone the HashSet into a separate variable
                                    }; // Lock is released here

                                    if up_queue != temp_up {
                                        let mut up_queue = hall_up_queue_clone.write().await;
                                        *up_queue = temp_up;
                                    }
                                }
                            }
                        } else {
                            // println!("DEBUG: No hall requests found for Elevator ID: {}", *ELEVATOR_NETWORK_ID);
                        }
                    } else {
                        // println!("DEBUG: Received invalid data for Manager Request");
                    }
                }
                Err(e) => {
                    println!("Error receiving from topic: {}", MANAGER_TOPIC);
                    println!("Error code: {}", e);
                }
            }
        }
    });

    // Global LEDs from Manager Thread ----------
    let manager_request_subscriber = network_session
        .declare_subscriber(MANAGER_TOPIC)
        .await
        .expect("Failed to declare subscriber for Manager Request Up");

    let global_leds_up_clone = global_leds_up.clone();
    let global_leds_down_clone = global_leds_down.clone();
    spawn(async move {
        loop {
            match manager_request_subscriber.recv_async().await {
                Ok(message) => {
                    if let Some(elevator_requests) = parse_message_to_elevator_requests(message) {
                        // Save Global LED states
                        let mut global_up_temp = HashSet::new();
                        let mut global_down_temp = HashSet::new();

                        // Iterate through all received elevator hall requests
                        for (_elevator_id, hall_requests) in &elevator_requests.requests {
                            for (floor, hall) in hall_requests.iter().enumerate() {
                                if hall[0] {
                                    global_down_temp.insert(floor as u8);
                                }
                                if hall[1] {
                                    global_up_temp.insert(floor as u8);
                                }
                            }
                        }

                        // Efficiently update global LED hall requests
                        {
                            let current_global_led_down = {
                                let current_global_led_down = global_leds_down_clone.read().await;
                                current_global_led_down.clone() // Clone the HashSet into a separate variable
                            }; // Lock is released here

                            if current_global_led_down != global_down_temp {
                                let mut current_global_led_down = global_leds_down_clone.write().await;
                                *current_global_led_down = global_down_temp;
                            }
                        }
                        {
                            let current_global_led_up = {
                                let current_global_led_up = global_leds_up_clone.read().await;
                                current_global_led_up.clone() // Clone the HashSet into a separate variable
                            }; // Lock is released here

                            if current_global_led_up != global_up_temp {
                                let mut current_global_led_up = global_leds_up_clone.write().await;
                                *current_global_led_up = global_up_temp;
                            }
                        }
                    } else {
                        // println!("DEBUG: Received invalid data for Manager LEDs Request");
                    }
                }
                Err(e) => {
                    println!("Error receiving from topic: {}", MANAGER_TOPIC);
                    println!("Error code: {}", e);
                }
            }
        }
    });

    // Separate thread if we are outside network
    // If offline we must reset global LEDs
    let global_leds_up_clone = global_leds_up.clone();
    let global_leds_down_clone = global_leds_down.clone();
    let on_the_network_clone = on_the_network.clone();

    spawn(async move {
        let mut interval = time::interval(Duration::from_secs(2)); // Check every 2 seconds

        loop {
            interval.tick().await;

            // Check network state
            let network_status = *on_the_network_clone.read().await;
            if !network_status {
                println!("NETWORK DISCONNECTED - RESETTING GLOBAL LEDS!");

                // Reset global LEDs if offline
                *global_leds_up_clone.write().await = HashSet::new();
                *global_leds_down_clone.write().await = HashSet::new();
            }
        }
    });
    // GET - NETWORK: Listen to manager (STOP) ==================================================================================================

    // 2 - WRITING: Button order light (START) ==================================================================================================
    // PROBLEM:
    // - Updating button lights (CAB, UP, DOWN) involves toggling lights for all halls sequentially,
    //   but the elevator hardware IO is slow, causing high latency when toggling unnecessary lights.
    // - In a distributed network, hall requests can come from different elevators,
    //   meaning we need to track global requests as well as our own.
    //
    // SOLUTION:
    // - Use local HashSets (`local_cab_queue`, `local_hall_up_queue`, `local_hall_down_queue`) to track
    //   current light states and compare them with the combined real queues (`cab_queue`, `hall_up_queue`, `hall_down_queue`).
    // - Merge global hall requests (`global_leds_up`, `global_leds_down`) with local ones before updating lights.
    // - Only toggle lights (ON/OFF) when a mismatch is detected:
    //   1. **Turn ON a light** if it's in either the local or global request queue but not already in the local LED state.
    //   2. **Turn OFF a light** if it's not in either queue but still exists in the local LED state.
    //
    // NETWORK FAILOVER HANDLING:
    // - If the network goes down or disconnects, the **global LED values reset to 0**, meaning all global hall request LEDs turn OFF.
    // - However, **local hall requests remain ON** ensuring proper behavior in case of network failure.
    //
    // BENEFITS:
    // - Reduces unnecessary IO operations, minimizing latency.
    // - Ensures faster and consistent light updates.
    // - Scales efficiently with more halls or button types.
    // - Ensures hall lights remain on **even if the network fails**, preventing misleading visual indicators.

    // Control order button lights for cab calls and hall calls
    let cab_queue_clone = cab_queue.clone();
    let hall_up_queue_clone = hall_up_queue.clone();
    let hall_down_queue_clone = hall_down_queue.clone();
    let global_leds_up_clone = global_leds_up.clone();
    let global_leds_down_clone = global_leds_down.clone();
    let elevator_clone = elevator.clone();

    spawn(async move {
        // Local HashMaps to keep track of the current button states
        let mut local_cab_queue: HashSet<u8> = HashSet::new();
        let mut local_hall_up_queue: HashSet<u8> = HashSet::new();
        let mut local_hall_down_queue: HashSet<u8> = HashSet::new();

        // Calculate perfect period so that we update all lights at predictable frequency
        let mut interval = time::interval(Duration::from_millis(POLL_INTERVAL));

        loop {
            // CAB Lights
            let cab_queue = cab_queue_clone.read().await;
            for hall in 0..*NUMBER_FLOORS {
                // Check if the light needs to be turned ON
                if cab_queue.contains(&hall) && !local_cab_queue.contains(&hall) {
                    elevator_clone.call_button_light(hall, 2, true).await; // Turn ON CAB light
                    local_cab_queue.insert(hall); // Update local state
                }
                // Check if the light needs to be turned OFF
                if !cab_queue.contains(&hall) && local_cab_queue.contains(&hall) {
                    elevator_clone.call_button_light(hall, 2, false).await; // Turn OFF CAB light
                    local_cab_queue.remove(&hall); // Update local state
                }
            }

            // Floor DOWN Lights
            let down_queue = hall_down_queue_clone.read().await;
            let global_down = global_leds_down_clone.read().await;

            // Create merged set of all active down requests (local + global)
            let merged_down: HashSet<u8> = down_queue.union(&*global_down).cloned().collect();

            for hall in 0..*NUMBER_FLOORS {
                if merged_down.contains(&hall) && !local_hall_down_queue.contains(&hall) {
                    elevator_clone.call_button_light(hall, 1, true).await; // Turn ON DOWN light
                    local_hall_down_queue.insert(hall);
                }
                if !merged_down.contains(&hall) && local_hall_down_queue.contains(&hall) {
                    elevator_clone.call_button_light(hall, 1, false).await; // Turn OFF DOWN light
                    local_hall_down_queue.remove(&hall);
                }
            }

            // Floor UP Lights
            let up_queue = hall_up_queue_clone.read().await;
            let global_up = global_leds_up_clone.read().await;

            // Create merged set of all active up requests (local + global)
            let merged_up: HashSet<u8> = up_queue.union(&*global_up).cloned().collect();

            for hall in 0..*NUMBER_FLOORS {
                if merged_up.contains(&hall) && !local_hall_up_queue.contains(&hall) {
                    elevator_clone.call_button_light(hall, 0, true).await; // Turn ON UP light
                    local_hall_up_queue.insert(hall);
                }
                if !merged_up.contains(&hall) && local_hall_up_queue.contains(&hall) {
                    elevator_clone.call_button_light(hall, 0, false).await; // Turn OFF UP light
                    local_hall_up_queue.remove(&hall);
                }
            }

            interval.tick().await;
        }
    });
    // 2 - WRITING: Button order light (STOP) ==================================================================================================

    // 3 - WRITING: Floor indicator (START) ==================================================================================================
    // Control hall indicator light
    let current_floor_light = Arc::clone(&current_floor);
    let elevator_clone = elevator.clone();
    spawn(async move {
        loop {
            let current_hall = {
                let current_hall = current_floor_light.read().await;
                *current_hall
            };

            if let Some(hall) = current_hall {
                elevator_clone.floor_indicator(hall).await;
            }

            tokio::time::sleep(Duration::from_millis(POLL_INTERVAL)).await; // Periodic update light
        }
    });
    // 3 - WRITING: Floor indicator (STOP) ==================================================================================================

    // 5 - WRITING: Stop button light (START) ==================================================================================================
    // Spawn a task to update the stop button light
    let stop_button_state_clone = Arc::clone(&stop_button_state);
    let elevator_clone = elevator.clone();
    spawn(async move {
        loop {
            let stop_button = {
                let stop_button = stop_button_state_clone.read().await;
                *stop_button
            };

            elevator_clone.stop_button_light(stop_button).await;

            tokio::time::sleep(Duration::from_millis(POLL_INTERVAL)).await; // Periodic update light
        }
    });
    // 5 - WRITING: Stop button light (STOP) ==================================================================================================

    // SEND - NETWORK: Send heartbeat to manager (START) ====================================================================================================
    // Send a steady heartbeat to show that this elevator node is in the network and is ready to receive requests
    // NOTE: The only times we intentionally STOP sending heartbeat is in emergency state
    // If someone on the cab presses STOP button we stop the heartbeat
    // This way manager node and the rest of the network gets notified that something went wrong with our elevator
    // This way some other elevator can handle our Hall calls
    // Once we get back to normal states we resume the heartbeat
    // Signaling to the network we are again ready to take the requests
    let heartbeat_publisher = network_session
        .declare_publisher(TOPICS.heartbeat)
        .await
        .expect("Failed to declare heartbeat publisher");

    let state_clone = state.clone();

    spawn(async move {
        loop {
            // Use scoped locks to prevent holding lock for to long
            let state = {
                let state = state_clone.read().await;
                *state // Copy the state, avoiding unnecessary clones
            };

            // STOP sending heartbeat IF someone pressed the emergency STOP button
            if state != State::EmergencyStopIdle {
                heartbeat_publisher.put("BeepBoop ^-^".as_bytes()).await.expect("Failed to send heartbeat");
            }

            tokio::time::sleep(Duration::from_millis(HEARTBEAT_INTERVAL)).await;
        }
    });
    // SEND - NETWORK: Send heartbeat to manager (STOP) ====================================================================================================

    // SEND - NETWORK: Backup Data (START) ====================================================================================================
    // Create publishers for backing up data
    let backup_publisher = network_session
        .declare_publisher(TOPICS.backup_temp)
        .await
        .expect("Failed to declare Backup publisher");

    // Publish and backup data
    let state_clone = state.clone();
    let direction_clone = direction.clone();
    let current_floor_clone = current_floor.clone();
    let cab_queue_clone = cab_queue.clone();
    let hall_up_queue_clone = hall_up_queue.clone();
    let hall_down_queue_clone = hall_down_queue.clone();
    spawn(async move {
        loop {
            // Use scoped locks to prevent holding lock for to long
            let state = {
                let state = state_clone.read().await;
                state.clone()
            };
            let direction = {
                let direction = direction_clone.read().await;
                direction.clone()
            };
            let current_floor = {
                let current_floor = current_floor_clone.read().await;
                current_floor.clone()
            };
            let cab_queue = {
                let cab_queue = cab_queue_clone.read().await;
                cab_queue.clone()
            };
            let hall_up_queue = {
                let hall_up_queue = hall_up_queue_clone.read().await;
                hall_up_queue.clone()
            };
            let hall_down_queue = {
                let hall_down_queue = hall_down_queue_clone.read().await;
                hall_down_queue.clone()
            };

            // Format data to backup data format
            let backup_data = ElevatorBackup { state, direction, current_floor, cab_queue, hall_up_queue, hall_down_queue };

            // Convert it into JSON format
            let json_backup_data = serde_json::to_string_pretty(&backup_data).expect("Failed to serialize backup data");

            // Send JSON to backup
            backup_publisher.put(json_backup_data.as_bytes()).await.expect("Failed to backup data");

            tokio::time::sleep(Duration::from_millis(BACKUP_INTERVAL)).await;
        }
    });
    // SEND - NETWORK: Backup Data (STOP) ====================================================================================================

    // SEND - NETWORK: Elevator States Update (START) ====================================================================================================
    // Data to send only when there is a change in any of the following states and this specific order:
    //
    // State: idle/moving/doorOpen
    // Floor: 0-255
    // Direction: Up/Down/Stop
    // Cab queue: [<floor 0: true/false>, .... , <floor N: true/false>]
    let elevator_states_publisher = network_session
        .declare_publisher(TOPICS.elevator_states)
        .await
        .expect("Failed to declare Elevator States publisher");

    // Request sending data
    let state_clone = state.clone();
    let current_floor_clone = current_floor.clone();
    let direction_clone = direction.clone();
    let cab_queue_clone = cab_queue.clone();

    // Since we only want to send data on changes, that means that we must keep track of all the changes internally
    // This way we know when there is a difference and if so we send the whole request until no changes
    spawn(async move {
        // Local copies to track changes
        let mut local_state = State::Idle;
        let mut local_floor = None::<u8>;
        let mut local_direction = Direction::Stop;
        let mut local_cab_queue: HashSet<u8> = HashSet::new();

        loop {
            // Use scoped locks to prevent holding multiple locks simultaneously
            let state = {
                let state = state_clone.read().await;
                state.clone() // Clone into a local variable, lock is released here
            };
            let floor = {
                let floor = current_floor_clone.read().await;
                floor.clone() // Clone into a local variable, lock is released here
            };
            let direction = {
                let direction = direction_clone.read().await;
                direction.clone() // Clone into a local variable, lock is released here
            };
            let cab_queue = {
                let cab = cab_queue_clone.read().await;
                cab.clone() // Clone into a local variable, lock is released here
            };

            // Check if any values have changed
            let state_changed = state != local_state;
            let floor_changed = floor != local_floor;
            let direction_changed = direction != local_direction;
            let cab_changed = cab_queue != local_cab_queue;

            // If any value changed, send an update
            if state_changed || floor_changed || direction_changed || cab_changed {
                // Format request into JSON ----------
                let formatted_state = match state {
                    State::Idle | State::EmergencyStop | State::EmergencyStopIdle => "idle".to_string(),
                    State::Up | State::Down => "moving".to_string(),
                    State::Door => "doorOpen".to_string(),
                };

                let formatted_floor = floor.unwrap_or(255); // If None, default to 255 (invalid floor)

                let formatted_direction = format!("{:?}", direction).to_lowercase(); // Convert direction enum to string

                let formatted_cab_queue: Vec<bool> = (0..*NUMBER_FLOORS).map(|floor| cab_queue.contains(&floor)).collect();

                let elevator_states_data = ElevState {
                    behaviour: formatted_state,
                    floor: formatted_floor,
                    direction: formatted_direction,
                    cabRequests: formatted_cab_queue,
                };

                let elevator_states_data_formatted = serde_json::to_string(&elevator_states_data).expect("Failed to format elevator states");

                // Send request ----------
                elevator_states_publisher
                    .put(elevator_states_data_formatted.as_bytes())
                    .await
                    .expect("Failed to send Elevator States");

                // Update local copies to prevent unnecessary updates
                local_state = state.clone();
                local_floor = floor.clone();
                local_direction = direction.clone();
                local_cab_queue = cab_queue.clone();
            }

            // Wait for the next interval, a small timeout to not overwhelm other threads
            tokio::time::sleep(Duration::from_millis(POLL_INTERVAL)).await;
        }
    });
    // SEND - NETWORK: Elevator States Update (STOP) ====================================================================================================

    // STATE MACHINE (START) ==================================================================================================
    // Before starting state machine we wait a bit
    // This is because we want all values to be updated from sensors before we start running state machine for the elevator
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Create publisher that sends remove hall button requests UP/DOWN to the manager and rest of the system
    let hall_requests_publisher = network_session
        .declare_publisher(HALL_REQUESTS_SYNC_TOPIC)
        .await
        .expect("Failed to declare Hall Requests publisher");

    // Declare all shared variables necessary for the state machine
    let state_clone = state.clone();
    let direction_clone = direction.clone();
    let cab_queue_clone = cab_queue.clone();
    let hall_up_queue_clone = hall_up_queue.clone();
    let hall_down_queue_clone = hall_down_queue.clone();
    let elevator_clone = elevator.clone();
    let current_floor_clone = current_floor.clone();
    let obstruction_state_clone = obstruction_state.clone();
    let stop_button_state_clone = stop_button_state.clone();

    spawn(async move {
        let mut motor_state = State::Idle;

        // Start with the data backed up state if it exists
        // We need to do some cursed way of ownership dereferencing
        // We move data to its own data variable independent of the lock
        // Its cursed, but thats what you get when dealing with custom data types that don't support this out the get go X-X
        let start_state = {
            let start_state = state_clone.read().await;
            (*start_state).clone() // Dereference and clone the value to make it independent
        };
        let mut _state = start_state.to_owned();
        let mut _prev_state = State::Idle;

        let start_direction = {
            let start_direction = direction_clone.read().await;
            (*start_direction).clone() // Dereference and clone the value to make it independent
        };
        let mut _direction = start_direction.to_owned();
        let mut _prev_direction = Direction::Stop;

        // Ensure the initial previous floor matches the current floor.
        // This prevents an edge case where, if the elevator crashes while between floors,
        // it forgets its last known state. Without this, the elevator could incorrectly
        // assume it should open its cab door upon restart, even if it is still moving.
        //
        // This issue only occurs if the same cab request is made again after a crash and
        // the elevator starts moving before completing its previous request.
        //
        // To prevent this, we set `visited_floor` to `current_floor`, ensuring that
        // any pending request to the last known floor is completed before shutting down.
        let mut visited_floor;
        {
            let current_floor = current_floor_clone.read().await;
            visited_floor = current_floor.unwrap_or(0); // Default to 0 if None
        }

        // Start elevator state machine at 1st hall
        // This forces elevator to go down on startup
        // Both for convenience and for safety
        {
            // Acquire a lock on the cab_calls_clone Mutex
            let mut cab_queue = cab_queue_clone.write().await;

            // Insert 0 to start elevator at 0th hall
            cab_queue.insert(0);
        }

        loop {
            // Save latest state from state machine so that backing up thread can handle backing up state
            // Lets be hones here... this function is cursed X-X
            // But cloning the lock is insanely slow, and stunts the whole thread
            // Its because we use custom enum state that does not natively support copying, meaning we have to clone
            // Cloning data is slow
            // This is the best I could think of for bypassing cloning :/
            // The worst part is that this actually works, it solves the issue and the code stays fast *O*
            //
            // "Do you think God stays in heaven because he, too, lives in fear of what he's created here on earth?" - Robert Rodriguez, writer of Spy Kids 2
            {
                if _prev_state != _state {
                    let mut state_backup = state_clone.write().await;

                    match _state {
                        State::Idle => {
                            *state_backup = State::Idle;
                            _prev_state = State::Idle;
                        }
                        State::Up => {
                            *state_backup = State::Up;
                            _prev_state = State::Up;
                        }
                        State::Down => {
                            *state_backup = State::Down;
                            _prev_state = State::Down;
                        }
                        State::Door => {
                            *state_backup = State::Door;
                            _prev_state = State::Door;
                        }
                        State::EmergencyStop => {
                            *state_backup = State::EmergencyStop;
                            _prev_state = State::EmergencyStop;
                        }
                        State::EmergencyStopIdle => {
                            *state_backup = State::EmergencyStopIdle;
                            _prev_state = State::EmergencyStopIdle;
                        }
                    }
                }
            } // Lock is released here

            // Save direction into shared variable
            // This way if its updated elevator request thread will send a request to the manager with updated direction state
            {
                if _prev_direction != _direction {
                    _prev_direction = _direction;

                    let mut direction_shared = direction_clone.write().await;
                    *direction_shared = match _direction {
                        Direction::Up => Direction::Up,
                        Direction::Stop => Direction::Stop,
                        Direction::Down => Direction::Down,
                    }
                }
            } // Lock is released here

            // Get stop button state
            let stop_button = {
                let stop_button = stop_button_state_clone.read().await;
                *stop_button
            }; // Lock is released here

            // Get current hall we are on
            let current_floor = current_floor_clone.read().await.unwrap_or_else(|| {
                println!();
                println!("#============================================================#");
                println!("ERROR: Current floor is not set! Exiting program.");
                println!("ERROR: Check that elevator IO is connected for floor sensor");
                println!("#============================================================#");
                println!();
                std::process::exit(1);
            }); // Lock is released here

            // Check if we have any cab calls under way
            // Clone the current state of cab_que into a separate variable
            // This way the lock is used up immediately and frees up the resource for other threads much faster
            let cab_queue = {
                let cab_queue = cab_queue_clone.read().await;
                cab_queue.clone() // Clone the HashSet into a separate variable
            }; // Lock is released here

            // Check if we have any halls calls under way
            // Clone the current state of hall calls into a separate variable so that other threads can use same resources faster
            let hall_up_queue = {
                let hall_up_queue = hall_up_queue_clone.read().await;
                hall_up_queue.clone() // Clone the HashSet into a separate variable
            }; // Lock is released here
            let hall_down_queue = {
                let hall_down_queue = hall_down_queue_clone.read().await;
                hall_down_queue.clone() // Clone the HashSet into a separate variable
            }; // Lock is released here

            match _state {
                State::Idle => {
                    let mut found_solution = None;

                    // Check for emergency stop first
                    found_solution = found_solution.or_else(|| state_machine::handle_emergency_stop(stop_button));

                    // Handle cab requests on current floor while we are still not moving
                    found_solution = found_solution.or_else(|| state_machine::handle_cab_request_current_floor(&cab_queue, current_floor));

                    // Handle direction-specific logic
                    match _direction {
                        Direction::Stop => {
                            // Handle hall calls if we have exhausted all requests
                            // Always try to find hall requests up first, only then down requests
                            found_solution = found_solution.or_else(|| state_machine::handle_hall_up_request_current_floor(&hall_up_queue, current_floor));
                            found_solution = found_solution.or_else(|| state_machine::handle_hall_down_request_current_floor(&hall_down_queue, current_floor));

                            // Handle random direction logic if all other options were exhausted
                            found_solution = found_solution.or_else(|| state_machine::find_random_request(&cab_queue, &hall_up_queue, &hall_down_queue, current_floor));
                        }
                        Direction::Up => {
                            // Before moving check that there are no new UP requests from the same floor
                            found_solution = found_solution.or_else(|| state_machine::handle_hall_up_request_current_floor(&hall_up_queue, current_floor));

                            // Look for requests above the current hall
                            found_solution = found_solution.or_else(|| state_machine::find_request_above(&cab_queue, current_floor));
                            found_solution = found_solution.or_else(|| state_machine::find_request_above(&hall_up_queue, current_floor));

                            // Look for any requests above that are DOWN if no more up queues that way
                            found_solution = found_solution.or_else(|| state_machine::find_request_above(&hall_down_queue, current_floor));

                            // If no further upwards requests found
                            // Search for special case requests that go opposite to the normal direction (ie DOWN)
                            found_solution = found_solution.or_else(|| state_machine::find_request_below(&cab_queue, current_floor));
                            found_solution = found_solution.or_else(|| state_machine::find_request_below(&hall_down_queue, current_floor));
                        }
                        Direction::Down => {
                            // Before moving check that there are no new DOWN requests from the same floor
                            found_solution = found_solution.or_else(|| state_machine::handle_hall_down_request_current_floor(&hall_down_queue, current_floor));

                            // Look for requests below the current hall
                            found_solution = found_solution.or_else(|| state_machine::find_request_below(&cab_queue, current_floor));
                            found_solution = found_solution.or_else(|| state_machine::find_request_below(&hall_down_queue, current_floor));

                            // Look for any requests bellow that are UP if no more down queues that way
                            found_solution = found_solution.or_else(|| state_machine::find_request_below(&hall_up_queue, current_floor));

                            // If no further downwards requests found
                            // Search for special case requests that go opposite to the normal direction (ie UP)
                            found_solution = found_solution.or_else(|| state_machine::find_request_above(&cab_queue, current_floor));
                            found_solution = found_solution.or_else(|| state_machine::find_request_above(&hall_up_queue, current_floor));
                        }
                    }

                    // Update state if a solution was found
                    if let Some(new_state) = found_solution {
                        _state = new_state;

                        // Special case exceptions to the rule
                        // If no new requests in the previous direction
                        // However there is a request from the opposite direction
                        // We must handle that request this turn
                        // So that means we must also clear the special case request light as well
                        if _direction == Direction::Down && _state == State::Up {
                            // Clear the Floor UP signal
                            {
                                let mut hall_up_queue = hall_up_queue_clone.write().await;
                                hall_up_queue.remove(&current_floor);

                                let request = create_hall_request_json(
                                    None,
                                    None,
                                    Some(current_floor), // UP Remove
                                    None,
                                );

                                hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button UP");
                            }
                        } else if _direction == Direction::Up && _state == State::Down {
                            // Clear the Floor DOWN signal
                            {
                                let mut hall_down_queue = hall_down_queue_clone.write().await;
                                hall_down_queue.remove(&current_floor);

                                let request = create_hall_request_json(
                                    None,
                                    None,
                                    None,
                                    Some(current_floor), // DOWN Remove
                                );

                                hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button UP");
                            }
                        }
                    } else {
                        // No solution was found
                        // Reset direction
                        _direction = Direction::Stop;

                        // Set motor to IDLE
                        if motor_state != State::Idle {
                            elevator_clone.motor_direction(0).await;

                            motor_state = State::Idle;
                        }
                    }
                }
                State::Up => {
                    // Set motor UP
                    if motor_state != State::Up {
                        elevator_clone.motor_direction(1).await;

                        motor_state = State::Up;
                    }

                    _direction = Direction::Up;

                    // If we hit a different floor go into Idle state
                    // These it will handle the rest of the logic
                    if visited_floor != current_floor {
                        _state = State::Idle;
                    }

                    // Check for emergency stop button
                    // Check for it last to ensure it overwrites any other state set in case of emergency
                    if stop_button {
                        _state = State::EmergencyStop;
                    }

                    // Update visited floor to the latest floor we are at
                    // No matter if we found a solution or not
                    // This is the floor that we have now visited
                    // no more queues will be handled for this visited floor at this stage
                    // Even if they come in to late, new queues for this floor will have to wait
                    visited_floor = current_floor;

                    //println!("DEBUG: Current Floor {:#?}", current_floor);
                }
                State::Down => {
                    // Set motor DOWN
                    if motor_state != State::Down {
                        elevator_clone.motor_direction(255).await;

                        motor_state = State::Down;
                    }

                    _direction = Direction::Down;

                    // If we hit a different floor go into Idle state
                    // These it will handle the rest of the logic
                    if visited_floor != current_floor {
                        _state = State::Idle;
                    }

                    // Check for emergency stop button
                    // Check for it last to ensure it overwrites any other state set in case of emergency
                    if stop_button {
                        _state = State::EmergencyStop;
                    }

                    // Update visited floor to the latest floor we are at
                    // No matter if we found a solution or not
                    // This is the floor that we have now visited
                    // no more queues will be handled for this visited floor at this stage
                    // Even if they come in to late, new queues for this floor will have to wait
                    visited_floor = current_floor;

                    //println!("DEBUG: Current Floor {:#?}", current_floor);
                }
                State::Door => {
                    // Set motor IDLE
                    if motor_state != State::Idle {
                        elevator_clone.motor_direction(0).await;

                        motor_state = State::Idle;
                    }

                    // remove all request on this specific hall we stopped at
                    // Exception: Don't remove the requests at the opposite site we were going at, as per specification :)
                    // NOTE: We also send the remove request to the network so that everyone on the network knows that we have handled the request and should remove it from the requests
                    {
                        let mut cab_queue = cab_queue_clone.write().await;
                        cab_queue.remove(&current_floor);
                    }
                    {
                        match _direction {
                            Direction::Down => {
                                if current_floor == 0 {
                                    {
                                        let mut hall_up_queue = hall_up_queue_clone.write().await;
                                        hall_up_queue.remove(&current_floor);
                                    } // UP Remove
                                    {
                                        let mut hall_down_queue = hall_down_queue_clone.write().await;
                                        hall_down_queue.remove(&current_floor);
                                    } // DOWN Remove

                                    let request = create_hall_request_json(
                                        None,
                                        None,
                                        Some(current_floor), // UP Remove
                                        Some(current_floor), // DOWN Remove
                                    );

                                    hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button UP");
                                } else {
                                    {
                                        let mut hall_down_queue = hall_down_queue_clone.write().await;
                                        hall_down_queue.remove(&current_floor);
                                    } // DOWN Remove

                                    let request = create_hall_request_json(
                                        None,
                                        None,
                                        None,
                                        Some(current_floor), // DOWN Remove
                                    );

                                    hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button UP");
                                }
                            }
                            Direction::Up => {
                                if current_floor == (*NUMBER_FLOORS - 1) {
                                    {
                                        let mut hall_up_queue = hall_up_queue_clone.write().await;
                                        hall_up_queue.remove(&current_floor);
                                    } // UP Remove
                                    {
                                        let mut hall_down_queue = hall_down_queue_clone.write().await;
                                        hall_down_queue.remove(&current_floor);
                                    } // DOWN Remove

                                    let request = create_hall_request_json(
                                        None,
                                        None,
                                        Some(current_floor), // UP Remove
                                        Some(current_floor), // DOWN Remove
                                    );

                                    hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button UP");
                                } else {
                                    {
                                        let mut hall_up_queue = hall_up_queue_clone.write().await;
                                        hall_up_queue.remove(&current_floor);
                                    } // UP Remove

                                    let request = create_hall_request_json(
                                        None,
                                        None,
                                        Some(current_floor), // UP Remove
                                        None,
                                    );

                                    hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button UP");
                                }
                            }
                            _ => {
                                {
                                    let mut hall_up_queue = hall_up_queue_clone.write().await;
                                    hall_up_queue.remove(&current_floor);
                                } // UP Remove
                                {
                                    let mut hall_down_queue = hall_down_queue_clone.write().await;
                                    hall_down_queue.remove(&current_floor);
                                } // DOWN Remove

                                let request = create_hall_request_json(
                                    None,
                                    None,
                                    Some(current_floor), // UP Remove
                                    Some(current_floor), // DOWN Remove
                                );

                                hall_requests_publisher.put(request.as_bytes()).await.expect("Failed to publish hall button UP");
                            }
                        }
                    }

                    // Open the door
                    elevator_clone.door_light(true).await;

                    // Timeout to wait for people to get out/in
                    tokio::time::sleep(Duration::from_millis(3000)).await;

                    // Check obstructions
                    while *obstruction_state_clone.read().await {
                        // Do nothing while we are blocked
                    }

                    // Close the door
                    elevator_clone.door_light(false).await;

                    _state = State::Idle;
                }
                State::EmergencyStop => {
                    // If user has pressed stop button we think of it as emergency
                    // Stop the elevator immediately
                    // Clear all cab calls
                    // Clear all hall calls
                    // Go into stop idle state
                    if motor_state != State::Idle {
                        elevator_clone.motor_direction(0).await;

                        motor_state = State::Idle;
                    }

                    {
                        let mut cab_queue = cab_queue_clone.write().await;
                        cab_queue.clear();
                    }

                    // NOTE: We don't signal to the rest of the network that we have removed our requests (ie publish remove requests)
                    // The network itself will figure out something went wrong with the elevator since we don't handle our requests no longer
                    // We will also STOP publishing heartbeat, prompting manager response to our unhandled requests
                    // This in turn will prompt the manager node to reallocate requests where it needs to be after noticing this elevator is in emergency state
                    {
                        let mut hall_up_queue = hall_up_queue_clone.write().await;
                        hall_up_queue.clear();
                    }

                    {
                        let mut hall_down_queue = hall_down_queue_clone.write().await;
                        hall_down_queue.clear();
                    }

                    _state = State::EmergencyStopIdle;
                }
                State::EmergencyStopIdle => {
                    // Check if the previous direction was nothing, indicating we were idling
                    // If so just go back to idling
                    if _direction == Direction::Stop {
                        _state = State::Idle;
                    }

                    // If it wasn't idle state we were in before, that means we are in between floors
                    // We have to manage this a bit more carefully
                    // Check if something new has happened in the CAB
                    // NOTE: We ignore the outside hall requests and the world as we are in an emergency
                    let something_new = !cab_queue.is_empty();

                    // We wait in stop state until something new happens
                    if something_new == true {
                        // Something new happened
                        // We need to check witch state we should go to

                        // Sometimes elevator gets stopped between floors
                        // check what the previous direction of movement was
                        // If there is a cab_queue, we check if the next floor is out previous floor
                        // If so we need to go to that floor before going to Idle
                        // Otherwise its a different floor so Idle state can handle logic of it for us
                        // NOTE: Again, we only care about what is happening inside the cab because we are in emergency
                        // This means we don't care about the outside world

                        // Check if the previous floor we departed before we stopped is in request queue
                        let request_to_same_floor = cab_queue.contains(&current_floor);

                        // If the request of the same previous floor is not there we are good
                        // We can go to Idle state that will take case of things for us
                        if !request_to_same_floor {
                            // We should go down to the
                            _state = State::Idle;
                        } else {
                            // Since the floor we want to go to now is the same as before we do nothing
                            // This is because we tried to get it to work to go back to the floor
                            // However for this we need to set elevator state HARDWARE wise to different floor
                            // This way it thinks its on different floor and we can go to our floor
                            // However microcontroller/Arduino saves the last state it had, and you can't edit it
                            // Even if you try sending reloading the config, it will still fail
                            // The only way to say to hardware that they need to clear their state is to turn the power off
                            // And because of this it is not realizable to go back to the same floor T_T
                            // So instead, if the button of that choice is clicked, we just delete it from the queue

                            {
                                let mut cab_queue = cab_queue_clone.write().await;
                                cab_queue.remove(&current_floor);
                            }
                        }
                    } else {
                        // Stay stuck in stop state
                    }
                }
            }
        }
    });
    // STATE MACHINE (STOP) ==================================================================================================

    loop {
        yield_now().await;
    }
}
