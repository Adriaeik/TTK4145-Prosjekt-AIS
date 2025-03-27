//! ## UDP Module
//! 
//! This module handles UDP communication in the network. It is responsible for broadcasting the worldview when acting as the master
//! and listening for worldview broadcasts when acting as a slave. The module ensures that received broadcasts originate from the expected network.
//! 
//! ## Overview
//! - **Master Node**: Broadcasts the current worldview on a UDP channel.
//! - **Slave Node**: Listens for worldview broadcasts from the network master and updates its state accordingly.
//! - **UDP Watchdog**: Detects timeouts when no valid broadcasts are received.
//! 
//! ## Key Features
//! - Uses a reusable UDP socket for broadcasting and listening.
//! - Ensures messages are from the correct network by checking a predefined key string.
//! - Implements a watchdog mechanism to detect loss of connection to the master.
//! 
//! ## Functions
//! 
//! - [`start_udp_broadcaster`]: Sends worldview data over UDP if this node is the master.
//! - [`start_udp_listener`]: Listens for worldview broadcasts from the master and updates state.
//! - Private helper functions: [`build_message`], [`parse_message`].
//! 
//! ## Usage
//! These functions should be called asynchronously in a Tokio runtime.

use crate::config;
use crate::network;
use crate::print;
use crate::world_view;
use crate::world_view::WorldView;

use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::thread::sleep;
use std::time::Duration;
use tokio::net::UdpSocket;
use socket2::{Domain, Socket, Type};
use tokio::sync::mpsc;
use tokio::sync::watch;


static UDP_TIMEOUT: OnceLock<AtomicBool> = OnceLock::new();


/* __________ START PUBLIC FUNCTIONS __________ */

// ### Starter og kjører udp-broadcaster
/// This function starts and runs the UDP-broadcaster
/// 
/// ## Parameters
/// `wv_watch_rx`: Rx on watch the worldview is being sent on in the system  
/// 
/// ## Behavior
/// - Sets up a reusable socket on the udp-broadcast address
/// - Continously reads the latest worldview, if self is master on the network, it broadcasts the worldview. 
/// 
/// ## Note
/// This function is permanently blocking, and should be called asynchronously
pub async fn start_udp_broadcaster(
    wv_watch_rx: watch::Receiver<WorldView>
) -> tokio::io::Result<()> {
    while !network::read_network_status() {
        
    }
    let mut prev_network_status = network::read_network_status();

    // Set up sockets
    let addr: &str = &format!("{}:{}", config::BC_ADDR, config::BROADCAST_PORT);
    let addr2: &str = &format!("{}:0", config::BC_LISTEN_ADDR);

    let broadcast_addr: SocketAddr = addr.parse().expect("Invalid address"); // UDP-broadcast address
    let socket_addr: SocketAddr = addr2.parse().expect("Invalid address");
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    
    socket.set_nonblocking(true)?;
    socket.set_reuse_address(true)?;
    socket.set_broadcast(true)?;
    socket.bind(&socket_addr.into())?;
    let udp_socket = UdpSocket::from_std(socket.into())?;

    let mut wv = world_view::get_wv(wv_watch_rx.clone());
    loop{
        let wv_watch_rx_clone = wv_watch_rx.clone();
        world_view::update_wv(wv_watch_rx_clone, &mut wv).await;
        // If you currently are master on the network
        if network::read_self_id() == wv.master_id {
            sleep(config::UDP_PERIOD);
            let message_bytes = build_message(&wv);

            // If you are connected to internet
            if network::read_network_status() {
                // If you also were connected to internet last time you ran this
                if !prev_network_status {
                    sleep(Duration::from_millis(500));
                    prev_network_status = true;
                }
                // Send your worldview on UDP broadcast
                match udp_socket.send_to(&message_bytes, &broadcast_addr).await {
                    Ok(_) => {
                        // print::ok(format!("Sent udp broadcast!"));
                    },
                    Err(_) => {
                        // print::err(format!("Error while sending UDP: {}", e));
                    }
                }

            }else {
                prev_network_status = false;
            }
        }
    }
}

