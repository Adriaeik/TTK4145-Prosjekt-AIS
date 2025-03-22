pub mod tcp_network;
pub mod udp_network;
pub mod local_network;


use crate::{init, config, print, ip_help_functions, world_view, };

use tokio::sync::{mpsc, watch};
use std::sync::atomic::{Ordering, AtomicU8, AtomicBool};
use std::sync::OnceLock;
use local_ip_address::local_ip;
use std::net::IpAddr;



/// Returns the local IPv4 address of the machine as `IpAddr`.
///
/// If no local IPv4 address is found, returns `local_ip_address::Error`.
///
/// # Example
/// ```
/// use elevatorpro::network::local_network::get_self_ip;
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
            return Err(e);
        }
    };
    Ok(ip)
}

/// Monitors the Ethernet connection status asynchronously.
///
/// This function continuously checks whether the device has a valid network connection.
/// It determines connectivity by verifying that the device's IP matches the expected network prefix.
/// The network status is stored in a shared atomic boolean [get_network_status()].
///
/// ## Behavior
/// - Retrieves the device's IP address using `utils::get_self_ip()`.
/// - Extracts the root IP using `utils::get_root_ip()` and compares it to `config::NETWORK_PREFIX`.
/// - Updates the network status (`true` if connected, `false` if disconnected).
/// - Prints status changes:  
///   - `"Vi er online"` when connected.  
///   - `"Vi er offline"` when disconnected.
///
/// ## Note
/// This function runs in an infinite loop and should be spawned as an asynchronous task.
///
/// ## Example
/// ```
/// use tokio;
/// # #[tokio::test]
/// # async fn test_watch_ethernet() {
/// tokio::spawn(async {
///     watch_ethernet().await;
/// });
/// # }
/// ```
pub async fn watch_ethernet(wv_watch_rx: watch::Receiver<Vec<u8>>, new_wv_after_offline_tx: mpsc::Sender<Vec<u8>>) {
    let mut last_net_status = false;
    let mut net_status;
    loop {
        let ip = get_self_ip();

        match ip {
            Ok(ip) => {
                if ip_help_functions::get_root_ip(ip) == config::NETWORK_PREFIX {
                    net_status = true;
                }
                else {
                    net_status = false   
                }
            }
            Err(_) => {
                net_status = false
            }
        }

        if last_net_status != net_status {  
            if net_status {
                let mut wv = world_view::get_wv(wv_watch_rx.clone());
                let self_elev = world_view::extract_self_elevator_container(wv.clone());
                wv = init::initialize_worldview(self_elev).await;
                let _ = new_wv_after_offline_tx.send(wv).await;
                print::ok("System is online".to_string());
            }
            else {
                print::warn("System is offline".to_string());
            }
            set_network_status(net_status);
            last_net_status = net_status;
        }
    }
}

static ONLINE: OnceLock<AtomicBool> = OnceLock::new(); 

/// Reads and returns a clone of the current network status
///
/// This function returns a copy of the network status the moment it was read.
/// that represents whether the system is online or offline.
///
/// # Returns
/// A bool`:
/// - `true` if the system is online.
/// - `false` if the system is offline.
/// 
/// # Note
/// - The initial value is `false` until explicitly changed. 
/// - The returned value is only a clone of the atomic boolean's value at read-time. The function should be called every time you need to check the online-status
pub fn read_network_status() -> bool {
    ONLINE.get_or_init(|| AtomicBool::new(false)).load(Ordering::SeqCst)
}

/// This function sets the network status
fn set_network_status(status: bool) {
    ONLINE.get_or_init(|| AtomicBool::new(false)).store(status, Ordering::SeqCst);
}

/// Atomic bool storing self ID, standard inited as config::ERROR_ID
pub static SELF_ID: OnceLock<AtomicU8> = OnceLock::new();

/// Reads and returns a clone of the current sself ID
///
/// This function returns a copy of the self ID.
///
/// # Returns
/// u8: Your ID on the network
/// 
/// # Note
/// - The value is [config::ERROR_ID] if [watch_ethernet] is not running.
pub fn read_self_id() -> u8 {
    SELF_ID.get_or_init(|| AtomicU8::new(config::ERROR_ID)).load(Ordering::SeqCst)
}

/// This function sets your self ID
/// 
/// # Note
/// This function should not be used, as network ID is assigned automatically under initialisation
pub fn set_self_id(id: u8) {
    SELF_ID.get_or_init(|| AtomicU8::new(config::ERROR_ID)).store(id, Ordering::SeqCst);
}



