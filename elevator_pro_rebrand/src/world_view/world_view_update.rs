//! Help functions to update local worldview

// use crate::elevator_logic::master::wv_from_slaves::update_call_buttons;
use crate::world_view;
use crate::{config, print, ip_help_functions::{self}};
use crate::network::local_network;
use crate::elevio;
// use crate::manager::task_allocator::Task;

use tokio::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::collections::HashMap;

use crate::world_view::{get_index_to_container, serial, Dirn, ElevatorBehaviour};


static ONLINE: OnceLock<AtomicBool> = OnceLock::new(); 

/// Retrieves the current network status as an atomic boolean.
///
/// This function returns a reference to a static `AtomicBool`
/// that represents whether the system is online or offline.
///
/// # Returns
/// A reference to an `AtomicBool`:
/// - `true` if the system is online.
/// - `false` if the system is offline.
///
/// The initial value is `false` until explicitly changed.
pub fn get_network_status() -> &'static AtomicBool {
    ONLINE.get_or_init(|| AtomicBool::new(false))
}


/// Calls join_wv. See [join_wv]
/// TODO: drop denne funksjonen, la join_wv være join_wv_from_udp for å droppe unødvendige funksjoner
pub fn join_wv_from_udp(wv: &mut Vec<u8>, master_wv: Vec<u8>) -> bool {
    *wv = join_wv(wv.clone(), master_wv);
    true
}

/// Merges the local worldview with the master worldview received over UDP.
///
/// This function updates the local worldview (`my_wv`) by integrating relevant data
/// from `master_wv`. It ensures that the local elevator's status and tasks are synchronized
/// with the master worldview.
///
/// ## Arguments
/// * `my_wv` - A serialized `Vec<u8>` representing the local worldview.
/// * `master_wv` - A serialized `Vec<u8>` representing the worldview received over UDP.
///
/// ## Returns
/// A new serialized `Vec<u8>` representing the updated worldview.
///
/// ## Behavior
/// - If the local elevator exists in both worldviews, it updates its state in `master_wv`.
/// - Synchronizes `door_open`, `obstruction`, `last_floor_sensor`, and `motor_dir`.
/// - Updates `calls` and `tasks_status` with local data.
/// - Ensures that `tasks_status` retains only tasks present in `tasks`.
/// - If the local elevator is missing in `master_wv`, it is added to `master_wv`.
pub fn join_wv(mut my_wv: Vec<u8>, master_wv: Vec<u8>) -> Vec<u8> {
    let my_wv_deserialised = serial::deserialize_worldview(&my_wv);
    let mut master_wv_deserialised = serial::deserialize_worldview(&master_wv);

    let my_self_index = world_view::get_index_to_container(local_network::SELF_ID.load(Ordering::SeqCst) , my_wv);
    let master_self_index = world_view::get_index_to_container(local_network::SELF_ID.load(Ordering::SeqCst) , master_wv);


    if let (Some(i_org), Some(i_new)) = (my_self_index, master_self_index) {
        let my_view = &my_wv_deserialised.elevator_containers[i_org];
        let master_view = &mut master_wv_deserialised.elevator_containers[i_new];

        // Synchronize elevator status
        // master_view.status = my_view.status;
        master_view.dirn = my_view.dirn;
        master_view.behaviour = my_view.behaviour;
        master_view.obstruction = my_view.obstruction;
        master_view.last_floor_sensor = my_view.last_floor_sensor;

        // Update call buttons and task statuses
        // master_view.calls = my_view.calls.clone();
        master_view.unsent_hall_request = my_view.unsent_hall_request.clone();

        /* Update task statuses */
        // let new_ids: HashSet<u16> = master_view.tasks.iter().map(|t| t.id).collect();
        // let old_ids: HashSet<u16> = master_view.tasks_status.iter().map(|t| t.id).collect();

        // // Add missing tasks from master's task list
        // for task in master_view.tasks.clone().iter() {
        //     if !old_ids.contains(&task.id) {
        //         master_view.tasks_status.push(task.clone());
        //     }
        // }
        // // Remove outdated tasks from task_status
        // master_view.tasks_status.retain(|t| new_ids.contains(&t.id));

        // Call buttons synchronization is handled through TCP reliability

    } else if let Some(i_org) = my_self_index {
        // If the local elevator is missing in master_wv, add it
        master_wv_deserialised.add_elev(my_wv_deserialised.elevator_containers[i_org].clone());
    }

    my_wv = serial::serialize_worldview(&master_wv_deserialised);
    //utils::print_info(format!("Oppdatert wv fra UDP: {:?}", my_wv));
    my_wv 
}

