/*Må lage TCP server -> 3 programmer snakker, alle kjører samme, automatisk velger ut en master
Programmene lager også en lokal backup i tilfelle hvor programmet krasjer eller lignende -> skriver ut krasjreport til en fil for debugging?
De 2 andre programmene oppdateres på en smart måte fra masteren, om connection dør mellom programmene velger de automatisk en ny master osv...
-> Bør være grunnmuren til det vi trenger for backup-systemene våre
Viktige ting å teste:
TCP timeout må fikses så det skjer smooth og automatisk
Programkrasj i master og backups må testes og fikses automatisk
planen er at lokal master/backup snakker over TCP

Vi trenger 2 av 3 args til programets startup:
: master
: backup
: ID

cargo run -- master ID -> lager ett av hovedprogrammene på en PC med ID (laveste ID blir master)
cargo run -- backup ID -> lager en lokal backup som vil få ID om den tar over
*/

use Byrokratiet_i_tokio_ny::Byrokrati::Sjefen;
use Byrokratiet_i_tokio_ny::Byrokrati::konsulent;
use Byrokratiet_i_tokio_ny::WorldView::WorldView;
use Byrokratiet_i_tokio_ny::WorldView::WorldViewChannel;


use local_ip_address::local_ip;

use tokio::sync::broadcast;
use tokio::sync::Mutex;
use Byrokratiet_i_tokio_ny::WorldView::WorldViewChannel::request_worldview;
use std::sync::Arc;

/// Håndterer start-up initialisering av programrolle
///
/// # Argumenter
///
/// *  `kommando` - (string) master eller backup, initialiserer programmet som en primary eller backup
/// * `ID` - (u8) initialiserer in ID assosiert med programmet
///
/// # Eksempel
///
/// ```
/// cargo run -- master 1 -> Kjører primær prosess med ID: 1
///  cargo run -- backup 256 -> Feil: Andre argument må være et positivt heltall. (u8)
/// ```

#[tokio::main] // Hvis du bruker async/await
async fn main() {
    //ikkje testa
    env_logger::init();
    // Initialiserer en sjefen struct
    /*Initialiser ein sjefpakke basert på argument (Rolle) */
    let sjefenpakke = match Sjefen::hent_sjefpakke() {
        Ok(sjef) => {
            println!("Opprettet SjefPakke: {:?}", sjef);
            sjef // Returner sjefen dersom alt gjekk bra
        }
        Err(e) => {
            eprintln!("Feil ved henting av SjefPakke: {}", e);
            return; // Avslutt programmet dersom ein feil oppstod
        }
    };
    //Finne IP :)
    let ip = match local_ip() {
        Ok(ip) => {
            ip
        }
        Err(e) => {
            eprintln!("Fant ikke IP (main.rs): {}", e);
            return;
        }
    }; 
    let id = konsulent::id_fra_ip(ip);
    let sjefen = Sjefen::Sjefen {
        ip: ip,
        id: id,
        rolle: Arc::new(Mutex::new(sjefenpakke.rolle)),
        master_ip: Arc::new(Mutex::new(ip)),
    };

    //Init til worldview
    let serialized_worldview = sjefen.start_clean().await;
    let worldview_arc = Arc::new(Mutex::new(serialized_worldview));
    
    //Init av tx til worldviewchannel
    let (tx, _) = broadcast::channel::<Vec<u8>>(1);
    let tx_arc = Arc::new(tx);
    let worldview_channel = WorldViewChannel::WorldViewChannel {tx: tx_arc};
    let worldview_channel_clone = worldview_channel.clone();
    
    
    
    
    // Eks på henting av WorldView
    // let mut worldview_rx = worldview_channel.tx.clone().subscribe();
    // request_worldview().await;
    // let temp_worldview = worldview_rx.recv().await;
    // println!("fikk Worldview: {:?}", temp);
    
    // Lager worldview_sender
    let worldview_arc_clone = worldview_arc.clone();
    let worldview_sender_task = tokio::spawn(async move {
        // Denne koden kjører i den asynkrone oppgaven (tasken)
        worldview_channel.send_worldview(worldview_arc_clone).await;
    });
    //Kjører programmet
    loop {
        /*
        Om start_clean og start_from_worldview returnerer worldview
        kan vi loope sånn her når man må starte på nytt, kanskje lettere?
        worldview = sjefen.start_from_worldview(worldview);
         */

        match sjefen.start_from_worldview(worldview_channel_clone).await {
            Ok(_) => {},
            Err(e) => println!("feil: {}", e),
        }
    }

}





