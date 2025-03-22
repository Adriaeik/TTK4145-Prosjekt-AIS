use std::env;
use std::net::SocketAddr;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{self, Write};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;
use tokio::time::{sleep, Duration, timeout};

use crate::{config, init, world_view};
use crate::print;

// Static variable to see if backup has started
static BACKUP_STARTED: AtomicBool = AtomicBool::new(false);

/// Creates a non blocking TCP listener with reusable address
fn create_reusable_listener(port: u16) -> TcpListener {
    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .expect("Invalid address");
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

/// Handles backup clients: Sends worldview continously
/// TODO: send litt raskere enn en gang i sekundet
async fn handle_backup_client(mut stream: TcpStream, rx: watch::Receiver<Vec<u8>>) {
    loop {
        let wv = rx.borrow().clone();
        if let Err(e) = stream.write_all(&wv).await {
            eprintln!("Backup send error: {}", e);
            BACKUP_STARTED.store(false, Ordering::SeqCst);
            start_backup_terminal();
            break;
        }
        sleep(Duration::from_millis(1000)).await;
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
pub async fn start_backup_server(wv_watch_rx: watch::Receiver<Vec<u8>>) {
    println!("Backup-server starting...");
    
    let listener = create_reusable_listener(config::BCU_PORT);
    let wv = world_view::get_wv(wv_watch_rx.clone());
    let (tx, rx) = watch::channel(wv.clone());
    
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
    
    // Oppdater kontinuerleg worldview til backup-klientane.
    loop {
        let new_wv = world_view::get_wv(wv_watch_rx.clone());
        tx.send(new_wv).expect("Failed to send to the backup-client");
        sleep(Duration::from_millis(1000)).await;
    }
}


pub async fn run_as_backup() -> Option<world_view::ElevatorContainer> {
    println!("Starting backup-client...");
    let mut current_wv = init::initialize_worldview(None).await;
    let mut retries = 0;
    
    loop {
        match timeout(
            config::MASTER_TIMEOUT,
            TcpStream::connect(format!("127.0.0.1:{}", config::BCU_PORT))
        ).await {
            Ok(Ok(mut stream)) => {
                retries = 0;
                let mut buf = vec![0u8; 1024];
                // Les data i ein løkke for kontinuerleg oppdatering
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) => {
                            eprintln!("Master connection has ended.");
                            break;
                        },
                        Ok(n) => {
                            current_wv = buf[..n].to_vec();
                            // Rydd skjermen og sett markøren øvst
                            print!("\x1B[2J\x1B[H");

                            // Sørg for at utskrifta skjer umiddelbart
                            io::stdout().flush().unwrap();

                            print::worldview(current_wv.clone());
                        },
                        Err(e) => {
                            eprintln!("Error while reading from master: {}", e);
                            break;
                        }
                    }
                    sleep(Duration::from_millis(500)).await;
                }
            },
            _ => {
                retries += 1;
                eprintln!("Failed to connect to master, retry {}.", retries);
                if retries > 50 {
                    eprintln!("Master failed, promoting backup to master!");
                    // Her kan failover-logikken setjast i gang, t.d. køyre master-logikken.
                    match world_view::extract_self_elevator_container(current_wv) {
                        Some(container) => return Some(container),
                        None => {
                            print::warn(format!("Failed to extract self elevator container"));
                            return None;
                        }
                    }
                    
                }
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}
