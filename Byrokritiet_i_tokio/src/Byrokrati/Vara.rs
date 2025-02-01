//! slave
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use super::PostNord;
use std::net::IpAddr;


pub async fn vara_process(self_ip: IpAddr, master_ip: SocketAddr){


    
    tokio::spawn(async move {
        match PostNord::abboner_master_nyhetsbrev(master_ip, self_ip).await {
            Ok(_) => {},
            Err(e) => eprintln!("Feil i PostNord::abboner_master_nyhetsbrev: {}", e),  
        }
    });
    

    loop {
        sleep(Duration::from_millis(100)).await; // Sover i 1 sekund
        //println!("Jeg lever i sjefen.rs primary_process loop");
    }
    

}



