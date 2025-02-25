use std::{sync::atomic::{Ordering, AtomicBool}, time::Duration};

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

// Definer ein global `AtomicU8`
pub static IS_MASTER: AtomicBool = AtomicBool::new(false); // Startverdi 0


pub async fn tcp_handler(chs: local_network::LocalChannels, mut socket_rx: mpsc::Receiver<(TcpStream, SocketAddr)>) {
    let mut wv = utils::get_wv(chs.clone());
    loop {
        IS_MASTER.store(true, Ordering::SeqCst);
        /* Mens du er master: Motta sockets til slaver, start handle_slave i ny task*/
        while utils::is_master(chs.clone()) {
            if world_view_update::get_network_status().load(Ordering::SeqCst) {
                while let Ok((socket, addr)) = socket_rx.try_recv() {
                    let chs_clone = chs.clone();
                    utils::print_info(format!("Ny slave tilkobla: {}", addr));
                    //TODO: Legg til disse threadsa i en vec, så de kan avsluttes når vi ikke er master mer
                    let _slave_task: JoinHandle<()> = tokio::spawn(async move {
                        handle_slave(socket, chs_clone).await;
                    });
                    tokio::task::yield_now().await; //Denne tvinger tokio til å sørge for at alle tasks i kø blir behandler
                                                    //Feilen før var at tasken ble lagd i en loop, og try_recv kaltes så tett att tokio ikke rakk å starte tasken før man fikk en ny melding(og den fikk litt tid da den mottok noe)
                }                
            }
            else {
                tokio::time::sleep(Duration::from_millis(100)).await; 
            }
        }
        //mista master -> indiker for avslutning av tcp-con og tasks
        IS_MASTER.store(false, Ordering::SeqCst);


        // sjekker at vi faktisk har ein socket å bruke med masteren
        let mut master_accepted_tcp = false;
        let mut stream:Option<TcpStream> = None;
        if let Some(s) = connect_to_master(chs.clone()).await {
            master_accepted_tcp = true;
            stream = Some(s);
        }
        wv = utils::get_wv(chs.clone());

        /* Mens du er slave: Sjekk om det har kommet ny master / connection til master har dødd */
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

pub async fn listener_task(_chs: local_network::LocalChannels, socket_tx: mpsc::Sender<(TcpStream, SocketAddr)>) {
    let self_ip = format!("{}.{}", config::NETWORK_PREFIX, utils::SELF_ID.load(Ordering::SeqCst));
    while !world_view_update::get_network_status().load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    /* Binder listener til PN_PORT */
    let listener = match TcpListener::bind(format!("{}:{}", self_ip, config::PN_PORT)).await {
        Ok(l) => {
            utils::print_ok(format!("Master lytter på {}:{}", self_ip, config::PN_PORT));
            l
        }
        Err(e) => {
            utils::print_err(format!("Feil ved oppstart av TCP-listener: {}", e));
            return; // evt gå i sigel elevator mode
        }
    };

    /* Når listener accepter ny tilkobling -> send socket og addr til tcp_handler gjennom socket_tx */
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                utils::print_master(format!("{} kobla på TCP", addr));
                if socket_tx.try_send((socket, addr)).is_err() {
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

/// ### Håndtering av slave, koblet til `stream`
/// 
/// For nå leser den kun fra slaven.
async fn handle_slave(mut stream: TcpStream, chs: local_network::LocalChannels) {
    print_info("Handle slave har starta!".to_string());
    loop {
        match read_from_stream(&mut stream, chs.clone()).await {
            Some(msg) => {
                let received_data = msg;
                let _ = chs.mpscs.txs.container.send(received_data).await;
                //utils::print_info(format!("Melding frå slave: {:?}", received_data));
            }
            None => {
                break;
            }
        }
    }
}

/// ## Leser fra `stream`
/// 
/// Select mellom å lese melding fra slave og sende meldingen til `world_view_handler` og å avslutte streamen om du ikke er master
async fn read_from_stream(stream: &mut TcpStream, chs: local_network::LocalChannels) -> Option<Vec<u8>> {
    let mut len_buf = [0u8; 2];
    tokio::select! {
        result = stream.read_exact(&mut len_buf) => {
            match result {
                Ok(0) => {
                    utils::print_info("Slave har kopla fra.".to_string());
                    utils::print_info(format!("Stenger stream til slave 1: {:?}", stream.peer_addr()));
                    let id = utils::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                    let _ = chs.mpscs.txs.remove_container.send(id).await;
                    let _ = stream.shutdown().await;
                    return None;
                }
                Ok(_) => {
                    let len = u16::from_be_bytes(len_buf) as usize;
                    let mut buffer = vec![0u8; len];

                    match stream.read_exact(&mut buffer).await { 
                        Ok(0) => {
                            utils::print_info("Slave har kopla fra.".to_string());
                            utils::print_info(format!("Stenger stream til slave 2: {:?}", stream.peer_addr()));
                            let id = utils::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                            let _ = chs.mpscs.txs.remove_container.send(id).await;
                            let _ = stream.shutdown().await;
                            return None;
                        }
                        Ok(_) => return Some(buffer),
                        Err(e) => {
                            utils::print_err(format!("Feil ved mottak av data fra slave: {}", e));
                            utils::print_info(format!("Stenger stream til slave 3: {:?}", stream.peer_addr()));
                            let id = utils::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                            let _ = chs.mpscs.txs.remove_container.send(id).await;
                            let _ = stream.shutdown().await;
                            return None;
                        }
                    }
                }
                Err(e) => {
                    utils::print_err(format!("Feil ved mottak av data fra slave: {}", e));
                    utils::print_info(format!("Stenger stream til slave 4: {:?}", stream.peer_addr()));
                    let id = utils::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
                    let _ = chs.mpscs.txs.remove_container.send(id).await;
                    let _ = stream.shutdown().await;
                    return None;
                }
            }
        }
        _ = async {
            while IS_MASTER.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        } => {
            let id = utils::ip2id(stream.peer_addr().expect("Peer har ingen IP?").ip());
            utils::print_info(format!("Mistar masterstatus, stenger stream til slave {}", id));
            let _ = chs.mpscs.txs.remove_container.send(id).await;
            let _ = stream.shutdown().await;
            return None;
        }
    }
} 

/// ### Sender egen elevator_container til master gjennom stream
/// Sender på format : `(lengde av container) as u16`, `container`
pub async fn send_tcp_message(chs: local_network::LocalChannels, stream: &mut TcpStream) {
    let self_elev_container = utils::extract_self_elevator_container(chs.clone());

    let self_elev_serialized = world_view::serialize_elev_container(&self_elev_container);
    let len = (self_elev_serialized.len() as u16).to_be_bytes(); // Konverter lengde til big-endian bytes
   
    
    if let Err(e) = stream.write_all(&len).await {
        utils::print_err(format!("Feil ved sending av data til master: {}", e));
        let _ = chs.mpscs.txs.tcp_to_master_failed.send(true).await; // Anta at tilkoblingen feila
    }
    if let Err(e) = stream.write_all(&self_elev_serialized).await {
        utils::print_err(format!("Feil ved sending av data til master: {}", e));
        let _ = chs.mpscs.txs.tcp_to_master_failed.send(true).await; // Anta at tilkoblingen feila
    }
    if let Err(e) = stream.flush().await {
        utils::print_err(format!("Feil ved flushing av stream: {}", e));
        let _ = chs.mpscs.txs.tcp_to_master_failed.send(true).await; // Anta at tilkoblingen feila
    }
}
