/*Konsulenten tar seg av */
// Dette programmet gir deg 3 sekund på å lukke vinduer
use std::fs::OpenOptions;
use std::io::Write;

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
pub fn id_fra_ip(ip: &str) -> Option<String> {
    ip.split('.')           // Del på punktum
        .nth(3)              // Hent den 4. delen (d)
        .map(|s| s.split(':') // Del på kolon
            .next()          // Ta kun første delen før kolon
            .unwrap_or("")   // Hvis ingen kolon finnes, bruk tom streng
            .to_string())    // Konverter til String
}




