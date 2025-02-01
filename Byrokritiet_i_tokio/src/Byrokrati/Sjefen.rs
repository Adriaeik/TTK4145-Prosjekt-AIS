//! sjefen handterer opretting av master / backup
//! 
use crate::Byrokrati::konsulent::id_fra_ip;

use super::IT_Roger;
use super::MrWorldWide;
use super::Tony;
use super::PostNord;
use super::Vara;
use std::net::SocketAddr;

use tokio::time::{sleep, Duration};
use std::env;
use tokio::sync::mpsc;
use tokio::macros::support::Future;
use std::sync::Arc;
use tokio::sync::broadcast;
use std::net::IpAddr;
use tokio::macros::support::Pin;

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
pub struct Sjefen {
    pub ip: IpAddr,
    pub id: u8,
    pub rolle: Rolle,
    pub master_ip: IpAddr
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
        Err("Bruk: <program> <kommando> <ID>")
    }
}




impl Sjefen{
    fn copy_for_backup(&self) -> Sjefen {
        Sjefen {
            ip: self.ip,
            id: self.id,
            rolle: Rolle::BACKUP,
            master_ip: self.master_ip,
        }
    }

    pub fn copy(&self) -> Sjefen {
        Sjefen {
            ip: self.ip,
            id: self.id,
            rolle: self.rolle,
            master_ip: self.master_ip,
        }
    }


    /// Basically sjefen sin main loop
    /// 
    /// Vil starte en tråd som oppretter og følger med på egen backup
    /// 
    /// Etterhvert skal den også høre/sende UDP broadcast til nettverket, med ID
    /// Skal bruke det til å koble seg opp med TCP til andre mastere på nettet for deling av worldview
    /// Denne burde også være en impl til sjefen for å droppe argumenter
    /// 
    /// Skal etter det fikse selve styresystemet (om du har lavest ID) 
    ///
    pub fn primary_process(&mut self) -> Pin<Box<dyn Future<Output = tokio::io::Result<()>> + Send>> {
        let mut self_copy = self.copy();
        Box::pin(async move {
            println!("En sjef er starta");        
            let self_backup = self_copy.copy_for_backup();
            //Lager en tokio task som holder styr på backup, har også en tråd i seg som kjører IT_Roger sine funksjoner for å snakke med den
            // Må sjekke om man allerede har en bakcup
            tokio::spawn(async move {
                self_backup.create_and_monitor_backup().await;
            });
            

            // let (tx_is_master, mut rx_is_main) = mpsc::channel::<bool>(1);
            // let (tx_master_ip, mut rx_master_ip) = mpsc::channel::<SocketAddr>(1);
            //Lager en tokio task som først hører etter broadcast, og kobler seg på nettverket. Om ingen broadcast på et sekund ? ish så starter den som hovedmaster
            // let self_copy = self.copy();
            // let broadcast_task = tokio::spawn(async move {
            //     match self_copy.start_broadcaster(tx_is_master, tx_master_ip).await {
            //         Ok(_) => {},
            //         Err(e) => eprintln!("Feil i MrWorldWide::start_broadcaster: {}", e),  
            //     }
            // });
            
            match self_copy.listen_to_network().await {
                Ok(addr) => if addr.to_string() == "0.0.0.0" {self_copy.rolle = Rolle::MASTER}
                                    else {self_copy.rolle = Rolle::SLAVE;
                                    self_copy.master_ip = addr;},
                Err(e) => eprintln!("feil i sjefen.rs primary_process() listen_to_network(): {}", e),
            }
            
            
            
            // let mut master_ip: SocketAddr;
            // loop {
            //     if let Some(addr) = rx_master_ip.recv().await {
            //         master_ip = addr;
            //         break; // Gå videre etter første gyldige melding
            //     }
            //     //Hvis `None`, venter den bare til neste melding uten å avslutte
            // }

            // loop {
            //     if let Some(msg) = rx_is_main.recv().await {
            //         is_main = msg;
            //         break; // Gå videre etter første gyldige melding
            //     }
            //     // Hvis `None`, venter den bare til neste melding uten å avslutte
            // }
            if self_copy.rolle == Rolle::SLAVE {
                let mut self_copy_clone = self_copy.copy();
                let vent = tokio::spawn(async move {
                    match self_copy_clone.abboner_master_nyhetsbrev().await {
                        Ok(_) => {},
                        Err(e) => eprintln!("Feil i PostNord::abboner_master_nyhetsbrev: {}", e),  
                    }
                });
                vent.await.unwrap();
            }


            self_copy.rolle = Rolle::MASTER;


            let self_copy_clone = self_copy.copy();
            let broadcast_task = tokio::spawn(async move {
                match self_copy_clone.start_master_broadcaster().await {
                    Ok(_) => {},
                    Err(e) => eprintln!("Feil i MrWorldWide::start_broadcaster: {}", e),  
                }
            });
            
            /*Lag channel å sende worldview på*/
            let (tx, _) = broadcast::channel::<String>(3); //Kunne vel i teorien vært 1
            let tx = Arc::new(tx);
            let /*mut */ tx_clone = Arc::clone(&tx); // Klon senderen for bruk i ny oppgave

            let self_copy_clone = self_copy.copy();
            let nyhetsbrev_task = tokio::spawn(async move {
                match self_copy_clone.publiser_nyhetsbrev(tx_clone).await {
                    Ok(_) => {
                        println!("går ut av publiser nyhetsbrev");

                    }, //Må si fra at du nå er slave
                    Err(e) => eprintln!("Feil i PostNord::publiser_nyhetsbrev: {}", e),  
                }
            });

            

            //Hent ID (siste tall i IP)
            let worldview = format!("Worldview:{}", self_copy.ip);
            //For nå:
            // sender Worldview:{id}
            while self_copy.rolle == Rolle::MASTER {
                let tx_clone_for_send = Arc::clone(&tx); // Klon senderen på nytt for sending
                let worldview_clone = worldview.clone();
                tokio::spawn(async move {
                    if let Err(e) = tx_clone_for_send.send(worldview_clone) {
                        // Hvis det er feil, betyr det at ingen abonnenter er tilgjengelige
                        //println!("Ingen abonnenter tilgjengelig for å motta meldingen: {}", e);
                    }
                });
                sleep(Duration::from_millis(100)).await; // Sover i 1 sekund
                //println!("Jeg lever i sjefen.rs primary_process loop");
            }
            nyhetsbrev_task.await?;
            broadcast_task.abort();
            
            self_copy.rolle = Rolle::SLAVE;
            self_copy.primary_process().await?;
    
    
            Ok(())
            
        })
        

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
}


