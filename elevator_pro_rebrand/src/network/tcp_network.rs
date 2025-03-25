//! # TCP Module
//! 
//! This module handles TCP communication between master and slave systems in a network.
//! It includes functions for setting up listeners, managing connections, and transferring data.
//!
//! ## Functions
//! - `listener_task`: Listens for incoming TCP connections.
//! - `tcp_handler`: Manages TCP communication by switching between master and slave behavior.
//! 
//! ## Key Features
//! - Manages the communication between nodes, dynamically changing behaviour when a node switches from master &harr; slave.
//! - All nodes has an active listener, accepting incoming connections.
//! - Slave nodes sends [ElevatorContainer]'s to the master.
//! - Master nodes recieves [ElevatorContainer]'s from slaves.
//! - All connections are set up on configured sockets, to handle extreme (>50%) packetloss.
//!
//! ## Usage
//! The module integrates with the system's network and worldview components to facilitate
//! reliable master-slave communication over TCP.

use std::{io::Error, sync::atomic::{AtomicBool, Ordering}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::{mpsc, watch}, task::JoinHandle, time::{sleep, Duration}};
use std::net::SocketAddr;
use socket2::{Domain, Protocol, SockAddr, Socket, TcpKeepalive, Type};
use crate::{config, ip_help_functions::{self}, network, print, world_view::{self, ElevatorContainer, WorldView}};


/* __________ START PUBLIC FUNCTIONS __________ */

/// AtomicBool representing if you are master on the network. 
/// 
/// The value is initialized as false
pub static IS_MASTER: AtomicBool = AtomicBool::new(false);


/// Handles the TCP listener
/// 
/// # Parameters
/// `socket_tx`: mpsc Sender on channel for sending stream and address for newly connected slaves
/// 
/// # Return
/// The functions returns if any fatal errors occures
/// 
/// # Behavior
/// The function sets up a listener as soon as the system is online.
/// While the program is online, it accepts new connections on the listener, and sends the socket over `socket_tx`. 
pub async fn listener_task(
    socket_tx: mpsc::Sender<(TcpStream, SocketAddr)>
) {
    /* On first init. make sure the system is online so no errors occures while setting up the listener */
    while !network::read_network_status() {
        tokio::time::sleep(config::TCP_PERIOD).await;
    }
    
    let socket = match create_tcp_socket() {
        Ok(sock) => sock,
        Err(e) => {
            print::err(format!("Failed to set up TCP listener: {}", e));
            panic!();
        }
    };
    
    let listener = match listener_from_socket(socket) {
        Some(list) => list,
        None => {
            panic!();
        }
    };

    loop {
        /* Check if you are online */
        if network::read_network_status() {
            sleep(Duration::from_millis(100)).await;
            /* Accept new connections */
            match listener.accept().await {
                Ok((socket, addr)) => {
                    print::master(format!("{} connected to TCP", addr));
                    if socket_tx.send((socket, addr)).await.is_err() {
                        print::err("socker_rx is closed, returning".to_string());
                        break;
                    }
                }
                Err(e) => {
                    print::err(format!("Error while accepting slave connection: {}", e));
                }
            }
        } else {
            sleep(config::OFFLINE_PERIOD).await;
        }
    }
}

