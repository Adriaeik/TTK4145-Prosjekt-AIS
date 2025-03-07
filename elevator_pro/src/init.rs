

use std::{sync::atomic::Ordering, net::SocketAddr, time::Duration, borrow::Cow, env};
use tokio::{time::{Instant, timeout}, net::UdpSocket};
use socket2::{Domain, Socket, Type};
use local_ip_address::local_ip;
use crate::{world_view::world_view::{self, serialize_worldview, ElevatorContainer, WorldView, Task, TaskStatus}, utils::{self, ip2id, print_err}, config};


/// ### Initialiserer worldview
/// 
/// 
pub async fn initialize_worldview() -> Vec<u8> {
    let mut worldview = WorldView::default();
    let mut elev_container = ElevatorContainer::default();
    let init_task = Task{
        id: 69,
        to_do: 0,
        status: TaskStatus::PENDING,
        is_inside: true,
    };
    elev_container.tasks.push(init_task.clone());
    elev_container.tasks_status.push(init_task.clone());

    // Hent lokal IP-adresse
    let ip = match local_ip() {
        Ok(ip) => ip,
        Err(e) => {
            print_err(format!("Fant ikke IP i starten av main: {}", e));
            panic!();
        }
    };

    // Hent ut egen ID (siste tall i IP-adressen)
    utils::SELF_ID.store(ip2id(ip), Ordering::SeqCst); //🐌 Seigast
    elev_container.elevator_id = utils::SELF_ID.load(Ordering::SeqCst);
    worldview.master_id = utils::SELF_ID.load(Ordering::SeqCst);
    worldview.add_elev(elev_container.clone());


    //Hør etter UDP i 1 sek?. Hvis den får en wordlview: oppdater
    let wv_from_udp = check_for_udp().await;
    if wv_from_udp.is_empty(){
        utils::print_info("Ingen andre på Nett".to_string());
        return serialize_worldview(&worldview);
    }

    //Hvis det er UDP-er på nettverker, koble deg til dem ved å sette worldview = dem sin + egen heis
    let mut wv_from_udp_deser = world_view::deserialize_worldview(&wv_from_udp);
    wv_from_udp_deser.add_elev(elev_container.clone());
    
    //Sett egen ID som master_ID hvis tidligere master har høyere ID enn deg
    if wv_from_udp_deser.master_id > utils::SELF_ID.load(Ordering::SeqCst) {
        wv_from_udp_deser.master_id = utils::SELF_ID.load(Ordering::SeqCst);
    }

    
    world_view::serialize_worldview(&wv_from_udp_deser) 
}



/// Hører etter UDP broadcaster i 1 sekund
/// 
/// Passer på at UDPen er fra 'vårt' nettverk før den 'aksepterer' den for retur
pub async fn check_for_udp() -> Vec<u8> {
    let broadcast_listen_addr = format!("{}:{}", config::BC_LISTEN_ADDR, config::DUMMY_PORT);
    let socket_addr: SocketAddr = broadcast_listen_addr.parse().expect("Ugyldig adresse");
    let socket_temp = Socket::new(Domain::IPV4, Type::DGRAM, None).expect("Feil å lage ny socket  iinit");
    
    
    socket_temp.set_reuse_address(true).expect("feil i set_resuse_addr i init");
    socket_temp.set_broadcast(true).expect("Feil i set broadcast i init");
    socket_temp.bind(&socket_addr.into()).expect("Feil i bind i init");
    let socket = UdpSocket::from_std(socket_temp.into()).expect("Feil å lage socket i init");
    let mut buf = [0; config::UDP_BUFFER];
    let mut read_wv: Vec<u8> = Vec::new();
    
    let mut message: Cow<'_, str> = std::borrow::Cow::Borrowed("a");

    let time_start = Instant::now();
    let duration = Duration::from_secs(1);

    while Instant::now().duration_since(time_start) < duration {
        let recv_result = timeout(duration, socket.recv_from(&mut buf)).await;

        match recv_result {
            Ok(Ok((len, _))) => {
                message = String::from_utf8_lossy(&buf[..len]).into_owned().into();
            }
            Ok(Err(e)) => {
                utils::print_err(format!("udp_broadcast.rs, udp_listener(): {}", e));
                continue;
            }
            Err(_) => {
                // Timeout skjedde – stopp løkka
                utils::print_warn("Timeout – ingen data mottatt innen 1 sekund.".to_string());
                break;
            }
        }

        // verifiser at UDPen er fra 'oss'
        if &message[1..config::KEY_STR.len() + 1] == config::KEY_STR {
            let clean_message = &message[config::KEY_STR.len() + 3..message.len() - 1]; // Fjerner `"`
            read_wv = clean_message
                .split(", ") // Del opp på ", "
                .filter_map(|s| s.parse::<u8>().ok()) // Konverter til u8, ignorer feil
                .collect(); // Samle i Vec<u8>

            break;
        }
    }
    drop(socket);
    read_wv
}


/// ### Leser argumenter på cargo run
/// 
/// Brukes for å endre hva som printes i runtime. Valgmuligheter:
/// 
/// `print_wv::(true/false)` &rarr; Printer worldview 2 ganger i sekundet  
/// `print_err::(ture/false)` &rarr; Printer error meldinger  
/// `print_wrn::(true/false)` &rarr; Printer warning meldinger   
/// `print_ok::(true/false)` &rarr; Printer ok meldinger   
/// `print_info::(true/false)` &rarr; Printer info meldinger   
/// `print_else::(true/false` &rarr; Printer andre meldinger, bla. master, slave, color meldinger   
/// `debug::` &rarr; Skrur av alle prints andre enn error meldinger   
/// `help` &rarr; Skriver ut alle mulige argumenter uten å starte programmet
/// 
/// Alle prints er på om ingen argumnter er gitt
pub fn parse_args() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        for arg in &args[1..] {
            let parts: Vec<&str> = arg.split("::").collect();
            if parts.len() == 2 {
                let key = parts[0].to_lowercase();
                let value = parts[1].to_lowercase();
                let is_true = value == "true";

                unsafe {
                    match key.as_str() {
                        "print_wv" => config::PRINT_WV_ON = is_true,
                        "print_err" => config::PRINT_ERR_ON = is_true,
                        "print_warn" => config::PRINT_WARN_ON = is_true,
                        "print_ok" => config::PRINT_OK_ON = is_true,
                        "print_info" => config::PRINT_INFO_ON = is_true,
                        "print_else" => config::PRINT_ELSE_ON = is_true,
                        "debug" => { // Debug modus: Kun error-meldingar
                            config::PRINT_WV_ON = false;
                            config::PRINT_WARN_ON = false;
                            config::PRINT_OK_ON = false;
                            config::PRINT_INFO_ON = false;
                            config::PRINT_ELSE_ON = false;
                        }
                        _ => {}
                    }
                }
            } else if arg.to_lowercase() == "help" {
                println!("Tilgjengelige argument:");
                println!("  print_wv::true/false");
                println!("  print_err::true/false");
                println!("  print_warn::true/false");
                println!("  print_ok::true/false");
                println!("  print_info::true/false");
                println!("  print_else::true/false");
                println!("  debug (kun error-meldingar vises)");
                std::process::exit(0);
            }
        }
    }
}