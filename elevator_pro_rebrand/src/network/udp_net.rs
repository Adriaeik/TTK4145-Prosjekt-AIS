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
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;


const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(5); // Tidsgrense for inaktivitet
const CLEANUP_INTERVAL: Duration = Duration::from_secs(1); // Hvor ofte inaktive sendere fjernes


pub async fn start_direct_udp_network(
    wv_watch_rx: watch::Receiver<WorldView>,
    container_tx: mpsc::Sender<ElevatorContainer>,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
    connection_to_master_failed_tx: mpsc::Sender<bool>,
    remove_container_tx: mpsc::Sender<u8>,
    sent_tcp_container_tx: mpsc::Sender<ElevatorContainer>,
) 
{
    while !network::read_network_status() {}
    let socket = match Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)) {
        Ok(sock) => sock,
        Err(e) => {panic!("Failed to create socket: {}", e)},
    }; 
    while socket.set_reuse_address(true).is_err() {}
    while socket.set_send_buffer_size(16_000_000).is_err() {}
    while socket.set_recv_buffer_size(16_000_000).is_err() {}
    
    let addr: SocketAddr = format!("{}.{}:{}", config::NETWORK_PREFIX, network::read_self_id(), 50000).parse().unwrap();

    while socket.bind(&addr.into()).is_err() {}

    while socket.set_nonblocking(true).is_err() {}
    
    let socket = match UdpSocket::from_std(socket.into()) {
        Ok(sock) => sock,
        Err(e) => {panic!("Failed to convert Socket to tokio: {}", e)},
    }; 


    let mut wv = world_view::get_wv(wv_watch_rx.clone());
    loop {
        receive_udp_master(
            &socket,
            &mut wv,
            wv_watch_rx.clone(),
            container_tx.clone(),
            packetloss_rx.clone(),
            remove_container_tx.clone(),
        ).await;
        
        send_udp_slave(
            &socket,
            &mut wv,
            wv_watch_rx.clone(),
            packetloss_rx.clone(),  
            connection_to_master_failed_tx.clone(),
            sent_tcp_container_tx.clone(),
        ).await;
    }
}



/// Holder informasjon om en aktiv sender
#[derive(Debug, Clone)]
struct ReceiverState 
{
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
    remove_container_tx: mpsc::Sender<u8>,
) 
{    
    world_view::update_wv(wv_watch_rx.clone(), wv).await;
    println!("Server listening on port {}", 50000);

    let state = Arc::new(Mutex::new(HashMap::<SocketAddr, ReceiverState>::new()));
    
    {
        // Start task detecting inctive slaves
        let state_cleanup = state.clone();
        let wv_watch_rx = wv_watch_rx.clone();
        let mut wv = wv.clone();
        monitor_slave_activity(
            wv_watch_rx,
            &mut wv,
            state_cleanup,
            remove_container_tx,
        );
    }

    let mut buf = [0; 65535];
    while wv.master_id == network::read_self_id() {
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
                // println!("Received valid packet from {}: seq {}", slave_addr, last_seq);
                //Meldinga er en forventet melding -> oppdater hashmappets state
                // println!("Ack? {:?}", code);
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
                    RecieveCode::AckOnly | RecieveCode::Ignore => {},
                }

                if code != RecieveCode::Ignore {
                    let packetloss = packetloss_rx.borrow().clone();
                    let redundancy = get_redundancy(packetloss.packet_loss, last_seen).await;
                    send_acks(
                        &socket,
                        last_seq,
                        &slave_addr,
                        redundancy
                    ).await;
                }
            },
            (None, _) => {
                // Seq nummer doesnt match, or data has been corrupted.
                // Treat it as if nothing was read.
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
/// * `wv` - A mutable reference to a [`WorldView`] struct.
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
    wv: &mut WorldView,
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

                //Remove inactive slaves, save SocketAddr to the removed ones
                let mut removed = Vec::new();
                state.retain(|k, s| 
                    {
                        let keep = now.duration_since(s.last_seen) < INACTIVITY_TIMEOUT;
                        if !keep 
                        {
                            removed.push(*k);
                        }
                        keep
                    }
                );

                for addr in removed 
                {
                    let _ = remove_container_tx.send(ip_help_functions::ip2id(addr.ip())).await;
                }
            }
            world_view::update_wv(wv_watch_rx.clone(), &mut wv).await;
        }
    });
}

