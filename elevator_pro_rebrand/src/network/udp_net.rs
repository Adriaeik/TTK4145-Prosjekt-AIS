use crate::config;
use crate::network;
use crate::print;
use crate::world_view;
use crate::world_view::ElevatorContainer;
use crate::world_view::WorldView;

use tokio::time::sleep;
use tokio::net::UdpSocket;
use socket2::{Domain, Socket, Type, Protocol};
use tokio::sync::{watch, mpsc, Mutex};
use std::net;
use std::{
    collections::HashMap,
    net::{SocketAddr, Ipv4Addr},
    sync::Arc,
    time::{Duration, Instant},
};
use std::sync::atomic::{AtomicBool, Ordering};


const ACK_REDUNDANCY: usize = 5; // Antall ACKs per pakke
const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(15); // Tidsgrense for inaktivitet
const CLEANUP_INTERVAL: Duration = Duration::from_secs(5); // Hvor ofte inaktive sendere fjernes

pub static IS_MASTER: AtomicBool = AtomicBool::new(false);


pub async fn start_udp_network(
    wv_watch_rx: watch::Receiver<WorldView>,
    container_tx: mpsc::Sender<ElevatorContainer>,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
) {
    while !network::read_network_status() {}
    let socket = match Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)) {
        Ok(sock) => sock,
        Err(e) => {panic!("Klarte ikke lage udp socket");}
    }; 
    while socket.set_reuse_address(true).is_err() {}
    while socket.set_send_buffer_size(16_000_000).is_err() {}
    while socket.set_recv_buffer_size(16_000_000).is_err() {}
    
    let addr: SocketAddr = format!("{}.{}:{}", config::NETWORK_PREFIX, network::read_self_id(), 50000).parse().unwrap();
    while socket.bind(&addr.into()).is_err() {}
    
    let socket = match UdpSocket::from_std(socket.into()) {
        Ok(sock) => sock,
        Err(e) => {panic!("Klarte ikke lage tokio udp socket");}
    }; 

    let mut wv = world_view::get_wv(wv_watch_rx.clone());
    loop {
        IS_MASTER.store(true, Ordering::SeqCst);
        receive_udp_master(
            &socket,
            &mut wv,
            wv_watch_rx.clone(),
            container_tx.clone(),
            packetloss_rx.clone(),
        ).await;
        
        IS_MASTER.store(false, Ordering::SeqCst);
        send_udp_slave(
            &socket,
            &mut wv,
            wv_watch_rx.clone(),
            packetloss_rx.clone(),  
        ).await;
    }
}



/// Holder informasjon om en aktiv sender
#[derive(Debug, Clone)]
struct ReceiverState {
    last_seq: u16,
    last_seen: Instant,
}

/// Starter en UDP-mottaker som håndterer flere samtidige sendere og sender redundante ACKs
async fn receive_udp_master(
    socket: &UdpSocket,
    wv: &mut WorldView,
    wv_watch_rx: watch::Receiver<WorldView>,
    container_tx: mpsc::Sender<ElevatorContainer>,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
) {    
    println!("Server listening on port {}", 50000);

    let state = Arc::new(Mutex::new(HashMap::<SocketAddr, ReceiverState>::new()));
    
    // Cleanup-task: Fjerner inaktive klienter
    let state_cleanup = state.clone();
    {
        let wv_watch_rx = wv_watch_rx.clone();
        let mut wv = wv.clone();
        tokio::spawn(async move {
            while wv.master_id == network::read_self_id() {
                sleep(CLEANUP_INTERVAL).await;
                let mut state = state_cleanup.lock().await;
                let now = Instant::now();
                state.retain(|_, s| now.duration_since(s.last_seen) < INACTIVITY_TIMEOUT);
                world_view::update_wv(wv_watch_rx.clone(), &mut wv).await;
            }
        });
    }

    let mut buf = [0; 65535];
    while wv.master_id == network::read_self_id() {
        // Mottar data
        let (len, slave_addr) = match socket.recv_from(&mut buf).await {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Error receiving UDP packet: {}", e);
                continue;
            }
        };

        let mut new_state = ReceiverState {
            last_seq: 0,
            last_seen: Instant::now(),
        };

        let mut state_locked = state.lock().await;
        let entry = state_locked.entry(slave_addr).or_insert(new_state.clone());
        let last_seq = entry.last_seq.clone();
        
        let msg = parse_message(&buf[..len], last_seq);
        
        match msg {
            Some(container) => {
                println!("Received valid packet from {}: seq {}", slave_addr, last_seq);
                //Meldinga er en forventet melding -> oppdater hashmappets state
                let _ = container_tx.send(container.clone()).await;

                new_state.last_seq = last_seq.wrapping_add(1);
                new_state.last_seen = Instant::now();

                let mut state_locked = state.lock().await;
                state_locked.insert(slave_addr, new_state);

                let packetloss = packetloss_rx.borrow().clone();
                let redundancy = get_redundancy(packetloss.packet_loss);

                send_acks(
                    &socket,
                    last_seq,
                    &slave_addr,
                    redundancy
                ).await;
            },
            None => {
                // println!("Ignoring out-of-order packet from {}", slave_addr);
                // Seq nummer doesnt match, or data has been corrupted.
                // Treat it as if nothing was read.
                //TODO: Should update last instant? maybe not in case seq number gets unsynced?
            }
        }
        world_view::update_wv(wv_watch_rx.clone(), wv).await;
    }
}


