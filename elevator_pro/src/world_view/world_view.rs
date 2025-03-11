use std::fmt::format;

use serde::{Serialize, Deserialize};
use crate::config;
use crate::utils;
use crate::elevio::poll::CallType;
use ansi_term::Colour::{Blue, Green, Red, Yellow, Purple};
use prettytable::{Table, Row, Cell, format, Attr, color};
use crate::elevio::poll::CallButton;
use crate::manager::task_allocator::Task;


/// Represents the status of a task within the system.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash)]
pub enum TaskStatus {
    /// The task is waiting to be assigned or executed.
    PENDING,

    /// The task has been successfully completed.
    DONE,

    /// The task has started execution, preventing reassignment by the master.
    STARTED,

    /// The task could not be completed.
    UNABLE = u8::MAX as isize,    
}
impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::PENDING
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum ElevatorStatus {
    Up,
    Down,
    Idle,
    DoorOpen,
    Error,
}
/// Represents the state of an elevator, including tasks, status indicators, and movement.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ElevatorContainer {
    /// Unique identifier for the elevator.
    pub elevator_id: u8, // Default: ERROR_ID

    pub num_floors: u8,

    /// List of external call requests.
    pub calls: Vec<CallButton>, // Default: empty vector

    /// List of assigned tasks for the elevator.
    pub task: Option<Task>, // Default: empty vector

    pub status: ElevatorStatus,

    /// Indicates whether the elevator detects an obstruction.
    pub obstruction: bool, // Default: false

    /// The last detected floor sensor position.
    pub last_floor_sensor: u8, // Default: 255 (undefined)
}

impl Default for ElevatorContainer {
    fn default() -> Self {
        Self {
            elevator_id: config::ERROR_ID,
            num_floors: config::DEFAULT_NUM_FLOORS,
            calls: Vec::new(),
            task: None,
            status: ElevatorStatus::Idle,
            obstruction: false,
            last_floor_sensor: 255, // Spesifikk verdi for sensor
        }
    }
}


/// Represents the system's current state (WorldView).
///
/// `WorldView` contains an overview of all elevators in the system, 
/// the master elevator's ID, and the call buttons pressed outside the elevators.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorldView {
    /// - `n`: Number of elevators in the system.
    n: u8, 
    /// - `master_id`: The ID of the master elevator.
    pub master_id: u8, 
    /// - `outside_button`: A list of call buttons pressed outside elevators.
    pub outside_button: Vec<CallButton>, 
    /// - `elevator_containers`: A list of `ElevatorContainer` structures containing
    ///   individual elevator information.
    pub elevator_containers: Vec<ElevatorContainer>,  
}


impl Default for WorldView {
    /// Creates a default `WorldView` instance with no elevators and an invalid master ID.
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
    /// Adds an elevator to the system.
    ///
    /// Updates the number of elevators (`n`) accordingly.
    ///
    /// ## Parameters
    /// - `elevator`: The `ElevatorContainer` to be added.
    pub fn add_elev(&mut self, elevator: ElevatorContainer) {
        self.elevator_containers.push(elevator);
        self.n = self.elevator_containers.len() as u8;
    }
    
    /// Removes an elevator with the given ID from the system.
    ///
    /// If no elevator with the specified ID is found, a warning is printed.
    ///
    /// ## Parameters
    /// - `id`: The ID of the elevator to remove.
    pub fn remove_elev(&mut self, id: u8) {
        let initial_len = self.elevator_containers.len();

        self.elevator_containers.retain(|elevator| elevator.elevator_id != id);
    
        if self.elevator_containers.len() == initial_len {
            utils::print_warn(format!("No elevator with ID {} was found. (remove_elev())", id));
        } else {
            utils::print_ok(format!("Elevator with ID {} was removed. (remove_elev())", id));
        }
        self.n = self.elevator_containers.len() as u8;
    }

    /// Returns the number of elevators in the system.
    pub fn get_num_elev(&self) -> u8 {
        return self.n;
    }


    /// Sets the number of elevators manually.
    ///
    /// **Note:** This does not affect the `elevator_containers` list. 
    /// Use `add_elev()` or `remove_elev()` to modify the actual elevators.
    ///
    /// ## Parameters
    /// - `n`: The new number of elevators.
    // TODO: Burde være veldig mulig å gjøre denne privat
    pub fn set_num_elev(&mut self, n: u8)  {
        self.n = n;
    }
}



