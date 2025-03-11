use crate::config::Config;
use crate::elevator::Elevator;
use crate::order::HallOrders;

use log::{debug, error};
use serde_json::{json /*Value*/};
use std::collections::HashMap;
use std::error::Error;
use std::process::Command;

// #[derive(Debug)]
// pub struct DistributionEntry {
//     pub elevator: Elevator,
//     pub requests: HallOrders,
// }
pub type Distribution = HashMap<String, HallOrders>;

pub fn distribute_orders(
    config: &Config,
    elevators: Vec<Elevator>,
    requests: HallOrders,
) -> Result<Distribution, Box<dyn Error>> {
    // Determine executable based on OS
    let os = std::env::consts::OS;
    let executable = match os {
        "linux" => "hall_request_assigner_linux",
        "macos" => "hall_request_assigner_macos",
        _ => {
            let err_msg = format!("Unsupported OS: {os}");
            // error!("{err_msg}");
            return Err(err_msg.into());
        }
    };

    // Fill hall requests array with existing orders
    const UP_INDEX: usize = 0;
    const DOWN_INDEX: usize = 1;
    let mut hall_requests = vec![[false, false]; config.number_of_floors as usize];
    for request in &requests.up {
        hall_requests[request.floor as usize][UP_INDEX] = true;
    }
    for request in &requests.down {
        hall_requests[request.floor as usize][DOWN_INDEX] = true;
    }
    debug!("Hall requests: {:?}", hall_requests);

    // Create JSON object for each elevator
    let mut elevator_states = HashMap::new();
    for elevator in &elevators {
        let elevator_name = &elevator.network_node_name;
        let current_floor = match elevator.current_floor {
            Some(floor) => floor,
            None => {
                let err_msg = format!("Current floor of {elevator_name} is None");
                // error!("{err_msg}");
                return Err(err_msg.into());
            }
        };

        let direction = match elevator.direction {
            Some(dir) => dir,
            None => {
                let err_msg = format!("Direction of {elevator_name} is None");
                // error!("{err_msg}");
                return Err(err_msg.into());
            }
        };

        let mut cab_requests = vec![false; config.number_of_floors as usize];
        for request in &elevator.cab_orders {
            cab_requests[request.floor as usize] = true;
        }

        let state = json!({
            "behaviour": elevator.behaviour.to_string(),
            "floor": current_floor,
            "direction": direction.to_string(),
            "cabRequests": cab_requests
        });

        elevator_states.insert(elevator.network_node_name.clone(), state);
    }
    debug!("Elevator states: {:?}", elevator_states);

    // Construct JSON input
    let input_json = json!({
        "hallRequests": hall_requests,
        "states": elevator_states
    });

    // Serialize to a string
    let input_json_string = serde_json::to_string_pretty(&input_json).map_err(|e| {
        let err_msg = format!("Failed to serialize input JSON: {e}");
        // error!("{err_msg}");
        err_msg
    })?;
    debug!("Input JSON: {}", input_json_string);

    // Call external process
    let output = Command::new(format!("src/binaries/{}", executable))
        // .arg("--includeCab") // Not necessary, since the cab orders have been included in the simulation, and can only be cleared by the elevator itself anyway
        .arg("--input")
        .arg(&input_json_string)
        .output()
        .map_err(|e| {
            let err_msg = format!("Failed to execute external command: {e}");
            // error!("{err_msg}");
            err_msg
        })?;

    if !output.status.success() {
        let err_msg = format!("External process failed with status: {:?}", output.status);
        // error!("{err_msg}");
        return Err(err_msg.into());
    }
    debug!("Command executed successfully");

    // Convert output to string
    let output_string = String::from_utf8(output.stdout).map_err(|e| {
        let err_msg = format!("Failed to convert output to UTF-8: {e}");
        // error!("{err_msg}");
        err_msg
    })?;
    debug!("Output string: {output_string}");

    // Parse output JSON
    let output_json: serde_json::Value = serde_json::from_str(&output_string).map_err(|e| {
        let err_msg = format!("Failed to parse output JSON: {e}");
        // error!("{err_msg}");
        err_msg
    })?;
    debug!("Output JSON: {:#?}", output_json);

    let output_obj = match output_json.as_object() {
        Some(obj) => obj,
        None => {
            let err_msg = format!("Expected JSON object, but got: {:#?}", output_json);
            // error!("{err_msg}");
            return Err(err_msg.into());
        }
    };

    // Convert JSON response to `Distribution`
    let mut order_distribution: Distribution = HashMap::new();
    for (elevator_name, entry) in output_obj {
        let mut requests = HallOrders {
            up: Vec::new(),
            down: Vec::new(),
        };
        let floors = entry.as_array().ok_or("Expected entry to be an array")?;
        for (floor, floor_requests) in floors.iter().enumerate() {
            // Ensure `floor_requests` is also a valid JSON array
            let directions = floor_requests.as_array().ok_or("Expected floor_requests to be an array")?;
            for (dir, request) in directions.iter().enumerate() {
                let is_requested = request.as_bool().ok_or("Expected request to be a boolean")?;
                if is_requested {
                    let direction = match dir {
                        0 => crate::types::Direction::Up,
                        1 => crate::types::Direction::Down,
                        _ => {
                            let err_msg = format!("Invalid direction index: {dir}");
                            error!("{err_msg}");
                            return Err(err_msg.into());
                        }
                    };
                    requests.add_order(direction, floor as u8);
                }
            }
        }
        order_distribution.insert(elevator_name.to_string(), requests);
    }

    debug!("Final distribution: {:#?}", order_distribution);
    Ok(order_distribution)
}