/// ### 'Leaves' the network, removes all elevators that are not the current one
/// 
/// This function updates the local worldview by removing all elevators that do not
/// belong to the current entity, identified by `SELF_ID`.
/// 
/// The function first deserializes the worldview, removes all elevators that do not
/// have the correct `elevator_id`, updates the number of elevators, and sets the master
/// ID to `SELF_ID`. Then, the updated worldview is serialized back into `wv`.
/// 
/// ## Parameters
/// - `wv`: A mutable reference to a `Vec<u8>` representing the worldview.
/// 
/// ## Return Value
/// - Always returns `true` after the update.
///
/// ## Example
/// ```rust
/// let mut worldview = vec![/* some serialized data */];
/// abort_network(&mut worldview);
/// ```
pub fn abort_network(wv: &mut Vec<u8>) -> bool {
    let mut deserialized_wv = serial::deserialize_worldview(wv);
    deserialized_wv.elevator_containers.retain(|elevator| elevator.elevator_id == local_network::SELF_ID.load(Ordering::SeqCst));
    deserialized_wv.set_num_elev(deserialized_wv.elevator_containers.len() as u8);
    deserialized_wv.master_id = local_network::SELF_ID.load(Ordering::SeqCst);
    *wv = serial::serialize_worldview(&deserialized_wv);
    true
}

/// ### Updates the worldview based on a TCP message from a slave
/// 
/// This function processes a TCP message from a slave elevator, updating the local
/// worldview by adding the elevator if it doesn't already exist, or updating its
/// status and call buttons if it does.
///
/// The function first deserializes the TCP container and the current worldview.
/// It then checks if the elevator exists in the worldview and adds it if necessary.
/// After that, it updates the elevator's status and call buttons by calling appropriate
/// helper functions. Finally, it serializes the updated worldview and returns `true`.
/// If the elevator cannot be found in the worldview, an error message is printed and `false` is returned.
///
/// ## Parameters
/// - `wv`: A mutable reference to a `Vec<u8>` representing the worldview.
/// - `container`: A `Vec<u8>` containing the serialized data of the elevator's state.
///
/// ## Return Value
/// - Returns `true` if the update was successful, `false` if the elevator was not found in the worldview.
///
/// ## Example
/// ```
/// let mut worldview = vec![/* some serialized data */];
/// let container = vec![/* some serialized elevator data */];
/// join_wv_from_tcp_container(&mut worldview, container).await;
/// ```
pub async fn join_wv_from_tcp_container(wv: &mut Vec<u8>, container: Vec<u8>) -> bool {
    let deser_container = serial::deserialize_elev_container(&container);
    let mut deserialized_wv = serial::deserialize_worldview(&wv);

    // Hvis slaven ikke eksisterer, legg den til som den er
    if None == deserialized_wv.elevator_containers.iter().position(|x| x.elevator_id == deser_container.elevator_id) {
        deserialized_wv.add_elev(deser_container.clone());
    }

    let self_idx = world_view::get_index_to_container(deser_container.elevator_id, serial::serialize_worldview(&deserialized_wv));
    
    if let Some(i) = self_idx {

        // Legg til slave sine sendte hall_request til worldview sin hall_request
        for (row1, row2) in deserialized_wv.hall_request.iter_mut().zip(deser_container.unsent_hall_request.iter()) {
            for (val1, val2) in row1.iter_mut().zip(row2.iter()) {
                if !*val1 && *val2 {
                    *val1 = true;
                }
            }
        }

        //Oppdater statuser + fjerner tasks som er TaskStatus::DONE
        // deserialized_wv.elevator_containers[i].unsent_hall_request = deser_container.unsent_hall_request.clone();
        deserialized_wv.elevator_containers[i].elevator_id = deser_container.elevator_id;
        deserialized_wv.elevator_containers[i].last_floor_sensor = deser_container.last_floor_sensor;
        deserialized_wv.elevator_containers[i].num_floors = deser_container.num_floors;
        deserialized_wv.elevator_containers[i].obstruction = deser_container.obstruction;
        deserialized_wv.elevator_containers[i].dirn = deser_container.dirn;
        deserialized_wv.elevator_containers[i].behaviour = deser_container.behaviour;
        // Master styrer task, ikke overskriv det med slaven sitt forrige WV

        //Oppdater call_buttons
        // master::wv_from_slaves::update_call_buttons(&mut deserialized_wv, &deser_container, i).await;
        *wv = serial::serialize_worldview(&deserialized_wv);
        return true;
    } else {
        //Hvis dette printes, finnes ikke slaven i worldview. I teorien umulig, ettersom slaven blir lagt til over hvis den ikke allerede eksisterte
        print::cosmic_err("The elevator does not exist join_wv_from_tcp_conatiner()".to_string());
        return false;
    }
}

