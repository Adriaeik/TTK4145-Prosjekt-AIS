//! sjefen handterer opretting av master / backup
//! 
use super::IT_Roger;

use tokio::time::{sleep, Duration, Instant, interval};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::env;
use std::process::Command;
use std::fs::OpenOptions;
use std::io::Write;
use socket2::{Socket, Domain, Type, Protocol};
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct AnsattPakke {
    pub addr: String,
    pub num_floors: u8,
    pub name: String,
}

/// De ulike rollene programmet kan ha:
/// 
/// MASTER er for den serveren med ansvar 
/// 
/// SLAVE er masterprogrammet som ikke har 'token'
/// 
/// BACKUP er det lokale backupprogrammet
#[derive(Clone, PartialEq, Debug)]
pub enum Rolle {
    MASTER,
    SLAVE,
    BACKUP,
}


/// Pakke med Rolle (se Rolle) og ID (siste tall i IP) for en sjef
#[derive(Clone, Debug)]
pub struct SjefPakke {
    pub rolle: Rolle,
    pub id: u8,
    // TODO: Lage IP
}

/// Hentar og analyserer argument frå kommandolinja for å returnere ein `SjefPakke`
///
/// # Eksempel
/// ```
/// let sjefenpakke = match hent_sjefpakke() {
/// Ok(sjef) => {sjef}
/// Err(e) => {
///     eprintln!("Feil ved henting av SjefPakke: {}", e);
///     return;
///     }
/// };
/// ```
pub fn hent_sjefpakke() -> Result<SjefPakke, &'static str> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 {
        let command = args[1].to_lowercase();
        let id: u8 = match args[2].parse() {
            Ok(num) => num,
            Err(_) => return Err("Andre argument må være et positivt heltall. (u8)"),
        };

        let rolle = match command.as_str() {
            "backup" => Rolle::BACKUP,
            "master" => Rolle::MASTER,
            _ => return Err("Ugyldig kommando. Bruk 'backup' eller 'master'."),
        };

        Ok(SjefPakke { id, rolle })
    } else {
        Err("Bruk: <program> <kommando> <ID>")
    }
}


// denne funksjonen må inn i impl sjefen for å få tilgang til egen id og worldview og alt andre etterhver
pub async fn primary_process() {
    // Spawn a separate task for å starte backup prosess i ny terminal + håndtere backup responsiveness
    // Oppdaterer også backup sin worldview
    
    let id = "14"; //endres til linja under når den tid kommer
    //->>>let id = self.id;
    tokio::spawn(async move {
        IT_Roger::create_and_monitor_backup( "255.255.255.255:8080", id).await;
    });
    

    // Må ha en seperate task som hører etter broadcast fra andre mastere her
    /*
     funksjonen må ta høyde for IDen din
        Først hører den etter en broadcast
        Om den aldri hører en broadcast, start fra deafault settings (en standard worldviewfil i repo)
        Første broadcast oppdaterer den sin egen worldview, svarer med broadcast med sin egen ID
        Pass på å sende ID tilbake så funksjoner under kan vite om du er aktiv / passiv master

    OM du ikke har lavest ID:
        Fortsett og høre etter broadcast, oppdater worldview og ID fra dem, send til riktig channel
        Svar med en broadcast av din ID så andre mastere vet at du er i systemet
    Om du har lavest ID:
        Bytt over til hovedmaster
        Sende egen worldview + ID på broadcast
        Hør etter ID-broadcast fra andre så du kan steppe ned om noen med lavere kommer
    OM du slutter å høre broadcast:
        Send en error på ID-channel¨
        Den delen av programmet bør derfor vite om du nå er laveste ID eller ikke
        Du vil få svar fra en annen tråd på om du nå er hovedmaster eller ikke, og fortsetter derfra
    */





    //Ha løkke som venter på at du har lowest id
    //loop {
    //    while !has_lowest_ID {
            // Sjekk channel om IDen som leses fra broadcast, 
            // hold styr på alle IDene på systemet, om det er lenge siden en ID har kommet over channel, fjern den

            // Om du får error fra ID-channelen, send tilbake til IT-Roger og
            // skru på has_lowest_id her og kjør hovedmaster-løkka
            
    //    }

        // Her kjører altså det som skjer om man har lowest ID
        // Burde altså være Master-løkka 
    //}

    
}



