//! Sikrer at fine wordviewpakker blir sendt dit de skal :)

use crate::config;
use crate::WorldView::WorldViewChannel;
use std::thread::sleep;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use super::{Sjefen, konsulent};
use termcolor::Color;


impl Sjefen::Sjefen {
    /// 🔹 **Hentar worldview frå master**
    pub async fn get_wv_from_master(&self) -> Option<Vec<u8>> {

        

    // Skriver ut grønn tekst
        let master_ip = self.master_ip.lock().await;
        
        println!("Prøver å koble på: {}:{} i get_wv_from_master()", *master_ip, config::PN_PORT);
        let stream = TcpStream::connect(format!("{}:{}", *master_ip, config::PN_PORT)).await;

        let mut stream: TcpStream = match stream {
            Ok(strm) => strm,
            Err(e) => {
                konsulent::print_farge(format!("Klarte ikke koble på TCP i get_wv_from_master(): {}", e), Color::Red);
                return None;
            }
        };
        
        konsulent::print_farge(format!("Koblet til master på ip: {}:{} i get_wv_from_master()", *master_ip, config::PN_PORT), Color::Green);

        let mut len_bytes = [0u8; 4];
        let bytes_read = stream.read_exact(&mut len_bytes).await;
        
        if bytes_read.is_err() {
            konsulent::print_farge("Serveren stengte tilkoblingen i get_wv_from_master() 2.".to_string(), Color::Yellow);
            return None;
        }

        let len = u32::from_be_bytes(len_bytes) as usize;
        let mut buf = vec![0u8; len];
        let read = stream.read_exact(&mut buf).await;

        if read.is_err() {
            konsulent::print_farge("Serveren stengte tilkoblingen i get_wv_from_master() 3.".to_string(), Color::Yellow);
            return None;
        }

        println!("Worldview frå master i get_wv_from_master(): {:?}", buf);

        // if self.id < master_id {
        //     println!("Min ID er lågare enn masteren sin, eg må bli ny master i get_wv_from_master()!");
        //     *self.master_ip.lock().await = self.ip;
        // }
        

        Some(buf[..len].to_vec())
    }

    /// 🔹 **Sender worldview-oppdateringar til klientar**
    pub async fn send_post(&self, mut socket: TcpStream, mut rx: broadcast::Receiver<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
        konsulent::print_farge("Startet en send_post i send_post()".to_string(), Color::Green);
        let mut buf = [0; 1024];
        
        let mut i:u8 = 0; //Til telling, proof of concept
        loop {
            sleep(Duration::from_millis(100));
            WorldViewChannel::request_worldview().await;
            tokio::select! {
                msg = async {
                    let mut latest_msg = None;
                    while let Ok(message) = rx.try_recv() {
                        latest_msg = Some(message); // Overskriv tidligere meldinger
                    }
                    latest_msg
                } => {
                    if let Some(mut message) = msg {
                        i = i % 255;
                        i += 1; //Til telling, proof of concept
                        let msg_len = message.len(); //Til telling, proof of concept
                        let len_b = (message.len() as u32).to_be_bytes();
                        socket.write_all(&len_b).await?;
                        message[msg_len-1] = i; //Til telling, proof of concept
                        socket.write_all(&message[..]).await?;
                        //println!("Sendt worldview på TCP nå i send_post()");
                    }
                }
    
                // Leser svar frå klient
                result = socket.read(&mut buf) => {
                    match result {
                        Ok(0) => {
                            konsulent::print_farge("Klienten lukka tilkoblinga. (send_post())".to_string(), Color::Yellow);
                            break; // 🔹 Avslutt loopen om klienten koplar frå
                        }
                        Ok(bytes_read) => {
                            konsulent::print_farge(format!("Mottok {} bytes fra klienten i send_post(): {:?}", bytes_read, &buf[..bytes_read]), Color::Blue);
                        }
                        Err(e) => {
                            konsulent::print_farge(format!("Feil ved lesing fra klient i send_post(): {}", e), Color::Red);
                            break;
                        }
                    }
                }
            }
        }
        konsulent::print_farge(format!("Lukker tilkobling til klient i send_post()"), Color::Yellow);
        Ok(())
    }
    
    /// 🔹 **Startar server for å sende worldview**
    pub async fn start_post_leveranse(&self, wv_channel: WorldViewChannel::WorldViewChannel, _shutdown_tx: broadcast::Sender<u8>) -> tokio::io::Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, config::PN_PORT)).await?;

        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    konsulent::print_farge(format!("{} Kobla til nyhetsbrevet!", socket.peer_addr().unwrap()), Color::Green);
                    let rx = wv_channel.tx.subscribe();
                    let self_clone = self.clone();
                    tokio::spawn(async move {
                        let peer_addr_copy = socket.peer_addr().unwrap();
                        match self_clone.send_post(socket, rx).await {
                            Ok(_) => {}
                            Err(e) => {
                                konsulent::print_farge(format!("Error i send_post til: {}: {}", peer_addr_copy, e), Color::Red);
                            }
                        }
                    });
                }
                Err(e) => {
                    konsulent::print_farge(format!("Feil i listener.accept(): {}", e), Color::Red);
                }
            }
        }
    }

    /// 🔹 **Startar `start_post_leveranse` i ei eiga oppgåve**
    pub fn start_post_leveranse_task(&self, wv_channel: WorldViewChannel::WorldViewChannel, shutdown_tx: broadcast::Sender<u8>) -> tokio::io::Result<()> {
        let self_clone = self.clone();
        
        tokio::spawn(async move {
            konsulent::print_farge("Starter nyhetsbrev-server (start_post_leveranse_task()".to_string(), Color::Green);
            if let Err(e) = self_clone.start_post_leveranse(wv_channel, shutdown_tx).await {
                konsulent::print_farge(format!("Feil i post_leveranse: {}", e), Color::Red);
            }
        });

        Ok(())
    }

    /// 🔹 **Klient som lyttar etter worldview-endringar frå master**
    pub async fn abboner_master_nyhetsbrev(&self, _shutdown_rx: broadcast::Receiver<u8>) -> tokio::io::Result<()> {
        println!("Prøver å koble på: {}:{} i abboner_master_nyhetsbrev()", *self.master_ip.lock().await, config::PN_PORT);
        let mut stream = TcpStream::connect(format!("{}:{}", *self.master_ip.lock().await, config::PN_PORT)).await?;
        
        konsulent::print_farge(format!("Har kobla til master på ip: {}:{} i abboner_master_nyhetsbrev()", *self.master_ip.lock().await, config::PN_PORT), Color::Green);

        // let master_id = konsulent::id_fra_ip(*self.master_ip.lock().await);
        // println!("Master ID: {}", master_id);
        // println!("Min ID: {}", self.id);

        loop {
            let mut len_bytes = [0u8; 4];
            let bytes_read = stream.read_exact(&mut len_bytes).await;
            
            if let Err(e) = bytes_read {
                konsulent::print_farge(format!("Serveren stengte tilkoblingen i get_wv_from_master() 2. Feil: {}", e), Color::Yellow);
                return Err(e); // Returnerer selve feilen
            }
            

            let len = u32::from_be_bytes(len_bytes) as usize;
            let mut buf = vec![0u8; len];
            let read = stream.read_exact(&mut buf).await;

            if let Err(e) = read {
                konsulent::print_farge("Serveren stengte tilkoblingen i get_wv_from_master() 3.".to_string(), Color::Yellow);
                return Err(e);
            }

            println!("Mottok melding i abboner_nyhetsbrev() på {} bytes: {:?} ", len, buf);
        }
    }
}
