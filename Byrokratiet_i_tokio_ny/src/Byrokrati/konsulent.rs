/*Konsulenten veit naturlegvis ikkje kva jobben hans egentlig er, 
og du må ikkje finne på å spørre han om kva jobben hans innebærer*/

// Dette programmet gir deg 3 sekund på å lukke vinduer
use std::fs::OpenOptions;
use std::io::Write;
use std::net::IpAddr;
use crate::config;

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



