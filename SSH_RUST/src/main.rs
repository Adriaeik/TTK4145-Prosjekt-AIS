use std::fs::File;
use std::io::{self, BufRead};
use std::env;
use std::process::Command;
use std::path::Path;

fn main() {
    let config_path = "config.txt";
    let ssh_password = "Sanntid15";
    let args: Vec<String> = env::args().collect();
    let update_repo = args.contains(&"--update_repo".to_string());
    let only_elev = args.contains(&"--only_elev".to_string());
    
    if let Ok(lines) = read_lines(config_path) {
        for line in lines {
            if let Ok(entry) = line {
                let parts: Vec<&str> = entry.split_whitespace().collect();
                if parts.len() < 2 {
                    eprintln!("Feil format i konfigurasjonsfilen");
                    continue;
                }
                let role = parts[0];
                let ip_address = parts[1];
                
                // Ekstraher siste byten av IP-adressa for bruk som ID
                let id = match ip_address.rsplit('.').next() {
                    Some(last_octet) => last_octet,
                    None => {
                        eprintln!("Feil: Kunne ikke ekstrahere siste byte fra IP");
                        continue;
                    }
                };
                
                // Oppdater system og installer nødvendige pakkar
                let update_command = format!(
                    "sshpass -p '{}' ssh -X student@{} 'sudo apt update && sudo apt upgrade -y && sudo apt install -y gnome-terminal sshpass cargo x11-xserver-utils xorg'",
                    ssh_password, ip_address
                );
                
                println!("\n \t Oppdaterer system og installerer avhengigheiter: {}", update_command);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&update_command)
                    .output()
                    .expect("Feil ved oppdatering av system");
                
                // Oppdater repo om `--update_repo` er sett
                if update_repo {
                    let update_repo_command = format!(
                        "sshpass -p '{}' ssh -X student@{} 'mkdir -p ~/fuckers && cd ~/fuckers && \
                        if [ ! -d TTK4145-Prosjekt-AIS ]; then git clone https://github.com/Adriaeik/TTK4145-Prosjekt-AIS; \
                        else cd TTK4145-Prosjekt-AIS && git stash && git pull origin main; fi'",
                        ssh_password, ip_address
                    );
                    println!("\n \t Oppdaterer repo: {}", update_repo_command);
                    let _ = Command::new("sh")
                        .arg("-c")
                        .arg(&update_repo_command)
                        .output()
                        .expect("Feil ved oppdatering av repo");
                }
                
                // Start elevatorserver i ny terminal utan dbus
                let elevator_server_command = format!(
                    "sshpass -p '{}' ssh -X student@{} 'export DISPLAY=:0 && gnome-terminal -- bash -c \"elevatorserver; exec bash\"'",
                    ssh_password, ip_address
                );
                println!("\n \t Starter elevatorserver i ny terminal: {}", elevator_server_command);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&elevator_server_command)
                    .output()
                    .expect("Feil ved start av elevatorserver");
                
                // Hopp over programstart om `--only_elev` er sett
                if !only_elev {
                    let command = format!(
                        "sshpass -p '{}' ssh -X student@{} 'export DISPLAY=:0 && gnome-terminal -- bash -c \"cd ~/fuckers/TTK4145-Prosjekt-AIS/Byrokritiet_i_tokio && cargo run -- {} {}; exec bash\"'",
                        ssh_password, ip_address, role, id
                    );
                    
                    println!("\n \t Kjører programmet i ny terminal: {}", command);
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .output()
                        .expect("Feil ved kjøring av SSH-kommando");
                    
                    println!("Output: {}", String::from_utf8_lossy(&output.stdout));
                    eprintln!("Feilmelding: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
