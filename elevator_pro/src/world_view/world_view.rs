use serde::{Serialize, Deserialize};
use crate::config;
use crate::utils;
use prettytable::{Table, Row, Cell, format};
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CallButton {
    pub floor: u8, // Default: 0
    pub call: u8,  // Default: 0
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Task {
    pub to_do: u8, // Default: 0
    pub status: u8, // 1: done, 0: to_do, 255: be master deligere denne på nytt
}




#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ElevatorContainer {
    pub elevator_id: u8,            // Default: 0
    pub calls: Vec<CallButton>,   // Default: vektor med Tasks
    pub tasks: Vec<Task>,         // Default: vektor med Tasks
    pub door_open: bool,            // Default: false
    pub obstruction: bool,          // Default: false
    pub motor_dir: u8,              // Default: 0
    pub last_floor_sensor: u8,      // Default: 255
}
impl Default for ElevatorContainer {
    fn default() -> Self {
        Self {
            elevator_id: 0,
            calls: Vec::new(),
            tasks: Vec::new(),
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
    pub elevator_containers: Vec<ElevatorContainer>,   //Info som gjelder per-heis

}


impl Default for WorldView {
     fn default() -> Self {
        Self {
            n: 0,
            master_id: config::ERROR_ID,
            outside_button: Vec::new(), 
            elevator_containers: Vec::new(),
        }
    }
}


impl WorldView {
    pub fn add_elev(&mut self, elevator: ElevatorContainer) {
        self.n = self.n + 1;
        utils::print_ok(format!("elevator med ID {} ble ansatt. (add_elev())", elevator.elevator_id));
        self.elevator_containers.push(elevator);
    }
    
    pub fn remove_elev(&mut self, id: u8) {
        let initial_len = self.elevator_containers.len();

        self.elevator_containers.retain(|elevator| elevator.elevator_id != id);
    
        if self.elevator_containers.len() == initial_len {
            utils::print_warn(format!("Ingen elevator med ID {} ble funnet. (remove_elev())", id));
        } else {
            utils::print_ok(format!("elevator med ID {} ble sparka. (remove_elev())", id));
        }
    }

    pub fn get_num_elev(&self) -> u8 {
        return self.n;
    }
}




pub fn serialize_worldview(worldview: &WorldView) -> Vec<u8> {
    let encoded = bincode::serialize(worldview);
    match encoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return serialized_data;
        }
        Err(e) => {
            utils::print_err(format!("Serialization failed: {} (world_view.rs, serialize_worldview())", e));
            panic!();
        }
    }
}

// Funksjon for å deserialisere WorldView
pub fn deserialize_worldview(data: &[u8]) -> WorldView {
    let decoded = bincode::deserialize(data);


    match decoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return serialized_data;
        }
        Err(e) => {
            utils::print_err(format!("Serialization failed: {} (world_view.rs, deserialize_worldview())", e));
            panic!();
        }
    }
}


pub fn serialize_elev_container(elev_container: &ElevatorContainer) -> Vec<u8> {
    let encoded = bincode::serialize(elev_container);
    match encoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return serialized_data;
        }
        Err(e) => {
            utils::print_err(format!("Serialization failed: {} (world_view.rs, serialize_elev_container())", e));
            panic!();
        }
    }
}

// Funksjon for å deserialisere WorldView
pub fn deserialize_elev_container(data: &[u8]) -> ElevatorContainer {
    let decoded = bincode::deserialize(data);


    match decoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return serialized_data;
        }
        Err(e) => {
            utils::print_err(format!("Serialization failed: {} (world_view.rs, deserialize_elev_container())", e));
            panic!();
        }
    }
}



//Eksempel på bruk!!
// async fn main() {
    
//     let mut am1 = ElevatorContainer::default();
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


// 📌 Tar inn ein vektor med `ElevatorContainer`-ar og skriv ut ein tabell
pub fn print_wv(worldview: Vec<u8>) {
    let wv_deser = deserialize_worldview(&worldview);
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);

    // 📌 Legg til hovudrad (header)
    table.add_row(Row::new(vec![
        Cell::new("ID"),
        Cell::new("Dør"),
        Cell::new("Obstruksjon"),
        Cell::new("Motor Retning"),
        Cell::new("Siste etasje"),
        Cell::new("Tasks (ToDo:Status)"),
        Cell::new("Calls (Etg:Call)"),
    ]));

    for elev in wv_deser.elevator_containers {
        let task_list = elev.tasks.iter()
            .map(|t| format!("{}:{}", t.to_do, t.status))
            .collect::<Vec<String>>()
            .join(", ");

        let call_list = elev.calls.iter()
            .map(|c| format!("{}:{}", c.floor, c.call))
            .collect::<Vec<String>>()
            .join(", ");

        table.add_row(Row::new(vec![
            Cell::new(&format!("{}", elev.elevator_id)),
            Cell::new(if elev.door_open { "Åpen" } else { "Lukket" }),
            Cell::new(if elev.obstruction { "Ja" } else { "Nei" }),
            Cell::new(&format!("{}", elev.motor_dir)),
            Cell::new(&format!("{}", elev.last_floor_sensor)),
            Cell::new(&task_list),
            Cell::new(&call_list),
        ]));
    }

    // 📌 Skriv ut tabellen
    table.printstd();
}
