use std::io::Write;
use std::net::IpAddr;
use std::u8;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::time::sleep;
use crate::{config, print, network::local_network, world_view::world_view::{self}, manager::task_allocator::Task};

use local_ip_address::local_ip;

use std::sync::atomic::{AtomicU8, Ordering};

/// Atomic bool storing self ID, standard inited as config::ERROR_ID
pub static SELF_ID: AtomicU8 = AtomicU8::new(config::ERROR_ID); // Startverdi 255






/// Returns the local IPv4 address of the machine as `IpAddr`.
///
/// If no local IPv4 address is found, returns `local_ip_address::Error`.
///
/// # Example
/// ```
/// use elevatorpro::utils::get_self_ip;
///
/// match get_self_ip() {
///     Ok(ip) => println!("Local IP: {}", ip), // IP retrieval successful
///     Err(e) => println!("Failed to get IP: {:?}", e), // No local IP available
/// }
/// ```
pub fn get_self_ip() -> Result<IpAddr, local_ip_address::Error> {
    let ip = match local_ip() {
        Ok(ip) => {
            ip
        }
        Err(e) => {
            print::warn(format!("Fant ikke IP i get_self_ip() -> Vi er offline: {}", e));
            return Err(e);
        }
    };
    Ok(ip)
}


/// Extracts your ID based on `ip`
/// 
/// ## Example
/// ```
/// let id = id_fra_ip("a.b.c.d:e");
/// ```
/// returnerer d
/// 
pub fn ip2id(ip: IpAddr) -> u8 {
    let ip_str = ip.to_string();
    let mut ip_int = config::ERROR_ID;
    let id_str = ip_str.split('.')           // Del på punktum
        .nth(3)              // Hent den 4. delen (d)
        .and_then(|s| s.split(':')  // Del på kolon hvis det er en port etter IP-en
            .next())         // Ta kun første delen før kolon
        .and_then(|s| s.parse::<u8>().ok());  // Forsøk å parse til u8

    match id_str {
        Some(value) => {
            ip_int = value;
        }
        None => {
            println!("Ingen gyldig ID funnet. (konsulent.rs, id_fra_ip())");
        }
    }
    ip_int
}

/// Extracts the root part of an IP address (removes the last segment).
///
/// ## Example
/// ```
/// use std::net::IpAddr;
/// use std::str::FromStr;
/// use elevatorpro::utils::get_root_ip;
///
/// let ip = IpAddr::from_str("192.168.1.42").unwrap();
/// let root_ip = get_root_ip(ip);
/// assert_eq!(root_ip, "192.168.1");
/// ```
///
/// Returns a string containing the first three segments of the IP address.
pub fn get_root_ip(ip: IpAddr) -> String {

    match ip {
        IpAddr::V4(addr) => {
            let octets = addr.octets();
            format!("{}.{}.{}", octets[0], octets[1], octets[2])
        }
        IpAddr::V6(addr) => {
            let segments = addr.segments();
            let root_segments = &segments[..segments.len() - 1]; // Fjern siste segment
            root_segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(":")
        }
    }
}





/// Fetches a clone of the latest local worldview (wv) from the system.
///
/// This function retrieves the most recent worldview stored in the provided `LocalChannels` object.
/// It returns a cloned vector of bytes representing the current serialized worldview.
///
/// # Parameters
/// - `chs`: The `LocalChannels` object, which contains the latest worldview data in `wv`.
///
/// # Return Value
/// Returns a vector of `u8` containing the cloned serialized worldview.
///
/// # Example
/// ```
/// use elevatorpro::utils::get_wv;
/// use elevatorpro::network::local_network::LocalChannels;
/// 
/// let local_chs = LocalChannels::new();
/// let _ = local_chs.watches.txs.wv.send(vec![1, 2, 3, 4]);
/// 
/// let fetched_wv = get_wv(local_chs.clone());
/// assert_eq!(fetched_wv, vec![1, 2, 3, 4]);
/// ```
///
/// **Note:** This function clones the current state of `wv`, so any future changes to `wv` will not affect the returned vector.
pub fn get_wv(chs: local_network::LocalChannels) -> Vec<u8> {
    chs.watches.rxs.wv.borrow().clone()
}

/// Asynchronously updates the worldview (wv) in the system.
///
/// This function reads the latest worldview data from a specific channel and updates
/// the given `wv` vector with the new data if it has changed. The function operates asynchronously,
/// allowing it to run concurrently with other tasks without blocking.
///
/// ## Parameters
/// - `chs`: The `LocalChannels` object, which holds the channels used for receiving worldview data.
/// - `wv`: A mutable reference to the `Vec<u8>` that will be updated with the latest worldview data.
///
/// ## Returns
/// - `true` if wv was updated, `false` otherwise.
///
/// ## Example
/// ```
/// # use tokio::runtime::Runtime;
/// use elevatorpro::utils::update_wv;
/// use elevatorpro::network::local_network::LocalChannels;
/// 
/// let chs = LocalChannels::new();
/// let mut wv = vec![1, 2, 3, 4];
/// 
/// # let rt = Runtime::new().unwrap();
/// # rt.block_on(async {/// 
/// chs.watches.txs.wv.send(vec![4, 3, 2, 1]);
/// let result = update_wv(chs.clone(), &mut wv).await;
/// assert_eq!(result, true);
/// assert_eq!(wv, vec![4, 3, 2, 1]);
/// 
/// let result = update_wv(chs.clone(), &mut wv).await;
/// assert_eq!(result, false);
/// assert_eq!(wv, vec![4, 3, 2, 1]);
/// # });
/// ```
///
/// ## Notes
/// - This function is asynchronous and requires an async runtime, such as Tokio, to execute.
/// - The `LocalChannels` channels allow for thread-safe communication across threads.
pub async fn update_wv(chs: local_network::LocalChannels, wv: &mut Vec<u8>) -> bool {
    let new_wv = chs.watches.rxs.wv.borrow().clone();  // Clone the latest data
    if new_wv != *wv {  // Check if the data has changed compared to the current state
        *wv = new_wv;  // Update the worldview if it has changed
        return true;
    }
    false
}


