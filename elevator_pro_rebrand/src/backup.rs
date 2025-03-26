//! # ⚠️ NOT part of the final solution – Legacy backup module
//!
//! **This module is NOT part of the final distributed system solution.**
//!
//! It was originally developed as an **concept for local fault tolerance**, 
//! where a backup process would start automatically in a separate terminal if the main
//! elevator program crashed. This idea was **inspired by the fault tolerance mechanisms 
//! presented in the real-time lab exercises** in TTK4145.
//!
//! ## Intended Failover Behavior (Not Active in Final Design):
//! - To **automatically restart** the elevator program locally in case of crashes.
//! - To allow the elevator to **serve pending tasks while offline**, 
//!   even without reconnecting to the network.
//! - To eventually **rejoin the network** and synchronize with the system if a connection was restored.
//!
//! ## ❌ Why is it not part of our solution?
//! After discussions with course assistants and a better understanding of the assignment,
//! it became clear that:
//! - The project aims to implement **a distributed system**, not local persistence or replication.
//! - A local failover process like this is conceptually similar to **writing to a file and reloading**, 
//!   which is **explicitly not the intended direction** of the assignment.
//! - All call redundancy and recovery should happen through the **shared synchronized worldview**,
//!   not through isolated local state or takeover logic.
//!
//! As a result, the failover behavior was disabled (e.g., by using high takeover timeouts),
//! and this module now functions purely as a **GUI client**:
//! - Connects to the master
//! - Receives `WorldView` updates
//! - Visualizes elevator state and network status using a colorized print
//!
//! ## 📌 Summary:
//! - This is a **separate visualization tool**, _not part of the distributed control logic_.
//! - It remains in the codebase as a helpful debug utility, but should not be considered a part of the system design.
//! 
//! ## 🧠 Note:
//! In industrial applications, local crash recovery _might_ be useful,
//! especially to avoid reinitializing the elevator in a potentially unstable state.
//! For example, if a bug caused a crash, restarting **at the same point** could lead to
//! an immediate second crash. A clean backup process, starting with the previous tasks,
//! can offer a more controlled re-entry.
//!
//! However, this type of resilience mechanism falls outside the scope and intention
//! of this assignment, which emphasizes **distributed coordination and recovery**
//! via the networked `WorldView`, not local persistence or reboot logic.

use std::env;
use std::net::ToSocketAddrs;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{self, Write};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;
use tokio::time::{sleep, timeout};
use serde::{Serialize, Deserialize};
use crate::network::ConnectionStatus;
use crate::world_view::{ WorldView, serialize};
use crate::{config, init, network, world_view};
use crate::print;


/// Struct representing the data sent from the main process to the backup client.
///
/// It contains two components:
/// - `worldview`: The current `WorldView` of the system, used for visualization and potential local control.
/// - `network_status`: The latest known network status (internet and elevator mesh).
///
/// This payload is serialized and transmitted over TCP to keep the backup client synchronized
/// with the live system state.
/// 
#[derive(Serialize, Deserialize, Clone, Debug)]
struct BackupPayload {
    pub worldview: world_view::WorldView,
    pub network_status: ConnectionStatus,
}

/// Atomic flag to ensure that the backup terminal is only launched once.
///
/// Prevents spawning multiple backup clients simultaneously. Once set to `true`,
/// repeated calls to `start_backup_terminal()` will have no effec
static BACKUP_STARTED: AtomicBool = AtomicBool::new(false);