/// Serializes a `WorldView` into a binary format.
///
/// Uses `bincode` for efficient serialization.
/// If serialization fails, the function logs the error and panics.
///
/// ## Parameters
/// - `worldview`: A reference to the `WorldView` to be serialized.
///
/// ## Returns
/// - A `Vec<u8>` containing the serialized data.
pub fn serialize_worldview(worldview: &WorldView) -> Vec<u8> {
    let encoded = bincode::serialize(worldview);
    match encoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return serialized_data;
        }
        Err(e) => {
            println!("{:?}", worldview);
            utils::print_err(format!("Serialization failed: {} (world_view.rs, serialize_worldview())", e));
            panic!();
        }
    }
}

/// Deserializes a `WorldView` from a binary format.
///
/// Uses `bincode` for deserialization.
/// If deserialization fails, the function logs the error and panics.
///
/// ## Parameters
/// - `data`: A byte slice (`&[u8]`) containing the serialized `WorldView`.
///
/// ## Returns
/// - A `WorldView` instance reconstructed from the binary data.
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

/// Serializes an `ElevatorContainer` into a binary format.
///
/// Uses `bincode` for serialization.
/// If serialization fails, the function logs the error and panics.
///
/// ## Parameters
/// - `elev_container`: A reference to the `ElevatorContainer` to be serialized.
///
/// ## Returns
/// - A `Vec<u8>` containing the serialized data.
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

/// Deserializes an `ElevatorContainer` from a binary format.
///
/// Uses `bincode` for deserialization.
/// If deserialization fails, the function logs the error and panics.
///
/// ## Parameters
/// - `data`: A byte slice (`&[u8]`) containing the serialized `ElevatorContainer`.
///
/// ## Returns
/// - An `ElevatorContainer` instance reconstructed from the binary data.
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

/// Retrieves the index of an `ElevatorContainer` with the specified `id` in the deserialized `WorldView`.
///
/// This function deserializes the provided `WorldView` data and iterates through the elevator containers
/// to find the one that matches the given `id`. If found, it returns the index of the container; otherwise, it returns `None`.
///
/// ## Parameters
/// - `id`: The ID of the elevator whose index is to be retrieved.
/// - `wv`: A serialized `WorldView` as a `Vec<u8>`.
///
/// ## Returns
/// - `Some(usize)`: The index of the `ElevatorContainer` in the `WorldView` if found.
/// - `None`: If no elevator with the given `id` exists.
pub fn get_index_to_container(id: u8, wv: Vec<u8>) -> Option<usize> {
    let wv_deser = deserialize_worldview(&wv);
    for i in 0..wv_deser.get_num_elev() {
        if wv_deser.elevator_containers[i as usize].elevator_id == id {
            return Some(i as usize);
        }
    }
    return None;
}


/// Logs `wv` in a nice format 
pub fn print_wv(worldview: Vec<u8>) {
    let print_stat;
    unsafe {
        print_stat = config::PRINT_WV_ON;
    }
    if !print_stat {
        return;
    }

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
        Cell::new(&Blue.bold().paint("Task").to_string()),
        Cell::new(&Blue.bold().paint("Calls (Etg:Call)").to_string()),
        Cell::new(&Blue.bold().paint("Tasks_status (ToDo:Status)").to_string()),
    ]));

    // Iterer over alle heisane
    for elev in wv_deser.elevator_containers {
        // Lag ein fargerik streng for ID
        let id_text = Yellow.bold().paint(format!("{}", elev.elevator_id)).to_string();

        // Door og obstruction i grøn/raud
        let door_status = if elev.status == ElevatorStatus::DoorOpen {
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
        let task_list = if elev.task.is_some() {
            Yellow.paint(format!("{:?}", elev.task)).to_string()
        } else {
            Green.paint("None").to_string()
        };
            

        // Vanleg utskrift av calls
        let call_list = elev.calls.iter()
            .map(|c| format!("{}:{:?}", c.floor, c.call))
            .collect::<Vec<String>>()
            .join(", ");

        let task_stat_list = match elev.status {
            ElevatorStatus::DoorOpen => {
                Yellow.paint(format!("{:?}", elev.status))
            },
            ElevatorStatus::Down => {
                Blue.paint(format!("{:?}", elev.status))
            },
            ElevatorStatus::Error => {
                Red.paint(format!("{:?}", elev.status))
            },
            ElevatorStatus::Idle => {
                Green.paint(format!("{:?}", elev.status))
            },
            ElevatorStatus::Up => {
                Blue.paint(format!("{:?}", elev.status))
            },
        };

        table.add_row(Row::new(vec![
            Cell::new(&id_text),
            Cell::new(&door_status),
            Cell::new(&obstruction_status),
            Cell::new(&format!("{}", elev.last_floor_sensor)),
            Cell::new(&task_list),
            Cell::new(&call_list),
            Cell::new(&task_stat_list),
        ]));
    }

    // Skriv ut tabellen med fargar (ANSI-kodar)
    table.printstd();
    print!("\n\n");
}