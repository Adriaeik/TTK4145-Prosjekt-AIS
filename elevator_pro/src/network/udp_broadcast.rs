use crate::config;
use crate::utils;
use super::local_network;

use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use socket2::{Domain, Socket, Type};
use tokio::time;



pub async fn start_udp_broadcaster(txs: local_network::BroadcastTxs, min_id: u8) -> tokio::io::Result<()> {
    let mut rxs_org = txs.subscribe();
    let addr: &str = &format!("{}:{}", config::BC_ADDR, config::DUMMY_PORT);
    let addr2: &str = &format!("{}:0", config::BC_LISTEN_ADDR);

    let broadcast_addr: SocketAddr = addr.parse().expect("ugyldig adresse"); // UDP-broadcast adresse
    let socket_addr: SocketAddr = addr2.parse().expect("Ugyldig adresse");
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    
    
    socket.set_reuse_address(true)?;
    socket.set_broadcast(true)?;
    socket.bind(&socket_addr.into())?;
    let udp_socket = UdpSocket::from_std(socket.into())?;

    loop{
        let mut rxs = rxs_org.resubscribe();
        //Hent ut nyeste melding på wv_rx
        let msg = async {
            let mut latest_msg = None;
            while let Ok(message) = rxs.wv.try_recv() {
                latest_msg = Some(message); // Overskriv tidligere meldinger
            }
            latest_msg
        }.await; 
    
        if let None = msg {
            //utils::print_err("Ingen wv på rxs.wv_rx".to_string());
        }
        if let Some(message) = msg {
            //Bare broadcast hvis du er master
            //if min_id == message[config::MASTER_IDX] {
                let mesage = format!("{:?}{:?}", config::KEY_STR, message).to_string();
                udp_socket.send_to(mesage.as_bytes(), &broadcast_addr).await?;
                utils::print_ok(format!("Sender UDP-broadcast: {}", mesage));
            //}
        }
        time::sleep(Duration::from_millis(100)).await;
    }
}

pub async fn start_udp_listener(txs: local_network::BroadcastTxs) -> tokio::io::Result<()> {
    let mut rxs = txs.subscribe();
    let broadcast_listen_addr = format!("{}:{}", config::BC_LISTEN_ADDR, config::DUMMY_PORT);
    let socket_addr: SocketAddr = broadcast_listen_addr.parse().expect("Ugyldig adresse");
    let socket_temp = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    
    
    socket_temp.set_reuse_address(true)?;
    socket_temp.set_broadcast(true)?;
    socket_temp.bind(&socket_addr.into())?;
    let socket = UdpSocket::from_std(socket_temp.into())?;
    let mut buf = [0; 1024];

    loop {
        let len: usize;
        println!("Prøver å motta");
        match socket.recv_from(&mut buf).await {
            Ok((length, _)) => {
                len = length;
                println!("mottok noe med lenge {}", len);
            }
            Err(e) => {
                utils::print_err(format!("udp_broadcast.rs, udp_listener(): {}", e));
                return Err(e);
            }
        }

        println!("Sjekker om meldingen har key_string");
        if buf.starts_with(config::KEY_STR) {
            let key_len = config::KEY_STR.len();
            let remaining_len = buf.len() - key_len;

            // Flytt innhaldet framover
            buf.copy_within(key_len.., 0);

            // Fyll slutten med nullar
            buf[remaining_len..].fill(0);



        }


        let wv = async {
            let mut latest_msg = None;
            while let Ok(message) = rxs.wv.try_recv() {
                latest_msg = Some(message); // Overskriv tidligere meldinger
            }
            latest_msg
        }.await; 
    
        if let None = wv {
            utils::print_err("Ingen wv på rxs.wv_rx".to_string());
        }
        if let Some(mut my_wv) = wv {
            //Bare broadcast hvis du er master
            if my_wv[config::MASTER_IDX] > buf[config::MASTER_IDX] {
                //Oppdater egen WV
                my_wv = buf[..len].to_vec();
                utils::print_info(format!("Mottok UDP: {:?}", my_wv));
                //TODO: Send denne wv tilbake til thread som behandler worldview
            }
        }

    }
}

