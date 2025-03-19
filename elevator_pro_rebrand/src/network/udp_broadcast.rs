//! ## Håndterer UDP-logikk i systemet

use crate::config;
use crate::print;
use crate::world_view;
use crate::world_view::world_view_update;
use super::local_network;

use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::thread::sleep;
use std::time::Duration;
use tokio::net::UdpSocket;
use socket2::{Domain, Socket, Type};
use tokio::sync::mpsc;
use std::borrow::Cow;
use tokio::sync::watch;

static UDP_TIMEOUT: OnceLock<AtomicBool> = OnceLock::new();

/// Returns AtomicBool indicating if UDP has timeout'd. 
/// 
/// Initialized as false.
pub fn get_udp_timeout() -> &'static AtomicBool {
    UDP_TIMEOUT.get_or_init(|| AtomicBool::new(false))
}

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
pub async fn start_udp_broadcaster(wv_watch_rx: watch::Receiver<Vec<u8>>) -> tokio::io::Result<()> {
    let mut prev_network_status = true;

    // Sett opp sockets
    let addr: &str = &format!("{}:{}", config::BC_ADDR, config::DUMMY_PORT);
    let addr2: &str = &format!("{}:0", config::BC_LISTEN_ADDR);

    let broadcast_addr: SocketAddr = addr.parse().expect("ugyldig adresse"); // UDP-broadcast adresse
    let socket_addr: SocketAddr = addr2.parse().expect("Ugyldig adresse");
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

        // Hvis du er master, broadcast worldview
        if local_network::SELF_ID.load(Ordering::SeqCst) == wv[config::MASTER_IDX] {
            //TODO: Lag bedre delay?
            sleep(config::UDP_PERIOD);
            let mesage = format!("{:?}{:?}", config::KEY_STR, wv).to_string();

            // Kun send hvis du har internett-tilkobling
            if world_view::world_view_update::get_network_status().load(Ordering::SeqCst) {
                // Gi den tid til å lese nye wv fra udp tilfelle den var ute av internett lenge
                if !prev_network_status {
                    sleep(Duration::from_millis(500));
                    prev_network_status = true;
                }
                udp_socket.send_to(mesage.as_bytes(), &broadcast_addr).await?;
            }else {
                prev_network_status = false;
            }
        }
    }
}

// ### Starter og kjører udp-listener
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
pub async fn start_udp_listener(wv_watch_rx: watch::Receiver<Vec<u8>>, udp_wv_tx: mpsc::Sender<Vec<u8>>) -> tokio::io::Result<()> {
    let mut prev_network_status = true;
    
    //Sett opp sockets
    let self_id = local_network::SELF_ID.load(Ordering::SeqCst);
    let broadcast_listen_addr = format!("{}:{}", config::BC_LISTEN_ADDR, config::DUMMY_PORT);
    let socket_addr: SocketAddr = broadcast_listen_addr.parse().expect("Ugyldig adresse");
    let socket_temp = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    
    socket_temp.set_nonblocking(true).expect("Failed to set non-blocking");
    socket_temp.set_reuse_address(true)?;
    socket_temp.set_broadcast(true)?;
    socket_temp.bind(&socket_addr.into())?;
    let socket = UdpSocket::from_std(socket_temp.into())?;
    let mut buf = [0; config::UDP_BUFFER];
    let mut read_wv: Vec<u8> = Vec::new();
    
    let mut message: Cow<'_, str> = std::borrow::Cow::Borrowed("a");
    let mut my_wv = world_view::get_wv(wv_watch_rx.clone());
    // Loop mottar og behandler udp-broadcaster
    loop {
        if world_view::world_view_update::get_network_status().load(Ordering::SeqCst) {
            match socket.recv_from(&mut buf).await {
                Ok((len, _)) => {
                    message = String::from_utf8_lossy(&buf[..len]);
                    // println!("WV length: {:?}", len);
                }
                Err(e) => {
                    // utils::print_err(format!("udp_broadcast.rs, udp_listener(): {}", e));
                    return Err(e);
                }
            }
        
             // Verifiser at broadcasten var fra 'oss'
            if &message[1..config::KEY_STR.len()+1] == config::KEY_STR { //Plusser på en, siden serialiseringa av stringen tar med '"'-tegnet
                let clean_message = &message[config::KEY_STR.len()+3..message.len()-1]; // Fjerner `"`
                read_wv = clean_message
                .split(", ") // Del opp på ", "
                .filter_map(|s| s.parse::<u8>().ok()) // Konverter til u8, ignorer feil
                .collect(); // Samle i Vec<u8>

                world_view::update_wv(wv_watch_rx.clone(), &mut my_wv).await;
                if read_wv[config::MASTER_IDX] != my_wv[config::MASTER_IDX] {
                    // mulighet for debug print
                } else {
                    // Betyr at du har fått UDP-fra nettverkets master -> Restart UDP watchdog
                    get_udp_timeout().store(false, Ordering::SeqCst);
                    // println!("Resetter UDP-watchdog");
                }
                
                // Hvis du har vert i offline mode: merge worldviews
                println!("På nettverk");
                if !prev_network_status {
                    print::err("Kom tilbake på nett".to_string());
                    world_view_update::merge_wv_after_offline(&mut my_wv, &read_wv);
                    let _ = udp_wv_tx.send(my_wv.clone()).await;
                }
                prev_network_status = true;
                
                // Hvis broadcast har lavere ID enn nettverkets tidligere master
                if my_wv[config::MASTER_IDX] >= read_wv[config::MASTER_IDX] {
                    if !(self_id == read_wv[config::MASTER_IDX]) {
                        //Oppdater egen WV
                        my_wv = read_wv;
                        let _ = udp_wv_tx.send(my_wv.clone()).await;
                    }
                }
                
                
            }
        } else {
            println!("Av nettverk");
            prev_network_status = false;
            sleep(config::OFFLINE_PERIOD);
        }
    }
}


// ### jalla udp watchdog
pub async fn udp_watchdog(tcp_to_master_failed_tx: mpsc::Sender<bool>) {
    loop {
        if get_udp_timeout().load(Ordering::SeqCst) == false {
            get_udp_timeout().store(true, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        else {
            get_udp_timeout().store(false, Ordering::SeqCst); //resetter watchdogen
            print::warn("UDP-watchdog: Timeout".to_string());
            let _ = tcp_to_master_failed_tx.send(true).await;
        }
    }
}

