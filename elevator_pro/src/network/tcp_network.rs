use std::{sync::atomic::Ordering, time::Duration};

use termcolor::Color;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
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
        while utils::is_master(self_id, chs.clone()) {
            if world_view_update::get_network_status().load(Ordering::SeqCst) {
                // utils::print_master("Eg er master".to_string());
            }
            else {
                tokio::time::sleep(Duration::from_millis(100)).await; 
            }

            
        }
        //mista master -> skru av tasks i listener_tasks

        //koble til og legg til master i list
        while !utils::is_master(self_id, chs.clone()) {
            if world_view_update::get_network_status().load(Ordering::SeqCst) {
                // utils::print_slave("Jeg er slave".to_string());
            }
            else {
                tokio::time::sleep(Duration::from_millis(100)).await; 
            }
            //Det slaven skal gjøre på TCP
        } 
        //ble master -> koble fra master  
      
    }

}