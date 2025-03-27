use crate::config;
use crate::ip_help_functions;
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
use once_cell::sync::Lazy;


const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(5); // Tidsgrense for inaktivitet
const CLEANUP_INTERVAL: Duration = Duration::from_secs(1); // Hvor ofte inaktive sendere fjernes

pub static IS_MASTER: AtomicBool = AtomicBool::new(false);

/// Initializes and manages a direct UDP network "connection" for communication between master and slave nodes.
/// 
/// This function sets up the UDP socket, configures necessary network parameters, and starts listening and sending
/// UDP packets for communication. It handles both sending/receiving data from the master and sending/recieving data from slaves, based on the systems current role.
/// 
/// # Arguments
/// - `wv_watch_rx` - Receiver for world view updates.
/// - `container_tx` - Channel for sending received elevator containers to other parts of the system.
/// - `packetloss_rx` - Receiver for tracking packet loss information.
/// - `connection_to_master_failed_tx` - Sender to notify if the connection to the master failed.
/// - `remove_container_tx` - Channel to notify when a slave becomes inactive.
/// - `sent_container_tx` - Channel to notify worldview updater about what data has been sent and acked by the master.
/// 
/// # Behaviour
/// - Initializes a non-blocking UDP socket and configures its parameters.
/// - Continuously listens for incoming UDP messages from slaves and processes them while the system is the network master.
/// - Sends periodic UDP packets to the master with the current state of the local elevator while the system is the network slave.
/// 
/// # Notes
/// - The function blocks until the network is ready and the socket is successfully configured.
/// - After socket setup, it enters a loop where it listens and sends UDP packets for slave-master communication.
/// - The loop continues indefinitely, processing messages and sending responses as needed.
pub async fn start_direct_udp_network(
    wv_watch_rx: watch::Receiver<WorldView>,
    container_tx: mpsc::Sender<ElevatorContainer>,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
    connection_to_master_failed_tx: mpsc::Sender<bool>,
    remove_container_tx: mpsc::Sender<u8>,
    sent_container_tx: mpsc::Sender<ElevatorContainer>,
) 
{
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

    while socket.set_nonblocking(true).is_err() {}
    
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
            remove_container_tx.clone(),
        ).await;
        
        IS_MASTER.store(false, Ordering::SeqCst);
        send_udp_slave(
            &socket,
            &mut wv,
            wv_watch_rx.clone(),
            packetloss_rx.clone(),  
            connection_to_master_failed_tx.clone(),
            sent_container_tx.clone(),
        ).await;
    }
}



/// Holder informasjon om en aktiv sender
#[derive(Debug, Clone)]
struct ReceiverState {
    last_seq: u16,
    last_seen: Instant,
}

