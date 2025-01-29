//! Tony er lagersjef. han driver også en italiensk mafia på siden
use super::IT_Roger;
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


pub async fn backup_process(ip: &str) {
    let ip_copy = ip.to_string();
    
    tokio::spawn(async move {
        IT_Roger::backup_connection( &ip_copy, "69").await;
    });
    loop {
        sleep(Duration::from_secs(1)).await; // Sover i 1 sekund
    }
}
