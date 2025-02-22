use std::{sync::atomic::Ordering, time::Duration};

use termcolor::Color;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::{io::AsyncReadExt, net::TcpListener};
use tokio::task::JoinHandle;
use tokio::net::TcpStream;
use std::net::SocketAddr;

use crate::world_view::world_view;
use crate::{config, utils, world_view::world_view_update};
use utils::{print_info, print_ok, print_err, get_wv};


use super::local_network;



pub async fn tcp_listener(mut chs: local_network::LocalChannels) {

    let (socket_tx, mut socket_rx) = mpsc::channel::<(TcpStream, SocketAddr)>(8);

    let chs_listener = chs.clone();
    let listener_handle = tokio::spawn(async move {
        utils::print_info("Starter tcp listener".to_string());
        let _ = listener_task(chs_listener, socket_tx).await;
    });

    let mut wv = utils::get_wv(chs.clone());
    
    loop {
        let mut master_accepted_tcp = false;
        chs.resubscribe_broadcast();




        while utils::is_master(chs.clone()) {
            if world_view_update::get_network_status().load(Ordering::SeqCst) {
                if let Ok((socket, addr)) = socket_rx.try_recv() {
                    utils::print_info(format!("Ny slave tilkobla: {}", addr));
                
                    let chs_clone = chs.clone();

                    //TODO: Legg til disse threadsa i en vec, så de kan avsluttes når vi ikke er master mer
                    tokio::spawn(async move {
                        handle_slave(socket, chs_clone).await;
                    });
                }                
            }
            else {
                tokio::time::sleep(Duration::from_millis(100)).await; 
            }

            
        }


        //mista master -> skru av tasks i listener_tasks
        // sjekker at vi faktisk har ein socket å bruke med masteren
        let mut master_accepted_tcp = false;
        let mut stream:Option<TcpStream> = None;
        if let Some(s) = connect_to_master(chs.clone()).await {
            master_accepted_tcp = true;
            stream = Some(s);
        }
        wv = utils::get_wv(chs.clone());
        while !utils::is_master(chs.clone()) && master_accepted_tcp {
            let prev_master = wv[config::MASTER_IDX];
            wv = utils::get_wv(chs.clone());
            let new_master = prev_master != wv[config::MASTER_IDX];
                
            
            if world_view_update::get_network_status().load(Ordering::SeqCst) {
                // utils::print_slave("Jeg er slave".to_string());
                if let Some(ref mut s) = stream {
                    if new_master {
                        println!("Fått ny master");

                        utils::close_tcp_stream(s).await;
                        tokio::time::sleep(Duration::from_millis(10)).await; //TODO: test om denne trengs
                        master_accepted_tcp = false;
                    }

                    send_tcp_message(chs.clone(), s).await;
                    //TODO: lag bedre delay
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }


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



/// Forsøker å koble til master via TCP.
/// Returnerer `Some(TcpStream)` ved suksess, `None` ved feil.
async fn connect_to_master(chs: local_network::LocalChannels) -> Option<TcpStream> {
    let wv = get_wv(chs.clone());

    if world_view_update::get_network_status().load(Ordering::SeqCst) {
        let master_ip = format!("{}.{}:{}", config::NETWORK_PREFIX, wv[config::MASTER_IDX], config::PN_PORT);
        print_info(format!("Prøver å koble på: {} i TCP_listener()", master_ip));

        match TcpStream::connect(&master_ip).await {
            Ok(stream) => {
                print_ok(format!("Har kobla på Master: {} i TCP_listener()", master_ip));
                Some(stream)
            }
            Err(e) => {
                print_err(format!("Klarte ikke koble på master tcp: {}", e));

                match chs.mpscs.txs.tcp_to_master_failed.send(true).await {
                    Ok(_) => print_info("Sa ifra at TCP til master feila".to_string()),
                    Err(err) => print_err(format!("Feil ved sending til tcp_to_master_failed: {}", err)),
                }

                None
            }
        }
    } else {
        None
    }
}

pub async fn listener_task(chs: local_network::LocalChannels, socket_tx: mpsc::Sender<(TcpStream, SocketAddr)>) {
    let self_ip = format!("{}.{}", config::NETWORK_PREFIX, utils::SELF_ID.load(Ordering::SeqCst));

    
    while !world_view_update::get_network_status().load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }


    let listener = match TcpListener::bind(format!("{}:{}", self_ip, config::PN_PORT)).await {
        Ok(l) => {
            utils::print_ok(format!("Master lyttar på {}:{}", self_ip, config::PN_PORT));
            l
        }
        Err(e) => {
            utils::print_err(format!("Feil ved oppstart av TCP-listener: {}", e));
            return; // evt gå i sigel elevator mode
        }
    };

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                utils::print_master(format!("{} kobla på TCP", addr));
                if socket_tx.send((socket, addr)).await.is_err() {
                    utils::print_err("Hovudløkken har stengt, avsluttar listener.".to_string());
                    break;
                }
            }
            Err(e) => {
                utils::print_err(format!("Feil ved tilkobling av slave: {}", e));
            }
        }
    }
}

async fn handle_slave(mut stream: TcpStream, mut chs: local_network::LocalChannels) {
    let mut buffer = vec![0; 1024];

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                utils::print_info("Slave har kopla frå.".to_string());
                break;
            }
            Ok(n) => {
                let received_data = &buffer[..n];
                utils::print_info(format!("Melding frå slave: {:?}", received_data));

                if let Err(e) = stream.write_all(b"Ack\n").await {
                    utils::print_err(format!("Feil ved sending til slave: {}", e));
                    break;
                }
            }
            Err(e) => {
                utils::print_err(format!("Feil ved mottak av data frå slave: {}", e));
                break;
            }
        }
    }
}

pub async fn send_tcp_message(chs: local_network::LocalChannels, s: &mut TcpStream) {
    let self_elev_container = utils::extract_self_elevator_container(chs.clone());


    if let Err(e) = s.write_all(&world_view::serialize_elev_container(&self_elev_container)).await {
        utils::print_err(format!("Feil ved sending av data til master: {}", e));
        let _ = chs.mpscs.txs.tcp_to_master_failed.send(true).await; // Anta at tilkoblingen feila
    }
    else{
        utils::print_info("Sendte elevator_container til master".to_string());
    }
}
