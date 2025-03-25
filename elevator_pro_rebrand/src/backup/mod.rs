use std::env;
use std::net::ToSocketAddrs;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{self, Write};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;
use tokio::time::{sleep, Duration, timeout};
use serde::{Serialize, Deserialize};
use crate::network::ConnectionStatus;
use crate::world_view::{ElevatorContainer, WorldView, serialize};
use crate::{config, init, network, world_view};
use crate::print;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BackupPayload {
    pub worldview: world_view::WorldView,
    pub network_status: ConnectionStatus,
}
// Static variable to see if backup has started
static BACKUP_STARTED: AtomicBool = AtomicBool::new(false);

/// Creates a non blocking TCP listener with reusable address
fn create_reusable_listener(
    port: u16
) -> TcpListener {
    let addr_str = format!("localhost:{}", port);
    // Resolve alle mulige adresser til "localhost"
    let addr_iter = addr_str
        .to_socket_addrs()
        .expect("Klarte ikkje resolve 'localhost'");

    // Prøv første IPv4-adresse
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

/// Startes the program with backup argument in a new terminal if the backup is not running.
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

/// Sender serialisert `BackupPayload` kontinuerleg til backup-klient
pub async fn handle_backup_client(
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


/// Function to start and maintain connection to the backup-program
/// 
/// ## Parameters
/// `wv_watch_rx`: Rx on watch the worldview is being sent on in the system  
/// 
/// ## Behavior
/// - Sets up a reusable TCP listener and starts a backup program in a new terminal
/// - Continously sends the latest worldview to the backup asynchronously
/// - Continously reads the latest worldview shich will be sent
/// 
/// ## Note
/// This function is permanently blocking, and should be ran asynchronously 
pub async fn start_backup_server(
    wv_watch_rx: watch::Receiver<WorldView>,
    mut network_watch_rx: watch::Receiver<network::ConnectionStatus>,
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

    

    // // Task for å printe ny nettverksstatus når den endrar seg
    // tokio::spawn(async move {
    //     loop {
    //         if network_watch_rx.changed().await.is_ok() {
    //             let status = network_watch_rx.borrow().clone();
    //             println!(
    //                 "🔄 Backup-mottatt status: on_internett={}, connected_on_elevator_network={}, packet_loss={}%",
    //                 status.on_internett,
    //                 status.connected_on_elevator_network,
    //                 status.packet_loss
    //             );
    //         }
    //     }
    // });
}


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
