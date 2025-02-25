use std::io::Write;
use std::net::IpAddr;
use std::u8;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::{config, world_view::world_view, network::local_network};

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
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue))).unwrap();
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


pub fn get_wv(mut chs: local_network::LocalChannels) -> Vec<u8> {
    let mut wv_option = None;
    chs.resubscribe_broadcast();
    let mut wv: Vec<u8> = Vec::new();

    while wv_option.is_none() {
        wv_option = {
            let mut latest_msg = None;
            while let Ok(message) = chs.broadcasts.rxs.wv.try_recv() {
                latest_msg = Some(message); // Overskriv tidligere meldinger
            }
            latest_msg
        };

        if let Some(ref msg) = wv_option {
            wv = msg.clone();
        } 
    }

    wv
}

pub fn update_wv(mut chs: local_network::LocalChannels, wv: &mut Vec<u8>) {
    chs.resubscribe_broadcast();
    while let Ok(new_wv) = chs.broadcasts.rxs.wv.try_recv() {
        *wv = new_wv; // Overstyr wv med den nyaste meldinga
    }
}


pub fn is_master(mut chs: local_network::LocalChannels) -> bool {
    let mut wv_option = None;
    chs.resubscribe_broadcast();
    let mut wv: Vec<u8> = Vec::new();

    while wv_option.is_none() {
        wv_option = {
            let mut latest_msg = None;
            while let Ok(message) = chs.broadcasts.rxs.wv.try_recv() {
                latest_msg = Some(message); // Overskriv tidligere meldinger
            }
            latest_msg
        };

        if let Some(ref msg) = wv_option {
            wv = msg.clone();
        } 
    }

    return SELF_ID.load(Ordering::SeqCst) == wv[config::MASTER_IDX];
}

pub fn extract_elevator_container(chs: local_network::LocalChannels, id: u8) -> world_view::ElevatorContainer {
    let wv = get_wv(chs.clone());
    let mut deser_wv = world_view::deserialize_worldview(&wv);

    deser_wv.elevator_containers.retain(|elevator| elevator.elevator_id == id);
    deser_wv.elevator_containers[0].clone()
}

pub fn extract_self_elevator_container(chs: local_network::LocalChannels) -> world_view::ElevatorContainer {
    extract_elevator_container(chs, SELF_ID.load(Ordering::SeqCst))
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
