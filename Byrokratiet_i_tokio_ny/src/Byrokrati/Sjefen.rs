use super::konsulent;
use crate::WorldView::WorldView;
use crate::WorldView::WorldViewChannel;

use termcolor::Color;
use tokio::sync::mpsc;
use tokio::time::Duration;
use core::panic;
use std::env;
use std::u8;
use std::sync::Arc;
use tokio::sync::broadcast;
use std::net::IpAddr;
use tokio::sync::Mutex;

use anyhow::Result;



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
                            *self.master_ip.lock().await = self.ip;
                        }else {
                            println!("Jeg ble slave, henter worldview fra {} (Sjefen.rs, start_clean())", *self.master_ip.lock().await);
                            match self.get_wv_from_master().await {
                                Some(worldview) => return worldview,
                                None => {
                                    konsulent::print_farge("Klarte ikke lese TCP stream fra master (sjefen.rs, start_clean())".to_string(), Color::Red);
                                    panic!();
                                }
                            }
                            //return worldview;
                        },
            Err(e) => eprintln!("feil i sjefen.rs start_clean() listen_to_network(): {}", e),
        }

        //Nå vet du at du skal bli master. Du må fikse WorldView basert på det du vet (egen heis, osv.)
        let mut worldview = WorldView::WorldView::default();
        let mut mor = WorldView::AlenemorDel::default();
        
        let self_id = konsulent::id_fra_ip(self.ip);
        
        mor.heis_id = self_id;

        worldview.master_id = self_id;
        worldview.rapporter_annsettelse_av_mor(mor);

        let worldview_serialised = WorldView::serialize_worldview(&worldview);
        match worldview_serialised {
            Ok(serialized_data) => {
                // Deserialisere WorldView fra binært format
                return serialized_data;
            }
            Err(e) => {
                konsulent::print_farge(format!("Serialization failed: {} (sjefen.rs, start_clean())", e), Color::Red);
                panic!();
            }
        }
    }



    pub async fn start_from_worldview(&self, wv_channel: WorldViewChannel::WorldViewChannel, wv_arc: Arc<Mutex<Vec<u8>>>) -> tokio::io::Result<()> {
        let (shutdown_tx, _) = broadcast::channel::<u8>(1);
        // shutdown_tx.send("DEt er ein ny master");
        
        // Må hente inn worldview
        // Må håndtere om den blir startet som backup eller master eller slave ettersom de har mye forskjellig funksjonalitet?
        // Hent inn worldview – her kan du handtere om modusen er backup, master eller slave
        
        // Mottar den serialiserte forma
        //let worldview = konsulent::get_worldview_from_channel(rx_wv).await;
        
        // sjekker om ein er master, bli her så lenge du er viktigast
        // bacup vil aldri kjøre denne funksjonen. startes kun fra master/slav_process
        let wv_channel_clone = WorldViewChannel::WorldViewChannel{tx: wv_channel.tx.clone()};
        if self.ip == *self.master_ip.lock().await {
            konsulent::print_farge("Jeg er master, starter master process".to_string(), Color::Yellow);
            
            match self.master_process(wv_channel_clone, shutdown_tx.clone(), wv_arc.clone()).await {
                Ok(_) => {Ok(())},
                Err(e) => {
                    eprintln!("Feil i master_process: {:?}", e);
                    konsulent::print_farge(format!("Feil i master_process: {:?}", e), Color::Red);
                    return Err(e);
                }
            }
            
        }
        else {
            let _kun_for_å_fjerne_warning_fjern_vareabel_og_handter_error_senere = self.slave_process(wv_channel_clone, shutdown_tx.clone(), wv_arc.clone()).await;
            return Ok(())
        }
     
        
        // Må:
        // nummer uno 1 én: finne ut om du er hovedmaster. om du er det her kommer andre noder til
        // å prøve å koble opp til TCPen din ganske fort, så sett opp det asap
        // Sette opp tilkoblinger dit den skal basert på worldviewen (forskjellig om du er hoved_master eller slave_master)
        // Do its thing...
        /*
        1. Er eg master?

        2. 
         */
    }
    
    async fn master_process(&self, wv_channel: WorldViewChannel::WorldViewChannel, shutdown_tx: broadcast::Sender<u8>, wv_arc: Arc<Mutex<Vec<u8>>>) -> tokio::io::Result<()> {
        println!("\nstarter Master prosess\n");
        konsulent::print_farge("Starter master_process".to_string(), Color::Green);


        /* Kanal til å sende tilbake worldview om du blir backup. bedre løsning må fikses*/
        let (rapport_tx, mut rapport_rx) = mpsc::channel::<Vec<u8>>(10);





        let wv_channel_clone = WorldViewChannel::WorldViewChannel{tx: wv_channel.tx.clone()};
        let _post_handle = self.start_post_leveranse_task(/*Arc*/wv_channel_clone, shutdown_tx.clone(), rapport_tx);

        let _udp_handle = self.start_udp_broadcast_task(shutdown_tx.clone());

    
        /*
        Evt ha e tråd følge med på UDP og post_handle, og starte den på nytt om nødvendig:
        match udp_handle.await {
            Ok(_) => println!("UDP-broadcast avslutta normalt."),
            Err(e) => eprintln!("Feil i UDP-task: {}", e),
        }
        */

        
        loop {
            if let Some(data) = rapport_rx.recv().await {
                konsulent::print_farge("En lavere ip er koblet på, blir slave...".to_string(), Color::Yellow);
                let _ = shutdown_tx.send(69);
                let mut wv_arc_unlock = wv_arc.lock().await;
                *wv_arc_unlock = data;
                return Ok(());
            }
        }

    }
    
    async fn slave_process(&self, _wv_channel: WorldViewChannel::WorldViewChannel, shutdown_tx: broadcast::Sender<u8>, wv_arc: Arc<Mutex<Vec<u8>>>) -> tokio::io::Result<()> {
        let abboner_task = self.clone().abboner_master_nyhetsbrev(shutdown_tx.clone().subscribe()).await;

        loop {
            if abboner_task.is_err() {
                konsulent::print_farge("Abboner master_nyhetsbrev feila, slave_process()".to_string(), Color::Red);
                panic!("Denne skal ikke panice etterhvert (slave_process(), abboner nyhetsbrev har error)");
            }
        }      
    }



    
}






