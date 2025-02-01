//! slave
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use super::PostNord;


pub async fn vara_process(ip: &str, master_ip: SocketAddr){


    
    let ip_copy = ip.to_string();
    tokio::spawn(async move {
        match PostNord::abboner_master_nyhetsbrev(master_ip, &ip_copy).await {
            Ok(_) => {},
            Err(e) => eprintln!("Feil i PostNord::abboner_master_nyhetsbrev: {}", e),  
        }
    });
    

    loop {
        sleep(Duration::from_millis(100)).await; // Sover i 1 sekund
        //println!("Jeg lever i sjefen.rs primary_process loop");
    }
    

}



