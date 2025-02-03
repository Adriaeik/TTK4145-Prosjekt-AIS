//! Sikrer at fine wordviewpakker blir sendt dit de skal :)


use crate::config;
use crate::WorldView::WorldView;


use std::net::SocketAddr;
use std::time::Duration;
use std::u8;
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::broadcast;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::net::IpAddr;
use super::Sjefen;
use super::konsulent;




impl Sjefen::Sjefen {
    pub async fn get_wv_from_master(&self) ->  Option<Vec<u8>> {
        let master_ip = self.master_ip.lock().await;
        
        println!("Prøver å koble på: {}:{}", *master_ip, config::PN_PORT);
        //NB!!!!
        // Må teste litt på sanntidslabben om riktig ip blir sent i udp_broadcasten, eller om man må sende den som en melding i udp broadcasten
        let stream = TcpStream::connect(format!("{}:{}", *master_ip, config::PN_PORT)).await;
        let mut Tcp_stream: TcpStream;
        match stream {
            Ok(strm) => {Tcp_stream = strm;},
            Err(e) => {
                println!("Klarte ikke koble på TCP: {}", e);
                return None
            }            
        }

        
        let mut buf = [0; 1024];
        println!("Har kobla til en master på ip: {}:{}", *master_ip, config::PN_PORT);



        let bytes_read = Tcp_stream.read(&mut buf).await.expect("Panick etter å lese TCPstrem (PostNord.rs, get_wv_from_master())");
        if bytes_read == 0 {
            println!("Serveren stengte tilkoblingen.");
            return None
        }
        let worldview: Vec<u8> = buf[..bytes_read].to_vec(); 
        Some(worldview)
    }




    
}