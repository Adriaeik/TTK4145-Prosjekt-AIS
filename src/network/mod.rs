//! ## Network module
//! 
//! This module is responsible for the network in the system, and is in some sense the most sentral part in the whole project.
//! The module has a lot of responsibilities, and is therefore splittet into a few sub-modules.
//! 
//! ## Sub-modules
//! - [udp_broadcast]
//! - [local_network]
//! 
//! ## Key Features
//! - Using UDP broadcast to publish WorldView on the network, and detecting a network when starting up.
//! - Using TCP to share elevator-spesific data from slave-nodes to master-nodes.
//! - Using a set of thread-safe channels to let different parts of the program to share information.
//! - Monitoring the network, automatically detecting connection loss and unoperatable levels of packetloss
//! 
//! ## Functions
//! - `watch_ethernet`: Updates the network status, making sure the program detects connection loss and high packet loss
//! - `read_network_status`: Gives a boolean indicating if your network connection is operatable.

pub mod udp_broadcast;
pub mod local_network;
pub mod udp_direct;


use crate::world_view::WorldView;
use crate::{init, config, print, ip_help_functions, world_view, };

use std::collections::VecDeque;
use serde::{Serialize, Deserialize};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration, Instant};
use tokio::sync::{mpsc, watch};
use std::sync::atomic::{Ordering, AtomicU8, AtomicBool};
use std::sync::OnceLock;
use std::thread::sleep;
use local_ip_address::local_ip;
use std::net::IpAddr;





static ONLINE: OnceLock<AtomicBool> = OnceLock::new(); 

/// Atomic bool storing self ID, standard inited as config::ERROR_ID
pub static SELF_ID: OnceLock<AtomicU8> = OnceLock::new();

/// Struct for wrapping network connection information
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionStatus 
{
    /// true if we have network on the subnet, regardless of the packet loss
    pub on_internett: bool,

    /// true if we decide to be connected to the elevator network
    pub connected_on_elevator_network: bool,

    /// percentage of packet loss (0 - 100)%
    pub packet_loss: u8,
}

impl ConnectionStatus 
{
    /// Creates a new instance of `Self`
    pub fn new() -> Self 
    {
        Self 
        {
            on_internett: false,
            connected_on_elevator_network: false,
            packet_loss: 0,
        }
    }

    /// Sets packetloss field based on packetloss as a float between 0 and 1
    fn set_packet_loss(&mut self, loss: f32) 
    {
        self.packet_loss = (loss * 100.0) as u8;
    }
}


/* _______________ START PUB FUNCTIONS _______________ */

/// Monitors the Ethernet connection status and updates the system state accordingly.
/// 
/// This function checks the current Ethernet status, including network connectivity and packet loss, and sends updates
/// to other parts of the system. It also handles transitions between offline and online states, re-initializing the world view
/// when the system comes back online.
/// 
/// # Arguments
/// - `wv_watch_rx` - Receiver for world view updates.
/// - `network_watch_tx` - Sender for broadcasting connection status updates.
/// - `new_wv_after_offline_tx` - Sender for broadcasting the new world view after coming back online.
/// 
/// # Behaviour
/// - Monitors network quality using packet loss information.
/// - Checks the device's IP and updates the connection status based on whether the device is online and connected to the elevator network.
/// - If the device goes from offline to online, it re-initializes the world view and sends the updated state to the system.
/// 
/// # Notes
/// - The function loops indefinitely, periodically checking the network status and updating the connection status, and should therefore be called asyncronously
pub async fn watch_ethernet(
    wv_watch_rx: watch::Receiver<WorldView>, 
    network_watch_tx: watch::Sender<ConnectionStatus>, 
    new_wv_after_offline_tx: mpsc::Sender<WorldView>
) 
{
    let mut last_net_status = false;
    // TODO:: legge på hystesrese
    let network_quality_rx = start_packet_loss_monitor(
        1, 
        5, 
        1000 as usize, 
        1.0
    ).await;

    loop 
    {
        let ip = get_self_ip();
        let mut connection_status = ConnectionStatus::new();
        let net_status: bool;
        match ip 
        {
            Ok(ip) if ip_help_functions::get_root_ip(ip) == config::NETWORK_PREFIX => 
            {
                let (is_ok, loss)  = network_quality_rx.borrow().clone();
                net_status = is_ok;
                
                connection_status.on_internett = true;
                connection_status.connected_on_elevator_network = is_ok;
                connection_status.set_packet_loss(loss);

                let _ = network_watch_tx.send(connection_status.clone());
            }
            _ => 
            {
                // IP does not exist or does not match the subnet prefix
                connection_status.on_internett = false;
                connection_status.connected_on_elevator_network = false;
                connection_status.packet_loss = 100;
                net_status = false;

                let _ = network_watch_tx.send(connection_status.clone());
            }
        }
        if last_net_status != net_status 
        {
            if net_status 
            {
                // Went from offline -> online
                let mut wv = world_view::get_wv(wv_watch_rx.clone());
                let self_elev = world_view::extract_self_elevator_container(&wv);
                wv = init::initialize_worldview(self_elev).await;
                let _ = new_wv_after_offline_tx.send(wv).await;

                print::ok("System is online".to_string());
            } else 
            {
                print::warn("System is offline".to_string());
            }
            set_network_status(net_status);
            last_net_status = net_status;
        }

        sleep(config::POLL_PERIOD);
    }
}

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
pub fn read_network_status() -> bool 
{
    ONLINE.get_or_init(|| AtomicBool::new(false)).load(Ordering::SeqCst)
}