/// This functions acks `seq_num` `redundancy` times to `addr` on `socket`
async fn send_acks(
    socket: &UdpSocket, 
    seq_num: u16, 
    addr: &SocketAddr, 
    redundancy: usize
) 
{
    for _ in 0..redundancy 
    {
        let data = seq_num.to_le_bytes();
        let _ = socket.send_to(&data, addr).await;
    }
}


/// Sends UDP packets from a slave node to the master and handles connection failures.
/// 
/// This function continuously updates the `WorldView` and transmits UDP packets while the node is a slave.  
/// If sending fails, it signals a connection failure.
/// 
/// # Arguments
/// * `socket` - A reference to the `UdpSocket` used for communication.
/// * `wv` - A mutable reference to [`WorldView`].
/// * `wv_watch_rx` - A [watch] reciever to receive worldview updates.
/// * `packetloss_rx` - A [watch] receiver to monitor packet loss conditions.
/// * `connection_to_master_failed_tx` - A [mpsc] sender used to signal a failed connection.
/// * `sent_container_tx` - A [mpsc] sender to send data for fallback TCP transmission.
/// 
/// # Behavior
/// - Updates the worldview before and after sending data.
/// - Repeatedly attempts to send UDP packets using [`send_udp()`].
/// - If sending fails, signals failure via `connection_to_master_failed_tx` and retries after [`config::SLAVE_TIMEOUT`].
/// - Uses a sequence number (`seq`) for packet tracking, which wraps around on overflow.
/// 
/// # Notes
/// - This function should run in an async task.
/// - Ensures robustness by detecting connection issues and handling packet loss.
/// - Exits when the node becomes the master.
async fn send_udp_slave(
    socket: &UdpSocket,
    wv: &mut WorldView,
    wv_watch_rx: watch::Receiver<WorldView>,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
    connection_to_master_failed_tx: mpsc::Sender<bool>,
    sent_container_tx: mpsc::Sender<ElevatorContainer>,
) 
{
    world_view::update_wv(wv_watch_rx.clone(), wv).await;
    let mut seq = 0;
    while wv.master_id != network::read_self_id() {
        world_view::update_wv(wv_watch_rx.clone(), wv).await;
        while send_udp(socket, wv, packetloss_rx.clone(), 50, seq, 20, sent_container_tx.clone()).await.is_err() {
            let _ = connection_to_master_failed_tx.send(true).await;
            sleep(config::SLAVE_TIMEOUT).await;
            world_view::update_wv(wv_watch_rx.clone(), wv).await;
            return;
        }
        seq = seq.wrapping_add(1);
        sleep(config::SLAVE_TIMEOUT).await;
    }
}


/// Sends a UDP packet to the master and waits for an acknowledgment.
/// 
/// This function transmits a UDP packet with redundancy based on packet loss conditions  
/// and implements a retry mechanism if an acknowledgment (ACK) is not received within a timeout.
/// 
/// # Arguments
/// 
/// * `socket` - A reference to the `UdpSocket` used for communication.
/// * `wv` - A reference to the `WorldView`, containing network and system state.
/// * `packetloss_rx` - A `watch::Receiver<network::ConnectionStatus>` to monitor packet loss.
/// * `timeout_ms` - The initial timeout in milliseconds before resending the packet.
/// * `seq_num` - The sequence number assigned to the packet for tracking.
/// * `retries` - The maximum number of retry attempts before failing.
/// * `sent_container_tx` - An `mpsc::Sender<ElevatorContainer>` to send successfully transmitted data.
/// 
/// # Behavior
/// 
/// - Determines the master's address based on `wv.master_id`.
/// - Extracts the slave's elevator container from `WorldView`.
/// - Sends the packet with redundancy based on current packet loss conditions.
/// - Implements a linear backoff strategy, increasing timeout after each failure.
/// - Listens for an acknowledgment from the master.
/// - If the correct ACK is received, it updates `last_seen_from_master` and sends data to `sent_container_tx`.
/// - If no ACK is received after `retries` attempts, it returns a timeout error.
/// 
/// # Notes
/// 
/// - Uses `tokio::select!` to wait for either an ACK or a timeout.
/// - The backoff timeout increases by 5ms on each failure.
/// - This function should be called within an async runtime.
async fn send_udp(
    socket: &UdpSocket,
    wv: &WorldView,
    packetloss_rx: watch::Receiver<network::ConnectionStatus>,
    timeout_ms: u64,
    seq_num: u16,
    retries: u16,
    sent_container_tx: mpsc::Sender<ElevatorContainer>,
)  -> std::io::Result<()> 
{

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
    let mut timeout = sleep(Duration::from_millis(backoff_timeout_ms));
    loop 
    {
        if should_send 
        {
            let packetloss = packetloss_rx.borrow().clone();
            let redundancy = get_redundancy(packetloss.packet_loss, last_seen_from_master).await;
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

        timeout = sleep(Duration::from_millis(backoff_timeout_ms));
    
        tokio::select! 
        {
            _ = timeout => {
                fails += 1;
                if fails > retries 
                {
                    return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("No Ack from master in {} retries!", retries)));
                }
                should_send = true;
            },
            result = socket.recv_from(&mut buf) => 
            {
                if let Ok((len, _addr)) = result 
                {
                    let seq_opt: Option<[u8; 2]> = buf[..len].try_into().ok();
                    if let Some(seq) = seq_opt 
                    {
                        if seq_num == u16::from_le_bytes(seq) 
                        {
                            last_seen_from_master = Instant::now();
                            sent_container_tx.send(sent_cont).await?;
                        }
                    }
                    // Hvis pakken ikke var riktig ACK, fortsett til neste forsøk.
                }
            },
        }

    }
}



