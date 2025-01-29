/*Konsulenten tar seg av */
// Dette programmet gir deg 3 sekund på å lukke vinduer
use tokio::time::{sleep, Duration, Instant, interval};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::env;
use std::process::Command;
use std::fs::OpenOptions;
use std::io::Write;
use socket2::{Socket, Domain, Type, Protocol};
use std::net::SocketAddr;

static BACKUP_STARTED: AtomicBool = AtomicBool::new(false);

/// Returnerer kommando for å åpne terminal til tilhørende OS         
///
/// # Eksempel
/// ```
/// let (cmd, args) = get_terminal_command(); 
/// ```
/// returnerer:
/// 
/// linux -> "gnome-terminal", "--""
/// 
/// windows ->  "cmd", "/C", "start"
fn get_terminal_command() -> (String, Vec<String>) {
    // Detect platform and return appropriate terminal command
    if cfg!(target_os = "windows") {
        ("cmd".to_string(), vec!["/C".to_string(), "start".to_string()])
    } else {
        ("gnome-terminal".to_string(), vec!["--".to_string()])
    }
}

fn log_to_csv(role: &str, event: &str, counter: i32) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("process_log.csv")
        .expect("Failed to open log file");
    writeln!(file, "{},{},{}", role, event, counter).expect("Failed to write to log file");
}

async fn create_reusable_listener(addr: &str) -> TcpListener {
    let socket_addr: SocketAddr = addr.parse().expect("Invalid address");
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).expect("Failed to create socket");
    socket.set_reuse_address(true).expect("Failed to set reuse address");
    socket.bind(&socket_addr.into()).expect("Failed to bind socket");
    socket.listen(128).expect("Failed to listen on socket");

    TcpListener::from_std(socket.into()).expect("Failed to create TcpListener")
}

pub async fn primary_process() {
    let counter = Arc::new(Mutex::new(1));
    let timeout_duration = Duration::from_secs(3);
    let last_received = Arc::new(Mutex::new(Instant::now()));

    // Start the backup process in a new terminal
    start_backup();

    let listener = create_reusable_listener("127.0.0.1:8080").await;
    println!("Primary is running on 127.0.0.1:8080");

    // Spawn a separate task to monitor backup responsiveness
    let last_received_clone = Arc::clone(&last_received);
    tokio::spawn(async move {
        monitor_backup(last_received_clone, timeout_duration).await;
    });

    loop {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut count = counter.lock().await;
            println!("Primary counting: {}", *count); // Print to terminal
            socket.write_all(format!("{}\n", *count).as_bytes()).await.expect("Failed to send count");

            let mut last = last_received.lock().await;
            *last = Instant::now();

            log_to_csv("Primary", "Counting", *count);
            *count += 1;
        }
    }
}

pub async fn backup_process() {
    let mut counter = 0;
    let timeout_duration = Duration::from_secs(3);
    let mut last_received = Instant::now();

    loop {
        match TcpStream::connect("127.0.0.1:8080").await {
            Ok(mut stream) => {
                let mut buf = String::new();
                let mut reader = BufReader::new(&mut stream);
                if reader.read_line(&mut buf).await.is_ok() {
                    last_received = Instant::now();
                    counter = buf.trim().parse::<i32>().unwrap_or(counter);
                    log_to_csv("Backup", "Received", counter);
                }
            }
            Err(_) => {
                // Retry silently
            }
        }

        if last_received.elapsed() > timeout_duration {
            println!("Primary is unresponsive. Taking over at {}", counter);
            log_to_csv("Backup", "Taking Over as Primary", counter);

            let listener = create_reusable_listener("127.0.0.1:8080").await;
            println!("New primary is running on 127.0.0.1:8080");

            // Start a new backup process as the new primary
            start_backup_with_reset();

            // Spawn a task to monitor the new backup
            let last_received = Arc::new(Mutex::new(Instant::now()));
            let last_received_clone = Arc::clone(&last_received);
            tokio::spawn(async move {
                monitor_backup(last_received_clone, timeout_duration).await;
            });

            loop {
                counter += 1;
                println!("Primary counting: {}", counter);
                log_to_csv("Primary", "Counting", counter);

                if let Ok((mut socket, _)) = listener.accept().await {
                    socket.write_all(format!("{}\n", counter).as_bytes()).await.expect("Failed to send count");

                    let mut last = last_received.lock().await;
                    *last = Instant::now();
                }

                sleep(Duration::from_secs(1)).await;
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

async fn monitor_backup(last_received: Arc<Mutex<Instant>>, timeout_duration: Duration) {
    let mut backup_timer = interval(Duration::from_secs(1));
    backup_timer.tick().await; // Start timer

    loop {
        backup_timer.tick().await;
        let elapsed = {
            let last = last_received.lock().await;
            last.elapsed()
        };

        if elapsed > timeout_duration {
            println!("Backup is unresponsive. Starting a new backup...");
            log_to_csv("Primary", "Backup Unresponsive", 0);

            // Reset BACKUP_STARTED and start a new backup
            start_backup_with_reset();

            let mut last = last_received.lock().await;
            *last = Instant::now();
        }
    }
}

fn start_backup() {
    if !BACKUP_STARTED.load(Ordering::SeqCst) {
        let (cmd, args) = get_terminal_command();
        let mut backup_args = args;
        backup_args.push(env::current_exe().unwrap().to_str().unwrap().to_string());
        backup_args.push("backup".to_string());
        backup_args.push("2".to_string());


        Command::new(cmd)
            .args(backup_args)
            .spawn()
            .expect("Failed to start backup process");

        BACKUP_STARTED.store(true, Ordering::SeqCst);
        log_to_csv("Primary", "Backup Started", 0);
    }
}

fn start_backup_with_reset() {
    BACKUP_STARTED.store(false, Ordering::SeqCst);
    start_backup();
}