use super::konsulent;
use crate::config;
use crate::WorldView::WorldView;
use crate::WorldView::WorldViewChannel;
use crate::Byrokrati::PostNord;

use termcolor::Color;
use tokio::time::sleep;
use tokio::time::Duration;
use core::panic;
use std::env;
use std::sync::atomic::Ordering;
use std::u8;
use std::sync::Arc;
use tokio::sync::broadcast;
use std::net::IpAddr;
use tokio::sync::Mutex;

use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};


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



    pub async fn start_from_worldview(&self, wv_channel: WorldViewChannel::WorldViewChannel, worldview_arc: Arc<Mutex<Vec<u8>>>) -> tokio::io::Result<()> {
        let (shutdown_tx, _) = broadcast::channel::<u8>(1);


        let wv_channel_clone = WorldViewChannel::WorldViewChannel{tx: wv_channel.tx.clone()};
        if self.id <= worldview_arc.lock().await[1] {
            
            match self.master_process(wv_channel_clone, shutdown_tx.clone(), worldview_arc).await {
                Ok(_) => {Ok(())},
                Err(e) => {
                    eprintln!("Feil i master_process: {:?}", e);
                    konsulent::print_farge(format!("Feil i master_process: {:?}", e), Color::Red);
                    return Err(e);
                }
            }
            
        }
        else {
            let _kun_for_å_fjerne_warning_fjern_vareabel_og_handter_error_senere = self.slave_process(wv_channel_clone, shutdown_tx.clone(), worldview_arc).await;
            return Ok(())
        }
    }
    
    async fn master_process(&self, wv_channel: WorldViewChannel::WorldViewChannel, shutdown_tx: broadcast::Sender<u8>, worldview_arc: Arc<Mutex<Vec<u8>>>) -> tokio::io::Result<()> {
        println!("\nstarter Master prosess\n");
        konsulent::print_farge("Starter master_process".to_string(), Color::Green);
        //let wv_rx = wv_channel.tx.clone().subscribe();
        // 1) start TCP -> publiser nyhetsbrev




        let wv_channel_clone = WorldViewChannel::WorldViewChannel{tx: wv_channel.tx.clone()};
        //let post_handle = self.start_post_leveranse_task(/*Arc*/wv_channel_clone, shutdown_tx.clone());
        let self_clone = self.clone();
        let shutdown_clone = shutdown_tx.clone();
        let post_handle = tokio::spawn(async move {
            konsulent::print_farge("Starter nyhetsbrev-server (start_post_leveranse_task()".to_string(), Color::Green);
            if let Err(e) = self_clone.start_post_leveranse(wv_channel_clone, shutdown_clone).await {
                konsulent::print_farge(format!("Feil i post_leveranse: {}", e), Color::Red);
            }
        });

        let _udp_handle = self.start_udp_broadcast_task(shutdown_tx.clone());

    
        /*
        Evt ha e tråd følge med på UDP og post_handle, og starte den på nytt om nødvendig:
        match udp_handle.await {
            Ok(_) => println!("UDP-broadcast avslutta normalt."),
            Err(e) => eprintln!("Feil i UDP-task: {}", e),
        }
        */

        let mut i: u8 = 0;
        loop{

            let ny_mamma = PostNord::get_ny_mamma().load(Ordering::SeqCst);
            match ny_mamma {
                config::ERROR_ID => {},
                _ => {
                    PostNord::get_ny_mamma().store(config::ERROR_ID, Ordering::SeqCst);
                    let worldview = WorldView::deserialize_worldview(&*worldview_arc.lock().await);
                    match worldview {
                        Ok(mut wv) => {
                            let mut mor = WorldView::AlenemorDel::default();
                            mor.heis_id = ny_mamma;
                            wv.rapporter_annsettelse_av_mor(mor);
                            let serialized_wv = WorldView::serialize_worldview(&wv);
                            match serialized_wv {
                                Ok(mut swv) => {
                                    if ny_mamma < self.id {
                                        swv[1] = ny_mamma;
                                    }
                                    *worldview_arc.lock().await = swv;
                                }
                                Err(e) => {konsulent::print_farge(format!("Feil i serialisering av worldview: {}", e), Color::Red);}
                            }
                            if ny_mamma < self.id {
                                break;
                                
                            }
                        }
                        Err(e) => {konsulent::print_farge(format!("Feil i deserialisering av worldview: {}", e), Color::Red);}
                    }
                }
            } 

            let dau_mamma = PostNord::get_dau_mamma().load(Ordering::SeqCst);
            match dau_mamma {
                config::ERROR_ID => {},
                _ => {
                    PostNord::get_dau_mamma().store(config::ERROR_ID, Ordering::SeqCst);

                    let worldview = WorldView::deserialize_worldview(&*worldview_arc.lock().await);
                    match worldview {
                        Ok(mut wv) => {
                            wv.rapporter_sparking_av_mor(dau_mamma);
                            let serialized_wv = WorldView::serialize_worldview(&wv);
                            match serialized_wv {
                                Ok(swv) => {
                                    *worldview_arc.lock().await = swv;
                                }
                                Err(e) => {konsulent::print_farge(format!("Feil i serialisering av worldview: {}", e), Color::Red);}
                            }
                        }
                        Err(e) => {konsulent::print_farge(format!("Feil i deserialisering av worldview: {}", e), Color::Red);}
                    }

                }
            } 

            //_ = shutdown_tx.send(69);
            // WorldViewChannel::request_worldview().await;
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as u8; // Alternativt .as_secs() for sekund

            let mut worldview = worldview_arc.lock().await;
            let msg_len = worldview.len();

                // kan bruke tellaren til å sjekke timeout
                worldview[msg_len - 1] = timestamp; // sekund tellar
        }

        tokio::time::sleep(Duration::from_millis(1000)).await;
        let _ = shutdown_tx.send(69);
        
        println!("før await");
        let _ = post_handle.await;
        println!("etter await");
        Ok(())

    }
    
    async fn slave_process(&self, _wv_channel: WorldViewChannel::WorldViewChannel, shutdown_tx: broadcast::Sender<u8>, worldview_arc: Arc<Mutex<Vec<u8>>>) -> tokio::io::Result<()> {
        PostNord::get_ny_wv().store(true, Ordering::SeqCst);
        let _fiks_i_fremtiden = self.clone().start_abboner_master_nyhetsbrev_task(shutdown_tx.clone().subscribe(), worldview_arc.clone());
        
        
        loop {
            // println!("i slaveloop");
            let wv_locked = worldview_arc.lock().await;
            //println!("Wolrdview mottat: {:?}", *wv_locked);
            // println!("Kan vi å printe?");
            PostNord::get_ny_wv().store(true, Ordering::SeqCst);
            // let wv_deserialized = WorldView::deserialize_worldview(&*vw_locked);
            // match wv_deserialized {
            //     Ok(mut wv) => {

                    
            //     }
            //     Err(e) => {konsulent::print_farge(format!("Feil i deserialisering av worldview: {}", e), Color::Red);}
            // }

        }      
    }



    
}