/// Creates a non-blocking TCP listener on the specified port, with address reuse enabled.
///
/// This helper sets up a low-level socket bound to `localhost:<port>`, configured
/// for asynchronous operation and reuse of the address.
///
/// # Parameters
/// - `port`: The TCP port number to bind to.
///
/// # Returns
/// A `TcpListener` ready for accepting incoming connections.
///
/// # Panics
/// This function will panic if:
/// - The address cannot be resolved.
/// - No valid IPv4 address is found.
/// - Socket creation or binding fails.
fn create_reusable_listener(
    port: u16
) -> TcpListener {
    let addr_str = format!("localhost:{}", port);
    let addr_iter = addr_str
        .to_socket_addrs()
        .expect("Klarte ikkje resolve 'localhost'");

    let addr = addr_iter
        .filter(|a| a.is_ipv4())
        .next()
        .expect("Fann ingen IPv4-adresse for localhost");
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .expect("Couldnt create socket");
    socket.set_nonblocking(true)
        .expect("Couldnt set non blocking");
    socket.set_reuse_address(true)
        .expect("Couldnt set reuse_address");
    socket.bind(&addr.into())
        .expect("Couldnt bind the socket");
    socket.listen(128)
        .expect("Couldnt listen on the socket");
    TcpListener::from_std(socket.into())
        .expect("Couldnt create TcpListener")
}

/// Launches a new terminal window and starts the program in backup mode.
///
/// Uses the current binary path and appends the `backup` argument, causing
/// the program to run as a backup client.
///
/// This function checks the `BACKUP_STARTED` flag to ensure only one
/// backup process is started.
///
/// # Notes
/// - Only supported on Unix-like systems using `gnome-terminal`.
/// - Has no effect if backup is already running.
fn start_backup_terminal() {
    if !BACKUP_STARTED.load(Ordering::SeqCst) {
        let current_exe = env::current_exe().expect("Couldnt extract the executable");
        let _child = Command::new("gnome-terminal")
            .arg("--geometry=400x24")
            .arg("--")
            .arg(current_exe.to_str().unwrap())
            .arg("backup")
            .spawn()
            .expect("Feil ved å starte backupterminalen");
        BACKUP_STARTED.store(true, Ordering::SeqCst);
    }
}

/// Continuously sends serialized `BackupPayload` updates to a connected backup client.
///
/// This task runs on the backup-server side. It listens on a watch channel
/// for updated payloads and transmits them to the connected client over TCP.
///
/// # Parameters
/// - `stream`: The TCP connection to the backup client.
/// - `rx`: A `watch::Receiver` for updated `BackupPayload` values.
///
/// # Behavior
/// - If sending fails, a warning is printed and the backup terminal is relaunched after delay.
/// - The loop exits after failure, assuming a new client will reconnect.
async fn handle_backup_client(
    mut stream: TcpStream, 
    rx: watch::Receiver<BackupPayload>
) {
    loop {
        let payload = rx.borrow().clone();
        let serialized = serialize(&payload);

        if let Err(e) = stream.write_all(&serialized).await {
            print::err(format!("Backup send error: {}", e));
            print::warn(format!("Prøver igjen om {:?}", config::BACKUP_TIMEOUT));
            sleep(config::BACKUP_TIMEOUT).await;
            BACKUP_STARTED.store(false, Ordering::SeqCst);
            start_backup_terminal();
            break;
        }

        sleep(config::BACKUP_SEND_INTERVAL).await;
    }
}