/// ### Removes a slave based on its ID
/// 
/// This function removes an elevator (slave) from the worldview by its ID. 
/// It first deserializes the current worldview, removes the elevator container 
/// with the specified ID, and then serializes the updated worldview back into 
/// the `wv` parameter.
///
/// ## Parameters
/// - `wv`: A mutable reference to a `Vec<u8>` representing the current worldview.
/// - `id`: The ID of the elevator (slave) to be removed.
///
/// ## Return Value
/// - Returns `true` if the removal was successful. In the current implementation, 
///   it always returns `true` after the removal, as long as no errors occur during 
///   the deserialization and serialization processes.
///
/// ## Example
/// ```rust
/// let mut worldview = vec![/* some serialized data */];
/// let elevator_id = 2;
/// remove_container(&mut worldview, elevator_id);
/// ```
pub fn remove_container(wv: &mut Vec<u8>, id: u8) -> bool {
    let mut deserialized_wv = serial::deserialize_worldview(&wv);
    deserialized_wv.remove_elev(id);
    *wv = serial::serialize_worldview(&deserialized_wv);
    true
}

/// ### Handles messages from the local elevator
/// 
/// This function processes messages received from the local elevator and updates 
/// the worldview accordingly. It supports different message types such as call 
/// buttons, floor sensors, stop buttons, and obstruction notifications. It also 
/// manages the state of the elevator container based on the received data.
///
/// ## Parameters
/// - `wv`: A mutable reference to a `Vec<u8>` representing the current worldview.
/// - `msg`: The `ElevMessage` received from the local elevator, containing the message type 
///   and associated data.
///
/// ## Return Value
/// - Returns `true` after processing the message and updating the worldview, indicating 
///   that the operation was successful.
///
/// ## Behavior
/// The function performs different actions based on the type of the message:
/// - **Call button (`CBTN`)**: Adds the call button to the `calls` list in the elevator container. 
///   If the current node is the master, it sends the updated container to the channel responsible for handling msg's from slaves, 
///   and clears the `calls` list.
/// - **Floor sensor (`FSENS`)**: Updates the `last_floor_sensor` field in the elevator container.
/// - **Stop button (`SBTN`)**: A placeholder for future functionality to handle stop button messages.
/// - **Obstruction (`OBSTRX`)**: Sets the `obstruction` field in the elevator container to the 
///   received value.
///
/// ## Example
/// ```rust
/// let mut worldview = vec![/* some serialized data */];
/// let msg = ElevMessage { msg_type: ElevMsgType::CBTN, /* other fields */ };
/// recieve_local_elevator_msg(&mut worldview, msg).await;
/// ```
pub async fn recieve_local_elevator_msg(master_container_tx: mpsc::Sender<Vec<u8>>, wv: &mut Vec<u8>, msg: elevio::ElevMessage) -> bool {
    let is_master = world_view::is_master(wv.clone());
    let mut deserialized_wv = serial::deserialize_worldview(&wv);
    let self_idx = world_view::get_index_to_container(local_network::SELF_ID.load(Ordering::SeqCst) , wv.clone());

    // Matcher hvilken knapp-type som er mottat
    match msg.msg_type {
        // Callbutton -> Legg den til i calls under egen heis-container
        elevio::ElevMsgType::CALLBTN => {
            print::info(format!("Callbutton: {:?}", msg.call_button));
            if let (Some(i), Some(call_btn)) = (self_idx, msg.call_button) {
                
                // Legger ti callbutton i tilsvarende vektor i elev_containeren
                match call_btn.call_type {
                    elevio::CallType::INSIDE => {
                        deserialized_wv.elevator_containers[i].cab_requests[call_btn.floor as usize] = true;
                    }
                    elevio::CallType::UP => {
                        deserialized_wv.elevator_containers[i].unsent_hall_request[call_btn.floor as usize][0] = true;
                    }
                    elevio::CallType::DOWN => {
                        deserialized_wv.elevator_containers[i].unsent_hall_request[call_btn.floor as usize][1] = true;
                    }
                    elevio::CallType::COSMIC_ERROR => {},
                }   


                //Om du er master i nettverket, oppdater call_buttons (Samme funksjon som kjøres i join_wv_from_tcp_container(). Behandler altså egen heis som en slave i nettverket) 
                if is_master {
                    let container = deserialized_wv.elevator_containers[i].clone();
                    
                    // update_call_buttons(&mut deserialized_wv, &container, i).await;
                    let _ = master_container_tx.send(serial::serialize_elev_container(&container)).await;
                    
                    deserialized_wv.elevator_containers[i].unsent_hall_request = vec![[false; 2]; deserialized_wv.elevator_containers[i].num_floors as usize];
                }
            }
        }

        // Floor_sensor -> oppdater last_floor_sensor i egen heis-container
        elevio::ElevMsgType::FLOORSENS => {
            print::info(format!("Floor: {:?}", msg.floor_sensor));
            if let (Some(i), Some(floor)) = (self_idx, msg.floor_sensor) {
                deserialized_wv.elevator_containers[i].last_floor_sensor = floor;
            }
            
        }

        // Stop_button -> funksjon kommer
        elevio::ElevMsgType::STOPBTN => {
            print::info(format!("Stop button: {:?}", msg.stop_button));
            
        }

        // Obstruction -> Sett obstruction lik melding fra heis i egen heis-container
        elevio::ElevMsgType::OBSTRX => {
            print::info(format!("Obstruction: {:?}", msg.obstruction));
            if let (Some(i), Some(obs)) = (self_idx, msg.obstruction) {
                deserialized_wv.elevator_containers[i].obstruction = obs;
            }
        }
    }
    *wv = serial::serialize_worldview(&deserialized_wv);
    true
}

