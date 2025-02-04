//! Sikrer at fine wordviewpakker blir sendt dit de skal :)


use crate::config;
use crate::WorldView::WorldView;
use crate::WorldView::WorldViewChannel;


use std::net::SocketAddr;
use std::time::Duration;
use std::u8;
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::broadcast;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::net::IpAddr;
use super::Sjefen;
use super::konsulent;




impl Sjefen::Sjefen {
    pub async fn get_wv_from_master(&self) ->  Option<Vec<u8>> {
        let master_ip = self.master_ip.lock().await;
        
        println!("Prøver å koble på: {}:{}", *master_ip, config::PN_PORT);
        //NB!!!!
        // Må teste litt på sanntidslabben om riktig ip blir sent i udp_broadcasten, eller om man må sende den som en melding i udp broadcasten
        let stream = TcpStream::connect(format!("{}:{}", *master_ip, config::PN_PORT)).await;
        let mut Tcp_stream: TcpStream;
        match stream {
            Ok(strm) => {Tcp_stream = strm;},
            Err(e) => {
                println!("Klarte ikke koble på TCP: {}", e);
                return None
            }            
        }

        
        let mut buf = [0; 1024];
        println!("Har kobla til en master på ip: {}:{}", *master_ip, config::PN_PORT);



        let bytes_read = Tcp_stream.read(&mut buf).await.expect("Panick etter å lese TCPstrem (PostNord.rs, get_wv_from_master())");
        if bytes_read == 0 {
            println!("Serveren stengte tilkoblingen.");
            return None
        }
        let worldview: Vec<u8> = buf[..bytes_read].to_vec(); 
        Some(worldview)
    }


        //postman Pat
    pub async fn send_post(&self, mut socket: tokio::net::TcpStream, mut rx: broadcast::Receiver<Vec<u8>>) -> tokio::io::Result<()> {  

        let mut buf = [0; 1024];
        loop {
            
            tokio::select! {
                msg = rx.recv() => match msg {
                    Ok(wv_msg) => {
                        println!("msg: {:?}", &wv_msg[..]);
                        let len_b = (wv_msg.len() as u32).to_be_bytes();
                        socket.write_all(&len_b).await?;
                        socket.write_all(&wv_msg[..]).await?;


                        // if let Err(e) = socket.write_all(&wv_msg[..]).await {
                        //     eprintln!("feil ved sending til klient i send_post: {} ",e);
                        //     return Err(e);
                        // }
                    }
                    Err(e) =>{
                        eprint!("Feil ved mottak fra broadcast kanal (wv_rx): {}", e);
                    }
                },  
        
                // Les ack
                result = socket.read(&mut buf) => match result {
                    Ok(0) =>{
                        //TODO:: oppdater worldview om dette
                        println!("TCP er lukket av slave");
                        return Ok(());
                    }
                    Ok(_) => {
                        println!("Mottok fra klienten: {}", String::from_utf8_lossy(&buf));
                    }
                    Err(e) => {
                        eprintln!("Feil ved lesing frå slaven: {}", e);
                        return Err(e);
                    }
                }
            }
        }
        
    }

    
    pub async fn start_post_leveranse(&self, wv_channel: WorldViewChannel::WorldViewChannel, shutdown_tx: broadcast::Sender<u8>) -> tokio::io::Result<()>{
        
        /*sette opp tcp listen 
        for hver som kobler seg opp:
        lag funksjon, kjør i ny task som:
        sender ut på TCPen hver gang rx'en får melding (worldview)
        */

        let listener = TcpListener::bind(format!("{}:{}", self.ip, config::PN_PORT)).await?;
        let mut tasks = Vec::new();
        loop {
            let mut shutdown_rx = shutdown_tx.subscribe();
            let self_clone = self.clone();
            tokio::select! {
                // 🟢 Prøver å godta nye tilkoblingar, men handterer feil
                accept_result = listener.accept() => match accept_result {
                    Ok((socket, addr)) => {
                        println!("🟢 Ny klient tilkobla frå: {}", addr);
            
                        let wv_rx = wv_channel.tx.clone().subscribe(); // Klon wv tx til rx
                        let self_clone = self.clone();
            
                        let task = tokio::spawn(async move {
                            if let Err(e) = self_clone.send_post(socket, wv_rx).await {
                                eprintln!("❌ En av slavene kobla seg av: {}", e);
                            }
                        });
            
                        tasks.push(task); // Lagrar tasken i vektor slik at den kan avsluttast
                    }
                    Err(e) => {
                        eprintln!("⚠️ Feil ved akseptering av ny tilkobling: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await; // Hindre umiddelbar retry-loop
                    }
                },
                
                _ = shutdown_rx.recv() => {
                    println!("Shutdown mottatt! Avsluttar alle tasks.");
                    for task in &tasks {
                        task.abort(); // Avbryt alle tasks
                    }
            
                    for task in tasks {
                        let _ = task.await; // Ventar på at dei avsluttar seg sjølv
                    }
            
                    println!("Alle tasks avslutta. Server shutdown.");
                    break Ok(());
                }
            }
        }

    }


    pub fn start_post_leveranse_task(&self, wv_channel: WorldViewChannel::WorldViewChannel, shutdown_tx: broadcast::Sender<u8>) -> tokio::io::Result<()>{
        let self_clone = self.clone();
        
        tokio::spawn(async move {
            println!("Starter nyhetsbrev");
    
            if let Err(e) = self_clone.start_post_leveranse(wv_channel.clone(), shutdown_tx.clone()).await {
                eprintln!("Feil i post_leveranse: {}", e);
            }
        });
        Ok(())
    }




/* Slave stuff her nede */
    pub async fn abboner_master_nyhetsbrev(&self, shutdown_rx: broadcast::Receiver<u8>) -> tokio::io::Result<()> {
        // let my_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap(); //kjent
        // let master_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap(); //kjent fra udp broadcast

        //les inn string til master ip fra channel her først

        
        println!("Prøver å koble på: {}:{}", *self.master_ip.lock().await, config::PN_PORT);
        //NB!!!!
        // Må teste litt på sanntidslabben om riktig ip blir sent i udp_broadcasten, eller om man må sende den som en melding i udp broadcasten
        let mut stream = TcpStream::connect(format!("{}:{}", *self.master_ip.lock().await, config::PN_PORT)).await?;
        // let mut buf = [0; 1024];
        println!("Har kobla til en master på ip: {}:{}", *self.master_ip.lock().await, config::PN_PORT);


        let master_id = konsulent::id_fra_ip(*self.master_ip.lock().await);

        println!("Masteren har id: {}", master_id);

        println!("Jeg har id: {}", self.id);


        loop {
            let mut len_bytes = [0u8; 4]; 
            let bytes_read = stream.read_exact(&mut len_bytes).await?;
            if bytes_read == 0 {
                println!("Serveren stengte tilkoblingen.");
                break;
            }else{
                // let message = String::from_utf8_lossy(&buf[..bytes_read]);
                let len = u32::from_be_bytes(len_bytes) as usize; //Konverter til `usize`
                let mut buf = vec![0u8; len]; // Lag buffer med riktig lengde

                stream.read_exact(&mut buf).await?;
                println!(" Melding fra server: {:?}", &buf);
                if self.id < master_id {
                    println!("Jeg har lavere ID enn master, jeg må bli master!!!!");
                    //Må kanskje passe på å lukke tidligere tråder?
                    *self.master_ip.lock().await = self.ip;
                    break;
                }
            }
        }

        Ok(())
    }
}