/// Function that handles TCP-connections in the system
/// 
/// # Parameters
/// `wv_watch_rx`: Reciever on watch the worldview is being sent on in the system   
/// `remove_container_tx`: mpsc Sender used to notify worldview updater if a slave should be removed  
/// `connection_to_master_failed`: Sender on mpsc channel signaling if connection to master has failed   
/// `sent_tcp_container_tx`: mpsc Sender for notifying worldview updater what data has been sent to and ACKed by master    
/// `container_tx`: mpsc Sender used pass recieved [ElevatorContainer]'s to the worldview_updater  
/// `socket_rx`: Reciever on mpsc channel recieving new TcpStreams and SocketAddress from the TCP listener   
/// 
/// # Behavior
/// The function loops:
/// - Call and await [tcp_while_master].
/// - Call and await [tcp_while_slave].
/// 
/// # Note
/// - If the function is called without internet connection, it will not do anything before internet connection is back up again.  
/// - The function is dependant on [listener_task] to be running for the master-behavior to work as excpected.
/// 
pub async fn tcp_handler(
    wv_watch_rx: watch::Receiver<WorldView>, 
    remove_container_tx: mpsc::Sender<u8>, 
    container_tx: mpsc::Sender<ElevatorContainer>, 
    connection_to_master_failed_tx: mpsc::Sender<bool>, 
    sent_tcp_container_tx: mpsc::Sender<ElevatorContainer>, 
    mut socket_rx: mpsc::Receiver<(TcpStream, SocketAddr)>
) 
{
    while !network::read_network_status() {
        
    }
    let mut wv = world_view::get_wv(wv_watch_rx.clone());
    loop {
        IS_MASTER.store(true, Ordering::SeqCst);
        tcp_while_master(&mut wv, wv_watch_rx.clone(), &mut socket_rx, remove_container_tx.clone(), container_tx.clone()).await;
        
        IS_MASTER.store(false, Ordering::SeqCst);
        tcp_while_slave(&mut wv, wv_watch_rx.clone(), connection_to_master_failed_tx.clone(), sent_tcp_container_tx.clone()).await;
    }
}



/* __________ END PUBLIC FUNCTIONS __________ */











/* __________ START PRIVATE FUNCTIONS __________ */

/// Function to read TcpStream to a slave
/// 
/// # Parameters
/// `stream`: The stream connected to the slave  
/// `remove_container_tx`: mpsc Sender used to notify worldview updater if a slave should be removed    
/// `container_tx`: mpsc Sender used pass recieved [ElevatorContainer] to the worldview_updater   
/// 
/// # Behavior
/// The function continously reads from the stream, and sends recieved [ElevatorContainer]'s on `container_tx`.
async fn start_reading_from_slave(
    mut stream: TcpStream, 
    remove_container_tx: mpsc::Sender<u8>, 
    container_tx: mpsc::Sender<ElevatorContainer>
) {
    loop {
        /* Tries to read from stream */
        let result = read_from_stream(remove_container_tx.clone(), &mut stream).await;
        match result {
            Some(msg) => {
                let _ = container_tx.send(msg).await;
            }
            None => {
                break;
            }
        }
        
    }
}


/// Function that handles TCP while you are master on the system
/// 
/// # Parameters
/// `wv`: A mutable refrence to the current worldview   
/// `wv_watch_rx`: Reciever on watch the worldview is being sent on in the system   
/// `socket_rx`: Reciever on mpsc channel recieving new TcpStreams and SocketAddress from the TCP listener   
/// `remove_container_tx`: mpsc Sender used to notify worldview updater if a slave should be removed   
/// `container_tx`: mpsc Sender used pass recieved [ElevatorContainer] to the worldview_updater  
/// 
/// # Behavior
/// While the system is master on the network:
/// - Recieve new TcpStreams on `socket_rx`.  
/// - If a new TcpStream is recieved, it runs [start_reading_from_slave] on the stream 
async fn tcp_while_master(
    wv: &mut WorldView, 
    wv_watch_rx: watch::Receiver<WorldView>, 
    socket_rx: &mut mpsc::Receiver<(TcpStream, SocketAddr)>, 
    remove_container_tx: mpsc::Sender<u8>, 
    container_tx: mpsc::Sender<ElevatorContainer>
) {
    /* While you are master */
    while world_view::is_master(&wv) {
        /* Check if you are online */
        if network::read_network_status() {
            /* Revieve TCP-streams to newly connected slaves */
            while let Ok((stream, addr)) = socket_rx.try_recv() {
                print::info(format!("New slave connected: {}", addr));

                let remove_container_tx_clone = remove_container_tx.clone();
                let container_tx_clone = container_tx.clone();
                let _slave_task: JoinHandle<()> = tokio::spawn(async move {
                    /* Start handling the slave. Also has watchdog function to detect timeouts on messages */
                    start_reading_from_slave(stream, remove_container_tx_clone, container_tx_clone).await;
                });
                /* Make sure other tasks are able to run */
                tokio::task::yield_now().await; 
            }                
        }
        else {
            tokio::time::sleep(Duration::from_millis(100)).await; 
        }
        world_view::update_wv(wv_watch_rx.clone(), wv).await;
    }
}

