use tokio::{net::{TcpListener, TcpStream}, sync::watch, time::{timeout, Duration}};
use crate::{init, config, utils, world_view::{self, world_view::{print_wv,WorldView}}};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use crate::network::local_network;

use std::env;
use std::process::Command;
use tokio::time::{sleep};


pub async fn start_backup_server(chs: local_network::LocalChannels) {
    println!("Backup-serveren startar...");

    // Start backupterminalen (klienten) ved å køyre same program med "--backup"
    let current_exe = env::current_exe().expect("Klarte ikkje hente ut den kjørbare fila");
    let _child = Command::new(current_exe)
        .arg("--backup")
        .spawn()
        .expect("Feil ved å starte backupterminalen");

    // Start server-delen: lytt på tilkoplingar frå backupterminalen
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config::BCU_PORT))
        .await
        .expect("Klarte ikkje binde backup-porten");
    let wv = utils::get_wv(chs.clone());
    let (tx, rx) = watch::channel(wv.clone());

    // Task for å handtere backup-klientar
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener
                .accept()
                .await
                .expect("Klarte ikkje akseptere backup-kopling");
            handle_backup_client(socket, rx.clone()).await;
        }
    });

    // Oppdater worldview kontinuerleg til backup-klientane
    loop {
        let new_wv = utils::get_wv(chs.clone());
        tx.send(new_wv).expect("Klarte ikkje sende til backup-klientane");
        sleep(Duration::from_secs(1)).await;
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
    println!("Starter backup-prosess...");
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
                // Leser meldingane frå master kontinuerleg
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) => {
                            // Master har avslutta koplinga
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
                    // Gje litt pause for å unngå for hyppig printing
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            },
            _ => {
                retries += 1;
                if retries > 3 {
                    eprintln!("Master feila, promoterer backup til master!");
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