async fn send_packet(
    socket: &UdpSocket, 
    seq_num: u16, 
    addr: &SocketAddr, 
    redundancy: usize, 
    wv: &WorldView
) -> std::io::Result<()> 
{
    let data_opt = build_message(wv, &seq_num);
    if let Some(data) =  data_opt 
    {
        for _ in 0..redundancy 
        {
            let _ = socket.send_to(&data, addr).await;
        }
        return Ok(())
    } else 
    {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to build UDP message to master"));
    }
}


fn build_message(
    wv: &WorldView,
    seq_num: &u16,
) -> Option<Vec<u8>> 
{
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
) -> (Option<ElevatorContainer>, RecieveCode) 
{
    if buf.len() < 2 
    {
        return (None, RecieveCode::Ignore);
    }

    let seq: [u8; 2] = match buf[0..2].try_into().ok() 
    {
        Some(number) => number,
        None => return (None, RecieveCode::Ignore),
    };
    let key = u16::from_le_bytes(seq);

    if key == expected_seq {
        return (world_view::deserialize(&buf[2..]), RecieveCode::Accept);
    } else if key == 0 && expected_seq != 0 {
        return (world_view::deserialize(&buf[2..]), RecieveCode::Rejoin);
    } else if key == expected_seq.wrapping_rem(1) {
        return (world_view::deserialize(&buf[2..]), RecieveCode::AckOnly);
    } else {
        return (None, RecieveCode::Ignore);
    }
}


#[derive(Debug, Clone, PartialEq)]
enum RecieveCode 
{
    Accept,
    AckOnly,
    Ignore,
    Rejoin
}


struct PID 
{
    kp: f64,
    ki: f64,
    kd: f64,
    prev_error: f64,
    integral: f64,
    last_time: Option<Instant>,
}

impl PID 
{
    fn new(kp: f64, ki: f64, kd: f64) -> Self 
    {
        Self 
        {
            kp,
            ki,
            kd,
            prev_error: 0.0,
            integral: 0.0,
            last_time: None,
        }
    }

    fn update(&mut self, setpoint: f64, measurement: f64, now: Instant) -> f64 
    {
        let error = -(setpoint - measurement);
        let dt = self.last_time
            .map_or(0.1, |last| 
            {
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



static REDUNDANCY_PID: Lazy<Mutex<PID>> = Lazy::new(|| {
    Mutex::new(PID::new(60.0, 14.05, 1.01))
});

fn clamp(val: f64, min: f64, max: f64) -> f64 
{
    val.max(min).min(max)
}

async fn get_redundancy(packetloss: u8, last_seen: Instant) -> usize 
{
    let now = Instant::now();
    let time_since_last = now.duration_since(last_seen).as_secs_f64(); // i sekund

    let setpoint = 0.1; // 10 ms ønsket tid mellom mottak
    let measurement = time_since_last;

    let output = 
    {
        let mut pid = REDUNDANCY_PID.lock().await;
        pid.update(setpoint, measurement, now)
    };

    let base = 1.0;
    let redundans = clamp((base + output)*(packetloss as f64+1.0)/100.0, 1.0, 300.0);

    redundans.round() as usize
}