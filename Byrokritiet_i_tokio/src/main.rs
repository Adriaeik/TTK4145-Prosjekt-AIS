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

use Byrokratiet_i_tokio::Byrokrati::Sjefen;
use Byrokratiet_i_tokio::Byrokrati::Tony;
use Byrokratiet_i_tokio::Byrokrati::konsulent;


use local_ip_address::local_ip;

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
    // TODO:: INititialiser ein strttup for x antall heisa.
    /*Initialiser ein sjefpakke basert på argument (Rolle, ID) */
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
    
    let ip = local_ip().expect("Kunne ikke hente IP");
    let mut ip_string = ip.to_string(); // Konverter til String
    ip_string.push_str(":6278"); //Port er fri





    /*Oprette programmet som sin rolle, visst master skal den 
    1) starte ein master_process 
    2) lage sin eigen backup 
    3) høre på broadcate og sjekke om det er mastera med lågare ID
    4) Dersom den har lågast ID skal den starte Bedriftspakker. 
    */
    let id = konsulent::id_fra_ip(ip);
    let sjefen = Sjefen::Sjefen {
        ip: ip,
        id: id,
        rolle: sjefenpakke.rolle,
    };

    if sjefen.rolle == Sjefen::Rolle::BACKUP {
        sjefen.backup_process().await;
    }
    else {
        sjefen.primary_process().await;
    }
}




