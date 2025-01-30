//! Ansvar for å broadcaste worldview

use tokio::net::UdpSocket;
use tokio::time::{sleep, timeout, Duration};
use std::net::SocketAddr;
use std::borrow::Cow;



pub async fn start_broadcaster(id: &str) -> tokio::io::Result<()> {
    //Første runde: hør etter kun én broadcast for å se om andre heiser er på nettverket!
    let mut master_address: Option<SocketAddr> = None;
    let mut message: Option<Cow<'_, str>> = None;
    
    
    let broadcast_listen_addr = "0.0.0.0:42069";
    let socket = UdpSocket::bind(broadcast_listen_addr).await?;
    let mut buf = [0; 1024];


    //Hører etter broadcast i 1 sek, om ingenting mottatt -> start som mainmaster
    match timeout(Duration::from_secs(1), socket.recv_from(&mut buf)).await {
        Ok(Ok((len, addr))) => {
            master_address = Some(addr);
            message = Some(String::from_utf8_lossy(&buf[..len]));
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
        Some(msg) if msg == "Gruppe23" => empty_network = false,
        None => {},
        _ => {eprintln!("Fikk melding, men ikke vår gruppe. hvis denne meldingen kommer sjekk kommentar over. noe må gjøres. (MrWorldwide.rs, start_broadcaster)");}
    }
    

    // starter å broadcaste egen id hvis nettverket er tomt 
    // Kobler seg til master på TCP hvis det er en master på nettverket
    if empty_network {
        start_master_broadcaster(id).await?;
    }
    else{
        connect_to_master_TCP(Option::expect(master_address, "Burde aldri skrives. Denne kjøres kun om vi har fått en adresse (MrWorldWide.rs, start_broadcaster())")).await?;
    }
    
    Ok(())
}


async fn start_master_broadcaster(id: &str) -> tokio::io::Result<()> {
    let addr: &str = "255.255.255.255:42069"; 
    let broadcast_addr: SocketAddr = addr.parse().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, e)
    })?; // UDP-broadcast adresse
    

    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    socket.set_broadcast(true)?;

    loop {
        socket.send_to("Gruppe23".to_string().as_bytes(), &broadcast_addr).await?;
        sleep(Duration::from_millis(100)).await;
        println!("Broadcaster ID: {}", "Gruppe23");
    }

}

async fn connect_to_master_TCP(addr: SocketAddr) -> tokio::io::Result<()> {
    //Koble til master her og hent worldview
    //Finne en måte å svare med egen ID om den er lavere enn master sin
    println!("Her skal jeg koble til master på TCP; addresse: {}:?", addr);

    Ok(())
}