/// This function handles tcp connection while you are a slave on the system
/// 
/// # Parameters
/// `wv`: A mutable refrence to the current worldview  
/// `wv_watch_rx`: Reciever on watch the worldview is being sent on in the system  
/// `connection_to_master_failed`: Sender on mpsc channel signaling if connection to master has failed   
/// `sent_tcp_container_tx`: mpsc Sender for notifying worldview updater what data has been sent to master  
/// 
/// 
/// # Behavior
/// The function tries to connect to the master.
/// While the system is a slave on the network and connection to the master is valid:
/// - Send TCP message to the master
/// - Check for new master on the system
async fn tcp_while_slave(
    wv: &mut WorldView, 
    wv_watch_rx: watch::Receiver<WorldView>, 
    connection_to_master_failed_tx: mpsc::Sender<bool>, 
    sent_tcp_container_tx: mpsc::Sender<ElevatorContainer>
) {
    /* Try to connect with master over TCP */
    let mut master_accepted_tcp = false;
    let mut stream:Option<TcpStream> = None;
    if let Some(s) = connect_to_master(wv_watch_rx.clone()).await {
        println!("Master accepted the TCP-connection");
        master_accepted_tcp = true;
        stream = Some(s);
    } else {
        println!("Master did not accept the TCP-connection");
        sleep(Duration::from_secs(100)).await;
        let _ = connection_to_master_failed_tx.send(true).await;
    }

    let mut prev_master: u8;
    let mut new_master = false;
    /* While you are slave and tcp-connection to master is good */
    while !world_view::is_master(wv) && master_accepted_tcp {
        /* Check if you are online */
        if network::read_network_status() {
            if let Some(ref mut s) = stream {
                /* Send TCP message to master */
                send_tcp_message(connection_to_master_failed_tx.clone(), sent_tcp_container_tx.clone(), s, wv).await;
                if new_master {
                    print::slave(format!("New master on the network"));
                    master_accepted_tcp = false;
                    let _ = sleep(config::SLAVE_TIMEOUT);
                }
                prev_master = wv.master_id;
                world_view::update_wv(wv_watch_rx.clone(), wv).await;
                if prev_master != wv.master_id {
                    new_master = true;
                }
                tokio::time::sleep(config::TCP_PERIOD).await; 
            }
        }
        else {
            let _ = sleep(config::SLAVE_TIMEOUT);
        }
    } 
}



/// Attempts to connect to master over TCP
/// 
/// # Parameters
/// `wv_watch_rx`: Reciever on watch the worldview is being sent on in the system   
/// 
/// # Return
/// `Some(TcpStream)`: Connection to master successfull, TcpStream is the stream to the master
/// `None`: Connection to master failed
/// 
/// # Behavior
/// The functions tries to connect to the current master, based on the master_id in the worldview. 
/// If the connection is successfull, it returns the stream, otherwise it returns None.
async fn connect_to_master(
    wv_watch_rx: watch::Receiver<WorldView>
) -> Option<TcpStream> {
    let wv = world_view::get_wv(wv_watch_rx.clone());

    /* Check if we are online */
    if network::read_network_status() {
        let master_ip = format!("{}.{}:{}", config::NETWORK_PREFIX, wv.master_id, config::PN_PORT);
        println!("Master id: {}", wv.master_id);
        print::info(format!("Trying to connect to : {} in connect_to_master()", master_ip));

        let socket = match create_tcp_socket() {
            Ok(sock) => {
                sock
            },
            Err(e) => {
                print::err(format!("Error while creating tcp-socket for connecting to master: {}", e));
                return None;
            } 
        };

        return connect_socket(socket, &master_ip);
    } else {
        None
    }
}



