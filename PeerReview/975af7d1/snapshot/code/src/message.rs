use serde;

use crate::single_elevator::elevator::Behaviour;
use crate::types;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct HallOrderMessage {
    pub floor: types::Floor,
    pub direction: types::Direction,
}

/// Send a message that an elevator has received a cab order
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CabOrderMessage {
    // pub elevator_id: types::ElevatorId,
    pub floor: types::Floor,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ElevatorEventMessage {
    // pub elevator_id: types::ElevatorId,
    pub behaviour: Behaviour,
    pub floor: u8,
    pub direction: types::Direction,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DataMessage {
    // join all other structs
    pub sender_node_name: String,
    pub message_id: types::MessageId,
    pub hall_order_message: Option<HallOrderMessage>,
    pub cab_order_message: Option<CabOrderMessage>,
    pub elevator_event_message: Option<ElevatorEventMessage>,
}

pub enum MessageType {
    HallOrder(HallOrderMessage),
    CabOrder(CabOrderMessage),
    ElevatorEventMessage(ElevatorEventMessage),
    Unknown,
}

pub trait Message {
    fn to_data_message(self, sender_node_name: &String) -> DataMessage;
}

macro_rules! impl_message {
    ($msg_type:ty, $field:ident) => {
        impl Message for $msg_type {
            fn to_data_message(self, sender_node_name: &String) -> DataMessage {
                let mut data_message = DataMessage {
                    message_id: uuid::Uuid::new_v4().as_u128(),
                    sender_node_name: sender_node_name.to_string(),
                    hall_order_message: None,
                    cab_order_message: None,
                    elevator_event_message: None,
                };
                data_message.$field = Some(self);
                data_message
            }
        }
    };
}

impl_message!(HallOrderMessage, hall_order_message);
impl_message!(CabOrderMessage, cab_order_message);
impl_message!(ElevatorEventMessage, elevator_event_message);