/// Starts and runs the UDP-listener
/// 
/// ## Parameters
/// `wv_watch_rx`: Rx on watch the worldview is being sent on in the system  
/// `udp_wv_tx`: mpsc sender used to update [local_network::update_wv_watch] about new worldviews recieved over UDP
/// 
/// ## Behaviour
/// - Sets up a reusable listener listening for udp-broadcasts
/// - Continously reads on the listener
/// - Checks for key-string on all recieved messages, making sure the message is from one of 'our' nodes. 
/// - If the message is from the current master or a node with lower ID than the current master, it sends it on `udp_wv_tx`
/// 
/// ## Note
/// This function is permanently blocking, and should be called asynchronously 
pub async fn start_udp_listener(
    wv_watch_rx: watch::Receiver<WorldView>, 
    udp_wv_tx: mpsc::Sender<WorldView>
) -> tokio::io::Result<()> 
{
    while !network::read_network_status() {
        
    }
    //Set up sockets
    let self_id = network::read_self_id();
    let broadcast_listen_addr = format!("{}:{}", config::BC_LISTEN_ADDR, config::BROADCAST_PORT);
    let socket_addr: SocketAddr = broadcast_listen_addr.parse().expect("Invalid address");
    let socket_temp = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    
    socket_temp.set_nonblocking(true).expect("Failed to set non-blocking");
    socket_temp.set_reuse_address(true)?;
    socket_temp.set_broadcast(true)?;
    socket_temp.bind(&socket_addr.into())?;
    let socket = UdpSocket::from_std(socket_temp.into())?;
    let mut buf = [0; config::UDP_BUFFER];
    
    let mut read_wv: Option<WorldView>;
    let mut my_wv = world_view::get_wv(wv_watch_rx.clone());

    loop {
        // Read message on UDP-broadcast address
        match socket.recv_from(&mut buf).await {
            Ok((len, _)) => {
                read_wv = parse_message(&buf[..len]);
            }
            Err(e) => {
                return Err(e);
            }
        }
        
        match read_wv {
            Some(mut read_wv) => {
                world_view::update_wv(wv_watch_rx.clone(), &mut my_wv).await;
                // Pass the recieved WorldView if the message came from the 
                // master or a node with a lower ID than current master, 
                // and this node is not the master
                if my_wv.master_id >= read_wv.master_id
                    && self_id != read_wv.master_id
                {
                    my_wv = read_wv;
                    let _ = udp_wv_tx.send(my_wv.clone()).await;
                }
            },
            None => continue,
        }
    }
}


/* __________ END PUBLIC FUNCTIONS __________ */




/* __________ START PRIVATE FUNCTIONS __________ */


/// Builds the UDP-broadcast message from the worldview
/// 
/// # Parameters
/// `wv`: Reference to the current [WorldView]
/// 
/// # Returns
/// -`Vec<u8>`: Containing serialized data of the message, ready to be sent
/// 
/// # Behavior
/// The function serializes a key, used for other nodes on the network to recognize this broadcast from others, 
/// and appends the serialized data of the worldview.
fn build_message(
    wv: &WorldView
) -> Vec<u8> {
    let mut buf = Vec::new();

    // Add the serialized key
    let key_bytes = world_view::serialize(&config::KEY_STR);
    buf.extend_from_slice(&key_bytes);

    // Add the serialized worldview
    let wv_bytes = world_view::serialize(&wv);
    buf.extend_from_slice(&wv_bytes);

    buf
}

/// Reconstructs a [WorldView] from recieved UDP-message
/// 
/// # Parameters
/// `buf`: Referance to a buffer containing the raw data read from UDP
/// 
/// # Returns
/// -`Option<WorldView>`: A WorldView reconstructed from the data, if no errors occures
/// -`None`: If an error occures while deserializing, or if the broadcast does not contain our key
/// 
/// # Behavior
/// The function first looks for the [config::KEY_STR] in the beginning og the message, returning `None` if it is not found.  
/// If it is found, the function tries to deserialize a [WorldView] from the rest of the message, returning it wrapped in an `Option` if it succeeded, returning `None` if it failed. 
pub fn parse_message(
    buf: &[u8]
) -> Option<WorldView> {
    // 1. Prøv å deserialisere nøkkelen
    let key_len = bincode::serialized_size(config::KEY_STR).unwrap() as usize;

    if buf.len() <= key_len {
        return None;
    }

    let (key_part, wv_part) = buf.split_at(key_len);

    let key: String = bincode::deserialize(key_part).ok()?;
    if key != config::KEY_STR {
        return None; // feil nøkkel
    }

    // 2. Deserialize resten til WorldView
    world_view::deserialize(wv_part)
}


/* __________ END PRIVATE FUNCTIONS __________ */

