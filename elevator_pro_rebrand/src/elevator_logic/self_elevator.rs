//! # Local Elevator Module
//!
//! This module is responsible for managing the local elevator instance, including:
//! - Initializing the local elevator (`init`)
//! - Handling communication with the elevator server (`start_elevator_server`)
//! - Polling and processing elevator sensor data (`read_from_local_elevator`)
//! - Updating the local elevator state (`update_elev_container_from_msgs`)
//!
//! ## Overview
//! The module establishes a communication channel with the local elevator hardware, allowing
//! sensor readings (call buttons, floor sensors, stop button, obstruction status) to be processed
//! asynchronously. It also provides an interface for starting the elevator server on different platforms.
//!
//!
//! ## Functionality
//! - **Initialization**: Sets up the elevator instance, starts the elevator server, and initializes
//!   message polling from the hardware.
//! - **Message Handling**: Processes messages received from the elevator, updating the `ElevatorContainer`
//!   accordingly.
//! - **Asynchronous Processing**: Uses `tokio` tasks to handle sensor polling and inter-process communication.

use super::timer::Timer;

use crate::elevio::{self, elev as e};
use crate::world_view::{ElevatorContainer, ElevatorBehaviour};
use crate::config;
use crate::print;
use crate::network;

use crossbeam_channel as cbc;
use tokio::time::{sleep, Duration};
use tokio::process::Command;
use tokio::sync::mpsc;



struct LocalElevTxs 
{
    call_button: cbc::Sender<elevio::CallButton>,
    floor_sensor: cbc::Sender<u8>,
    stop_button: cbc::Sender<bool>,
    obstruction: cbc::Sender<bool>,
}

struct LocalElevRxs 
{
    call_button: cbc::Receiver<elevio::CallButton>,
    floor_sensor: cbc::Receiver<u8>,
    stop_button: cbc::Receiver<bool>,
    obstruction: cbc::Receiver<bool>,
}

struct LocalElevChannels 
{
    pub rxs: LocalElevRxs,
    pub txs: LocalElevTxs,
}

impl LocalElevChannels 
{
    pub fn new() -> Self 
    {
        let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::CallButton>();
        let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
        let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();
        let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();

        LocalElevChannels 
        { 
            rxs: LocalElevRxs { call_button: call_button_rx, floor_sensor: floor_sensor_rx, stop_button: stop_button_rx, obstruction: obstruction_rx }, 
            txs: LocalElevTxs { call_button: call_button_tx, floor_sensor: floor_sensor_tx, stop_button: stop_button_tx, obstruction: obstruction_tx } 
        }
    }
}


/// ### Get local IP address
fn get_ip_address() -> String 
{
    let self_id = network::read_self_id();
    format!("{}.{}", config::NETWORK_PREFIX, self_id)
}

/// ### Starts the elevator_server
async fn start_elevator_server() 
{
    let ip_address = get_ip_address();
    let ssh_password = config::SSH_PASSWORD; 

    if cfg!(target_os = "windows") 
    {
        print::info(format!("Starting elevatorserver on Windows..."));
        Command::new("cmd")
            .args(&["/C", "start", "elevatorserver"])
            .spawn()
            .expect("Failed to start elevator server");
    } else 
    {
        print::info(format!("Starting elevatorserver on Linux..."));
        
        // Start the elevator server without opening a terminal
        let elevator_server_command = format!(
            "sshpass -p '{}' ssh student@{} 'nohup elevatorserver > /dev/null 2>&1 &'",
            ssh_password, ip_address
        );

        print::info(format!("\nStarting elevatorserver in new terminal:\n\t{}", elevator_server_command));

        let _ = Command::new("sh")
            .arg("-c")
            .arg(&elevator_server_command)
            .output().await
            .expect("Error while starting elevatorserver");
    }

    print::ok(format!("Elevator server started."));
}

// ### Kjører den lokale heisen

/// Runs the local elevator
/// 
/// ## Parameters
/// `wv_watch_rx`: Rx on watch the worldview is being sent on in the system  
/// `update_elev_state_tx`: mpsc sender used to update [local_network::update_wv_watch] when the elevator is in a new state  
/// `local_elev_tx`: mpsc sender used to update [local_network::update_wv_watch] when a message has been recieved form the elevator  
/// 
/// ## Behavior
/// - The function starts the elevatorserver on the machine, and starts polling for messages  
/// - The function starts a thread which forwards messages from the elevator to [local_network::update_wv_watch]
/// - The function starts a thread which executes the first task for your own elevator in the worldview
/// 
/// ## Note
/// This function loops over a tokio::yield_now(). This is added in case further implementation is added which makes the function permanently-blocking, forcing the user to spawn this function in a tokio task. In theroy, this could be removed, but for now: call this function asynchronously
pub async fn init(
    local_elev_tx: mpsc::Sender<elevio::ElevMessage>
) -> e::Elevator {
    // Start elevator-serveren. 
    start_elevator_server().await;
    let local_elev_channels: LocalElevChannels = LocalElevChannels::new();
    let _ = sleep(config::SLAVE_TIMEOUT);
    let elevator: e::Elevator = e::Elevator::init(config::LOCAL_ELEV_IP, config::DEFAULT_NUM_FLOORS)
        .expect("Error while initiating elevator");
    
    // Start polling messages from elevator
    // ______START:: READ BUTTONS_______________
    {
        let elevator = elevator.clone();
        tokio::spawn(async move {
            elevio::poll::call_buttons(elevator, local_elev_channels.txs.call_button, config::ELEV_POLL)
        });
    }
    {
        let elevator = elevator.clone();
        tokio::spawn(async move {
            elevio::poll::floor_sensor(elevator, local_elev_channels.txs.floor_sensor, config::ELEV_POLL)
        });
    }
    {
        let elevator = elevator.clone();
        tokio::spawn(async move {
            elevio::poll::stop_button(elevator, local_elev_channels.txs.obstruction, config::ELEV_POLL)
        });
    }
    {
        let elevator = elevator.clone();
        tokio::spawn(async move {
            elevio::poll::obstruction(elevator, local_elev_channels.txs.stop_button, config::ELEV_POLL)
        });
    }
    // ______STOPP:: READ BUTTONS_______________
   
    {
        tokio::spawn(async move {
            let _ = read_from_local_elevator(local_elev_channels.rxs, local_elev_tx).await;
        });
    } 


    elevator
}

