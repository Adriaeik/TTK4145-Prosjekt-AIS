use std::{fmt::format, io::Write};
use std::net::IpAddr;
use std::u8;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::{config, network::local_network, world_view::world_view::{self, Task}};

use local_ip_address::local_ip;

use std::sync::atomic::{AtomicU8, Ordering};

// Definer ein global `AtomicU8`
pub static SELF_ID: AtomicU8 = AtomicU8::new(u8::MAX); // Startverdi 0




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

/// Returnerer lokal IPv4-addresse til maskinen som `IpAddr` 
/// 
/// Om lokal IPv4-addresse ikke fins, returneres `local_ip_address::Error`
pub fn get_self_ip() -> Result<IpAddr, local_ip_address::Error> {
    let ip = match local_ip() {
        Ok(ip) => {
            ip
        }
        Err(e) => {
            print_warn(format!("Fant ikke IP i get_self_ip() -> Vi er offline: {}", e));
            return Err(e);
        }
    };
    Ok(ip)
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

/// Henter roten av IPen
/// 
/// # Eksempel
/// ```
/// let id = id_fra_ip("a.b.c.d");
/// ```
/// returnerer "a.b.c"
/// 
pub fn get_root_ip(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(addr) => {
            let octets = addr.octets();
            format!("{}.{}.{}", octets[0], octets[1], octets[2])
        }
        IpAddr::V6(addr) => {
            let segments = addr.segments();
            let root_segments = &segments[..segments.len() - 1]; // Fjern siste segment
            root_segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(":")
        }
    }
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
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(102, 178, 255/*lyseblå*/)))).unwrap();
    writeln!(&mut stdout, "[INFO]:    {}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");
}

pub fn print_master(msg: String) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255, 51, 255/*Rosa*/)))).unwrap();
    writeln!(&mut stdout, "[MASTER]:  {}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");
}

pub fn print_slave(msg: String) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(153, 76, 0/*Tilfeldig*/)))).unwrap();
    writeln!(&mut stdout, "[SLAVE]:   {}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");
}

pub fn print_cosmic_err(fun: String) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Skriv ut "[ERROR]:" i rød
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
    write!(&mut stdout, "[ERROR]: ").unwrap();

    // Definer regnbuefargene
    let colors = [
        Color::Red,
        Color::Yellow,
        Color::Green,
        Color::Cyan,
        Color::Blue,
        Color::Magenta,
    ];

    // Resten av meldingen i regnbuefarger
    let message = format!("Cosmic rays flipped a bit! 👽 ⚛️ 🔄 1️⃣ 0️⃣ IN: {}", fun);
    for (i, c) in message.chars().enumerate() {
        let color = colors[i % colors.len()];
        stdout.set_color(ColorSpec::new().set_fg(Some(color))).unwrap();
        write!(&mut stdout, "{}", c).unwrap();
    }

    // Tilbakestill fargen
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!();
}

/// Henter klone av nyeste wv i systemet
pub fn get_wv(chs: local_network::LocalChannels) -> Vec<u8> {
    chs.watches.rxs.wv.borrow().clone()
}

pub async fn update_wv(mut chs: local_network::LocalChannels, wv: &mut Vec<u8>) {
    if chs.watches.rxs.wv.changed().await.is_ok() {
        *wv = chs.watches.rxs.wv.borrow().clone();
    }
}

/// Sjekker om du er master, basert på nyeste worldview
pub fn is_master(/*chs: local_network::LocalChannels */wv: Vec<u8>) -> bool {
    // let wv: Vec<u8> = get_wv(chs.clone());
    return SELF_ID.load(Ordering::SeqCst) == wv[config::MASTER_IDX];
}

pub fn get_elev_tasks(chs: local_network::LocalChannels) -> Vec<Task> {
    chs.watches.rxs.elev_task.borrow().clone()
}

/// Henter klone av elevator_container med `id` fra nyeste worldview
pub fn extract_elevator_container(wv: Vec<u8>, id: u8) -> world_view::ElevatorContainer {
    let mut deser_wv = world_view::deserialize_worldview(&wv);

    deser_wv.elevator_containers.retain(|elevator| elevator.elevator_id == id);
    deser_wv.elevator_containers[0].clone()
}

/// Henter klone av elevator_container med `SELF_ID` fra nyeste worldview
pub fn extract_self_elevator_container(wv: Vec<u8>) -> world_view::ElevatorContainer {
    extract_elevator_container(wv, SELF_ID.load(Ordering::SeqCst))
}



pub async fn close_tcp_stream(stream: &mut TcpStream) {
    // Hent IP-adresser
    let local_addr = stream.local_addr().map_or_else(
        |e| format!("Ukjent (Feil: {})", e),
        |addr| addr.to_string(),
    );

    let peer_addr = stream.peer_addr().map_or_else(
        |e| format!("Ukjent (Feil: {})", e),
        |addr| addr.to_string(),
    );

    // Prøv å stenge streamen (Asynkront)
    match stream.shutdown().await {
        Ok(_) => print_info(format!(
            "TCP-forbindelsen er avslutta korrekt: {} -> {}",
            local_addr, peer_addr
        )),
        Err(e) => print_err(format!(
            "Feil ved avslutting av TCP-forbindelsen ({} -> {}): {}",
            local_addr, peer_addr, e
        )),
    }
}