/// Checks if the current system is the master based on the latest worldview data.
///
/// This function compares the system's `SELF_ID` with the value at `MASTER_IDX` in the provided worldview (`wv`).
///
/// ## Returns
/// - `true` if the current system's `SELF_ID` matches the value at `MASTER_IDX` in the worldview.
/// - `false` otherwise.
pub fn is_master(wv: Vec<u8>) -> bool {
    return SELF_ID.load(Ordering::SeqCst) == wv[config::MASTER_IDX];
}

/// Retrieves the latest elevator tasks from the system.
///
/// This function borrows the value from the `elev_task` channel and clones it, returning a copy of the tasks.
/// It is used to fetch the current tasks for the local elevator.
///
/// ## Parameters
/// - `chs`: A `LocalChannels` struct that contains the communication channels for the system.
///
/// ## Returns
/// - A `Vec<Task>` containing the current elevator tasks.
pub fn get_elev_tasks(chs: local_network::LocalChannels) -> Vec<Task> {
    chs.watches.rxs.elev_task.borrow().clone()
}

/// Retrieves a clone of the `ElevatorContainer` with the specified `id` from the provided worldview.
///
/// This function deserializes the provided worldview (`wv`), filters the elevator containers based on the given `id`,
/// and returns a clone of the matching `ElevatorContainer`. If no matching elevator is found, the behavior is undefined.
///
/// ## Parameters
/// - `wv`: The latest worldview in serialized state.
/// - `id`: The `id` of the elevator container to extract.
///
/// ## Returns
/// - A clone of the `ElevatorContainer` with the specified `id`, or the first match found.
///
/// **Note:** If no elevator container with the specified `id` is found, this function will panic due to indexing.
pub fn extract_elevator_container(wv: Vec<u8>, id: u8) -> world_view::ElevatorContainer {
    let mut deser_wv = world_view::deserialize_worldview(&wv);

    deser_wv.elevator_containers.retain(|elevator| elevator.elevator_id == id);
    deser_wv.elevator_containers[0].clone()
}

/// Retrieves a clone of the `ElevatorContainer` with `SELF_ID` from the latest worldview.
///
/// This function calls `extract_elevator_container` with `SELF_ID` to fetch the elevator container that matches the
/// current `SELF_ID` from the provided worldview (`wv`). The `SELF_ID` is a static identifier loaded from memory,
/// which represents the current elevator's unique identifier.
///
/// ## Parameters
/// - `wv`: The latest worldview in serialized state.
///
/// ## Returns
/// - A clone of the `ElevatorContainer` associated with `SELF_ID`.
///
/// **Note:** This function internally calls `extract_elevator_container` to retrieve the correct elevator container.
pub fn extract_self_elevator_container(wv: Vec<u8>) -> world_view::ElevatorContainer {
    extract_elevator_container(wv, SELF_ID.load(Ordering::SeqCst))
}


/// Closes the provided TCP stream asynchronously, logging the result.
///
/// This function attempts to close the provided TCP stream by invoking the `shutdown` method on the stream asynchronously.
/// It also retrieves the local and peer addresses of the stream, printing them in the log messages. If the stream is
/// closed successfully, a info message is printed. If an error occurs during the process, an error message is logged.
///
/// ## Parameters
/// - `stream`: The TCP stream to close (mutable reference to `TcpStream`).
///
/// ## Logs
/// - On success: Logs an info message such as "TCP connection closed successfully: <local_addr> -> <peer_addr>".
/// - On error: Logs an error message such as "Failed to close TCP connection (<local_addr> -> <peer_addr>): <error>".
pub async fn close_tcp_stream(stream: &mut TcpStream) {
    // Hent IP-adresser
    let local_addr = stream.local_addr().map_or_else(
        |e| format!("Ukjent (Feil: {})", e),
        |addr| addr.to_string(),
    );

    let peer_addr = stream.peer_addr().map_or_else(
        |e| format!("Ukjent (Feil: {})", e),
        |addr| addr.to_string(),
    );

    // Prøv å stenge streamen (Asynkront)
    match stream.shutdown().await {
        Ok(_) => print::info(format!(
            "TCP-forbindelsen er avslutta korrekt: {} -> {}",
            local_addr, peer_addr
        )),
        Err(e) => print::err(format!(
            "Feil ved avslutting av TCP-forbindelsen ({} -> {}): {}",
            local_addr, peer_addr, e
        )),
    }
}

/// Sleeps for duration specified in config::SLAVE_TIMEOUT
pub async fn slave_sleep() {
    let _ = sleep(config::SLAVE_TIMEOUT);
}
