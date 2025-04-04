


use crate::config;
use crate::world_view::{ElevatorBehaviour, WorldView};

use std::collections::HashMap; 
use std::env;
use std::fs::File;
use std::io::Write;
use serde::{Serialize, Deserialize};
use tokio::process::Command;



#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevatorState 
{
    behaviour: String,
    floor: i32,
    direction: String,
    cabRequests: Vec<bool>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct HallRequests 
{
    hallRequests: Vec<[bool; 2]>,
    states: HashMap<String, ElevatorState>,
}

/// This function executes the cost algorithm, and returns the output
pub async fn run_cost_algorithm(
    json_str: String
) -> String 
{
    let cost_path = env::current_dir()
        .unwrap()
        .join("libs")
        .join("Project_resources")
        .join("cost_fns")
        .join("hall_request_assigner")
        .join("hall_request_assigner");

    let output = Command::new("sudo")
        .arg(cost_path)
        .arg("--input")
        .arg(json_str)
        .output()
        .await
        .expect("Failed to start algorithm");

    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// This function creates the input to the cost function algorithm based on the worldview
pub async fn create_hall_request_json(
    wv: &WorldView
) -> Option<String> 
{
    let mut states = HashMap::new();
    for elev in wv.elevator_containers.iter() 
    {
        let key = elev.elevator_id.to_string();
        if elev.behaviour != ElevatorBehaviour::TravelError && elev.behaviour != ElevatorBehaviour::ObstructionError && elev.behaviour != ElevatorBehaviour::CosmicError 
        {
            states.insert(
                key,
                ElevatorState 
                {
                    behaviour: match elev.behaviour.clone() 
                    {
                        ElevatorBehaviour::DoorOpen => 
                        {
                            format!("doorOpen")
                        }
                        _ => 
                        {
                            format!("{:?}", elev.behaviour.clone()).to_lowercase()
                        }
                    },
                    
                    floor: if (0..elev.num_floors).contains(&elev.last_floor_sensor) 
                    {
                        elev.last_floor_sensor as i32
                    } else 
                    {
                        config::ERROR_ID as i32
                    },

                    direction: format!("{:?}", elev.dirn.clone()).to_lowercase(),
                    cabRequests: elev.cab_requests.clone(),
                },
            );
        }
    }

    if states.is_empty() {return None}

    let request = HallRequests 
    {
        hallRequests: wv.hall_request.clone(),
        states,
    };

    let s = serde_json::to_string_pretty(&request).expect("Failed to serialize");
    
    let mut file = File::create("hall_request.json").expect("Failed to create file");
    file.write_all(s.as_bytes()).expect("Failed to write to file");
    Some(s)
}