/// Function to read message from slave
/// 
/// # Parameters
/// `remove_container_tx`: mpsc Sender for channel used to indicate a slave should be removed at worldview updater  
/// `stream`: the stream to read from
/// 
/// # Return
/// `Some(Vec<u8>)`: The [ElevatorContainer] if it was read succesfully
/// `None`: If reading from stream fails, or you become slave
/// 
/// # Behavior
/// The function reads from stream. It first reads a header (2 bytes) indicating the message length.
/// Based on the header it reads the message. If everything works without error, it sends an ACK on the stream, and returns the message.
/// The function also asynchronously checks for loss of master status, and returns None if that is the case.
async fn read_from_stream(
    remove_container_tx: mpsc::Sender<u8>, 
    stream: &mut TcpStream
) -> Option<ElevatorContainer> {
    let id = ip_help_functions::ip2id(stream.peer_addr().expect("Slave has no IP?").ip());
    let mut len_buf = [0u8; 2];
    tokio::select! {
        result = stream.read_exact(&mut len_buf) => {
            match result {
                Ok(0) => {
                    print::info("Slave disconnected.".to_string());
                    let _ =  remove_container_tx.send(id).await;
                    return None;
                }
                Ok(_) => {
                    let len = u16::from_be_bytes(len_buf) as usize;
                    let mut buffer = vec![0u8; len];

                    match stream.read_exact(&mut buffer).await { 
                        Ok(0) => {
                            print::info("Slave disconnected".to_string());
                            let _ =  remove_container_tx.send(id).await;
                            return None;
                        }
                        Ok(_) => {
                            //TODO: ikke let _ = 
                            let _ =  stream.write_all(&[69]).await;
                            let _ = stream.flush().await;
                            
                            return world_view::deserialize(&buffer) 

                        },
                        Err(e) => {
                            print::err(format!("Error while reading from stream: {}", e));
                            let _ =  remove_container_tx.send(id).await;
                            return None;
                        }
                    }
                }
                Err(e) => {
                    print::err(format!("Error while reading from stream: {}", e));
                    let _ =  remove_container_tx.send(id).await;
                    return None;
                }
            }
        }
        _ = async {
            while IS_MASTER.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        } => {
            let id = ip_help_functions::ip2id(stream.peer_addr().expect("Peer has no IP?").ip());
            print::info(format!("Losing master status! Removing slave {}", id));
            let _ =  remove_container_tx.send(id).await;
            return None;
        }
    }
} 

 
/// Function that sends tcp message to master
/// 
/// # Parameters
/// `connection_to_master_failed_tx`: mpsc Sender for signaling to worldview updater that connection to master failed  
/// `sent_tcp_container_tx`: mpsc Sender for notifying worldview updater what data has been sent to master  
/// `stream`: The TcpStream to the master  
/// `wv`: The current worldview  
/// 
/// # Behavior
/// The functions extracts the systems own elevatorcontainer from the worldview.  
/// The function writes the following on the stream's transmission-buffer:
/// - Length of the message
/// - The message  
/// After this, it flushes the stream, and reads one byte from stream (used as ACKing from master)
/// Once the ACK is recieved, it sends the sent data over `sent_tcp_container_tx`. 
/// If writing to or reading from the stream fails, it signals on `connection_to_master_failed_tx`
async fn send_tcp_message(
    connection_to_master_failed_tx: mpsc::Sender<bool>, 
    sent_tcp_container_tx: mpsc::Sender<ElevatorContainer>, 
    stream: &mut TcpStream, 
    wv: &WorldView
) {
    let self_elev_container = match world_view::extract_self_elevator_container(wv) {
        Some(container) => container,
        None => {
            print::warn(format!("Failed to extract self elevator container"));
            return;
        }
    };
    
    let self_elev_serialized = world_view::serialize(&self_elev_container);
    
    /* Find number of bytes in the data to be sent */
    let len = (self_elev_serialized.len() as u16).to_be_bytes();    

    /* Send the message */
    if let Err(_) = stream.write_all(&len).await {
        let _ = connection_to_master_failed_tx.send(true).await;
    } else if let Err(_) = stream.write_all(&self_elev_serialized).await {
        let _ = connection_to_master_failed_tx.send(true).await; 
    } else if let Err(_) = stream.flush().await {
        let _ = connection_to_master_failed_tx.send(true).await; 
    } else {
        let mut buf: [u8; 1] = [0];
        match stream.read_exact(&mut buf).await {
            Ok(_) => {
                let _ = sent_tcp_container_tx.send(self_elev_container.clone()).await;
            },
            Err(e) => {
                print::err(format!("Master did not ACK the message: {}", e));
                let _ = connection_to_master_failed_tx.send(true).await;
            }
        }
    }
}


