/*Konsulenten veit naturlegvis ikkje kva jobben hans egentlig er, 
og du må ikkje finne på å spørre han om kva jobben hans innebærer*/

// Dette programmet gir deg 3 sekund på å lukke vinduer
use std::fs::OpenOptions;
use std::io::Write;
use std::net::IpAddr;
use anyhow::{Context, Result};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::config;
use crate::WorldView::WorldView;
use crate::WorldView::WorldViewChannel;
use super::konsulent;

use core::panic;
use local_ip_address::local_ip;

use tokio::sync::Mutex;
use std::sync::Arc;

use super::Sjefen;

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

/// Henter IDen din fra IPen
/// 
/// # Eksempel
/// ```
/// let id = id_fra_ip("a.b.c.d:e");
/// ```
/// returnerer d
/// 
pub fn id_fra_ip(ip: IpAddr) -> u8 {
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
            let mut octets = addr.octets();
            format!("{}.{}.{}", octets[0], octets[1], octets[2])
        }
        IpAddr::V6(addr) => {
            let segments = addr.segments();
            let root_segments = &segments[..segments.len() - 1]; // Fjern siste segment
            root_segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(":")
        }
    }
}


pub fn er_master(self_id : u8, master_id : u8) -> bool{
    return self_id == master_id;
}

pub async fn get_worldview_from_channel(mut rx_wv: tokio::sync::broadcast::Receiver<Vec<u8>>) -> WorldView::WorldView {
    WorldViewChannel::request_worldview().await;

    // Mottar den serialiserte forma
    let worldview_s = rx_wv
        .recv()
        .await
        .context("Feila å lese frå worldview_rx i start_from_worldview()").expect("feil i lesing av worldview fra channel");
    
    // Deserialiserer til ein struct, og mappar feilen til ein trådsikker variant
        let worldview = WorldView::deserialize_worldview(&worldview_s)
        .map_err(|e| anyhow::anyhow!("deserialize_worldview() feila i start_from_worldview(): {}", e)).expect("feil i lesing av worldview fra channel");

    worldview
}



pub async fn init_serialised_worldview() -> (Sjefen::Sjefen, Vec<u8>) {
    // Initialiserer en sjefen struct
    /*Initialiser ein sjefpakke basert på argument (Rolle) */
    let sjefenpakke = match Sjefen::hent_sjefpakke() {
        Ok(sjef) => {
            println!("Opprettet SjefPakke: {:?}", sjef);
            sjef // Returner sjefen dersom alt gjekk bra
        }
        Err(e) => {
            panic!("Feil ved henting av SjefPakke: {}", e); // Avslutt programmet dersom ein feil oppstod
        }
    };
    //Finne IP :)
    let ip = match local_ip() {
        Ok(ip) => {
            ip
        }
        Err(e) => {
            panic!("Fant ikke IP (main.rs): {}", e);
        }
    }; 
    let id = konsulent::id_fra_ip(ip);
    let sjefen = Sjefen::Sjefen {
        ip: ip,
        id: id,
        rolle: Arc::new(Mutex::new(sjefenpakke.rolle)),
        master_ip: Arc::new(Mutex::new(ip)),
    };

    //Init til worldview
    let serialized_worldview = sjefen.start_clean().await;

    (sjefen,serialized_worldview)
}


pub fn print_farge(msg: String, color: Color) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(color))).unwrap();
    writeln!(&mut stdout, "{}", msg).unwrap();
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!("\r\n");

}