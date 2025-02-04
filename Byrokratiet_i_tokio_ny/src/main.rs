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


use tokio::sync::broadcast;
use tokio::sync::Mutex;
use Byrokratiet_i_tokio_ny::WorldView::WorldViewChannel::request_worldview;
use std::sync::Arc;

/// Håndterer start-up initialisering av programrolle
///
/// # Argumenter
///
/// *  `kommando` - (string) master eller backup, initialiserer programmet som en primary eller backup
///
/// # Eksempel
///
/// ```
/// cargo run -- master -> Kjører primær prosess 
///  cargo run -- backup -> Feil: Andre argument må være et positivt heltall. (u8)
/// ```

#[tokio::main] // Hvis du bruker async/await
async fn main() {
    //ikkje testa
    env_logger::init();

    //Leiger utenladsk hjelp for å lage en sjef og initiell worldview
    let (sjefen, wv_serial_init) = konsulent::init_serialised_worldview().await;

    //gjør arc fordi lættis
    let worldview_arc = Arc::new(Mutex::new(wv_serial_init));
    
    //Init av tx til worldviewchannel
    let (tx, _) = broadcast::channel::<Vec<u8>>(1);
    let temp_rx_wv = tx.subscribe();
    let worldview_channel = WorldViewChannel::WorldViewChannel {tx: tx};
    
    
    
    
    // Lager worldview_sender
    let (shutdown_tx, _) = broadcast::channel::<>(1);
    //Oppdatterer worldview_channelen når request_worldview() blir kalt
    worldview_channel.clone().spawn_send_worldview(worldview_arc.clone(), shutdown_tx.clone()).await;
    
    
    
    //Kjører programmet
    loop {
        /*
        Om start_clean og start_from_worldview returnerer worldview
        kan vi loope sånn her når man må starte på nytt, kanskje lettere?
        worldview = sjefen.start_from_worldview(worldview);
        */
        match sjefen.start_from_worldview(worldview_channel.clone()).await {
            Ok(_) => {},
            Err(e) => {
                println!("feil: {}", e);
                shutdown_tx.send(1).expect("HORE2");
                
                break;
            }
        }
    }
    println!("I bunn av programmet");
    
}
















// Eks på henting av WorldView
// let mut worldview_rx = worldview_channel.tx.clone().subscribe();
// request_worldview().await;
// let temp_worldview = worldview_rx.recv().await;
// println!("fikk Worldview: {:?}", temp);