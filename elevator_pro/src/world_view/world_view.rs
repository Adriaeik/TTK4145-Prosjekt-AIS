use serde::{Serialize, Deserialize};
use crate::config;
use crate::utils;
use crate::elevio::poll::CallType;
use ansi_term::Colour::{Blue, Green, Red, Yellow, Purple};
use ansi_term::Style;
use prettytable::{Table, Row, Cell, format, Attr, color};
use crate::elevio::poll::CallButton;



#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Task {
    pub id: u16,
    pub to_do: u8, // Default: 0
    pub status: TaskStatus, // 1: done, 0: to_do, 255: be master deligere denne på nytt
    pub is_inside: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskStatus {
    PENDING,
    DONE,
    UNABLE = u8::MAX as isize,    
}
impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::PENDING
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ElevatorContainer {
    pub elevator_id: u8,            // Default: 0
    pub calls: Vec<CallButton>,     // Default: vektor med Tasks
    pub tasks: Vec<Task>,           // Default: vektor med Tasks
    pub tasks_status: Vec<Task>,    // Default: vektor med Tasks Slave skriver, Master leser
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
            tasks_status: Vec::new(),
            door_open: false,
            obstruction: false,
            motor_dir: 0,
            last_floor_sensor: 255, // Spesifikk verdi for sensor
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
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
        utils::print_ok(format!("elevator med ID {} ble ansatt. (add_elev())", elevator.elevator_id));
        self.elevator_containers.push(elevator);
        self.n = self.elevator_containers.len() as u8;
    }
    
    pub fn remove_elev(&mut self, id: u8) {
        let initial_len = self.elevator_containers.len();

        self.elevator_containers.retain(|elevator| elevator.elevator_id != id);
    
        if self.elevator_containers.len() == initial_len {
            utils::print_warn(format!("Ingen elevator med ID {} ble funnet. (remove_elev())", id));
        } else {
            utils::print_ok(format!("elevator med ID {} ble sparka. (remove_elev())", id));
        }
        self.n = self.elevator_containers.len() as u8;
    }

    pub fn get_num_elev(&self) -> u8 {
        return self.n;
    }

    pub fn set_num_elev(&mut self, n: u8)  {
        self.n = n;
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

pub fn get_index_to_container(id: u8, wv: Vec<u8>) -> Option<usize> {
    let wv_deser = deserialize_worldview(&wv);
    for i in 0..wv_deser.get_num_elev() {
        if wv_deser.elevator_containers[i as usize].elevator_id == id {
            return Some(i as usize);
        }
    }
    return None;
}



pub fn print_wv(worldview: Vec<u8>) {
    let wv_deser = deserialize_worldview(&worldview);
    let mut gen_table = Table::new();
    gen_table.set_format(*format::consts::FORMAT_CLEAN);
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);

    // Overskrift i blå feittskrift
    println!("{}", Purple.bold().paint("WORLD VIEW STATUS"));

    //Legg til generell worldview-info
    //Funka ikke når jeg brukte fargene på lik måte som under. gudene vet hvorfor
    gen_table.add_row(Row::new(vec![
        Cell::new("Num heiser").with_style(Attr::ForegroundColor(color::BRIGHT_BLUE)),
        Cell::new("MasterID").with_style(Attr::ForegroundColor(color::BRIGHT_BLUE)),
        Cell::new("Outside Buttons").with_style(Attr::ForegroundColor(color::BRIGHT_BLUE)),
    ]));

    let n_text = format!("{}", wv_deser.get_num_elev()); // Fjern ANSI og bruk prettytable farge
    let m_id_text = format!("{}", wv_deser.master_id);
    let button_list = wv_deser.outside_button.iter()
    .map(|c| match c.call {
        CallType::INSIDE => format!("{}:{:?}({})", c.floor, c.call, c.elev_id),
        _ => format!("{}:{:?}:PUBLIC", c.floor, c.call),
    })
    .collect::<Vec<String>>()
    .join(", ");

    gen_table.add_row(Row::new(vec![
        Cell::new(&n_text).with_style(Attr::ForegroundColor(color::BRIGHT_YELLOW)),
        Cell::new(&m_id_text).with_style(Attr::ForegroundColor(color::BRIGHT_YELLOW)),
        Cell::new(&button_list),
    ]));

    gen_table.printstd();



    // Legg til heis-spesifikke deler
    // Legg til hovudrad (header) med blå feittskrift
    table.add_row(Row::new(vec![
        Cell::new(&Blue.bold().paint("ID").to_string()),
        Cell::new(&Blue.bold().paint("Dør").to_string()),
        Cell::new(&Blue.bold().paint("Obstruksjon").to_string()),
        Cell::new(&Blue.bold().paint("Motor Retning").to_string()),
        Cell::new(&Blue.bold().paint("Siste etasje").to_string()),
        Cell::new(&Blue.bold().paint("Tasks (ToDo:Status)").to_string()),
        Cell::new(&Blue.bold().paint("Calls (Etg:Call)").to_string()),
    ]));

    // Iterer over alle heisane
    for elev in wv_deser.elevator_containers {
        // Lag ein fargerik streng for ID
        let id_text = Yellow.bold().paint(format!("{}", elev.elevator_id)).to_string();

        // Door og obstruction i grøn/raud
        let door_status = if elev.door_open {
            Yellow.paint("Åpen").to_string()
        } else {
            Green.paint("Lukket").to_string()
        };

        let obstruction_status = if elev.obstruction {
            Red.paint("Ja").to_string()
        } else {
            Green.paint("Nei").to_string()
        };

        // Farge basert på `to_do`
        let task_list = elev.tasks.iter()
            .map(|t| {
                let color = match t.to_do {
                    0..=3 => Green,
                    4..=10 => Yellow,
                    _ => Red,
                };
                format!("{}:{}",
                color.paint(t.id.to_string()),
                    color.paint(t.to_do.to_string())
                )
            })
            .collect::<Vec<String>>()
            .join(", ");

        // Vanleg utskrift av calls
        let call_list = elev.calls.iter()
            .map(|c| format!("{}:{:?}", c.floor, c.call))
            .collect::<Vec<String>>()
            .join(", ");

        table.add_row(Row::new(vec![
            Cell::new(&id_text),
            Cell::new(&door_status),
            Cell::new(&obstruction_status),
            Cell::new(&format!("{}", elev.motor_dir)),
            Cell::new(&format!("{}", elev.last_floor_sensor)),
            Cell::new(&task_list),
            Cell::new(&call_list),
        ]));
    }

    // Skriv ut tabellen med fargar (ANSI-kodar)
    table.printstd();
    print!("\n\n");
}