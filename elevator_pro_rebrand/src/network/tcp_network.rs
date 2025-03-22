//! ## Håndterer TCP-logikk i systemet

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, task::JoinHandle, sync::{mpsc, watch}, time::{sleep, Duration, Instant}};
use std::net::SocketAddr;
use crate::{config, print, ip_help_functions::{self}, world_view::{self, serial, world_view_update}};
use super::local_network;

/// AtomicBool representing if you are master on the network. 
/// 
/// The value is initialized as false
pub static IS_MASTER: AtomicBool = AtomicBool::new(false);

/// ### TcpWatchdog
/// 
/// Håndterer timeout på TCP connections hos master, og lesing fra slave
struct TcpWatchdog {
    timeout: Duration,
}

impl TcpWatchdog {
    /// Starter en asynkron løkke der vi veksler mellom å lese fra stream og sjekke for timeout.
    async fn start_reading_from_slave(&self, mut stream: TcpStream, remove_container_tx: mpsc::Sender<u8>, container_tx: mpsc::Sender<Vec<u8>>) {
        let mut last_success = Instant::now();

        loop {
            // Kalkulerer hvor lang tid vi har igjen før timeout inntreffer.
            let remaining = self.timeout
                .checked_sub(last_success.elapsed())
                .unwrap_or(Duration::from_secs(0));

            // Lager en sleep-future basert på gjenværende tid.
            let sleep_fut = sleep(remaining);
            tokio::pin!(sleep_fut);

            tokio::select! {
                // Forsøker å lese fra stream med de nødvendige parameterne.
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
                // Triggeres dersom ingen melding er mottatt innen timeout‑tiden.
                _ = &mut sleep_fut => {
                    print::err(format!("Timeout: Ingen melding mottatt innen {:?}", self.timeout));
                    let id = ip_help_functions::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                    print::info(format!("Stenger stream til slave {}", id));
                    let _ = remove_container_tx.send(id).await;
                    let _ = stream.shutdown().await;
                    break;
                }
            }
        }
    }
}


// ### Håndterer TCP-connections
pub async fn tcp_handler(
                        wv_watch_rx: watch::Receiver<Vec<u8>>, 
                        remove_container_tx: mpsc::Sender<u8>, 
                        container_tx: mpsc::Sender<Vec<u8>>, 
                        tcp_to_master_failed_tx: mpsc::Sender<bool>, 
                        sent_tcp_container_tx: mpsc::Sender<Vec<u8>>, 
                        mut socket_rx: mpsc::Receiver<(TcpStream, SocketAddr)>
                    ) 
{
    use crate::world_view::world_view_update;
    while !world_view_update::read_network_status() {
        
    }
    let mut wv = world_view::get_wv(wv_watch_rx.clone());
    loop {
        IS_MASTER.store(true, Ordering::SeqCst);
        tcp_while_master(&mut wv, wv_watch_rx.clone(), &mut socket_rx, remove_container_tx.clone(), container_tx.clone()).await;

        //mista master
        IS_MASTER.store(false, Ordering::SeqCst);
        tcp_while_slave(&mut wv, wv_watch_rx.clone(), tcp_to_master_failed_tx.clone(), sent_tcp_container_tx.clone()).await;

        //ny master
    }
}