/// ### Updates local call buttons and task statuses after they are sent over TCP to the master
/// 
/// This function processes the tasks and call buttons that have been sent to the master over TCP. 
/// It removes the updated tasks and sent call buttons from the local worldview, ensuring that the 
/// local state reflects the changes made by the master.
///
/// ## Parameters
/// - `wv`: A mutable reference to a `Vec<u8>` representing the current worldview.
/// - `tcp_container`: A vector containing the serialized data of the elevator container 
///   that was sent over TCP, including the tasks' status and call buttons.
///
/// ## Return Value
/// - Returns `true` if the update was successful and the worldview was modified.
/// - Returns `false` if the elevator does not exist in the worldview.
///
/// ## Example
/// ```rust
/// let mut worldview = vec![/* some serialized data */];
/// let tcp_container = vec![/* some serialized container data */];
/// clear_from_sent_tcp(&mut worldview, tcp_container);
/// ```
pub fn clear_from_sent_tcp(wv: &mut Vec<u8>, tcp_container: Vec<u8>) -> bool {
    let mut deserialized_wv = serial::deserialize_worldview(&wv);
    let self_idx = world_view::get_index_to_container(local_network::SELF_ID.load(Ordering::SeqCst) , wv.clone());
    let tcp_container_des = serial::deserialize_elev_container(&tcp_container);

    // Lagre task-IDen til alle sendte tasks. 
    // let tasks_ids: HashSet<u16> = tcp_container_des
    //     .tasks_status
    //     .iter()
    //     .map(|t| t.id)
    //     .collect();
    
    if let Some(i) = self_idx {
        /*_____ Fjern Tasks som master har oppdatert _____ */
        // deserialized_wv.elevator_containers[i].tasks_status.retain(|t| tasks_ids.contains(&t.id));
        /*_____ Fjern sendte Hall request _____ */
   
        for (row1, row2) in deserialized_wv.elevator_containers[i].unsent_hall_request
                                                        .iter_mut().zip(tcp_container_des.unsent_hall_request.iter()) {
            for (val1, val2) in row1.iter_mut().zip(row2.iter()) {
                if *val1 && *val2 {
                    *val1 = false;
                }
            }
        }


        *wv = serial::serialize_worldview(&deserialized_wv);
        return true;
    } else {
        print::cosmic_err("The elevator does not exist clear_sent_container_stuff()".to_string());
        return false;
    }
}


pub fn distribute_tasks(wv: &mut Vec<u8>, map: HashMap<u8, Vec<[bool; 2]>>) -> bool {
    let mut wv_deser = world_view::serial::deserialize_worldview(&wv.clone());

    for elev in wv_deser.elevator_containers.iter_mut() {
        if let Some(tasks) = map.get(&elev.elevator_id) {
            elev.tasks = tasks.clone();
        }
    }

    *wv = world_view::serial::serialize_worldview(&wv_deser);

    true
}




