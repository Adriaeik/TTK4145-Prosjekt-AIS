//! ## Håndterer TCP-logikk i systemet

use std::{fmt::{format, Debug}, io::Error, net::IpAddr, sync::atomic::{AtomicBool, Ordering}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpSocket, TcpStream}, sync::{mpsc, watch}, task::JoinHandle, time::{sleep, Duration, Instant}};
use std::net::SocketAddr;
use socket2::{Domain, Protocol, SockAddr, Socket, TcpKeepalive, Type};
use crate::{config, print, network, ip_help_functions::{self}, world_view::{self, serial}};


/* __________ START PUBLIC FUNCTIONS __________ */

/// AtomicBool representing if you are master on the network. 
/// 
/// The value is initialized as false
pub static IS_MASTER: AtomicBool = AtomicBool::new(false);


/// Handles the TCP listener
/// 
/// # Parameters
/// `socket_tx`: mpsc Sender on channel for sending newly connected slaves
/// 
/// # Return
/// The functions returns if any fatal errors occures
/// 
/// # Behavior
/// The function sets up a listener as soon as the system is online.
/// While the program is online, it accepts new connections on the listener, and sends the socket over `socket_tx`. 
/// 
pub async fn listener_task(socket_tx: mpsc::Sender<(TcpStream, SocketAddr)>) {
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

    /* Bind the listener on port [config::PN_PORT] */
    // let listener = match TcpListener::bind(format!("{}:{}", self_ip, config::PN_PORT)).await {
    //     Ok(l) => {
    //         print::ok(format!("System listening on {}:{}", self_ip, config::PN_PORT));
    //         l
    //     }
    //     Err(e) => {
    //         print::err(format!("Error while setting up TCP listener: {}", e));
    //         return;
    //     }
    // };

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
/// `sent_tcp_container_tx`: mpsc Sender for notifying worldview updater what data has been sent to master    
/// `container_tx`: mpsc Sender used pass recieved slave-messages to the worldview_updater  
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
    wv_watch_rx: watch::Receiver<Vec<u8>>, 
    remove_container_tx: mpsc::Sender<u8>, 
    container_tx: mpsc::Sender<Vec<u8>>, 
    connection_to_master_failed_tx: mpsc::Sender<bool>, 
    sent_tcp_container_tx: mpsc::Sender<Vec<u8>>, 
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

/// Handles timeout on TCP connection at master, and reading from slave
struct TcpWatchdog {
    timeout: Duration,
}

impl TcpWatchdog {
    /// Starts a loop where reading from stream and checking for timeout runs asynchronously
    /// 
    /// # Parameters
    /// `stream`: The TCP-stream to be read from
    /// `remove_container_tx`: mpsc Sender used to notify the worldview updater if a slave should be remover  
    /// `container_tx`: mpsc Sender used to pass recieved slave-messages to the worldview_updater  
    /// 
    /// # Behavior
    /// The function loops:
    /// - Calculate time before a timeout occures
    /// - Asynchronously select between:
    ///     - Sending the data successfully recieved on the TCP stream over `container_tx`
    ///     - Sending the ID of the slave on `remove_container_tx` on timeout event
    async fn start_reading_from_slave(&self, mut stream: TcpStream, remove_container_tx: mpsc::Sender<u8>, container_tx: mpsc::Sender<Vec<u8>>) {
        let mut last_success = Instant::now();

        loop {
            /* Calculate how long until timout occures */
            let remaining = self.timeout
                .checked_sub(last_success.elapsed())
                .unwrap_or(Duration::from_secs(0));

            /* Creates a sleep-future based on remaining time before timeout */
            let sleep_fut = sleep(remaining);
            tokio::pin!(sleep_fut);

            tokio::select! {
                /* Tries to read from stream */
                result = read_from_stream(remove_container_tx.clone(), &mut stream) => {
                    match result {
                        Some(msg) => {
                            let _ = container_tx.send(msg).await;
                            last_success = Instant::now()
                        }
                        None => {
                            break;
                        }
                    }
                }
                /* Triggers if no message is recieved within the timeout-duration */
                _ = &mut sleep_fut => {
                    print::err(format!("Timeout: No message recieved within: {:?}", self.timeout));
                    let id = ip_help_functions::ip2id(stream.peer_addr().expect("Peer has no IP?").ip());
                    print::info(format!("Closing stream to slave {}", id));
                    let _ = remove_container_tx.send(id).await;
                    close_tcp_stream(&mut stream).await;
                    break;
                }
            }
        }
    }
}


/// Function that handles TCP while you are master on the system
/// 
/// # Parameters
/// `wv`: A mutable refrence to the current serialized worldview   
/// `wv_watch_rx`: Reciever on watch the worldview is being sent on in the system   
/// `socket_rx`: Reciever on mpsc channel recieving new TcpStreams and SocketAddress from the TCP listener   
/// `remove_container_tx`: mpsc Sender used to notify worldview updater if a slave should be removed   
/// `container_tx`: mpsc Sender used pass recieved slave-messages to the worldview_updater  
/// 
/// # Behavior
/// While the system is master on the network:
/// - Recieve new TcpStreams on `socket_rx`.  
/// - If a new TcpStream is recieved, it starts [TcpWatchdog::start_reading_from_slave] on the stream 
async fn tcp_while_master(wv: &mut Vec<u8>, wv_watch_rx: watch::Receiver<Vec<u8>>, socket_rx: &mut mpsc::Receiver<(TcpStream, SocketAddr)>, remove_container_tx: mpsc::Sender<u8>, container_tx: mpsc::Sender<Vec<u8>>) {
    /* While you are master */
    while world_view::is_master(wv.clone()) {
        /* Check if you are online */
        if network::read_network_status() {
            /* Revieve TCP-streams to newly connected slaves */
            while let Ok((stream, addr)) = socket_rx.try_recv() {
                print::info(format!("New slave connected: {}", addr));

                let remove_container_tx_clone = remove_container_tx.clone();
                let container_tx_clone = container_tx.clone();
                let _slave_task: JoinHandle<()> = tokio::spawn(async move {
                    let tcp_watchdog = TcpWatchdog {
                        timeout: Duration::from_millis(config::TCP_TIMEOUT),
                    };
                    /* Start handling the slave. Also has watchdog function to detect timeouts on messages */
                    tcp_watchdog.start_reading_from_slave(stream, remove_container_tx_clone, container_tx_clone).await;
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
/// `wv`: A mutable refrence to the current serialized worldview  
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
async fn tcp_while_slave(wv: &mut Vec<u8>, wv_watch_rx: watch::Receiver<Vec<u8>>, connection_to_master_failed_tx: mpsc::Sender<bool>, sent_tcp_container_tx: mpsc::Sender<Vec<u8>>) {
    /* Try to connect with master over TCP */
    let mut master_accepted_tcp = false;
    let mut stream:Option<TcpStream> = None;
    if let Some(s) = connect_to_master(wv_watch_rx.clone()).await {
        println!("Master accepted the TCP-connection");
        // s.set_nodelay(true);
        // s.set_linger(Some(Duration::from_millis(1000)));
        // s.set_ttl(10);
        master_accepted_tcp = true;
        stream = Some(s);
    } else {
        println!("Master adid not accept the TCP-connection");
        let _ = connection_to_master_failed_tx.send(true).await;
    }

    let mut prev_master: u8;
    let mut new_master = false;
    /* While you are slave and tcp-connection to master is good */
    while !world_view::is_master(wv.clone()) && master_accepted_tcp {
        /* Check if you are online */
        if network::read_network_status() {
            if let Some(ref mut s) = stream {
                /* Send TCP message to master */
                send_tcp_message(connection_to_master_failed_tx.clone(), sent_tcp_container_tx.clone(), s, wv.clone()).await;
                if new_master {
                    print::slave(format!("New master on the network"));
                    master_accepted_tcp = false;
                    let _ = sleep(config::SLAVE_TIMEOUT);
                }
                prev_master = wv[config::MASTER_IDX];
                world_view::update_wv(wv_watch_rx.clone(), wv).await;
                if prev_master != wv[config::MASTER_IDX] {
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
/// `connection_to_master_failed`: Sender on mpsc channel signaling if connection to master has failed
/// 
/// # Return
/// `Some(TcpStream)`: Connection to master successfull, TcpStream is the stream to the master
/// `None`: Connection to master failed
/// 
/// # Behavior
/// The functions tries to connect to the current master, based on the master_id in the worldview. 
/// If the connection is successfull, it returns the stream, otherwise it returns None.
/// If the connection failed, it sends a signal to the worldview updater over `connection_to_master_failed_tx` indicating that the connection failed.
async fn connect_to_master(wv_watch_rx: watch::Receiver<Vec<u8>>) -> Option<TcpStream> {
    let wv = world_view::get_wv(wv_watch_rx.clone());

    /* Check if we are online */
    if network::read_network_status() {
        let master_ip = format!("{}.{}:{}", config::NETWORK_PREFIX, wv[config::MASTER_IDX], config::PN_PORT);
        println!("Master id: {}", wv[config::MASTER_IDX]);
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


        

        // Originalt:
        // /* Try to connect to master */
        // match TcpStream::connect(&master_ip).await {
        //     Ok(stream) => {
        //         print::ok(format!("Connected to Master: {} i TCP_listener()", master_ip));
        //         // Klarte å koble til master, returner streamen
        //         Some(stream)
        //     }
        //     Err(e) => {
        //         print::err(format!("Failed to connect to master over tcp: {}", e));

        //         match connection_to_master_failed_tx.send(true).await {
        //             Ok(_) => print::info("Notified that connection to master failed".to_string()),
        //             Err(err) => print::err(format!("Error while sending message on connection_to_master_failed: {}", err)),
        //         }
        //         None
        //     }
        // }
    } else {
        None
    }
}





/// ## Leser fra `stream`
/// 
/// Select mellom å lese melding fra slave og sende meldingen til `world_view_handler` og å avslutte streamen om du ikke er master

/// Function to read message from slave
/// 
/// # Parameters
/// `remove_container_tx`: mpsc Sender for channel used to indicate a slave should be removed at worldview updater  
/// `stream`: the stream to read from
/// 
/// # Return
/// `Some(Vec<u8>)`: The serialized message if it was read succesfully
/// `None`: If reading from stream fails, or you become slave
/// 
/// # Behavior
/// The function reads from stream. It first reads a header (2 bytes) indicating the message length.
/// Based on the header it reads the message. If everything works without error, it returns the message.
/// The function also asynchronously checks for loss of master status, and returns None if that is the case.
///  
async fn read_from_stream(remove_container_tx: mpsc::Sender<u8>, stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut len_buf = [0u8; 2];
    tokio::select! {
        result = stream.read_exact(&mut len_buf) => {
            match result {
                Ok(0) => {
                    print::info("Slave disconnected.".to_string());
                    let id = ip_help_functions::ip2id(stream.peer_addr().expect("Slave has no IP?").ip());
                    let _ =  remove_container_tx.send(id).await;
                    return None;
                }
                Ok(_) => {
                    let len = u16::from_be_bytes(len_buf) as usize;
                    let mut buffer = vec![0u8; len];

                    match stream.read_exact(&mut buffer).await { 
                        Ok(0) => {
                            print::info("Slave disconnected".to_string());
                            let id = ip_help_functions::ip2id(stream.peer_addr().expect("Slave has no IP?").ip());
                            let _ =  remove_container_tx.send(id).await;
                            return None;
                        }
                        Ok(_) => {
                            //TODO: ikke let _ = 
                            let _ =  stream.write_all(&[69]).await;
                        
                            return Some(buffer)
                        },
                        Err(e) => {
                            print::err(format!("Error while reading from stream: {}", e));
                            let id = ip_help_functions::ip2id(stream.peer_addr().expect("Slave has no IP?").ip());
                            let _ =  remove_container_tx.send(id).await;
                            return None;
                        }
                    }
                }
                Err(e) => {
                    print::err(format!("Error while reading from stream: {}", e));
                    let id = ip_help_functions::ip2id(stream.peer_addr().expect("Slave has no IP?").ip());
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

/// ### Sender egen elevator_container til master gjennom stream
/// Sender på format : `(lengde av container) as u16`, `container`
/// 
/// Function that sends tcp message to master
/// 
/// # Parameters
/// `connection_to_master_failed_tx`: mpsc Sender for signaling to worldview updater that connection to master failed  
/// `sent_tcp_container_tx`: mpsc Sender for notifying worldview updater what data has been sent to master  
/// `stream`: The TcpStream to the master  
/// `wv`: The current worldview in serial state  
/// 
/// # Behavior
/// The functions extracts the systems own elevatorcontainer from the worldview.  
/// The function writes the following on the stream's transmission-buffer:
/// - Length of the message
/// - The message  
/// After this, it flushes the stream, and sends the sent data over `ent_tcp_container_tx`. If writing to the stream fails, it signals on `connection_to_master_failed_tx`
async fn send_tcp_message(connection_to_master_failed_tx: mpsc::Sender<bool>, sent_tcp_container_tx: mpsc::Sender<Vec<u8>>, stream: &mut TcpStream, wv: Vec<u8>) {
    let self_elev_container = match world_view::extract_self_elevator_container(wv) {
        Some(container) => container,
        None => {
            print::warn(format!("Failed to extract self elevator container"));
            return;
        }
    };
    
    let self_elev_serialized = serial::serialize_elev_container(&self_elev_container);
    
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
            Ok(_) => {},
            Err(e) => {
                print::err(format!("Master did not ACK the message: {}", e));
                let _ = connection_to_master_failed_tx.send(true).await;
            }
        }
        let _ = sent_tcp_container_tx.send(self_elev_serialized).await;
    }
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
async fn close_tcp_stream(stream: &mut TcpStream) {
    /* Get local and peer address */
    let local_addr = stream.local_addr().map_or_else(
        |e| format!("Unknown (Error: {})", e),
        |addr| addr.to_string(),
    );
    let peer_addr = stream.peer_addr().map_or_else(
        |e| format!("Unknown (Error: {})", e),
        |addr| addr.to_string(),
    );

    /* Try to shutdown the stream */
    match stream.shutdown().await {
        Ok(_) => print::info(format!(
            "TCP-connection closed successfully: {} -> {}",
            local_addr, peer_addr
        )),
        Err(e) => print::err(format!(
            "Failed to close TCP-connection ({} -> {}): {}",
            local_addr, peer_addr, e
        )),
    }
}

/* __________ END PRIVATE FUNCTIONS __________ */

fn listener_from_socket(socket: Socket) -> Option<TcpListener> {
    let self_ip: SocketAddr = match format!("{}.{}:{}", config::NETWORK_PREFIX, network::read_self_id(), config::PN_PORT).parse() {
        Ok(addr) => addr,
        Err(e) => {
            print::err(format!("Failed to setup self listener socketaddr: {}", e));
            return None;
        }
    };

    match socket.bind(&self_ip.into()) {
        Ok(_) => {},
        Err(e) => {
            print::err(format!("Failed to bind socket: {}", e));
            return None
        }
    }
    match socket.listen(128) {
        Ok(_) => {},
        Err(e) => {
            print::err(format!("Failed to start listening: {}", e));
            return None;
        }
    }

    // Set non blocking, so it can be used bu tokio
    socket.set_nonblocking(true).expect("Coulndt set socket non-blocking");

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

fn connect_socket(socket: Socket, target: &String) -> Option<TcpStream> {
    let master_sock_addr: SockAddr = match target.parse::<SocketAddr>() {
        Ok(addr) => SockAddr::from(addr),
        Err(e) => {
            print::err(format!("Failed to parse string: {} into address: {}", target, e));
            return None;
        }
    };


    match socket.connect(&master_sock_addr) {
        Ok(()) => {
            print::ok(format!("Connected to Master: {} i TCP_listener()", target));
        },
        Err(e) => {
            print::err(format!("Failed to connect to master: {}", e));
            std::thread::sleep(Duration::from_secs(100));
            
            return None;
        },
    };     

    // Set non blocking, so it can be used bu tokio
    socket.set_nonblocking(true).expect("Coulndt set socket non-blocking");

    let std_stream: std::net::TcpStream = socket.into();

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

fn create_tcp_socket() -> Result<Socket, Error> {
    let socketres = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP));
    let socket: Socket = match socketres {
        Ok(sock) => sock,
        Err(e) => {return Err(e);},
    };

    // Set read and write buffers
    socket.set_send_buffer_size(16_777_216)?;
    socket.set_recv_buffer_size(16_777_216)?;

    // Set keepalive settings
    let keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(5))
        .with_interval(Duration::from_secs(5));

    socket.set_tcp_keepalive(&keepalive)?;

    Ok(socket)
}


