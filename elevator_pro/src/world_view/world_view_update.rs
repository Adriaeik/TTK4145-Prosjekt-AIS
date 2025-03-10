use crate::world_view::world_view;
use crate::{config, utils};

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;


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
    let my_wv_deserialised = world_view::deserialize_worldview(&my_wv);
    let mut master_wv_deserialised = world_view::deserialize_worldview(&master_wv);

    let my_self_index = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , my_wv);
    let master_self_index = world_view::get_index_to_container(utils::SELF_ID.load(Ordering::SeqCst) , master_wv);


    if let (Some(i_org), Some(i_new)) = (my_self_index, master_self_index) {
        let my_view = &my_wv_deserialised.elevator_containers[i_org];
        let master_view = &mut master_wv_deserialised.elevator_containers[i_new];

        // Synchronize elevator status
        master_view.door_open = my_view.door_open;
        master_view.obstruction = my_view.obstruction;
        master_view.last_floor_sensor = my_view.last_floor_sensor;
        master_view.motor_dir = my_view.motor_dir;

        // Update call buttons and task statuses
        master_view.calls = my_view.calls.clone();
        master_view.tasks_status = my_view.tasks_status.clone();

        /* Update task statuses */
        let new_ids: HashSet<u16> = master_view.tasks.iter().map(|t| t.id).collect();
        let old_ids: HashSet<u16> = master_view.tasks_status.iter().map(|t| t.id).collect();

        // Add missing tasks from master's task list
        for task in master_view.tasks.clone().iter() {
            if !old_ids.contains(&task.id) {
                master_view.tasks_status.push(task.clone());
            }
        }
        // Remove outdated tasks from task_status
        master_view.tasks_status.retain(|t| new_ids.contains(&t.id));

        // Call buttons synchronization is handled through TCP reliability

    } else if let Some(i_org) = my_self_index {
        // If the local elevator is missing in master_wv, add it
        master_wv_deserialised.add_elev(my_wv_deserialised.elevator_containers[i_org].clone());
    }

    my_wv = world_view::serialize_worldview(&master_wv_deserialised);
    //utils::print_info(format!("Oppdatert wv fra UDP: {:?}", my_wv));
    my_wv 
}

/// ### Monitors the Ethernet connection status asynchronously.
///
/// This function continuously checks whether the device has a valid network connection.
/// It determines connectivity by verifying that the device's IP matches the expected network prefix.
/// The network status is stored in a shared atomic boolean (`get_network_status()`).
///
/// # Behavior
/// - Retrieves the device's IP address using `utils::get_self_ip()`.
/// - Extracts the root IP using `utils::get_root_ip()` and compares it to `config::NETWORK_PREFIX`.
/// - Updates the network status (`true` if connected, `false` if disconnected).
/// - Prints status changes:  
///   - `"Vi er online"` when connected.  
///   - `"Vi er offline"` when disconnected.
///
/// # Note
/// This function runs in an infinite loop and should be spawned as an asynchronous task.
///
/// # Example
/// ```
/// 
/// tokio::spawn(async {
///     watch_ethernet().await;
/// });
/// ```
pub async fn watch_ethernet() {
    let mut last_net_status = false;
    let mut net_status = false;
    loop {
        let ip = utils::get_self_ip();

        match ip {
            Ok(ip) => {
                if utils::get_root_ip(ip) == config::NETWORK_PREFIX {
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
            if net_status {utils::print_ok("Vi er online".to_string());}
            else {utils::print_warn("Vi er offline".to_string());}
            last_net_status = net_status;
        }
    }
}



