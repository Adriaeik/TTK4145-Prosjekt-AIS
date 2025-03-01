use crossbeam_channel as cbc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time;
use serde::{Serialize, Deserialize};
use std::hash::{Hash, Hasher};

use crate::utils;

use super::elev::{self, DIRN_STOP, DIRN_DOWN, DIRN_UP};

//Lager enum for call
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallType {
    UP = 0,
    DOWN,
    INSIDE,
    COSMIC_ERROR,
}
impl From<u8> for CallType {
    fn from(value: u8) -> Self {
        match value {
            0 => CallType::UP,
            1 => CallType::DOWN,
            2 => CallType::INSIDE,
            _ => {
                utils::print_cosmic_err();
                CallType::COSMIC_ERROR
            },
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq)] // Added support for (De)serialization and cloning
pub struct CallButton {
    pub floor: u8,
    pub call: CallType,
    pub elev_id: u8,
}

impl PartialEq for CallButton {
    fn eq(&self, other: &Self) -> bool {
        // Hvis call er INSIDE, sammenligner vi også elev_id
        if self.call == CallType::INSIDE {
            utils::print_warn(format!("{:?} og {:?} er like? {}", self, other, self.floor == other.floor && self.call == other.call && self.elev_id == other.elev_id));
            self.floor == other.floor && self.call == other.call && self.elev_id == other.elev_id
        } else {
            // For andre CallType er det tilstrekkelig å sammenligne floor og call
            utils::print_warn(format!("{:?} og {:?} er like? {}", self, other, self.floor == other.floor && self.call == other.call));
            self.floor == other.floor && self.call == other.call
        }
    }
}
impl Hash for CallButton {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Sørger for at hash er konsistent med eq
        self.floor.hash(state);
        self.call.hash(state);
        if self.call == CallType::INSIDE {
            self.elev_id.hash(state);
        }
    }
}

pub fn call_buttons(elev: elev::Elevator, ch: cbc::Sender<CallButton>, period: time::Duration) {
    let mut prev = vec![[false; 3]; elev.num_floors.into()];
    loop {
        for f in 0..elev.num_floors {
            for c in 0..3 {
                let v = elev.call_button(f, c);
                if v && prev[f as usize][c as usize] != v {
                    ch.send(CallButton { floor: f, call: CallType::from(c), elev_id: utils::SELF_ID.load(Ordering::SeqCst)}).unwrap();
                }
                prev[f as usize][c as usize] = v;
            }
        }
        thread::sleep(period)
    }
}

pub fn floor_sensor(elev: elev::Elevator, ch: cbc::Sender<u8>, period: time::Duration) {
    let mut prev = u8::MAX;
    loop {
        if let Some(f) = elev.floor_sensor() {
            if f != prev {
                ch.send(f).unwrap();
                prev = f;
            }
        }
        thread::sleep(period)
    }
}

pub fn stop_button(elev: elev::Elevator, ch: cbc::Sender<bool>, period: time::Duration) {
    let mut prev = false;
    loop {
        let v = elev.stop_button();
        if prev != v {
            ch.send(v).unwrap();
            prev = v;
        }
        thread::sleep(period)
    }
}

pub fn obstruction(elev: elev::Elevator, ch: cbc::Sender<bool>, period: time::Duration) {
    let mut prev = false;
    loop {
        let v = elev.obstruction();
        if prev != v {
            ch.send(v).unwrap();
            prev = v;
        }
        thread::sleep(period)
    }
}