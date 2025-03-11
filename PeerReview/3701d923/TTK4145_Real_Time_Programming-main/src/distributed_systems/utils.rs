// Libraries for network status and diagnostics
// * NOTE: The `tokio_icmp_echo` library requires ICMP sockets, which are ONLY available for root users.
// *       Therefore, we must run this program as `sudo cargo run`.
// *       The library is used to ping the router for network connectivity status.
use futures::{future, StreamExt};
use std::collections::HashSet;
use std::net::IpAddr;
use std::process::Command;
use tokio_icmp_echo::Pinger;

// Libraries for Distributed Networks
use zenoh::sample::Sample;

// Libraries for asynchronous multithreaded activities
use tokio::time::{timeout, Duration};

// Library for formatting
use crate::elevator_logic::utils::{Direction, State};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::collections::HashMap;

// ====================================================================================================
// Parses the payload of a Zenoh `Sample` message into a `String`.
//
// - Converts the payload of a Zenoh `Sample` to a UTF-8 `String`.
// - If the payload is invalid UTF-8, it defaults to "Invalid UTF-8".
// - Ensures the final output is always a `String`.
//
// Arguments:
// - `message`: The Zenoh `Sample` containing the payload to parse.
//
// Returns:
// - A `String` representation of the payload.
//
// Example:
// let parsed_message = parse_message(sample);
// println!("{}", parsed_message); // Outputs the payload as a `String`.
// ====================================================================================================
pub fn parse_message_to_string(message: Sample) -> String {
    return message.payload().try_to_string().unwrap_or_else(|_| "Invalid UTF-8".into()).to_string();
    // Convert Cow<'_, str> to String
}

/// Parses a Zenoh message payload into a `HashSet<u8>`.
///
/// - Removes curly braces `{}` from the payload.
/// - Assumes the payload is a comma-separated list of integers (e.g., "1,2,3").
/// - Skips invalid entries.
///
/// Returns:
/// - `HashSet<u8>` containing the parsed integers.
pub fn parse_message_to_hashset_u8(message: Sample) -> HashSet<u8> {
    // Convert the payload to a string
    let payload = message.payload().try_to_string().unwrap_or_else(|_| "".into()); // Default to empty string on error

    // Remove the outer curly braces (if any)
    let payload = payload.trim().trim_start_matches('{').trim_end_matches('}');

    // Parse the cleaned payload into a HashSet<u8>
    let parsed_set: HashSet<u8> = payload
        .split(',') // Split the string by commas
        .filter_map(|s| {
            let trimmed = s.trim();
            match trimmed.parse::<u8>() {
                Ok(value) => Some(value), // Valid integer
                Err(_) => {
                    println!("Skipping invalid entry: {}", trimmed); // Debug invalid entries
                    None // Skip invalid entries
                }
            }
        })
        .collect(); // Collect valid values into a HashSet

    parsed_set
}

// A data structure to backup data and parse it with the helper function
#[derive(Debug, Serialize, Deserialize)]
pub struct ElevatorBackup {
    pub state: State,
    pub direction: Direction,
    pub current_floor: Option<u8>,
    pub cab_queue: HashSet<u8>,
    pub hall_up_queue: HashSet<u8>,
    pub hall_down_queue: HashSet<u8>,
}

pub fn parse_message_to_elevator_backup(message: Sample) -> Option<ElevatorBackup> {
    message
        .payload()
        .try_to_string()
        .ok()
        .and_then(|json_str| from_str::<ElevatorBackup>(&json_str).ok())
}

// A way to convert message we receive from manager node to a understandable format
#[derive(Debug, Serialize, Deserialize)]
pub struct ElevatorRequests {
    pub requests: HashMap<String, Vec<Vec<bool>>>, // Elevator ID -> 2D bool array
}

pub fn parse_message_to_elevator_requests(message: Sample) -> Option<ElevatorRequests> {
    message
        .payload()
        .try_to_string()
        .ok()
        .and_then(|json_str| from_str::<HashMap<String, Vec<Vec<bool>>>>(&json_str).ok())
        .map(|parsed| ElevatorRequests { requests: parsed })
}

