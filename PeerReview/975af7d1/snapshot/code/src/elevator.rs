use crate::order::Order;
use crate::single_elevator::elevator as single_elevator;
use crate::types;

#[derive(Debug, Clone, PartialEq)]
pub struct Elevator {
    pub network_node_name: String,
    // pub network_node_id: types::NetworkNodeId, // If the names collide too often, replace with network_node_id
    pub current_floor: Option<u8>,
    pub behaviour: single_elevator::Behaviour,
    pub direction: Option<types::Direction>,
    pub cab_orders: Vec<Order>,
}

impl Elevator {
    pub fn new(network_node_name: String) -> Self {
        Self {
            network_node_name: network_node_name,
            current_floor: None,
            behaviour: single_elevator::Behaviour::Idle,
            direction: None,
            cab_orders: Vec::new(),
        }
    }
}
