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
pub fn get_terminal_command() -> (String, Vec<String>) {
    // Detect platform and return appropriate terminal command
    if cfg!(target_os = "windows") {
        ("cmd".to_string(), vec!["/C".to_string(), "start".to_string()])
    } else {
        ("gnome-terminal".to_string(), vec!["--".to_string()])
    }
}

pub fn log_to_csv(role: &str, event: &str, counter: i32) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("process_log.csv")
        .expect("Failed to open log file");
    writeln!(file, "{},{},{}", role, event, counter).expect("Failed to write to log file");
}