/// Creates a `TcpListener` from a given socket and binds it to the system's network address.
/// 
/// This function constructs a `SocketAddr` using the system's network prefix, the device's self ID,
/// and the configured port. It then binds the socket to this address and starts listening for incoming
/// TCP connections.
/// 
/// # Arguments
/// * `socket` - The `Socket` instance to use for listening.
/// 
/// # Returns
/// * `Some(TcpListener)` - If binding and listening are successful.
/// * `None` - If an error occurs.
/// 
/// # Errors
/// * If the system's network address cannot be created, an error is printed and `None` is returned.
/// * If binding the socket to the address fails, an error is printed and `None` is returned.
/// * If listening on the socket fails, an error is printed and `None` is returned.
/// * If converting the socket into a `TcpListener` fails, an error is printed and `None` is returned.
fn listener_from_socket(
    socket: Socket
) -> Option<TcpListener> {
    // Attempts to parse the socket address to self_ip
    let self_ip: SocketAddr = match format!("{}.{}:{}", config::NETWORK_PREFIX, network::read_self_id(), config::PN_PORT).parse() {
        Ok(addr) => addr,
        Err(e) => {
            print::err(format!("Failed to setup self listener socketaddr: {}", e));
            return None;
        }
    };

    // Attemps to bind the socket to self_ip
    match socket.bind(&self_ip.into()) {
        Ok(_) => {},
        Err(e) => {
            print::err(format!("Failed to bind socket: {}", e));
            return None
        }
    }

    // Attemps to start listening on the socket
    match socket.listen(128) {
        Ok(_) => {},
        Err(e) => {
            print::err(format!("Failed to start listening: {}", e));
            return None;
        }
    }

    // Set non blocking, so it can be used by Tokio
    socket.set_nonblocking(true).expect("Coulndt set socket non-blocking");

    // Convert the socket to an asynchrounus TcpListener
    match TcpListener::from_std(socket.into()) {
        Ok(listener) => {
            print::ok(format!("System listening on {}:{}", self_ip, config::PN_PORT));
            return Some(listener);
        },
        Err(e) => {
            print::err(format!("Failed to parse socket to tcplistener: {}", e));
            return None;
        }
    };
}


