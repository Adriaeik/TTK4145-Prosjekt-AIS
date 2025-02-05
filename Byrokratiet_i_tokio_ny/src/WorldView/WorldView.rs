use serde::{Serialize, Deserialize};
use std::error::Error;
use crate::{config, Byrokrati::konsulent};
use termcolor::Color;


#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CallButton {
    pub floor: u8, // Default: 0
    pub call: u8,  // Default: 0
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlenemorDel {
    pub heis_id: u8,                // Default: 0
    pub inside_button: Vec<CallButton>, // Default: 6 default callbutton
    pub door_open: bool,            // Default: false
    pub obstruction: bool,          // Default: false
    pub motor_dir: u8,              // Default: 0
    pub last_floor_sensor: u8,      // Default: 255
}
impl Default for AlenemorDel {
    fn default() -> Self {
        Self {
            heis_id: 0,
            inside_button: Vec::new(), // 6 knapper med default-verdi
            door_open: false,
            obstruction: false,
            motor_dir: 0,
            last_floor_sensor: 255, // Spesifikk verdi for sensor
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct WorldView {
    //Generelt nettverk
    n: u8,                             // Antall heiser
    pub master_id: u8,                     // Master IP
    //Generelle oppgaver til heisen
    pub outside_button: Vec<CallButton>,            // Array til knappene trykt på utsiden
    //Heisspesifikt
    pub heis_spesifikke: Vec<AlenemorDel>,   //Info som gjelder per-heis

}


impl Default for WorldView {
     fn default() -> Self {
        Self {
            n: 0,
            master_id: config::ERROR_ID,
            outside_button: Vec::new(), 
            heis_spesifikke: Vec::new(),
        }
    }
}


impl WorldView {
    pub fn rapporter_annsettelse_av_mor(&mut self, mor: AlenemorDel) {
        self.n = self.n + 1;
        self.heis_spesifikke.push(mor);
    }
    
    pub fn rapporter_sparking_av_mor(&mut self, id: u8) {
        let initial_len = self.heis_spesifikke.len();

        self.heis_spesifikke.retain(|mor| mor.heis_id != id);
    
        if self.heis_spesifikke.len() == initial_len {
            konsulent::print_farge(format!("Ingen mor med ID {} ble funnet. (rapporter_sparking_av_mor())", id), Color::Yellow);
        } else {
            println!("Mor med ID {} ble fjernet.", id);
            konsulent::print_farge(format!("Mor med ID {} ble fjernet. (rapporter_sparking_av_mor())", id), Color::Green);
        }
    }

    pub fn hvor_mange_modre_har_jeg(&self) -> u8 {
        return self.n;
    }
}



pub fn serialize_worldview(worldview: &WorldView) -> Result<Vec<u8>, Box<dyn Error>> {
    let encoded: Vec<u8> = bincode::serialize(worldview)?;
    Ok(encoded)
}

// Funksjon for å deserialisere WorldView
pub fn deserialize_worldview(data: &[u8]) -> Result<WorldView, Box<dyn Error>> {
    let decoded: WorldView = bincode::deserialize(data)?;
    Ok(decoded)
}





//Eksempel på bruk!!
// async fn main() {
    
//     let mut am1 = AlenemorDel::default();
//     am1.heis_id = 69;
//     let mut worldview = WorldView::default();
//     worldview.add_heis(am1);


//     // Serialisere WorldView til binært format (Result<Vec<u8>, Box<dyn Error>>)
//     let serialized = serialize_worldview(&worldview);

//     match serialized {
//         Ok(serialized_data) => {
//             // Deserialisere WorldView fra binært format
//             let deserialized = deserialize_worldview(&serialized_data);

//             match deserialized {
//                 Ok(worldview) => {
//                     println!("Deserialized: {:?}", worldview);
//                     println!("worldview size: {}", std::mem::size_of_val(&worldview));
//                 }
//                 Err(e) => {
//                     println!("Deserialization failed: {}", e);
//                 }
//             }
//         }
//         Err(e) => {
//             println!("Serialization failed: {}", e);
//         }
//     }
// }