// ====================================================================================================
// Awaits a future with a timeout.
//
// - Waits for a future to complete within a specified timeout duration.
// - If the future completes successfully within the timeout, returns `Some` with the result.
// - If the timeout is exceeded or the future errors out, returns `None`.
//
// Arguments:
// - `duration_ms`: The timeout duration in milliseconds.
// - `future`: The future to await, which must return `Result<T, zenoh::Error>`.
//
// Returns:
// - `Option<T>`: `Some` if the future completes successfully within the timeout, `None` otherwise.
//
// Example:
// let result = wait_with_timeout(5000, some_async_operation()).await;
// if let Some(value) = result {
//     println!("Operation succeeded: {:?}", value);
// } else {
//     println!("Operation timed out.");
// }
// ====================================================================================================
pub async fn wait_with_timeout<T>(duration_ms: u64, future: impl futures::Future<Output = Result<T, zenoh::Error>>) -> Option<T> {
    let duration = Duration::from_millis(duration_ms);
    return timeout(duration, future).await.ok().and_then(|res| res.ok());
}

// ====================================================================================================
// Retrieves the router's IP address.
//
// - Executes the `ip route` command to find the default gateway (router).
// - Extracts and parses the IP address from the command's output.
// - If the command fails or no default gateway is found, returns `None`.
//
// Returns:
// - `Option<IpAddr>`: The router's IP address if found, or `None` otherwise.
//
// Example:
// if let Some(router_ip) = get_router_ip().await {
//     println!("Router IP: {}", router_ip);
// } else {
//     println!("Failed to find the router IP.");
// }
// ====================================================================================================
pub async fn get_router_ip() -> Option<IpAddr> {
    // Run the `ip route` command and capture the output
    let output = Command::new("ip").arg("route").output().expect("Failed to execute ip route command");

    // Check if the command executed successfully
    if !output.status.success() {
        println!("get_router_ip(): Command Failed!");
        return None;
    }

    // Parse the command output to find the default gateway
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("default via") {
            if let Some(ip_str) = line.split_whitespace().nth(2) {
                return ip_str.parse().ok(); // Parse the IP address
            }
        }
    }

    println!("get_router_ip(): No default gateway found!");
    return None;
}

// ====================================================================================================
// Enum representing the network status of the node.
//
// Variants:
// - `Connected`: Indicates the node is connected to the network (at least one ping succeeded).
// - `Disconnected`: Indicates the node is disconnected (all pings failed or an error occurred).
//
// Debug trait is derived to allow easy debugging with `println!("{:?}", NetworkStatus::Connected)`.
// ====================================================================================================
#[derive(Debug)]
pub enum NetworkStatus {
    Connected,
    Disconnected,
}

// ====================================================================================================
// Checks the network connectivity to the router.
//
// - Sends ICMP echo requests (pings) to the router's IP address.
// - Tracks whether at least one ping succeeds to determine connectivity status.
// - Uses the `tokio_icmp_echo` library for non-blocking pings.
//
// Arguments:
// - `router_ip`: The IP address of the router to ping.
//
// Returns:
// - `NetworkStatus`: `Connected` if at least one ping succeeds, `Disconnected` otherwise.
//
// Example:
// let status = network_status(router_ip).await;
// match status {
//     NetworkStatus::Connected => println!("Network is connected."),
//     NetworkStatus::Disconnected => println!("Network is disconnected."),
// }
// ====================================================================================================
pub async fn network_status(router_ip: IpAddr) -> NetworkStatus {
    // Create a new pinger instance
    let pinger = match Pinger::new().await {
        Ok(p) => p,
        Err(_) => return NetworkStatus::Disconnected, // Assume disconnected if pinger setup fails
    };

    // Create a stream for sending ICMP packets to the router IP
    let stream = pinger.chain(router_ip).stream();

    // Set the number of ICMP echo requests to send (number of tries)
    let tries = 5;

    // Track whether at least one ping succeeds
    let mut is_connected = false;

    // Process up to `tries` number of ping responses
    stream
        .take(tries)
        .for_each(|mb_time| {
            match mb_time {
                Ok(Some(_)) => is_connected = true, // Mark as connected on successful ping
                Ok(None) => {}                      // Do nothing on timeout
                Err(_) => {}                        // Do nothing on error
            }
            future::ready(())
        })
        .await;

    // Return the appropriate network status
    if is_connected {
        return NetworkStatus::Connected;
    } else {
        return NetworkStatus::Disconnected;
    }
}
