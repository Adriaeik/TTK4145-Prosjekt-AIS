use std::{sync::atomic::Ordering, time::Duration};

use termcolor::Color;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::net::TcpStream;

use crate::{config, utils, world_view::world_view_update};

use super::local_network;



pub async fn tcp_listener(self_id: u8, mut chs: local_network::LocalChannels) {
    let self_ip = format!("{}{}", config::NETWORK_PREFIX, self_id);

    
    while !world_view_update::get_network_status().load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(100)).await; 
    };

    let listener = TcpListener::bind(format!("{}:{}", self_ip, config::PN_PORT)).await;
    let mut listeners_tasks: Vec<JoinHandle<()>> = Vec::new();

    let mut wv = utils::get_wv(chs.clone());
    
    loop {
        let mut master_accepted_tcp = false;




        while utils::is_master(self_id, chs.clone()) {
            if world_view_update::get_network_status().load(Ordering::SeqCst) {
                // utils::print_master("Eg er master".to_string());
                // dele worldview som vi har laga
            }
            else {
                tokio::time::sleep(Duration::from_millis(100)).await; 
            }

            
        }
        //mista master -> skru av tasks i listener_tasks

        //koble til og legg til master i list
        wv = utils::get_wv(chs.clone());
        if world_view_update::get_network_status().load(Ordering::SeqCst){
            utils::print_info(format!("Prøver å koble på: {}.{}:{} i TCP_listener()", config::NETWORK_PREFIX, wv[config::MASTER_IDX], config::PN_PORT));
            let mut stream = TcpStream::connect(format!("{}.{}:{}", config::NETWORK_PREFIX, wv[config::MASTER_IDX], config::PN_PORT)).await;
            match stream {
                Ok(s) => {
                    utils::print_ok(format!("Har kobla på Master: {}.{}:{} i TCP_listener()", config::NETWORK_PREFIX, wv[config::MASTER_IDX], config::PN_PORT));
                    master_accepted_tcp = true;
                },
                Err(e) => {
                    utils::print_err(format!("Klarte ikke koble på master tcp: {}", e));
                    master_accepted_tcp = false;
                    match chs.mpscs.txs.tcp_to_master_failed.send(true).await {
                        Ok(_) => utils::print_info("Sa ifra at TCP til master feila".to_string()),
                        Err(err) => utils::print_err(format!("Feil ved sending til tcp_to_master_failed: {}", err)),
                    }
                }
            }
        }
        while !utils::is_master(self_id, chs.clone()) & master_accepted_tcp {
            if world_view_update::get_network_status().load(Ordering::SeqCst) {
                // utils::print_slave("Jeg er slave".to_string());
                // Snde din worldview vweed oppstart av connection
                // fortelle kva du har fullført eller ikkje fått til
                /* Mister slalven nettverk skal den fullføre sine tasks -> for så å fortsette i singel mode 
                    Altså trenger ikkje master å deligere deligerte meldinger på nytt*/
                // channel.motta.tasks //henter sine oppgåver fra WV på UDP
                // tcp_send(heis_konteiner) //: vec<Tasks>+statuser, nye_knappetrykk: vec<CallBtn>) //Send på fast frekvens, fungerer også som heartbeat
            }
            else {
                tokio::time::sleep(Duration::from_millis(100)).await; 
            }
            //Det slaven skal gjøre på TCP
        } 
        //ble master -> koble fra master  
      
    }

}


