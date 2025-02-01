//! Sikrer at fine wordviewpakker blir sendt dit de skal :)


use crate::config;

use std::net::SocketAddr;
use std::time::Duration;
use std::u8;
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::broadcast;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::sync::Arc;
use std::net::IpAddr;
use super::Sjefen;

use super::konsulent::id_fra_ip;

impl Sjefen::Sjefen {

    pub async fn publiser_nyhetsbrev(&self, mut tx: Arc<broadcast::Sender<String>>) -> tokio::io::Result<()> {
       


        let listener = TcpListener::bind(format!("{}:{}", self.ip, config::PN_PORT)).await?;
        println!("Nyhetsbrev oppretta på {}:{}", self.ip.to_string(), config::PN_PORT);

        // let (tx, _) = broadcast::channel::<String>(3); //Kunne vel i teorien vært 1
        // let tx = Arc::new(tx);
        
        let self_id = id_fra_ip(self.ip);
        // Håndter alle innkommende tilkoblinger
        loop {
            let (socket, _) = listener.accept().await?;  // Nå kan vi kalle accept() på listeneren
            let tx_clone = Arc::clone(&tx); // Klon senderen for bruk i ny oppgave
            // Start en ny oppgave for hver klient
            let self_copy = self.copy();
            tokio::spawn(async move {
                let rx = tx_clone.subscribe(); // Opprett en ny receiver for hver klient
                if let Err(e) = self_copy.send_nyhetsbrev(socket, rx).await {
                    eprintln!("Feil i kommunikasjon med klient: {}", e);
                }
            });
        }

    }


    async fn send_nyhetsbrev(&self, mut socket: TcpStream, mut rx: broadcast::Receiver<String>) -> Result<(), Box<dyn std::error::Error>> {
        // Håndter kommunikasjonen med klienten her.
        let peer_addr = socket.peer_addr().unwrap();
        let peer_id = id_fra_ip(peer_addr.ip());
        println!("Den nye klienten har id: {}", peer_id);



        let mut buf = [0; 10];
        loop {
            tokio::select! {
                // Lytt på meldinger fra broadcast-kanalen
                msg = rx.recv() => {
                    match msg {
                        Ok(message) => {
                            // Send melding til klienten
                            if let Err(e) = socket.write_all(message.as_bytes()).await {
                                eprintln!("Feil ved sending til klient i send_nyhetsbrev(): {}", e);
                                return Err(Box::new(e));
                            }
                            if peer_id < self.id {
                                //Husk å stenge broadcast tråder ogsånt før man starter reset. Prøvde å gjøre det uten  å
                                //stenge tråder og den åpna en milliarer trillioner backuper igjen
                                println!("Klienten har lavere ip, nå må jeg slutte å være master!");
                            }
                        }
                        Err(e) => {
                            eprintln!("Feil ved mottak fra broadcast-kanal: {}", e);
                            return Err(Box::new(e));
                        }
                    }
                }
                
                // Les ack
                result = socket.read(&mut buf) => {
                    match result {
                        Ok(0) => {
                            println!("Koblingen er lukket av klienten.");
                            return Ok(()); // Klienten har koblet fra
                        }
                        Ok(_) => {
                            // Her kan du behandle innkommende data fra klienten om nødvendig.
                            // Eksempel: logge data som ble mottatt
                            println!("Mottok fra klient: {}", String::from_utf8_lossy(&buf));
                        }
                        Err(e) => {
                            eprintln!("Feil ved lesing fra klient: {}", e);
                            return Err(Box::new(e));
                        }
                    }
                }
            }
        }
    }



    pub async fn abboner_master_nyhetsbrev(&mut self, master_ip: SocketAddr) -> tokio::io::Result<()> {
        // let my_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap(); //kjent
        // let master_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap(); //kjent fra udp broadcast

        //les inn string til master ip fra channel her først

        let ip_string = master_ip.to_string(); // Konverter til String
        let mut iponly: &str = "a";
        match ip_string.split_once(':') {
            Some((ip, _)) => {iponly = ip;}
            None => {}
        }
        println!("Prøver å koble på: {}:{}", iponly, config::PN_PORT);
        //NB!!!!
        // Må teste litt på sanntidslabben om riktig ip blir sent i udp_broadcasten, eller om man må sende den som en melding i udp broadcasten
        let mut stream = TcpStream::connect(format!("{}:{}",iponly, config::PN_PORT)).await?;
        let mut buf = [0; 1024];
        println!("Har kobla til en master på ip: {}:{}", iponly, config::PN_PORT);


        let master_id = id_fra_ip(master_ip.ip());
        println!("Masteren har id: {}", master_id);

        println!("Jeg har id: {}", self.id);


        loop {
            let bytes_read = stream.read(&mut buf).await?;
            if bytes_read == 0 {
                println!("Serveren stengte tilkoblingen.");
                break;
            }
            let message = String::from_utf8_lossy(&buf[..bytes_read]);
            println!("Melding fra server: {}", message);
            if self.id < master_id {
                println!("Jeg har lavere ID enn master, jeg må bli master!!!!");
                //Må kanskje passe på å lukke tidligere tråder?
                self.primary_process().await;
            }
        }

        Ok(())
    }

}