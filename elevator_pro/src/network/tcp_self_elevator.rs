use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::{io::AsyncReadExt, net::TcpListener};
use tokio::task::JoinHandle;
use tokio::net::TcpStream;
use std::net::SocketAddr;
use tokio::time::{sleep, Duration, Instant};
use crossbeam_channel as cbc;
use tokio::process::Command;

use crate::world_view::world_view;
use crate::{config, utils, world_view::world_view_update, elevio, elevio::poll::CallButton, elevio::elev as e};
use utils::{print_info, print_ok, print_err, get_wv};
use std::env;

use super::local_network;



struct LocalElevTxs {
    call_button: cbc::Sender<CallButton>,
    floor_sensor: cbc::Sender<u8>,
    stop_button: cbc::Sender<bool>,
    obstruction: cbc::Sender<bool>,
}

struct LocalElevRxs {
    call_button: cbc::Receiver<CallButton>,
    floor_sensor: cbc::Receiver<u8>,
    stop_button: cbc::Receiver<bool>,
    obstruction: cbc::Receiver<bool>,
}

struct LocalElevChannels {
    pub rxs: LocalElevRxs,
    pub txs: LocalElevTxs,
}

impl LocalElevChannels {
    pub fn new() -> Self {
        let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>();
        let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
        let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();
        let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();

        LocalElevChannels { 
            rxs: LocalElevRxs { call_button: call_button_rx, floor_sensor: floor_sensor_rx, stop_button: stop_button_rx, obstruction: obstruction_rx }, 
            txs: LocalElevTxs { call_button: call_button_tx, floor_sensor: floor_sensor_tx, stop_button: stop_button_tx, obstruction: obstruction_tx } 
        }
    }
}



fn start_elevator_server() {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "start", "elevatorserver"])
            .spawn()
            .expect("Failed to start elevator server");
    } else {
        Command::new("gnome-terminal")
            .args(&["--", "bash", "-c", "elevatorserver; exec bash"])
            .spawn()
            .expect("Failed to start elevator server");
    }

    print_info(format!("starta server"));
}

async fn init_local_elevator_connection(txs: LocalElevTxs, elevator: e::Elevator) -> std::io::Result<()> {
    print_ok(format!("Elevator started:\n{:#?}", elevator));
    {
        let elevator = elevator.clone();
        tokio::spawn(async move {
            elevio::poll::call_buttons(elevator, txs.call_button, config::ELEV_POLL)
        });
    }
    {
        let elevator = elevator.clone();
        tokio::spawn(async move {
            elevio::poll::floor_sensor(elevator, txs.floor_sensor, config::ELEV_POLL)
        });
    }
    {
        let elevator = elevator.clone();
        tokio::spawn(async move {
            elevio::poll::stop_button(elevator, txs.obstruction, config::ELEV_POLL)
        });
    }
    {
        let elevator = elevator.clone();
        tokio::spawn(async move {
            elevio::poll::obstruction(elevator, txs.stop_button, config::ELEV_POLL)
        });
    }
    Ok(())

}

fn init_to_closest_under_floor(rxs: LocalElevRxs, elevator: e::Elevator) -> u8 {
    elevator.motor_direction(e::DIRN_DOWN);
    let a = rxs.floor_sensor.recv(); 
    elevator.motor_direction(e::DIRN_STOP);
    a.unwrap()   
}


pub async fn run_local_elevator() -> std::io::Result<()> {
    start_elevator_server();
    let local_elev_channels: LocalElevChannels = LocalElevChannels::new();
    println!("Lagd chs");
    sleep(Duration::from_millis(100)).await;
    let elevator: e::Elevator = e::Elevator::init(config::LOCAL_ELEV_IP, config::DEFAULT_NUM_FLOORS).expect("Feil!");
    println!("Lagd elevator");
    let _ = init_local_elevator_connection(local_elev_channels.txs, elevator.clone()).await;
    println!("Kobla på elev server");
    let floor = init_to_closest_under_floor(local_elev_channels.rxs, elevator.clone());
    print_ok(format!("Vi er på floor: {}", floor));

    Ok(())
}

// pub async fn read_from_local_elevator() -> std::io::Result<()> {
//     loop {
//         cbc::select! {
//             recv(call_button_rx) -> a => {
//                 let call_button = a.unwrap();
//                 println!("{:#?}", call_button);
//                 elevator.call_button_light(call_button.floor, call_button.call, true);
//             },
//             recv(floor_sensor_rx) -> a => {
//                 let floor = a.unwrap();
//                 println!("Floor: {:#?}", floor);
//                 dirn =
//                     if floor == 0 {
//                         e::DIRN_UP
//                     } else if floor == elev_num_floors-1 {
//                         e::DIRN_DOWN
//                     } else {
//                         dirn
//                     };
//                 elevator.motor_direction(dirn);
//             },
//             recv(stop_button_rx) -> a => {
//                 let stop = a.unwrap();
//                 println!("Stop button: {:#?}", stop);
//                 for f in 0..elev_num_floors {
//                     for c in 0..3 {
//                         elevator.call_button_light(f, c, false);
//                     }
//                 }
//             },
//             recv(obstruction_rx) -> a => {
//                 let obstr = a.unwrap();
//                 println!("Obstruction: {:#?}", obstr);
//                 elevator.motor_direction(if obstr { e::DIRN_STOP } else { dirn });
//             },
//         }
//     }
// }








