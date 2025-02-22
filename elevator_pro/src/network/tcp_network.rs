// use tokio::net::TcpListener;
// use tokio::task::JoinHandle;
// use crate::{config, utils};



// pub async fn tcp_listener() {
//     let self_ip = utils::get_self_ip();
//     let listener = TcpListener::bind(format!("{}:{}", self_ip, config::PN_PORT)).await;
//     let mut shutdown_rx = shutdown_tx.subscribe();
//     let mut listeners_tasks: Vec<JoinHandle<()>> = Vec::new();

//     loop {
//         let prev_is_master = is_master;
//         let is_master = min_id == wv_lavest_id;
//         if is_master & !prev_is_master {
//             //Koble fra tilkobling på master_connection
            
//         }
//         else if is_master {
//             //Aksepter inkommende connections -> legg til i connection-array.
//             //Send tasks mottatt fra task-kanal til riktig heis
//             //Hvis ikke ACKA eller annet feil -> si fra til worldview
//         } 
//         else if !is_master & prev_is_master {
//             //Koble fra alle slave-connections
//             //koble til master, joinhandle er master_connection
//         }
//         else if !is_master {
//             //Vent på å motta task
//             //Mottat task, ACK den
//             //Send mottat task på kanal til anvarlig for egen heis
//         }
        
//     }
// }