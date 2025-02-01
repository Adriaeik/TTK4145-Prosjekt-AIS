//! Ansvar for å broadcaste worldview

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
    pub async fn listen_to_network(&self) -> tokio::io::Result<IpAddr> {
        
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


        if self.rolle == Sjefen::Rolle::SLAVE {
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
            //Send tilbake en none ellerno
            let a =  IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
            Ok(a)
        }
        else{
            match master_address {
                Some(addr) => Ok(addr.ip()), // Returnerer IpAddr hvis SocketAddr er Some
                None => Err(tokio::io::Error::new(tokio::io::ErrorKind::NotFound, "SocketAddr is None")), // Returnerer feil hvis SocketAddr er None
            }
        }


    }

    pub async fn start_broadcaster(&self, tx_is_master: mpsc::Sender<bool>, tx_master_ip: mpsc::Sender<SocketAddr>) -> tokio::io::Result<()> {
        //Første runde: hør etter kun én broadcast for å se om andre heiser er på nettverket!
    //     let mut master_address: Option<SocketAddr> = None;
    //     let mut message: Option<Cow<'_, str>> = None;

    //     let broadcast_listen_addr = format!("{}:{}", config::BC_LISTEN_ADDR, config::DUMMY_PORT);
    //     let socket_addr: SocketAddr = broadcast_listen_addr.parse().expect("Ugyldig adresse");
    //     let socket_temp = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        
        
    //     socket_temp.set_reuse_address(true)?;
    //     socket_temp.set_broadcast(true)?;
    //     socket_temp.bind(&socket_addr.into())?;
    //     let socket = UdpSocket::from_std(socket_temp.into())?;
    //     let mut buf = [0; 1024];


    //     if self.rolle == Sjefen::Rolle::SLAVE {
    //         //Hører etter broadcast i 1 sek, om ingenting mottatt -> start som mainmaster
    //         match timeout(Duration::from_secs(1), socket.recv_from(&mut buf)).await {
    //             Ok(Ok((len, addr))) => {
    //                 master_address = Some(addr);
    //                 message = Some(String::from_utf8_lossy(&buf[..len]));

    //                 println!("Mottok UPD-broadcast fra: {:?}", master_address);
    //             }
    //             Ok(Err(e)) => {
    //                 println!("Feil ved mottak initListenBroadcast (MrWorldwide.rs, start_broadcaster): {}", e);
    //                 return Err(e);
    //             }
    //             Err(_) => {
    //                 println!("Timeout! Ingen melding mottatt på 1 sekund. (MrWorldwide.rs, start_broadcaster)");
    //             }
    //         }
    //     }


    //     let mut empty_network = true;
    //     //Hvis meldingen man leser er "Gruppe23", så er ikke nettverket tomt
    //     //Muligens en bedre å gjøre dette på, så den ikke gir opp om første meldingen ikke er gruppe23
    //     //Hvis den ikke er det (enten noe annet eller ingenting) går den videre uten å gjøre noe
    //     match &message {
    //         Some(msg) if msg == "Gruppe25" => empty_network = false,
    //         None => {},
    //         _ => {eprintln!("Fikk melding, men ikke vår gruppe. hvis denne meldingen kommer sjekk kommentar over. noe må gjøres. (MrWorldwide.rs, start_broadcaster)");}
    //     }
        

    //     // starter å broadcaste egen id hvis nettverket er tomt 
    //     // Kobler seg til master på TCP hvis det er en master på nettverket
    //     if empty_network {
    //         tx_master_ip.send(socket_addr).await.unwrap(); //ubrukelig, så programmet ikke henger
    //         tx_is_master.send(true).await.unwrap();


    //         start_master_broadcaster(self.id).await?;
    //     }
    //     else{
    //         tx_master_ip.send(master_address.unwrap()).await.unwrap();
    //         tx_is_master.send(false).await.unwrap();
    //     }
        
        Ok(())
    }


    pub async fn start_master_broadcaster(&self) -> tokio::io::Result<()> {
        //Send melding til sjefen (bruk channel) at netverket er tomt, vi må gjøre det som trengs da
        let addr: &str = &format!("{}:{}", config::BC_ADDR, config::DUMMY_PORT); //🎯 
        let addr2: &str = &format!("{}:0", config::BC_LISTEN_ADDR);



        let broadcast_addr: SocketAddr = addr.parse().expect("ugyldig adresse"); // UDP-broadcast adresse
        let socket_addr: SocketAddr = addr2.parse().expect("Ugyldig adresse");
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        
        
        socket.set_reuse_address(true)?;
        socket.set_broadcast(true)?;
        socket.bind(&socket_addr.into())?;
        let udp_socket = UdpSocket::from_std(socket.into())?;
        
        
        
        loop {
            udp_socket.send_to("Gruppe25".to_string().as_bytes(), &broadcast_addr).await?;
            sleep(Duration::from_millis(100)).await;
            println!("Broadcaster ID: {}", "Gruppe25");
        }

    }
}