/// Starts the backup server, listening for incoming backup clients and
/// transmitting the current system state (`WorldView`) and network status.
///
/// # Parameters
/// - `wv_watch_rx`: Watch receiver for current `WorldView`.
/// - `network_watch_rx`: Watch receiver for current `ConnectionStatus`.
///
/// # Behavior
/// - Spawns a TCP listener to accept connections from a backup client.
/// - On connection, launches a handler that sends periodic `BackupPayload` updates.
/// - Spawns a task to continuously refresh the payload with the latest worldview and network status.
///
/// # Notes
/// - This function is blocking and must be run as an asynchronous task.
/// - It starts the backup terminal once at initialization.
/// - Failures to send payloads are printed but do not crash the server.
pub async fn start_backup_server(
    wv_watch_rx: watch::Receiver<WorldView>,
    network_watch_rx: watch::Receiver<network::ConnectionStatus>,
) {
    println!("Backup-server starting...");
    
    let listener = create_reusable_listener(config::BCU_PORT);
    let wv = world_view::get_wv(wv_watch_rx.clone());
    let initial_payload = BackupPayload {
        worldview: wv.clone(),
        network_status: ConnectionStatus::new(),
    };
    let (tx, rx) = watch::channel(initial_payload);
    
    
    start_backup_terminal();
    
    // Task to handle the backup.
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener
                .accept()
                .await
                .expect("Failed to accept backup-connection");
            handle_backup_client(socket, rx.clone()).await;
        }
    });
    
    // Task for å oppdatere world view
    let tx_clone = tx.clone();
    let wv_rx_clone = wv_watch_rx.clone();

    tokio::spawn(async move {
        loop {
            let new_wv = world_view::get_wv(wv_rx_clone.clone());
            let status = network_watch_rx.borrow().clone();

            let payload = BackupPayload {
                worldview: new_wv,
                network_status: status,
            };

            if tx_clone.send(payload).is_err() {
                println!("Klarte ikkje sende payload til backup-klient");
            }

            sleep(config::BACKUP_WORLDVIEW_REFRESH_INTERVAL).await;
        }
    });
}

/// Entry point for the backup program (invoked with `cargo run -- backup`).
///
/// Connects to the main process and listens for serialized `BackupPayload`
/// updates over TCP. Displays the current worldview and network status in the terminal.
///
/// # Behavior
/// - Continuously tries to connect to the main process until success or timeout.
/// - Deserializes incoming data and prints system state via `print::worldview`.
/// - If the main process connection fails repeatedly beyond the threshold,
///   the backup promotes itself and returns its local elevator container.
///
/// # Returns
/// - `Some(ElevatorContainer)` if failover is triggered and the backup should take over.
/// - `None` if failover failed or not applicable.
///
/// # Notes
/// In the current solution, this failover logic is disabled using a high timeout.
/// The function is now used solely as a live GUI for displaying the system state.
pub async fn run_as_backup() -> Option<world_view::ElevatorContainer> {
    println!("Starting backup-client...");
    let mut current_wv = init::initialize_worldview(None).await;
    let mut retries = 0;
    
    loop {
        match timeout(
            config::MASTER_TIMEOUT,
            TcpStream::connect(format!("localhost:{}", config::BCU_PORT))
        ).await {
            Ok(Ok(mut stream)) => {
                retries = 0;
                let mut buf = vec![0u8; 1024];

                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) => {
                            eprintln!("Master connection has ended.");
                            break;
                        },
                        Ok(n) => {
                            let raw = &buf[..n];
                            let payload: Option<BackupPayload> = bincode::deserialize(raw).ok();

                            if let Some(payload) = payload {
                                current_wv = payload.worldview;
                                let status = payload.network_status;

                                print!("\x1B[2J\x1B[H");
                                io::stdout().flush().unwrap();

                                print::worldview(&current_wv, Some(status));
                            } else {
                                println!("Klarte ikkje deserialisere payload.");
                                continue;
                            }
                        },
                        Err(e) => {
                            eprintln!("Error while reading from master: {}", e);
                            break;
                        }
                    }
                }

            },
            _ => {
                retries += 1;
                eprintln!("Failed to connect to master, retry {}.", retries);
                if retries > config::BACKUP_FAILOVER_THRESHOLD {
                    eprintln!("Master failed, promoting backup to master!");
                    // Her kan failover-logikken setjast i gang, t.d. køyre master-logikken.
                    match world_view::extract_self_elevator_container(&current_wv).to_owned() {
                        Some(container) => return Some(container.to_owned()),
                        None => {
                            print::warn(format!("Failed to extract self elevator container"));
                            return None;
                        }
                    }
                    
                }
            }
        }
        sleep(config::BACKUP_RETRY_DELAY).await;
    }
}
