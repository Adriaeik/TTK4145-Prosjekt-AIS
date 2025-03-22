use std::collections::HashMap;
use tokio::{sync::{mpsc, watch}, time::sleep};
use crate::{config, world_view};
use crate::print;
mod json_serial;



/// Main task for managing elevator coordination.
/// 
/// Continuously listens for updates in the global world view (`wv_watch_rx`).
/// If the current node is the designated master, it computes and distributes tasks
/// to all known elevators using a cost-based assignment algorithm.
/// If the node is not the master, it waits for a defined slave timeout before checking again.
/// 
/// Behavior:
/// - Master nodes actively calculate and delegate hall requests.
/// - Slave nodes remain idle and periodically check for changes in master status.
/// 
/// Parameters:
/// - `wv_watch_rx`: A watch channel providing updates to the shared world view state.
/// - `delegated_tasks_tx`: A channel used to send the delegated hall tasks to other modules.
pub async fn start_manager(
    wv_watch_rx: watch::Receiver<Vec<u8>>, 
    delegated_tasks_tx: mpsc::Sender<HashMap<u8, Vec<[bool; 2]>>>
) {
    let mut wv = world_view::get_wv(wv_watch_rx.clone());

    loop {
        // Update local copy of the world view
        if world_view::update_wv(wv_watch_rx.clone(), &mut wv).await {
            // Check if this node is the master
            if world_view::is_master(wv.clone()) {
                // Calculate and send out delegated hall requests
                let _ = delegated_tasks_tx.send(get_elev_tasks(wv.clone()).await).await;
            } else {
                // If not master, wait before checking again
                sleep(config::SLAVE_TIMEOUT).await;
            }
        }

        // Polling delay to limit update frequency
        sleep(config::POLL_PERIOD).await;
    }
}

/// Generates a set of hall requests assigned to each elevator based on cost minimization.
///
/// This function first serializes the global world view to JSON, 
/// then passes the data to an external cost algorithm module which calculates the most optimal
/// assignment of hall calls to elevators. If successful, it returns a map of elevator IDs
/// to their respective assigned requests.
///
/// Behavior:
/// - If the cost algorithm returns a valid JSON string, it is parsed and returned as a HashMap.
/// - If the cost algorithm fails or returns an empty string, an error is logged and an empty map is returned.
/// - If JSON parsing fails, an error is also logged and an empty map is returned.
///
/// Parameters:
/// - `wv`: The serialized global world view as a byte vector.
///
/// Returns:
/// - A `HashMap` where each key is an elevator ID (`u8`), and each value is a list of `[bool; 2]` 
///   arrays indicating hall call assignments (up/down).
async fn get_elev_tasks(wv: Vec<u8>) -> HashMap<u8, Vec<[bool; 2]>> {
    let json_str = json_serial::create_hall_request_json(wv).await;

    if let Some(str) = json_str {
        let json_cost_str = json_serial::run_cost_algorithm(str.clone()).await;

        if json_cost_str.trim().is_empty() {
            print::err(format!(
                "run_cost_algorithm returned an empty string. "
            ));
            return HashMap::new();
        }

        return serde_json::from_str(&json_cost_str).unwrap_or_else(|e| {
            print::err(format!("Failed to parse JSON from cost algorithm: {}", e));
            HashMap::new()
        });
    }

    print::err("create_hall_request_json returned None.".to_string());
    HashMap::new()
}
