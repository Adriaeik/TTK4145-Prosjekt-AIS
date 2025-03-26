//! This private module provides helper functions for modifying and merging `WorldView` instances
//! within the distributed elevator control system.
//!
//! It is only accessible from the `local_network` module and is responsible for:
//! - Merging local elevator state into the global worldview (via UDP, TCP, or reconnection).
//! - Cleaning up after disconnections (e.g., removing other elevators from the view).
//! - Updating hall/cab requests and elevator statuses after communication.
//! - Resolving conflicts between competing `WorldView` instances during re-entry or master election.
//!
//! ## Purpose
//! In a distributed elevator network, each elevator must maintain an up-to-date view of all other
//! elevators and tasks. This module supports that goal by:
//! - Integrating state from slaves into the master's worldview.
//! - Ensuring task handover and cleanup is handled consistently.
//! - Managing elevator status transitions between connected and offline states.
//!
//! ## Scope and Access
//! This module is private by design and tightly coupled with `local_network`. It is not intended
//! for external access or direct use by high-level logic such as elevator motion or button handling.
//!
//! ## Common Usage Scenarios
//! - Receiving a `WorldView` over UDP during initial network discovery.
//! - Integrating a serialized `ElevatorContainer` received via TCP from a slave.
//! - Cleaning the worldview when a node disconnects or goes offline.
//! - Distributing or clearing tasks after successful transmission.
//!
//! ## Design Notes
//! The merging logic assumes known roles (e.g., master or slave) and behaves accordingly.
//! For instance, a newly elected master may overwrite or discard conflicting data to maintain
//! system consistency.

use crate::world_view::{self, Dirn, ElevatorBehaviour, ElevatorContainer, WorldView};
use crate::print;
use crate::network;