async fn send_acks(
    socket: &UdpSocket, 
    seq_num: u16, 
    addr: &SocketAddr, 
    redundancy: usize
) {
    for _ in 0..redundancy {
        let data = seq_num.to_le_bytes();
        let _ = socket.send_to(&data, addr).await;
    }
}



async fn send_udp_slave(
    socket: &UdpSocket,
    wv: &mut WorldView,
    wv_watch_rx: watch::Receiver<WorldView>,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
) {
    let mut seq = 0;
    let prev_master = wv.master_id;
    while wv.master_id != network::read_self_id() {
        world_view::update_wv(wv_watch_rx.clone(), wv).await;
        while send_udp(socket, wv, packetloss_rx.clone(), 50, seq, 10).await.is_err() {
            println!("Seq: {}", seq);
            if prev_master != wv.master_id {
                return;
            }
            sleep(config::SLAVE_TIMEOUT).await;
        }
        seq = seq.wrapping_add(1);
    }
}


async fn send_udp(
    socket: &UdpSocket,
    wv: &WorldView,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
    timeout_ms: u64,
    seq_num: u16,
    retries: u16,
)  -> std::io::Result<()> {

    // Må sikre at man er online
    // TODO: Send inn ferdig binda socket, den kan heller lages i slave_loopen!
    
    let server_addr: SocketAddr = format!("{}.{}:{}", config::NETWORK_PREFIX, wv.master_id, 50000).parse().unwrap();
    let mut buf = [0; 65535];
    
    
    let mut fails = 0;
    let mut backoff_timeout_ms = timeout_ms;
    while fails <= retries {
        let packetloss = packetloss_rx.borrow().clone();
        let redundancy = get_redundancy(packetloss.packet_loss);
        println!("Sending packet nr. {} with {} copies (estimated loss: {}%)", seq_num, redundancy, packetloss.packet_loss);
        send_packet(
            &socket, 
            seq_num, 
            &server_addr, 
            redundancy, 
            &wv
        ).await?;

    
        let timeout = sleep(Duration::from_millis(backoff_timeout_ms));
        // Add 10 ms timeout for each retransmission. 
        // In a real network: should probably be exponential.
        // In Sanntidslabben: Packetloss is software, slow ACKs is packetloss, not congestion or long travel links. 
        // The only reason this is added here is because the new script (which doesnt work) has an option for latency.
        backoff_timeout_ms += 10;
        tokio::select! {
            _ = timeout => {
                fails += 1;
                println!(
                    "Timeout (seq: {}, dest: {}). Retransmitting attempt {}/{}...",
                    seq_num, server_addr, fails, retries
                );
                if fails > retries {
                    return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("No Ack from master in {} retries!", retries)));
                }
                continue;
            },
            result = socket.recv_from(&mut buf) => {
                if let Ok((len, addr)) = result {
                    let seq_opt: Option<[u8; 2]> = buf[..len].try_into().ok();
                    if let Some(seq) = seq_opt {
                        if seq_num == u16::from_le_bytes(seq) {
                            println!("Master acked the cont");
                            return Ok(());
                        }
                    }
                    // Hvis pakken ikke var riktig ACK, fortsett til neste forsøk.
                }
            },
        }

    }
    return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("No Ack from master in {} retries!", retries)));
}


fn get_redundancy(packetloss: u8) -> usize {
    match packetloss {
        p if p < 25 => 4,
        p if p < 50 => 8,
        p if p < 75 => 16,
        p if p < 90 => 20,
        _ => 50,
    }
}


async fn send_packet(
    socket: &UdpSocket, 
    seq_num: u16, 
    addr: &SocketAddr, 
    redundancy: usize, 
    wv: &WorldView
) -> std::io::Result<()> {
    let data_opt = build_message(wv, &seq_num);
    if let Some(data) =  data_opt {
        for _ in 0..redundancy {
            let _ = socket.send_to(&data, addr).await;
        }
        return Ok(())
    } else {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to build UDP message to master"));
    }
}


fn build_message(
    wv: &WorldView,
    seq_num: &u16,
) -> Option<Vec<u8>> {
    let mut buf = Vec::new();

    let seq = seq_num.to_le_bytes();
    buf.extend_from_slice(&seq);

    
    let cont = world_view::extract_self_elevator_container(&wv)?;

    let ec_bytes = world_view::serialize(&cont);
    buf.extend_from_slice(&ec_bytes);

    Some(buf)
}

fn parse_message(
    buf: &[u8],
    expected_seq: u16,
) -> Option<ElevatorContainer> {
    if buf.len() < 2 {
        return None;
    }

    let key_part: [u8; 2] = buf[0..2].try_into().ok()?; // Konverter slice til array
    let key = u16::from_le_bytes(key_part);

    if key != expected_seq {
        return None; // Feil sekvensnummer
    }

    world_view::deserialize(&buf[2..])
}