/// Attempts to connect a given socket to a specified target address.
/// 
/// This function takes a `Socket` and a `target` string, parses the string into a `SockAddr`,
/// and tries to establish a TCP connection. If successful, it converts the socket into a `TcpStream`
/// and returns it wrapped in an `Option`.
/// 
/// # Arguments
/// * `socket` - The `Socket` instance to connect.
/// * `target` - A `String` containing the target IP address and port in the format "IP:PORT".
/// 
/// # Returns
/// * `Some(TcpStream)` - If the connection is successful.
/// * `None` - If an error occures.
/// 
/// # Errors
/// * If `target` cannot be parsed into a valid `SocketAddr`, an error is printed and `None` is returned.
/// * If the connection attempt fails, an error is printed and `None` is returned.
/// * If converting the socket into a `TcpStream` fails, an error message is printed and `None` is returned.
fn connect_socket(
    socket: Socket, 
    target: &String
) -> Option<TcpStream> {
    // Attempts to parse the address
    let master_sock_addr: SockAddr = match target.parse::<SocketAddr>() {
        Ok(addr) => SockAddr::from(addr),
        Err(e) => {
            print::err(format!("Failed to parse string: {} into address: {}", target, e));
            return None;
        }
    };

    // Attempts to connect the socket to the destination address
    match socket.connect(&master_sock_addr) {
        Ok(()) => {
            print::ok(format!("Connected to Master: {} i TCP_listener()", target));
        },
        Err(e) => {
            print::err(format!("Failed to connect to master: {}", e));
            return None;
        },
    };     

    // Set non blocking, so it can be used by Tokio
    socket.set_nonblocking(true).expect("Coulndt set socket non-blocking");

    // Convert the socket into a standard TcpStream
    let std_stream: std::net::TcpStream = socket.into();

    // Convert the standard TcpStream to an asynchrounus tokio TcpStream
    match TcpStream::from_std(std_stream) {
        Ok(stream) => {
            return Some(stream);
        },
        Err(e) => {
            eprintln!("Failed to convert socket to TcpStream: {}", e);
            return None;
        }
    };
}

/// Creates and configures a TCP socket with optimized settings for performance and reliability.
/// 
/// The function sets buffer sizes, keepalive settings, timeouts, and platform-specific options.
/// 
/// # Returns
/// * `Ok(Socket)` - A configured TCP socket.
/// * `Err(Error)` - If socket creation or configuration fails.
/// 
/// # Platform-Specific Behavior
/// * Some options, such as `set_thin_linear_timeouts`, `set_tcp_user_timeout`, `set_quickack`, and `set_cork`,
///   are only available on Linux and will not be set on Windows.
fn create_tcp_socket() -> Result<Socket, Error> {
    // Create a new TCP socket
    let socketres = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP));
    let socket: Socket = match socketres {
        Ok(sock) => sock,
        Err(e) => {return Err(e);},
    };

    // Set send and receive buffer sizes to 16MB for high-throughput connections. 
    socket.set_send_buffer_size(16_777_216)?;
    socket.set_recv_buffer_size(16_777_216)?;

    // Configure TCP keepalive to detect broken connections.
    

    #[cfg(target_os = "linux")]
    {
        let keepalive = TcpKeepalive::new() 
            .with_time(Duration::from_secs(10))         // Start sending keepalive probes after 10 seconds of inactivity.
            .with_interval(Duration::from_secs(1))                  // Send keepalive probes every 1 second.
            // On Linux, specify the number of keepalive probes before the connection is considered dead.
            // This setting is not available on Windows.
            .with_retries(10);
        socket.set_tcp_keepalive(&keepalive.to_owned().clone())?;
    }

    #[cfg(target_os = "windows")]
    {
        let keepalive = TcpKeepalive::new() 
            .with_time(Duration::from_secs(10))         // Start sending keepalive probes after 10 seconds of inactivity.
            .with_interval(Duration::from_secs(1));                  // Send keepalive probes every 1 second.
        socket.set_tcp_keepalive(&keepalive.to_owned().clone())?;
    }

    // Set read and write timeouts to 10 seconds.
    socket.set_read_timeout(Some(Duration::from_secs(20)))?;
    socket.set_write_timeout(Some(Duration::from_secs(20)))?;

    #[cfg(target_os = "linux")]
    {
        // Enable thin linear timeouts (Linux only), drastically improving retransmission timing under congestion.
        socket.set_thin_linear_timeouts(true)?;

        // Set TCP user timeout (Linux only) to close the connection if no acknowledgments are received within 10s.
        socket.set_tcp_user_timeout(Some(Duration::from_secs(20)))?;

        // Enable TCP Quick ACK (Linux only), reducing latency by immediately acknowledging received packets.
        socket.set_quickack(true)?;
    }

    // Disable Nagle’s algorithm to minimize latency for interactive or real-time applications.
    // This ensures small packets are sent immediately instead of being buffered.
    socket.set_nodelay(true)?;

    Ok(socket)
}


/* __________ END PRIVATE FUNCTIONS __________ */

