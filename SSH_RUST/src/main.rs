use std::fs::File;
use std::io::{self, BufRead};
use std::process::Command;
use std::path::Path;

fn main() {
    let config_path = "config.txt";
    let ssh_password = "Sanntid15";
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
                
                println!("Oppdaterer system og installerer avhengigheiter: {}", update_command);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&update_command)
                    .output()
                    .expect("Feil ved oppdatering av system");
                
                // Stopp eventuelle prosessar som allereie køyrer
                let kill_command = format!(
                    "sshpass -p '{}' ssh -X student@{} 'pkill -f Byrokritiet_i_tokio || true'",
                    ssh_password, ip_address
                );
                
                println!("Stopper eventuelle kjørende prosesser: {}", kill_command);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&kill_command)
                    .output()
                    .expect("Feil ved stopp av eksisterende prosesser");
                
                // Fjern eksisterande mappe og klon frå bunn
                let clean_command = format!(
                    "sshpass -p '{}' ssh -X student@{} 'rm -rf ~/fuckers && mkdir -p ~/fuckers && cd ~/fuckers && \
                    git clone https://github.com/Adriaeik/TTK4145-Prosjekt-AIS'",
                    ssh_password, ip_address
                );
                
                println!("Fjernar eksisterande mappe og klonar på nytt: {}", clean_command);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&clean_command)
                    .output()
                    .expect("Feil ved sletting og kloning av repo");
                
                let command = format!(
                    "sshpass -p '{}' ssh -X student@{} 'export DISPLAY=:0 && echo DISPLAY=$DISPLAY && cd ~/fuckers/TTK4145-Prosjekt-AIS/Byrokritiet_i_tokio && \
                    gnome-terminal -- bash -c \"cargo run -- {} {}; exec bash\"'",
                    ssh_password, ip_address, role, id
                );
                
                println!("Kjører kommando: {}", command);
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

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}