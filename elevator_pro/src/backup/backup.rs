use tokio::{net::{TcpListener, TcpStream}, sync::watch, time::{timeout, Duration}};
use crate::{init, config, utils, world_view::{self, world_view::{print_wv,WorldView}}};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use crate::network::local_network;

use std::env;
use std::process::Command;
use tokio::time::{sleep};



fn start_backup_terminal() {
    // Definer ønskja vindaugegeometri, til dømes 80 kolonner og 24 rader
    let geometry = "--geometry=400x24";
    
    // Få terminalkommando og standard argument
    let (cmd, mut args) = utils::get_terminal_command();
    
    // Legg til geometry-argumentet før resten av argumenta
    args.insert(0, geometry.to_string());;
    let mut backup_args = args;
            backup_args.push(env::current_exe().unwrap().to_str().unwrap().to_string());
            backup_args.push("backup".to_string());


            Command::new(cmd)
                .args(backup_args)
                .spawn()
                .expect("Failed to start backup process");
}

pub async fn start_backup_server(chs: local_network::LocalChannels) {
    println!("Backup-serveren startar...");
    
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config::BCU_PORT))
        .await
        .expect("Klarte ikkje binde backup-porten");
    let wv = utils::get_wv(chs.clone());
    let (tx, rx) = watch::channel(wv.clone());
    
    start_backup_terminal();
    
    // Task for å handtere backup-klientar
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.expect("Klarte ikkje akseptere backup-kopling");
            handle_backup_client(socket, rx.clone()).await;
        }
    });
    // Oppdater worldview til backup-klientane
    loop {
        let new_wv = utils::get_wv(chs.clone());
        tx.send(new_wv).expect("Klarte ikkje sende til backup-klientane");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    
}



async fn handle_backup_client(mut stream: TcpStream, rx: watch::Receiver<Vec<u8>>) {
    loop {
        let wv = rx.borrow().clone();
        if let Err(e) = stream.write_all(&wv).await {
            eprintln!("Backup send error: {}", e);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

pub async fn run_as_backup() {
    println!("Starter backup-klient...");
    let mut current_wv = init::initialize_worldview().await;
    let mut retries = 0;
    
    loop {
        match timeout(
            config::MASTER_TIMEOUT,
            TcpStream::connect(format!("127.0.0.1:{}", config::BCU_PORT))
        ).await {
            Ok(Ok(mut stream)) => {
                retries = 0;
                let mut buf = vec![0u8; 1024];
                // Les data i ein løkke for kontinuerleg oppdatering
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) => {
                            eprintln!("Master koplinga vart avslutta.");
                            break;
                        },
                        Ok(n) => {
                            current_wv = buf[..n].to_vec();
                            print_wv(current_wv.clone());
                        },
                        Err(e) => {
                            eprintln!("Lesefeil frå master: {}", e);
                            break;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            },
            _ => {
                retries += 1;
                eprintln!("Kunne ikkje koble til master, retry {}.", retries);
                if retries > 3 {
                    eprintln!("Master feila, promoterer backup til master!");
                    // Her kan du setje i gang failover-logikk, t.d. kalle master::run_master()
                    return;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
