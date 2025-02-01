//! slave
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use super::{PostNord, Sjefen};
use std::net::IpAddr;

// pub struct Vara {
//     pub ip: IpAddr,
//     pub id: u8,
//     pub rolle: Sjefen::Rolle,
// }


// impl Vara {

//     pub fn copy_to_sjef(&self) -> Sjefen::Sjefen {
//         Sjefen::Sjefen { ip: self.ip, id: self.id, rolle: Sjefen::Rolle::MASTER }
//     }

//     fn copy(&self) -> Vara {
//         Vara { ip: self.ip, id: self.id, rolle: self.rolle }
//     }

//     pub async fn vara_process(&self, master_ip: SocketAddr){


//         let mut self_copy: Vara = self.copy();
        
//         tokio::spawn(async move {
//             match self_copy.abboner_master_nyhetsbrev(master_ip).await {
//                 Ok(_) => {},
//                 Err(e) => eprintln!("Feil i PostNord::abboner_master_nyhetsbrev: {}", e),  
//             }
//         });
        

//         loop {
//             sleep(Duration::from_millis(100)).await; // Sover i 1 sekund
//         }
        

//     }
// }


