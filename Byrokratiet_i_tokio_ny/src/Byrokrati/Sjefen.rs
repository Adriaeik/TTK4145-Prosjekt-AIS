use crate::config;
use super::MrWorldWide;
use super::konsulent;
use crate::WorldView::WorldView;

use tokio::time::{sleep, Duration};
use std::default;
use std::env;
use tokio::sync::mpsc;
use tokio::macros::support::Future;
use std::sync::Arc;
use tokio::sync::broadcast;
use std::net::IpAddr;
use tokio::macros::support::Pin;
use tokio::sync::Mutex;

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
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Rolle {
    MASTER,
    SLAVE,
    BACKUP,
}


/// Pakke med Rolle (se Rolle) og ID (siste tall i IP) for en sjef
#[derive(Clone, Debug)]
pub struct SjefPakke {
    pub rolle: Rolle,
    // TODO: Lage IP
}

/// Sjefen!!!!
/// IP-addresse og ID er const etter init, så de blir kun shallow-copy 
/// rolle og master_ip kan endres, så de er arc<mutex<>>
pub struct Sjefen {
    pub ip: IpAddr,
    pub id: u8,
    pub rolle: Arc<Mutex<Rolle>>,
    pub master_ip: Arc<Mutex<IpAddr>>,
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

    if args.len() > 1 {
        let command = args[1].to_lowercase();
        
        let rolle = match command.as_str() {
            "backup" => Rolle::BACKUP,
            "master" => Rolle::SLAVE,
            _ => return Err("Ugyldig kommando. Bruk 'backup' eller 'master'."),
        };

        Ok(SjefPakke {rolle})
    } else {
        Err("Bruk: <program> <kommando>")
    }
}




impl Sjefen{
    /// Kloner sjefen
    /// Er ikke helt kloning, IP og ID er shallow copy
    /// Bør gå bra, da disse burde være const for en gitt sjef, kun rolle og master_ip (for nå) er dynamiske
    pub fn clone(&self) -> Sjefen {
        Sjefen {
            ip: self.ip,
            id: self.id,
            rolle: Arc::clone(&self.rolle),
            master_ip: Arc::clone(&self.master_ip),
        }
    }


    /// Funksjon som starter en master_process uten worldview
    /// 
    /// Funksjonen leter etter aktiv UDP broadcast fra Gruppe25, og returnerer worldview fra master om det finnes
    /// 
    /// Returnerer worldview med egen id som master_id, og legger til egen heis med default verdier på alle statuser hvis ingen master finnes.
    pub async fn start_clean(&self) -> Vec<u8> {
        //Legger til egen IP i masted_IP om du er master, legger til master sin IP om det er en master
        match self.listen_to_network().await {
            Ok(_) => if *self.master_ip.lock().await == self.ip {
                            println!("Jeg ble master, sjefen.rs start_clean()");
                        }else {
                            println!("Jeg ble slave, henter worldview fra {} (Sjefen.rs, start_clean())", *self.master_ip.lock().await);
                            match self.get_wv_from_master().await {
                                Some(worldview) => return worldview,
                                None => {panic!("Klarte ikke lese TCP stream fra master (sjefen.rs, start_clean())");}
                            }
                            //return worldview;
                        },
            Err(e) => eprintln!("feil i sjefen.rs start_clean() listen_to_network(): {}", e),
        }

        //Nå vet du at du skal bli master. Du må fikse WorldView basert på det du vet (egen heis, osv.)
        let mut worldview = WorldView::WorldView::default();
        let mut heis = WorldView::AlenemorDel::default();
        
        let self_id = konsulent::id_fra_ip(self.ip);
        
        heis.heis_id = self_id;

        worldview.master_id = self_id;
        worldview.add_heis(heis);

        let worldview_serialised = WorldView::serialize_worldview(&worldview);
        match worldview_serialised {
            Ok(serialized_data) => {
                // Deserialisere WorldView fra binært format
                return serialized_data;
            }
            Err(e) => {
                panic!("Serialization failed: {} (sjefen.rs, start_clean())", e);
            }
        }
    }






    

    pub fn start_from_worldview(&self) {
        // Må hente inn worldview
        // Må håndtere om den blir startet som backup eller master eller slave ettersom de har mye forskjellig funksjonalitet?

        // Må:
        // nummer uno 1 én: finne ut om du er hovedmaster. om du er det her kommer andre noder til
        // å prøve å koble opp til TCPen din ganske fort, så sett opp det asap
        // Sette opp tilkoblinger dit den skal basert på worldviewen (forskjellig om du er hoved_master eller slave_master)
        // Do its thing...
    }

}