/// Reads and returns a clone of the current sself ID
///
/// This function returns a copy of the self ID.
///
/// # Returns
/// u8: Your ID on the network
/// 
/// # Note
/// - The value is [config::ERROR_ID] if [watch_ethernet] is not running.
pub fn read_self_id() -> u8 
{
    SELF_ID.get_or_init(|| AtomicU8::new(config::ERROR_ID)).load(Ordering::SeqCst)
}

/// This function sets your self ID
/// 
/// # Note
/// This function should not be used, as network ID is assigned automatically under initialisation
pub fn set_self_id(id: u8) 
{
    SELF_ID.get_or_init(|| AtomicU8::new(config::ERROR_ID)).store(id, Ordering::SeqCst);
}

/* _______________ END PUB FUNCTIONS _______________ */









/* _______________ START PRIVATE FUNCTIONS _______________ */

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
fn get_self_ip() -> Result<IpAddr, local_ip_address::Error> {
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

async fn wait_for_ip() -> IpAddr {
    loop {
        if let Ok(ip) = get_self_ip() {
            return ip;
        } else {
            sleep(config::POLL_PERIOD);
        }
    }
}


/// Startar ein pingmålar som returnerer (status, pakketap)
/// Starts pinging self to estimate packetloss
///
/// # Arguments
/// - `interval_ms`: time between pings
/// - `timeout_ms`: ping-timeout
/// - `max_window`: Size of the moving average window
/// - `max_loss_rate`: Maximum acceptet packetloss 
///
/// # Return
/// - `watch::Receiver<(bool, f32)>`
///
/// # Notes
/// - The watch is updated with a tuple containing a bool indicating if we are above the `max_loss_rate`, and a f32 being estimated loss rate
/// - For the final version of this project, UDP is being used for direct communication between master and slave, and the `max_loss_rate` should be fine if set to 1.00.
/// - In the earlier TCP-version (available on the GitHub), `max_loss_rate` is reccomended to be below 0.60
async fn start_packet_loss_monitor(
    interval_ms: u64,
    timeout_ms: u64,
    max_window: usize,
    max_loss_rate: f32,
) -> watch::Receiver<(bool, f32)> {
    use tokio::sync::watch;
    use socket2::{Socket, Domain, Type};
    let (tx, rx) = watch::channel((true, 0.0)); // start som OK
    let addr = format!("{}:{}", wait_for_ip().await, config::BROADCAST_PORT);

    
    tokio::spawn(async move {
        let mut last_loss: f32 = 0.0;
        let mut last_status: bool = false;
        let mut last_instant = Instant::now();
        let mut window: VecDeque<bool> = VecDeque::from(vec![true; max_window]);

        loop {
            // Create a socket and ping itself
            let success = {
                let socket_addr: std::net::SocketAddr = match addr.parse() {
                    Ok(addr) => addr,
                    Err(_) => {
                        break false;
                    }
                };
            
                let socket_temp = match Socket::new(Domain::IPV4, Type::DGRAM, None) {
                    Ok(s) => s,
                    Err(_) => {
                        break false;
                    }
                };

                if socket_temp.set_nonblocking(true).is_err() {break false}
                
                if socket_temp.set_reuse_address(true).is_err() {break false}
                
                if socket_temp.set_broadcast(true).is_err() {break false}

                if socket_temp.bind(&socket_addr.into()).is_err() {break false}
                
                match UdpSocket::from_std(socket_temp.into()) {

                    Ok(socket) => {
                        let payload = b"ping";
                        if socket.send_to(payload, &addr).await.is_err() {
                            false
                        } else {
                            let mut buf = [0u8; 16];
                            timeout(Duration::from_millis(timeout_ms), socket.recv_from(&mut buf))
                            .await
                            .ok()
                            .map(|r| r.is_ok())
                            .unwrap_or(false)
                        }
                    }
                    Err(_) => {
                        false
                    },
                }
                
            };

            // Update the window
            window.push_back(success);
            if window.len() > max_window {
                window.pop_front();
            }
            
            // Calculate average loss in the window
            let fail_count = window.iter().filter(|&&ok| !ok).count();
            let raw_loss = fail_count as f32 / window.len() as f32;
            let loss_rate = 1.0 - (1.0 - raw_loss).sqrt();
            
            let new_status = loss_rate <= max_loss_rate;
            // Update the watch if parameters has changed
            if (last_status != new_status) || (loss_rate - last_loss).abs() > 0.01 {
                if Instant::now() - last_instant > Duration::from_secs(5) {
                    last_instant = Instant::now();
                    let _ = tx.send((new_status, loss_rate));
                    last_status = new_status;
                }else {
                    let _ = tx.send((new_status, loss_rate));
                    last_loss = loss_rate;
                }
            }

            // Pause between pings
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
    });

    rx
}

/// This function sets the network status
fn set_network_status(status: bool) {
    ONLINE.get_or_init(|| AtomicBool::new(false)).store(status, Ordering::SeqCst);
}


/* _______________ END PRIVATE FUNCTIONS _______________ */