/// Send forth messages from local elevator to worldview updater
async fn read_from_local_elevator(rxs: LocalElevRxs, local_elev_tx: mpsc::Sender<elevio::ElevMessage>) -> std::io::Result<()> {
    loop {
        if let Ok(call_button) = rxs.call_button.try_recv() {
            let msg = elevio::ElevMessage {
                msg_type: elevio::ElevMsgType::CALLBTN,
                call_button: Some(call_button),
                floor_sensor: None,
                stop_button: None,
                obstruction: None,
            };
            let _ = local_elev_tx.send(msg).await;
        }

        if let Ok(floor) = rxs.floor_sensor.try_recv() {
            let msg = elevio::ElevMessage {
                msg_type: elevio::ElevMsgType::FLOORSENS,
                call_button: None,
                floor_sensor: Some(floor),
                stop_button: None,
                obstruction: None,
            };
            let _ = local_elev_tx.send(msg).await;
        }

        if let Ok(stop) = rxs.stop_button.try_recv() {
            let msg = elevio::ElevMessage {
                msg_type: elevio::ElevMsgType::STOPBTN,
                call_button: None,
                floor_sensor: None,
                stop_button: Some(stop),
                obstruction: None,
            };
            let _ = local_elev_tx.send(msg).await;
        }

        if let Ok(obstr) = rxs.obstruction.try_recv() {
            let msg = elevio::ElevMessage {
                msg_type: elevio::ElevMsgType::OBSTRX,
                call_button: None,
                floor_sensor: None,
                stop_button: None,
                obstruction: Some(obstr),
            };
            let _ = local_elev_tx.send(msg).await;
        }
        sleep(Duration::from_millis(10)).await;
    }
}


/// ### Handles messages from the local elevator
/// 
/// This function processes messages received from the local elevator and updates 
/// the worldview accordingly. It supports different message types such as call 
/// buttons, floor sensors, stop buttons, and obstruction notifications. It also 
/// manages the state of the elevator container based on the received data.
///
/// ## Parameters
/// - `local_elev_rx`: A mutable reference to the mpsc reciever recieving messages sent from [read_from_local_elevator].
/// - `container`: A mutable reference to the elevatorcontainer.
///
/// ## Behavior
/// The function reads all available messages on the mpsc reciever. Then it performs different actions based on the type of the message:
/// - **Call button (`CBTN`)**: Adds the call button to the `calls` list in the elevator container.
/// - **Floor sensor (`FSENS`)**: Updates the `last_floor_sensor` field in the elevator container.
/// - **Stop button (`SBTN`)**: A placeholder for future functionality to handle stop button messages.
/// - **Obstruction (`OBSTRX`)**: Sets the `obstruction` field in the elevator container to the 
///   received value.
pub async fn update_elev_container_from_msgs(local_elev_rx: &mut mpsc::Receiver<elevio::ElevMessage>, container: &mut ElevatorContainer, cab_priority_timer: &mut Timer, error_timer: &mut Timer) {
    loop{
        match local_elev_rx.try_recv() {
            Ok(msg) => {
                match msg.msg_type {
                    elevio::ElevMsgType::CALLBTN => {
                        if let Some(call_btn) = msg.call_button {
                            print::info(format!("Callbutton: {:?}", call_btn));
                            
                            match call_btn.call_type {
                                elevio::CallType::INSIDE => {
                                    cab_priority_timer.release_timer();
                                    container.cab_requests[call_btn.floor as usize] = true;
                                }
                                elevio::CallType::UP => {
                                    container.unsent_hall_request[call_btn.floor as usize][0] = true;
                                }
                                elevio::CallType::DOWN => {
                                    container.unsent_hall_request[call_btn.floor as usize][1] = true;
                                }
                                elevio::CallType::COSMIC_ERROR => {},
                            }   
                        }
                    }
            
                    elevio::ElevMsgType::FLOORSENS => {
                        print::info(format!("Floor: {:?}", msg.floor_sensor));
                        if let Some(floor) = msg.floor_sensor {
                            container.last_floor_sensor = floor;
                        }
                        
                    }
            
                    elevio::ElevMsgType::STOPBTN => {
                        print::info(format!("Stop button: {:?}", msg.stop_button));
                        if let Some(stop) = msg.stop_button {
                            container.stop = stop;
                        }
                    }
            
                    elevio::ElevMsgType::OBSTRX => {
                        print::info(format!("Obstruction: {:?}", msg.obstruction));
                        if let Some(obs) = msg.obstruction {
                            container.obstruction = obs;
                            if !obs && error_timer.timer_timeouted() {
                                error_timer.timer_start();
                                container.behaviour = ElevatorBehaviour::Idle; //må vekk
                            }
                        }
                    }
                }
            },
            Err(_) => {
                break;
            }
        }
    }

}









