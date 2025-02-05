///! Broadcaster sin IP til heile verden, så alle kan sjå
use tokio::net::UdpSocket;
use tokio::time::{sleep, timeout, Duration};
use std::net::SocketAddr;
use std::borrow::Cow;
use socket2::{Socket, Domain, Type};
use tokio::sync::broadcast;
use termcolor::Color;

use crate::config;
use crate::Byrokrati::konsulent;

use super::Sjefen;



impl Sjefen::Sjefen {
    pub async fn listen_to_network(&self) -> tokio::io::Result<()> {
            
        let mut master_address: Option<SocketAddr> = None;
        let mut message: Option<Cow<'_, str>> = None;

        let broadcast_listen_addr = format!("{}:{}", config::BC_LISTEN_ADDR, config::DUMMY_PORT);
        let socket_addr: SocketAddr = broadcast_listen_addr.parse().expect("Ugyldig adresse");
        let socket_temp = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        
        
        socket_temp.set_reuse_address(true)?;
        socket_temp.set_broadcast(true)?;
        socket_temp.bind(&socket_addr.into())?;
        let socket = UdpSocket::from_std(socket_temp.into())?;
        let mut buf = [0; 1024];


        //Hører etter broadcast i 1 sek, om ingenting mottatt -> start som mainmaster
        match timeout(Duration::from_secs(1), socket.recv_from(&mut buf)).await {
            Ok(Ok((len, addr))) => {
                master_address = Some(addr);
                message = Some(String::from_utf8_lossy(&buf[..len]));

                println!("Mottok UPD-broadcast fra: {:?}", master_address);
            }
            Ok(Err(e)) => {
                konsulent::print_farge(format!("Feil ved mottak initListenBroadcast (MrWorldwide.rs, start_broadcaster): {}", e), Color::Red);
                return Err(e);
            }
            Err(_) => {
                println!("Timeout! Ingen melding mottatt på 1 sekund. (MrWorldwide.rs, start_broadcaster) \r\n");
            }
        }
        

        let mut empty_network = true;
        //Hvis meldingen man leser er "Gruppe23", så er ikke nettverket tomt
        //Muligens en bedre å gjøre dette på, så den ikke gir opp om første meldingen ikke er gruppe23
        //Hvis den ikke er det (enten noe annet eller ingenting) går den videre uten å gjøre noe
        match &message {
            Some(msg) if msg == "Gruppe25" => empty_network = false,
            None => {},
            _ => {konsulent::print_farge("Fikk melding, men ikke vår gruppe. hvis denne meldingen kommer sjekk kommentar over. noe må gjøres. (MrWorldwide.rs, start_broadcaster)".to_string(), Color::Red);}
        }
        

        // starter å broadcaste egen id hvis nettverket er tomt 
        // Kobler seg til master på TCP hvis det er en master på nettverket
        if empty_network {
            let mut master_ip = self.master_ip.lock().await;
            *master_ip = self.ip;
            Ok(())
        }
        else{
            match master_address {
                Some(addr) => {
                    let mut m_ip = self.master_ip.lock().await;
                    *m_ip = addr.ip();
                    Ok(())
                }, // Returnerer IpAddr hvis SocketAddr er Some
                None => Err(tokio::io::Error::new(tokio::io::ErrorKind::NotFound, "SocketAddr is None")), // Returnerer feil hvis SocketAddr er None
            }
        }
    }
    
    
    pub async fn start_udp_broadcast(&self, mut shutdown_rx: broadcast::Receiver<u8>) -> tokio::io::Result<()> {
        //Send melding til sjefen (bruk channel) at netverket er tomt, vi må gjøre det som trengs da
        let addr: &str = &format!("{}:{}", config::BC_ADDR, config::DUMMY_PORT); //🎯 
        let addr2: &str = &format!("{}:0", config::BC_LISTEN_ADDR);

        

        let broadcast_addr: SocketAddr = addr.parse().expect("ugyldig adresse i start_udp_broadcast()"); // UDP-broadcast adresse
        let socket_addr: SocketAddr = addr2.parse().expect("Ugyldig adresse i start_udp_broadcast()");
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        
        
        socket.set_reuse_address(true)?;
        socket.set_broadcast(true)?;
        socket.bind(&socket_addr.into())?;
        let udp_socket = UdpSocket::from_std(socket.into())?;
        
        
        loop {
            tokio::select! {
                // 🔹 Hovudoppgåve: Send UDP-meldingar
                _ = async {
                udp_socket.send_to("Gruppe25".as_bytes(), &broadcast_addr).await?;
                    sleep(Duration::from_millis(100)).await;
                    //println!("Broadcaster ID: Gruppe25");
                    Ok::<(), tokio::io::Error>(())
                } => {},
                
                // 🔹 Shutdown: Stoppar broadcasting om signalet kjem
                _ = shutdown_rx.recv() => {
                    println!("Shutdown mottatt! Stoppar UDP-broadcast...");
                    break;
                }
            }
        }
        Ok(())
        
    }
    
    
    pub fn start_udp_broadcast_task(&self, shutdown_tx: broadcast::Sender<u8>) -> tokio::task::JoinHandle<()> {
        let self_clone = self.clone();
        
        tokio::spawn(async move {
            konsulent::print_farge("Starter å sende UDP-broadcast".to_string(), Color::Green);
            println!("Starter å sende UDP-broadcast");
    
            if let Err(e) = self_clone.start_udp_broadcast(shutdown_tx.clone().subscribe()).await {
                eprintln!("Feil i UDP-broadcast: {}", e);
                konsulent::print_farge(format!("Feil i UDP-broadcast: {}", e), Color::Red);
            }
        })
    }
    

    
    
    
}