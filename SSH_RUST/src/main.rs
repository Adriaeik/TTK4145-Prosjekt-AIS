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
                
                // Stopp eventuelle prosessar som allereie køyrer
                let kill_command = format!(
                    "sshpass -p '{}' ssh student@{} 'pkill -f Byrokritiet_i_tokio || true'",
                    ssh_password, ip_address
                );
                
                println!("Stopper eventuelle kjørende prosesser: {}", kill_command);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&kill_command)
                    .output()
                    .expect("Feil ved stopp av eksisterende prosesser");
                
                let command = format!(
                    "sshpass -p '{}' ssh student@{} 'mkdir -p fuckers && cd fuckers && \
                    if [ ! -d \"TTK4145-Prosjekt-AIS\" ]; then git clone https://github.com/Adriaeik/TTK4145-Prosjekt-AIS; fi && \
                    cd TTK4145-Prosjekt-AIS && cd Byrokritiet_i_tokio && cargo run -- {} {}'",
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
