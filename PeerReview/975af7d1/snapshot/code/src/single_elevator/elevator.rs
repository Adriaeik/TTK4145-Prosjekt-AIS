use crossbeam_channel as cbc;

use crate::config::{self, NUM_BUTTONS};
use crate::single_elevator::timer;
use crate::types::{Direction, Orders};

// Behaviour
#[derive(PartialEq, Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Behaviour {
    Idle,
    DoorOpen,
    Moving,
}
impl Behaviour {
    // Not yet used
    pub fn to_string(&self) -> String {
        match self {
            Behaviour::Idle => "idle".to_string(),
            Behaviour::Moving => "moving".to_string(),
            Behaviour::DoorOpen => "doorOpen".to_string(),
        }
    }
}

impl Direction {
    pub fn to_string(&self) -> String {
        match self {
            Direction::Up => "up".to_string(),
            Direction::Down => "down".to_string(),
            Direction::Stop => "stop".to_string(),
        }
    }
}

// Button
#[derive(PartialEq, Copy, Clone)]
pub enum Button {
    HallUp,
    HallDown,
    Cab,
}
impl Button {
    pub fn to_string(&self) -> String {
        match self {
            Button::HallUp => "HallUp".to_string(),
            Button::HallDown => "HallDown".to_string(),
            Button::Cab => "Cab".to_string(),
        }
    }
}
impl From<u8> for Button {
    fn from(value: u8) -> Self {
        match value {
            0 => Button::HallUp,
            1 => Button::HallDown,
            2 => Button::Cab,
            _ => panic!("Invalid button value"),
        }
    }
}

// ClearRequestVariant
pub enum ClearRequestVariant {
    All,
    InDir,
}

// State
pub struct State {
    floor: Option<u8>,
    previous_floor: Option<u8>,
    requests: Orders,

    pub direction: Direction,

    // pub door_timer: timer::Timer,
    pub obstruction: bool,
    pub timer_tx: cbc::Sender<timer::TimerMessage>,
    pub behaviour: Behaviour,

    pub config: config::Config,
}
impl State {
    pub fn print(&self) {
        let floor = match self.floor {
            Some(f) => f.to_string(),
            None => "Undefined".to_string(),
        };

        println!("  +--------------------+");
        println!("  |floor = {:<2}          |", floor);
        println!(
            "  |dirn  = {:<12.12}|",
            match self.direction {
                Direction::Down => "Down",
                Direction::Stop => "Stop",
                Direction::Up => "Up",
            }
        );
        println!(
            "  |behav = {:<12.12}|",
            match self.behaviour {
                Behaviour::Idle => "Idle",
                Behaviour::DoorOpen => "DoorOpen",
                Behaviour::Moving => "Moving",
            }
        );
        println!(
            "  |obstr = {:<12.12}|",
            match self.obstruction {
                true => "yes",
                false => "no",
            }
        );
        println!("  +--------------------+");
        println!("  |  | up  | dn  | cab |");
        for f in (0..self.config.number_of_floors).rev() {
            print!("  | {}", f);
            for btn in 0..NUM_BUTTONS {
                if (f == self.config.number_of_floors - 1 && btn == Button::HallUp as u8)
                    || (f == 0 && btn == Button::HallDown as u8)
                {
                    print!("|     ");
                } else {
                    print!(
                        "|  {}  ",
                        if self.get_request(f, btn.into()) {
                            "#"
                        } else {
                            "-"
                        }
                    );
                }
            }
            println!("|");
        }
        println!("  +--------------------+");
    }

    pub fn new(config: config::Config, timer_tx: cbc::Sender<timer::TimerMessage>) -> Self {
        Self {
            floor: None,
            previous_floor: None,
            direction: Direction::Stop,
            obstruction: false,
            timer_tx,
            requests: Orders::new(&config),
            behaviour: Behaviour::Idle,
            config,
        }
    }

    pub fn get_request(&self, floor: u8, button: Button) -> bool {
        assert!(
            floor < self.config.number_of_floors,
            "Floor out of bounds in get_request",
        );
        self.requests[floor as usize][button as usize]
    }

    pub fn set_request(&mut self, floor: u8, button: Button, value: bool) {
        assert!(
            floor < self.config.number_of_floors,
            "Floor out of bounds in set_request",
        );
        self.requests[floor as usize][button as usize] = value;
    }

    pub fn set_all_requests(&mut self, requests: Orders) {
        self.requests = requests;
    }

    pub fn get_floor(&self) -> Option<u8> {
        self.floor
    }
    // pub fn get_previous_floor(&self) -> Option<u8> {
    //     self.previous_floor
    // }
    pub fn set_floor(&mut self, new_floor: u8) {
        self.previous_floor = self.floor;
        self.floor = Some(new_floor);
    }

    pub fn start_door_timer(&self) {
        self.timer_tx
            .send(timer::TimerMessage::Start(
                self.config.door_open_duration_seconds,
            ))
            .unwrap();
    }
    // pub fn stop_door_timer(&self) {
    //     self.timer_tx.send(timer::TimerMessage::Stop).unwrap();
    // }
}