use std::collections::HashMap;




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
pub fn join_wv_from_udp(my_wv: &mut WorldView, master_wv: &mut WorldView) -> bool {
    let my_self_index = world_view::get_index_to_container(network::read_self_id() , my_wv);
    let master_self_index = world_view::get_index_to_container(network::read_self_id() , master_wv);
    
    
    if let (Some(i_org), Some(i_new)) = (my_self_index, master_self_index) {
        let my_view = &my_wv.elevator_containers[i_org];
        let master_view = &mut master_wv.elevator_containers[i_new];
        
        
        
        // Synchronize elevator status
        master_view.dirn = my_view.dirn;
        master_view.behaviour = my_view.behaviour;
        master_view.obstruction = my_view.obstruction;
        master_view.last_floor_sensor = my_view.last_floor_sensor;
        master_view.unsent_hall_request = my_view.unsent_hall_request.clone();
        master_view.cab_requests = my_view.cab_requests.clone();


    } else if let Some(i_org) = my_self_index {
        // If the local elevator is missing in master_wv, add it
        master_wv.add_elev(my_wv.elevator_containers[i_org].clone());
    }

    *my_wv = master_wv.clone();
    true
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
pub fn abort_network(wv: &mut WorldView) -> bool {
    wv.elevator_containers.retain(|elevator| elevator.elevator_id == network::read_self_id());
    wv.set_num_elev(wv.elevator_containers.len() as u8);
    wv.master_id = network::read_self_id();
    //TODO hent ut self index istedenfor 0 indeks
    wv.hall_request = merge_hall_requests(&wv.hall_request, &wv.elevator_containers[0].tasks);
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
pub async fn join_wv_from_tcp_container(wv: &mut WorldView, container: &ElevatorContainer) -> bool {

    // If the slave does not exist, add it as-is
    if None == wv.elevator_containers.iter().position(|x| x.elevator_id == container.elevator_id) {
        wv.add_elev(container.clone());
    }

    let self_idx = world_view::get_index_to_container(container.elevator_id, &wv);
    
    if let Some(i) = self_idx {
        // Add the slave's sent hall_requests to worldview's hall_requests
        for (row1, row2) in wv.hall_request.iter_mut().zip(container.unsent_hall_request.iter()) {
            for (val1, val2) in row1.iter_mut().zip(row2.iter()) {
                if !*val1 && *val2 {
                    *val1 = true;
                }
            }
        }

        // Add slaves unfinished tasks to hall_requests
        wv.hall_request = merge_hall_requests(&wv.hall_request, &wv.elevator_containers[i].tasks);
        
        if world_view::is_master(wv) {
            wv.elevator_containers[i].unsent_hall_request = vec![[false; 2]; wv.elevator_containers[i].num_floors as usize];
        }

        //Update statuses
        wv.elevator_containers[i].cab_requests = container.cab_requests.clone();
        wv.elevator_containers[i].elevator_id = container.elevator_id;
        wv.elevator_containers[i].last_floor_sensor = container.last_floor_sensor;
        wv.elevator_containers[i].num_floors = container.num_floors;
        wv.elevator_containers[i].obstruction = container.obstruction;
        wv.elevator_containers[i].dirn = container.dirn;
        wv.elevator_containers[i].behaviour = container.behaviour;

        //Remove taken hall_requests
        for (idx, [up, down]) in wv.hall_request.iter_mut().enumerate() {
            if (wv.elevator_containers[i].behaviour == ElevatorBehaviour::DoorOpen) && (wv.elevator_containers[i].last_floor_sensor == (idx as u8)) {
                if wv.elevator_containers[i].dirn == Dirn::Up {
                    *up = false;
                } else if wv.elevator_containers[i].dirn == Dirn::Down {
                    *down = false;
                }
            }
        }

        // Back up the cab requests
        update_cab_request_backup(&mut wv.cab_requests_backup, wv.elevator_containers[i].clone());

        return true;
    } else {
        // If this is printed, the slave does not exist in the worldview. This is theoretically impossible, as the slave is added to the worldview just before this if it does not already exist.
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
pub fn remove_container(wv: &mut WorldView, id: u8) -> bool {
    wv.remove_elev(id);
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
pub fn clear_from_sent_tcp(wv: &mut WorldView, tcp_container: ElevatorContainer) -> bool {
    let self_idx = world_view::get_index_to_container(network::read_self_id() , &wv);
    
    if let Some(i) = self_idx {
        /*_____ Remove sent Hall request _____ */
   
        for (row1, row2) in wv.elevator_containers[i].unsent_hall_request
                                                        .iter_mut().zip(tcp_container.unsent_hall_request.iter()) {
            for (val1, val2) in row1.iter_mut().zip(row2.iter()) {
                if *val1 && *val2 {
                    *val1 = false;
                }
            }
        }
        return true;
    } else {
        // If this is printed, you do not exist in your worldview
        print::cosmic_err("The elevator does not exist clear_sent_container_stuff()".to_string());
        return false;
    }
}

/// This function allocates tasks from the given map to the corresponding elevator_container's tasks vector
/// 
/// # Parameters
/// `wv`: A mutable reference to a serialized worldview
///  
/// # Behavior
/// - Iterates through every elevator_container in the worldview
/// - If any tasks in the map matches the elevators ID, it sets the elevators tasks equal to the map's tasks
/// 
/// # Return
/// true
/// 
pub fn distribute_tasks(wv: &mut WorldView, map: HashMap<u8, Vec<[bool; 2]>>) -> bool {
    for elev in wv.elevator_containers.iter_mut() {
        if let Some(tasks) = map.get(&elev.elevator_id) {
            elev.tasks = tasks.clone();
        }
    }

    true
}


/// Updates states to the elevator in wv with same ID as container 
pub fn update_elev_states(wv: &mut WorldView, container: ElevatorContainer) -> bool {
    let idx = world_view::get_index_to_container(container.elevator_id, wv);

    if let Some(i) = idx {
        wv.elevator_containers[i].cab_requests = container.cab_requests;
        wv.elevator_containers[i].dirn = container.dirn;
        wv.elevator_containers[i].obstruction = container.obstruction;
        wv.elevator_containers[i].behaviour = container.behaviour;
        wv.elevator_containers[i].last_floor_sensor = container.last_floor_sensor;
        wv.elevator_containers[i].unsent_hall_request = container.unsent_hall_request;
    }
    true
}

/// Updates the backup hashmap for cab_requests, så they are remembered on the network in the case of power loss on a node
/// 
/// ## Parameters
/// `backup`: A mutable reference to the backup hashmap in the worldview
/// `container`: The new ElevatorContainer recieved
/// 
/// ## Behaviour
/// Insert the container's cab_requests in key: container.elevator_id. If no old keys matches the id, a new entry is added. 
fn update_cab_request_backup(backup: &mut HashMap<u8, Vec<bool>>, container: ElevatorContainer) {
    backup.insert(container.elevator_id, container.cab_requests);
}





/// Merges local worldview with networks worldview after being offline
/// 
/// # Parameters
/// `my_wv`: Mutable reference to the local worldview
/// `read_wv`: Reference to the networks worldview
pub fn merge_wv_after_offline(my_wv: &mut WorldView, read_wv: &mut WorldView) {
    /* If you become the new master on the system */
    if my_wv.master_id < read_wv.master_id {
        read_wv.hall_request = merge_hall_requests(&read_wv.hall_request, &my_wv.hall_request);
        read_wv.master_id = my_wv.master_id;
        let my_wv_elevs: Vec<ElevatorContainer> = my_wv.elevator_containers.clone();

        /* Map the IDs in the networks worldview */
        let existing_ids: std::collections::HashSet<u8> = read_wv
            .elevator_containers
            .iter()
            .map(|e| e.elevator_id)
            .collect();

        /* Add elevators you had which the network didnt know about (yourself) */
        for elev in my_wv_elevs {
            if !existing_ids.contains(&elev.elevator_id) {
                read_wv.elevator_containers.push(elev);
            }
        }

    } else {
        read_wv.hall_request = merge_hall_requests(&read_wv.hall_request, &my_wv.hall_request);
    }

    *my_wv = read_wv.clone();
}

/// Function to merge hall requests
/// 
/// # Parameters
/// `hall_req_1`: Reference to one hall request vector  
/// `hall_req_2`: Reference to other hall request vector
/// 
/// # Return
/// The merged hall request vector
/// 
/// # Behavior
/// The function merges the requests by performing an element-wise OR operation on all indexes.
/// If one vector is longer than the other, the shorter one is treated as if it had all extra values set to false.
/// 
/// # Example
/// ```
/// use elevatorpro::world_view::world_view_update::merge_hall_requests;
/// 
/// let hall_req_1 = vec![[true, false], [false, false]];
/// let hall_req_2 = vec![[false, true], [false, true]];
/// let merged_vec = merge_hall_requests(&hall_req_1, &hall_req_2);
/// 
/// assert_eq!(merged_vec, vec![[true, true], [false, true]]);
/// 
/// 
/// let hall_req_3 = vec![[true, false], [false, false], [true, false]];
/// let merged_vec_2 = merge_hall_requests(&hall_req_3, &merged_vec);
/// 
/// assert_eq!(merged_vec_2, vec![[true, true], [false, true], [true, false]]);
/// 
/// ```
/// 
fn merge_hall_requests(hall_req_1: &Vec<[bool; 2]>, hall_req_2: &Vec<[bool; 2]>) -> Vec<[bool; 2]> {
    let mut merged_hall_req = hall_req_1.clone();
    merged_hall_req
        .iter_mut()
        .zip(hall_req_2)
        .for_each(|(read, my)| {
            read[0] |= my[0];
            read[1] |= my[1];
        });
    
    if hall_req_2.len() > hall_req_1.len() {
        merged_hall_req
            .extend_from_slice(&hall_req_2[hall_req_1.len()..]);
    }

    merged_hall_req
}