async fn tcp_while_master(wv: &mut Vec<u8>, wv_watch_rx: watch::Receiver<Vec<u8>>, socket_rx: &mut mpsc::Receiver<(TcpStream, SocketAddr)>, remove_container_tx: mpsc::Sender<u8>, container_tx: mpsc::Sender<Vec<u8>>) {
    /* While you are master */
    while world_view::is_master(wv.clone()) {
        /* Check if you are online */
        if world_view_update::read_network_status() {
            /* Revieve TCP-sockets to newly connected slaves */
            while let Ok((socket, addr)) = socket_rx.try_recv() {
                print::info(format!("New slave connected: {}", addr));

                let remove_container_tx_clone = remove_container_tx.clone();
                let container_tx_clone = container_tx.clone();
                let _slave_task: JoinHandle<()> = tokio::spawn(async move {
                    let tcp_watchdog = TcpWatchdog {
                        timeout: Duration::from_millis(config::TCP_TIMEOUT),
                    };
                    /* Start handling the slave. Also has watchdog function to detect timeouts on messages */
                    tcp_watchdog.start_reading_from_slave(socket, remove_container_tx_clone, container_tx_clone).await;
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

async fn tcp_while_slave(wv: &mut Vec<u8>, wv_watch_rx: watch::Receiver<Vec<u8>>, tcp_to_master_failed_tx: mpsc::Sender<bool>, sent_tcp_container_tx: mpsc::Sender<Vec<u8>>) {
    /* Try to connect with master over TCP */
    let mut master_accepted_tcp = false;
    let mut stream:Option<TcpStream> = None;
    if let Some(s) = connect_to_master(wv_watch_rx.clone(), tcp_to_master_failed_tx.clone()).await {
        println!("Master accepta tilkobling");
        master_accepted_tcp = true;
        stream = Some(s);
    } else {
        println!("Master accepta IKKE tilkobling");
    }

    let mut prev_master: u8;
    let mut new_master = false;
    /* While you are slave and tcp-connection to master is good */
    while !world_view::is_master(wv.clone()) && master_accepted_tcp {
        /* Check if you are online */
        if world_view_update::read_network_status() {
            if let Some(ref mut s) = stream {
                if new_master {
                    print::slave(format!("New master on the network"));
                    master_accepted_tcp = false;
                    let _ = sleep(config::SLAVE_TIMEOUT);
                }
                prev_master = wv[config::MASTER_IDX];
                world_view::update_wv(wv_watch_rx.clone(), wv).await;
                /* Send TCP message to master */
                send_tcp_message(tcp_to_master_failed_tx.clone(), sent_tcp_container_tx.clone(), s, wv.clone()).await;
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
/// `tcp_to_master_failed`: Sender on mpsc channel signaling if tcp connection to master has failed
/// 
/// # Return
/// `Some(TcpStream)`: Connection to master successfull, TcpStream is the stream to the master
/// `None`: Connection to master failed
/// 
/// # Behavior
/// The functions tries to connect to the current master, based on the master_id in the worldview. 
/// If the connection is successfull, it returns the stream, otherwise it returns None.
/// If the connection failed, it sends a signal to the worldview updater over `tcp_to_master_failed_tx` indicating that the connection failed.
async fn connect_to_master(wv_watch_rx: watch::Receiver<Vec<u8>>, tcp_to_master_failed_tx: mpsc::Sender<bool>) -> Option<TcpStream> {
    let wv = world_view::get_wv(wv_watch_rx.clone());

    /* Check if we are online */
    if world_view_update::read_network_status() {
        let master_ip = format!("{}.{}:{}", config::NETWORK_PREFIX, wv[config::MASTER_IDX], config::PN_PORT);
        print::info(format!("Trying to connect to : {} in connect_to_master()", master_ip));

        /* Try to connect to master */
        match TcpStream::connect(&master_ip).await {
            Ok(stream) => {
                print::ok(format!("Connected to Master: {} i TCP_listener()", master_ip));
                // Klarte å koble til master, returner streamen
                Some(stream)
            }
            Err(e) => {
                print::err(format!("Failed to connect to master over tcp: {}", e));

                match tcp_to_master_failed_tx.send(true).await {
                    Ok(_) => print::info("Notified that connection to master failed".to_string()),
                    Err(err) => print::err(format!("Error while sending message on tcp_to_master_failed: {}", err)),
                }
                None
            }
        }
    } else {
        None
    }
}

// ### Starter og kjører TCP-listener
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
    let self_ip = format!("{}.{}", config::NETWORK_PREFIX, local_network::SELF_ID.load(Ordering::SeqCst));
    /* On first init. make sure the system is online so no errors occures while setting up the listener */
    while !world_view_update::read_network_status() {
        tokio::time::sleep(config::TCP_PERIOD).await;
    }

    /* Bind the listener on port [config::PN_PORT] */
    let listener = match TcpListener::bind(format!("{}:{}", self_ip, config::PN_PORT)).await {
        Ok(l) => {
            print::ok(format!("System listening on {}:{}", self_ip, config::PN_PORT));
            l
        }
        Err(e) => {
            print::err(format!("Error while setting up TCP listener: {}", e));
            return;
        }
    };

    loop {
        /* Check if you are online */
        if world_view_update::read_network_status() {
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



/// ## Leser fra `stream`
/// 
/// Select mellom å lese melding fra slave og sende meldingen til `world_view_handler` og å avslutte streamen om du ikke er master
async fn read_from_stream(remove_container_tx: mpsc::Sender<u8>, stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut len_buf = [0u8; 2];
    tokio::select! {
        result = stream.read_exact(&mut len_buf) => {
            match result {
                Ok(0) => {
                    print::info("Slave har kopla fra.".to_string());
                    print::info(format!("Stenger stream til slave 1: {:?}", stream.peer_addr()));
                    let id = ip_help_functions::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                    let _ =  remove_container_tx.send(id).await;
                    // let _ = stream.shutdown().await;
                    return None;
                }
                Ok(_) => {
                    let len = u16::from_be_bytes(len_buf) as usize;
                    let mut buffer = vec![0u8; len];

                    match stream.read_exact(&mut buffer).await { 
                        Ok(0) => {
                            print::info("Slave har kopla fra.".to_string());
                            print::info(format!("Stenger stream til slave 2: {:?}", stream.peer_addr()));
                            let id = ip_help_functions::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                            let _ =  remove_container_tx.send(id).await;
                            // let _ = stream.shutdown().await;
                            return None;
                        }
                        Ok(_) => return Some(buffer),
                        Err(e) => {
                            print::err(format!("Feil ved mottak av data fra slave: {}", e));
                            print::info(format!("Stenger stream til slave 3: {:?}", stream.peer_addr()));
                            let id = ip_help_functions::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                            let _ =  remove_container_tx.send(id).await;
                            // let _ = stream.shutdown().await;
                            return None;
                        }
                    }
                }
                Err(e) => {
                    print::err(format!("Feil ved mottak av data fra slave: {}", e));
                    print::info(format!("Stenger stream til slave 4: {:?}", stream.peer_addr()));
                    let id = ip_help_functions::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                    let _ =  remove_container_tx.send(id).await;
                    // let _ = stream.shutdown().await;
                    return None;
                }
            }
        }
        _ = async {
            while IS_MASTER.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        } => {
            let id = ip_help_functions::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
            print::info(format!("Mistar masterstatus, stenger stream til slave {}", id));
            let _ =  remove_container_tx.send(id).await;
            // let _ = stream.shutdown().await;
            return None;
        }
    }
} 

/// ### Sender egen elevator_container til master gjennom stream
/// Sender på format : `(lengde av container) as u16`, `container`
pub async fn send_tcp_message(tcp_to_master_failed_tx: mpsc::Sender<bool>, sent_tcp_container_tx: mpsc::Sender<Vec<u8>>, stream: &mut TcpStream, wv: Vec<u8>) {
    let self_elev_container = match world_view::extract_self_elevator_container(wv) {
        Some(container) => container,
        None => {
            print::warn(format!("Failed to extract self elevator container"));
            return;
        }
    };
    
    let self_elev_serialized = serial::serialize_elev_container(&self_elev_container);
    
    let len = (self_elev_serialized.len() as u16).to_be_bytes(); // Konverter lengde til big-endian bytes    

    if let Err(_) = stream.write_all(&len).await {
        // utils::print_err(format!("Feil ved sending av data til master: {}", e));
        let _ = tcp_to_master_failed_tx.send(true).await; // Anta at tilkoblingen feila
    } else if let Err(_) = stream.write_all(&self_elev_serialized).await {
        // utils::print_err(format!("Feil ved sending av data til master: {}", e));
        let _ = tcp_to_master_failed_tx.send(true).await; // Anta at tilkoblingen feila
    } else if let Err(_) = stream.flush().await {
        // utils::print_err(format!("Feil ved flushing av stream: {}", e));
        let _ = tcp_to_master_failed_tx.send(true).await; // Anta at tilkoblingen feila
    } else {
        // send_succes_I = true;     
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

