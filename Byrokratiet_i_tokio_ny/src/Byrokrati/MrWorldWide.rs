///! Broadcaster sin IP til heile verden, så alle kan sjå
use tokio::net::UdpSocket;
use tokio::time::{sleep, timeout, Duration};
use std::net::{IpAddr, SocketAddr};
use std::borrow::Cow;
use socket2::{Socket, Domain, Type};
use tokio::sync::mpsc;
use std::net::{Ipv4Addr};

use crate::config;

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
                println!("Feil ved mottak initListenBroadcast (MrWorldwide.rs, start_broadcaster): {}", e);
                return Err(e);
            }
            Err(_) => {
                println!("Timeout! Ingen melding mottatt på 1 sekund. (MrWorldwide.rs, start_broadcaster)");
            }
        }
        

        let mut empty_network = true;
        //Hvis meldingen man leser er "Gruppe23", så er ikke nettverket tomt
        //Muligens en bedre å gjøre dette på, så den ikke gir opp om første meldingen ikke er gruppe23
        //Hvis den ikke er det (enten noe annet eller ingenting) går den videre uten å gjøre noe
        match &message {
            Some(msg) if msg == "Gruppe25" => empty_network = false,
            None => {},
            _ => {eprintln!("Fikk melding, men ikke vår gruppe. hvis denne meldingen kommer sjekk kommentar over. noe må gjøres. (MrWorldwide.rs, start_broadcaster)");}
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


}