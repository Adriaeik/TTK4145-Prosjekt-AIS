use std::fs::OpenOptions;
use std::io::Write;
use std::net::IpAddr;
use anyhow::Context;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::config;

use core::panic;
use local_ip_address::local_ip;

use tokio::sync::Mutex;
use std::sync::Arc;






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

/// Henter IDen din fra IPen
/// 
/// # Eksempel
/// ```
/// let id = id_fra_ip("a.b.c.d:e");
/// ```
/// returnerer d
/// 
pub fn ip2id(ip: IpAddr) -> u8 {
    let ip_str = ip.to_string();
    let mut ip_int = config::ERROR_ID;
    let id_str = ip_str.split('.')           // Del på punktum
        .nth(3)              // Hent den 4. delen (d)
        .and_then(|s| s.split(':')  // Del på kolon hvis det er en port etter IP-en
            .next())         // Ta kun første delen før kolon
        .and_then(|s| s.parse::<u8>().ok());  // Forsøk å parse til u8

    match id_str {
        Some(value) => {
            ip_int = value;
        }
        None => {
            println!("Ingen gyldig ID funnet. (konsulent.rs, id_fra_ip())");
        }
    }
    ip_int
}

pub fn print_color(msg: String, color: Color) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(color))).unwrap();
    writeln!(&mut stdout, "[CUSTOM]:  {}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");
}

pub fn print_err(msg: String) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
    writeln!(&mut stdout, "[ERROR]:   {}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");
}

pub fn print_warn(msg: String) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
    writeln!(&mut stdout, "[WARNING]: {}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");
}

pub fn print_ok(msg: String) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
    writeln!(&mut stdout, "[OK]:      {}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");
}

pub fn print_info(msg: String) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue))).unwrap();
    writeln!(&mut stdout, "[INFO]:    {}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");
}