// / Push `some_task` to elevator with id `id` in `wv`
// / 
// / ## Return
// / `true`: Elevator with `id` was found and given the task  
// / `false`: Otherwise
// pub fn push_task(wv: &mut Vec<u8>, id: u8, some_task: Option<Task>) -> bool {
//     let mut deser_wv = serial::deserialize_worldview(&wv);

//     let index = get_index_to_container(id, wv.clone());
//     if let Some(i) = index {
//         deser_wv.elevator_containers[i].task = some_task;
//         *wv = serial::serialize_worldview(&deser_wv);
//         return true
//     } else {
//         return false
//     }



//     // Fjern `button` frå `outside_button` om han finst
//     // if let Some(index) = deser_wv.outside_button.iter().position(|b| *b == button) {
//     //     deser_wv.outside_button.swap_remove(index);
//     // }
    
//     // let self_idx = world_view::get_index_to_container(id, wv.clone());

//     // if let Some(i) = self_idx {
//     //     // **Hindrar duplikatar: sjekk om task.id allereie finst i `tasks`**
//     //     // NB: skal i teorien være unødvendig å sjekke dette
//     //     if !deser_wv.elevator_containers[i].tasks.iter().any(|t| t.id == task.id) {
//     //         deser_wv.elevator_containers[i].tasks.push(task);
//     //         *wv = world_view::serialize_worldview(&deser_wv);
//     //         return true;
//     //     }
//     // }
    
//     // false
// }

// // / ### Oppdaterer status til `new_status` til task med `id` i egen heis_container.tasks_status
// /// Updates status of elevator with id matching [local_network::SELF_ID] to status in wv
// /// 
// /// ## Returns
// /// `true`: Elevator with SELF_ID was found, and status was updated
// /// `false`: otherwise
// pub fn update_elev_state(wv: &mut Vec<u8>, status: ElevatorStatus) -> bool {
//     let mut wv_deser = serial::deserialize_worldview(&wv);
//     let self_idx = world_view::get_index_to_container(local_network::SELF_ID.load(Ordering::SeqCst), wv.clone());

//     if let Some(i) = self_idx {
//         wv_deser.elevator_containers[i].status = status;
//         *wv = serial::serialize_worldview(&wv_deser);
//         return true;
//     }
//     // println!("Satt {:?} på id: {}", new_status, task_id);
//     false
// }

/// Monitors the Ethernet connection status asynchronously.
///
/// This function continuously checks whether the device has a valid network connection.
/// It determines connectivity by verifying that the device's IP matches the expected network prefix.
/// The network status is stored in a shared atomic boolean [get_network_status()].
///
/// ## Behavior
/// - Retrieves the device's IP address using `utils::get_self_ip()`.
/// - Extracts the root IP using `utils::get_root_ip()` and compares it to `config::NETWORK_PREFIX`.
/// - Updates the network status (`true` if connected, `false` if disconnected).
/// - Prints status changes:  
///   - `"Vi er online"` when connected.  
///   - `"Vi er offline"` when disconnected.
///
/// ## Note
/// This function runs in an infinite loop and should be spawned as an asynchronous task.
///
/// ## Example
/// ```
/// use tokio;
/// # #[tokio::test]
/// # async fn test_watch_ethernet() {
/// tokio::spawn(async {
///     watch_ethernet().await;
/// });
/// # }
/// ```
pub async fn watch_ethernet() {
    let mut last_net_status = false;
    let mut net_status;
    loop {
        let ip = local_network::get_self_ip();

        match ip {
            Ok(ip) => {
                if ip_help_functions::get_root_ip(ip) == config::NETWORK_PREFIX {
                    net_status = true;
                }
                else {
                    net_status = false   
                }
            }
            Err(_) => {
                net_status = false
            }
        }

        if last_net_status != net_status {  
            get_network_status().store(net_status, Ordering::SeqCst);
            if net_status {print::ok("Vi er online".to_string());}
            else {print::warn("Vi er offline".to_string());}
            last_net_status = net_status;
        }
    }
}

// / Updates tasks in `wv` to `tasks`
// / 
// / ## Returns
// / `true`: always
// pub fn publish_tasks(wv: &mut Vec<u8>, tasks: Vec<Task>) -> bool {
//     let mut wv_deser = serial::deserialize_worldview(&wv);
//     wv_deser.pending_tasks = tasks;
//     *wv = serial::serialize_worldview(&wv_deser);
//     true
// }

