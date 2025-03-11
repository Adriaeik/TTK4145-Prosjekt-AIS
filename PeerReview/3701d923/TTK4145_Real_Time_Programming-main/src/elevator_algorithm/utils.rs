use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone)]
pub struct ElevState {
    pub behaviour: String,
    pub floor: u8,
    pub direction: String,
    pub cabRequests: Vec<bool>,
}

impl ElevState {
    pub fn from_elevmsg(msg: ElevMsg) -> (Vec<Vec<bool>>, ElevState) {
        (
            msg.hallRequests,
            ElevState { behaviour: msg.behaviour, floor: msg.floor, direction: msg.direction, cabRequests: msg.cabRequests },
        )
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone)]
pub struct AlgoInput {
    pub hallRequests: Vec<Vec<bool>>,
    pub states: HashMap<String, ElevState>,
}

impl AlgoInput {
    pub fn new() -> Self {
        AlgoInput { hallRequests: vec![], states: HashMap::new() }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct ElevMsg {
    pub hallRequests: Vec<Vec<bool>>,
    pub behaviour: String,
    pub floor: u8,
    pub direction: String,
    pub cabRequests: Vec<bool>,
}
