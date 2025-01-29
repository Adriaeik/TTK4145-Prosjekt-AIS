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


pub async fn backup_process() {
    tokio::spawn(async move {
        IT_Roger::backup_connection( "10.24.210.159:8080", "69").await;
    });
    loop {
        sleep(Duration::from_secs(1)).await; // Sover i 1 sekund
    }
}
