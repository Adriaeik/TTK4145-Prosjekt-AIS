use std::fs::File;
use std::io::{self, BufRead};
use std::env;
use std::process::Command;
use std::path::Path;

fn main() {
    let config_path = "config.txt";
    let ssh_password = "Sanntid15";
    let args: Vec<String> = env::args().collect();
    let update_repo = args.contains(&"update_repo".to_string());
    let only_elev = args.contains(&"only_elev".to_string());
    
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
                
                println!("\nOppdaterer system og installerer avhengigheiter: \n \t  {}", update_command);
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
                    println!("\nOppdaterer repo: {}", update_repo_command);
                    let _ = Command::new("sh")
                        .arg("-c")
                        .arg(&update_repo_command)
                        .output()
                        .expect("Feil ved oppdatering av repo");
                }
                // drep pågåande program
                let kill_command = format!("pkill -f {}", "Byrokritiet_i_tokio"); // Erstatt med korrekt prosessnavn
                println!("\nDreper evt. pågåande program: \n \t  {}", kill_command);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&kill_command)
                    .output()
                    .expect("Feil ved forsøk på å drepe eksisterande prosessar");

                
                // Start elevatorserver i ny terminal utan dbus
                let elevator_server_command = format!(
                    "sshpass -p '{}' ssh -X student@{} 'export DISPLAY=:0 && gnome-terminal -- bash -c \"elevatorserver; exec bash\"'",
                    ssh_password, ip_address
                );
                println!("\nStarter elevatorserver i ny terminal: \n \t  {}", elevator_server_command);
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
                    
                    println!("\nKjører programmet i ny terminal: \n \t  {}", command);
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


/*
GIT test
pub fn primary_process(&mut self) -> Pin<Box<dyn Future<Output = tokio::io::Result<()>> + Send>> {
        let mut self_copy = self.copy();
    
        Box::pin(async move {
            println!("En sjef er starta");        
            let self_backup = self_copy.copy_for_backup();
    
            // Lager en tokio-task som holder styr på backup
            tokio::spawn(async move {
                self_backup.create_and_monitor_backup().await;
            });
    
            // Lytt til nettverket for å bestemme rolle
            match self_copy.listen_to_network().await {
                Ok(addr) => {
                    if addr.to_string() == "0.0.0.0" {
                        self_copy.rolle = Rolle::MASTER;
                    } else {
                        self_copy.rolle = Rolle::SLAVE;
                        self_copy.master_ip = addr;
                    }
                }
                Err(e) => eprintln!("feil i sjefen.rs primary_process() listen_to_network(): {}", e),
            }
    
            if self_copy.rolle == Rolle::SLAVE {
                let mut self_copy_clone = self_copy.copy();
                let vent = tokio::spawn(async move {
                    match self_copy_clone.abboner_master_nyhetsbrev().await {
                        Ok(_) => {},
                        Err(e) => eprintln!("Feil i PostNord::abboner_master_nyhetsbrev: {}", e),  
                    }
                });
                vent.await.unwrap();
            }
    
            self_copy.rolle = Rolle::MASTER;
    
            // Start broadcasting som master
            let self_copy_clone = self_copy.copy();
            let broadcast_task = tokio::spawn(async move {
                match self_copy_clone.start_master_broadcaster().await {
                    Ok(_) => {},
                    Err(e) => eprintln!("Feil i MrWorldWide::start_broadcaster: {}", e),  
                }
            });
    
            // Start publisering av nyhetsbrev
            let (tx, _) = broadcast::channel::<String>(3);
            let tx = Arc::new(tx);
            let tx_clone = Arc::clone(&tx);
    
            let self_copy_clone = self_copy.copy();
            let nyhetsbrev_task = tokio::spawn(async move {
                match self_copy_clone.publiser_nyhetsbrev(tx_clone).await {
                    Ok(_) => {
                        println!("Går ut av publiser nyhetsbrev");
                    }, 
                    Err(e) => eprintln!("Feil i PostNord::publiser_nyhetsbrev: {}", e),  
                }
            });
    
            // Hent ID (siste tall i IP)
            let worldview = format!("Worldview:{}", self_copy.ip);
            
            while self_copy.rolle == Rolle::MASTER {
                let tx_clone_for_send = Arc::clone(&tx);
                let worldview_clone = worldview.clone();
                tokio::spawn(async move {
                    let _ = tx_clone_for_send.send(worldview_clone);
                });
                sleep(Duration::from_millis(100)).await;
            }
            
            nyhetsbrev_task.await?;
            broadcast_task.abort();
            
            self_copy.rolle = Rolle::SLAVE;
            self_copy.primary_process().await?;
            
            Ok(())
        })
    }
*/