/// Listens for incoming UDP messages from slave nodes and processes them accordingly.
/// 
/// # Arguments
/// - `socket` - The UDP socket used for communication.
/// - `wv` - Mutable reference to the world view state.
/// - `wv_watch_rx` - A [watch] receiver for world view updates.
/// - `container_tx` - Channel for sending received elevator containers.
/// - `packetloss_rx` - A [watch] receiver for tracking packet loss.
/// - `remove_container_tx` - [mpsc] sender for notifying when a slave becomes inactive.
/// 
/// # Behaviour
/// - If a message is received with the expected sequence number, it is processed and acknowledged.
/// - If a message is out of order or corrupted, it is ignored.
/// - Inactive slaves are periodically detected and removed.
/// - The function runs continuously while the local node is the master.
/// 
/// # Notes
/// - The function relies on `parse_message` to extract data and determine the appropriate response.
/// - `monitor_slave_activity` is spawned as a separate task to handle inactive slave removal.
/// - If packet loss is high, the redundancy factor for ACK messages increases.
/// - This function should be run inside a Tokio task to prevent blocking.
async fn receive_udp_master(
    socket: &UdpSocket,
    wv: &mut WorldView,
    wv_watch_rx: watch::Receiver<WorldView>,
    container_tx: mpsc::Sender<ElevatorContainer>,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
    remove_container_tx: mpsc::Sender<u8>,
) {    
    world_view::update_wv(wv_watch_rx.clone(), wv).await;
    println!("Server listening on port {}", 50000);

    let state = Arc::new(Mutex::new(HashMap::<SocketAddr, ReceiverState>::new()));
    
    // Cleanup-task: Fjerner inaktive klienter
    let state_cleanup = state.clone();
    {
        let wv_watch_rx = wv_watch_rx.clone();
        let wv = wv.clone();
        monitor_slave_activity(
            wv_watch_rx,
            wv,
            state_cleanup,
            remove_container_tx,
        ).await;
    }

    let mut buf = [0; 65535];
    while wv.master_id == network::read_self_id() {
        // println!("min id: {}, master ID: {}", network::read_self_id(), wv.master_id);
        // Mottar data
        let (len, slave_addr) = match socket.try_recv_from(&mut buf) {
            Ok(res) => res,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Egen case for når bufferet er tomt
                sleep(config::POLL_PERIOD).await;
                world_view::update_wv(wv_watch_rx.clone(), wv).await;
                continue;
            }
            Err(e) => {
                eprintln!("Error receiving UDP packet: {}", e);
                world_view::update_wv(wv_watch_rx.clone(), wv).await;
                continue;
            }
        };

        let mut new_state = ReceiverState {
            last_seq: 0,
            last_seen: Instant::now(),
        };

        let mut state_locked = state.lock().await;
        let entry = state_locked.entry(slave_addr).or_insert(new_state.clone());
        let last_seen = entry.last_seen;
        let last_seq = entry.last_seq.clone();
        
        let msg = parse_message(&buf[..len], last_seq);
        
        match msg {
            (Some(container), code) => {
                match code {
                    RecieveCode::Accept | RecieveCode::Rejoin=> {
                        let _ = container_tx.send(container.clone()).await;
                        new_state.last_seq = last_seq.wrapping_add(1);
                        if code == RecieveCode::Rejoin {
                            new_state.last_seq = 0;
                        }
                        new_state.last_seen = Instant::now();
                        state_locked.insert(slave_addr, new_state);
                        
                    },
                    RecieveCode::AckOnly => {},
                    RecieveCode::Ignore => {},
                }

                if code != RecieveCode::Ignore {
                    let packetloss = packetloss_rx.borrow().clone();
                        let redundancy = get_redundancy(packetloss.packet_loss, last_seen).await;
                        print::warn(format!(
                            "Sending {} ACKs to {} (loss: {}%, time since last: {:.2}s)",
                            redundancy,
                            slave_addr,
                            packetloss.packet_loss,
                            Instant::now().duration_since(last_seen).as_secs_f64()
                        ));
                        send_acks(
                            &socket,
                            last_seq,
                            &slave_addr,
                            redundancy
                        ).await;
                }
            },
            (None, _) => {
                // println!("Ignoring out-of-order packet from {}", slave_addr);
                // Seq nummer doesnt match, or data has been corrupted.
                // Treat it as if nothing was read.
                //TODO: Should update last instant? maybe not in case seq number gets unsynced?
            }
        }
        world_view::update_wv(wv_watch_rx.clone(), wv).await;
    }
}

