use std::time::Duration;

use elevator_pro::{network::{local_network, udp_broadcast}, utils, world_view::world_view};
use tokio::{sync::broadcast, time::sleep};
use local_ip_address::local_ip;


#[tokio::main]
async fn main() {
    let mut worldview = world_view::WorldView::default();
    let mut mor = world_view::AlenemorDel::default();
    let ip = match local_ip() {
        Ok(ip) => {
            ip
        }
        Err(e) => {
            utils::print_err(format!("Fant ikke IP i starten av main: {}", e));
            panic!();
        }
    }; 
    let self_id = utils::ip2id(ip);
    mor.heis_id = self_id;
    worldview.master_id = self_id;
    worldview.add_elev(mor);
    let mut worldview_serialised = world_view::serialize_worldview(&worldview);

    
    /*Init av lokale channels  */
    //Kun bruk mpsc-rxene fra main_local_chs
    let mut main_local_chs = local_network::LocalChannels::new();
    let chs_udp_listen = main_local_chs.clone();
    let chs_udp_bc = main_local_chs.clone();
                                                            




    let broadcast_task = tokio::spawn(async move {
        // Denne koden kjører i den asynkrone oppgaven (tasken)
        utils::print_info("Starter å høre etter UDP-broadcast".to_string());
        let _ = udp_broadcast::start_udp_listener(chs_udp_listen).await;
    });

    let broadcast_task = tokio::spawn(async move {
        // Denne koden kjører i den asynkrone oppgaven (tasken)
        utils::print_info("Starter UDP-broadcaster".to_string());
        let _ = udp_broadcast::start_udp_broadcaster(chs_udp_bc, self_id).await;
    });

    
    loop {
        match main_local_chs.mpscs.rxs.udp_wv.try_recv() {
            Ok(msg) => {
                let my_wv_deserialised = world_view::deserialize_worldview(&worldview_serialised);
                let mut master_wv_deserialised = world_view::deserialize_worldview(&msg);

                for heis in my_wv_deserialised.heis_spesifikke {
                    if heis.heis_id == self_id {
                        master_wv_deserialised.add_elev(heis);
                    }
                }
                worldview_serialised = world_view::serialize_worldview(&master_wv_deserialised);
                utils::print_info(format!("Oppdatert wv fra UDP: {:?}", worldview_serialised));
            },
            Err(_) => {},
        }

        let _ = main_local_chs.broadcasts.txs.wv.send(worldview_serialised.clone());
    }

    // let tcp_task = start_tcp_listener(); //TCP connection mellom master eller slaver

    // let heis_logikk_task = start_process(); //Selve kjøre-mekanismene


}


// Kanskje lage is_master = min_id == lavest_id_i_wv i en global multithread vareabel
// Så kan worldview-thread oppdatere den (låse + skrive) og alle kan lese av den uten å låse (så wv ikke blir låst hver gang den skal beregnes)




// fn start_tcp_listener() {
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


// fn start_process() {
//     loop {
//         let is_master = min_id == wv_lavest_id;

//         if is_master {
//             //Finn ut hvilken oppgaver som må gjøres
//             //Deleger oppgaver til heiser
//             //Send på kanal hvilken heis som skal gjøre hvilken task
//         }
//         else {
//             //Vent på Task fra kanal ansvarlig for egen heis
//             //Si fra når Task er gjort
//         }
//     }
// }



