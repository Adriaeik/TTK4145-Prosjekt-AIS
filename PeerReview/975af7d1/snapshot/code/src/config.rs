use log::info;
use serde::{Deserialize, Serialize};
// use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;

/*
================================
 Section for defining constants
================================
*/
// This should really be implemented using a struct of available button presses, as this won't (shouldn't) be able to change
pub const NUM_BUTTONS: u8 = 3;
/*
================================
         End of section
================================
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClearRequestVariant {
    All,
    InDir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub number_of_elevators: u8,
    pub number_of_floors: u8,
    pub polling_interval_ms: u64, // u64 since this is what Duration::from_millis() expects
    pub clear_request_variant: ClearRequestVariant,
    pub door_open_duration_seconds: f64,
    pub simulation_travel_duration_seconds: f64,
}

/// Load configuration from file config.json
/// If the file does not exist, it will be created with default values
impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = "config.json";
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap_or_else(|e| {
                panic!("Failed to open config-file: {e}");
            });

        let reader = BufReader::new(file);
        let config: Config = serde_json::from_reader(reader)?;
        Ok(config)
    }

    pub fn print(&self) {
        info!("===================== CONFIGURATION ======================");
        info!("Number of elevators: {}", self.number_of_elevators);
        info!("Number of floors: {}", self.number_of_floors);
        info!(
            "Polling interval: {} milliseconds",
            self.polling_interval_ms
        );
        info!("Clear request variant: {:?}", self.clear_request_variant);
        info!(
            "Door open duration: {} seconds",
            self.door_open_duration_seconds
        );
        info!(
            "Simulation travel duration: {} seconds",
            self.simulation_travel_duration_seconds
        );
        info!("==========================================================");
    }
}