/// Periodically monitors slave (client) activity and removes inactive ones based on a timeout as long as the current node is master on the system.
///
/// This function runs as an asynchronous task and checks if any slave has been inactive
/// for longer than `INACTIVITY_TIMEOUT`. If so, it removes them from the `state_cleanup` map
/// and notifies the worldview updater.
///
/// # Arguments
/// * `wv_watch_rx` - A [watch] reciever to observe worldview updates.
/// * `wv` - A mutable [`WorldView`] struct.
/// * `state_cleanup` - A shared [HashMap] tracking the last known state of each slave,
///   protected by a [Mutex] for concurrent access.
/// * `remove_container_tx` - An [mpsc] sender used to notify the worldview updater
///   about removed slaves.
///
/// # Behavior
/// - Runs in a loop while the node is the master.
/// - Sleeps for [`CLEANUP_INTERVAL`] between iterations.
/// - Checks the `state_cleanup` map and removes entries that exceed [`INACTIVITY_TIMEOUT`].
/// - Sends the IDs of removed slaves to `remove_container_tx`.
/// - Updates the worldview to the latest.
///
/// This function is essential for maintaining an up-to-date list of active nodes in the system.
async fn monitor_slave_activity(
    wv_watch_rx: watch::Receiver<WorldView>,
    mut wv: WorldView,
    state_cleanup:  Arc<Mutex<HashMap<SocketAddr, ReceiverState>>>,
    remove_container_tx: mpsc::Sender<u8>,
)
{
    tokio::spawn(async move {
        while wv.master_id == network::read_self_id() {
            sleep(CLEANUP_INTERVAL).await;
            {
                let mut state = state_cleanup.lock().await;
                let now = Instant::now();

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
    connection_to_master_failed_tx: mpsc::Sender<bool>,
    sent_tcp_container_tx: mpsc::Sender<ElevatorContainer>,
) {
    world_view::update_wv(wv_watch_rx.clone(), wv).await;
    let mut seq = 0;
    while wv.master_id != network::read_self_id() {
        world_view::update_wv(wv_watch_rx.clone(), wv).await;
        while send_udp(socket, wv, packetloss_rx.clone(), 50, seq, 20, sent_tcp_container_tx.clone()).await.is_err() {
            let _ = connection_to_master_failed_tx.send(true).await;
            sleep(config::SLAVE_TIMEOUT).await;
            world_view::update_wv(wv_watch_rx.clone(), wv).await;
            return;
        }
        seq = seq.wrapping_add(1);
        sleep(config::SLAVE_TIMEOUT).await;
    }
}


async fn send_udp(
    socket: &UdpSocket,
    wv: &WorldView,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
    timeout_ms: u64,
    seq_num: u16,
    retries: u16,
    sent_tcp_container_tx: mpsc::Sender<ElevatorContainer>,
)  -> std::io::Result<()> {

    // Må sikre at man er online
    // TODO: Send inn ferdig binda socket, den kan heller lages i slave_loopen!
    
    let server_addr: SocketAddr = format!("{}.{}:{}", config::NETWORK_PREFIX, wv.master_id, 50000).parse().unwrap();
    let mut buf = [0; 65535];
    
    let mut last_seen_from_master = Instant::now();

    let mut fails = 0;
    let mut backoff_timeout_ms = timeout_ms;

    let mut should_send: bool = true;
    let sent_cont = match world_view::extract_self_elevator_container(wv) {
        Some(cont) => cont.clone(),
        None => {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Self container not found in worldview"))
        }
    };

    loop 
    {
        if should_send 
        {
            let packetloss = packetloss_rx.borrow().clone();
            let redundancy = get_redundancy(packetloss.packet_loss, last_seen_from_master).await;
            // println!(
            //     "Sending packet {} with redundancy {} (loss: {}%, time since last ACK: {:.2}s)",
            //     seq_num,
            //     redundancy,
            //     packetloss.packet_loss,
            //     Instant::now().duration_since(last_seen_from_master).as_secs_f64()
            // );
            println!("Sending with redundancy: {}", redundancy);
            // println!("Sending packet nr. {} with {} copies (estimated loss: {}%)", seq_num, redundancy, packetloss.packet_loss);
            send_packet(
                &socket, 
                seq_num, 
                &server_addr, 
                redundancy, 
                &wv
            ).await?;
            backoff_timeout_ms += 5;
            should_send = false;
        }

        let timeout = sleep(Duration::from_millis(backoff_timeout_ms));
    
        // Add 10 ms timeout for each retransmission. 
        // In a real network: should probably be exponential.
        // In Sanntidslabben: Packetloss is software, slow ACKs is packetloss, not congestion or long travel links. 
        // The only reason this is added here is because the new script (which doesnt work) has an option for latency.
        // backoff_timeout_ms += 5;
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
                should_send = true;
            },
            result = socket.recv_from(&mut buf) => {
                if let Ok((len, addr)) = result {
                    let seq_opt: Option<[u8; 2]> = buf[..len].try_into().ok();
                    if let Some(seq) = seq_opt {
                        if seq_num == u16::from_le_bytes(seq) {
                            last_seen_from_master = Instant::now();
                            let _ = sent_container_tx.send(sent_cont).await;
                            return Ok(())
                        }
                    }
                    // Hvis pakken ikke var riktig ACK, fortsett til neste forsøk.
                }
            },
        }

    }
}


/// Sends a UDP packet to the specified address with redundancy.
/// 
/// This function constructs a message based on the `WorldView` and transmits it multiple times  
/// to improve reliability in high packet loss environments.
/// 
/// # Arguments
/// * `socket` - A reference to the `UdpSocket` used for sending data.
/// * `seq_num` - The sequence number of the packet, used for tracking.
/// * `addr` - The destination `SocketAddr` (typically the master node).
/// * `redundancy` - The number of times the packet should be sent for reliability.
/// * `wv` - A reference to the `WorldView`, containing system state and data to be transmitted.
/// 
/// # Behavior
/// - Calls `build_message()` to construct the UDP packet payload.
/// - If message construction succeeds, it sends the packet `redundancy` times.
/// - If message construction fails, it returns an error.
/// - Ignores send errors (e.g., packet drops) and continues sending the remaining redundant packets.
/// 
/// # Notes
/// - The redundancy factor should be chosen based on network conditions.
/// - This function does not wait for an acknowledgment; it only transmits packets.
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


/// Builds a UDP message containing the sequence number and serialized elevator container.
/// 
/// Returns `None` if extracting the elevator container fails.
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

/// Parses a received UDP message and determines its validity.
/// 
/// Returns an `ElevatorContainer` if valid, along with a `RecieveCode` indicating the action to take.
fn parse_message(
    buf: &[u8],
    expected_seq: u16,
) -> (Option<ElevatorContainer>, RecieveCode) {
    if buf.len() < 2 {
        return (None, RecieveCode::Ignore);
    }


    let seq: [u8; 2] = match buf[0..2].try_into().ok() {
        Some(number) => number,
        None => return (None, RecieveCode::Ignore),
    };
    let key = u16::from_le_bytes(seq);

    if key == expected_seq {
        return (world_view::deserialize(&buf[2..]), RecieveCode::Accept);
    } else if key == 0 && expected_seq != 1 {
        return (world_view::deserialize(&buf[2..]), RecieveCode::Rejoin);
    } else if key == expected_seq.wrapping_rem(1) {
        return (world_view::deserialize(&buf[2..]), RecieveCode::AckOnly);
    } else {
        return (None, RecieveCode::Ignore);
    }
}


#[derive(Debug, Clone, PartialEq)]
enum RecieveCode {
    Accept,
    AckOnly,
    Ignore,
    Rejoin
}


struct PID {
    kp: f64,
    ki: f64,
    kd: f64,
    prev_error: f64,
    integral: f64,
    last_time: Option<Instant>,
}

impl PID {
    fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            prev_error: 0.0,
            integral: 0.0,
            last_time: None,
        }
    }

    fn update(&mut self, setpoint: f64, measurement: f64, now: Instant) -> f64 {
        let error = -(setpoint - measurement);
        let dt = self.last_time.map_or(0.1, |last| {
            let secs = now.duration_since(last).as_secs_f64();
            if secs < 0.001 { 0.001 } else { secs }
        });

        self.integral += clamp(error * dt, -20.0, 20.0);
        let derivative = (error - self.prev_error) / dt;
        self.prev_error = error;
        self.last_time = Some(now);

        self.kp * error + self.ki * self.integral + self.kd * derivative
    }
}

use once_cell::sync::Lazy;

static REDUNDANCY_PID: Lazy<Mutex<PID>> = Lazy::new(|| {
    Mutex::new(PID::new(60.0, 14.05, 1.01)) // Tuning-verdiar: test gjerne!
});

fn clamp(val: f64, min: f64, max: f64) -> f64 {
    val.max(min).min(max)
}

pub async fn get_redundancy(packetloss: u8, last_seen: Instant) -> usize {
    let now = Instant::now();
    let time_since_last = now.duration_since(last_seen).as_secs_f64(); // i sekund

    let setpoint = 0.1; // 10 ms ønsket tid mellom mottak
    let measurement = time_since_last;

    let output = {
        let mut pid = REDUNDANCY_PID.lock().await;
        pid.update(setpoint, measurement, now)
    };

    let base = 1.0;
    let redundans = clamp((base + output)*(packetloss as f64+1.0)/100.0, 1.0, 300.0);

    // println!(
    //     "[PID] Last seen: {:.3}s | Error: {:.3} | Redundancy: {:.1}",
    //     time_since_last,
    //     setpoint - measurement,
    //     redundans
    // );

    redundans.round() as usize
}