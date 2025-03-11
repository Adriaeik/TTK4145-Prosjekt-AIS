use crate::config;
use crate::order::HallOrders;
use serde;

pub type Floor = u8;
pub type ElevatorId = String;
pub type NetworkNodeId = u128;
pub type MessageId = u128;

#[rustfmt::skip] // Prevent rustfmt from reordering the enum variants
#[derive(PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize, Debug)]
pub enum Direction {
    Up   =  1,
    Down = -1,
    Stop =  0,
}

#[derive(Debug)]
pub struct Orders(Vec<[bool; config::NUM_BUTTONS as usize]>); // Struct in order to have default new() function
impl Orders {
    pub fn new(config: &config::Config) -> Self {
        Orders(vec![
            [false; config::NUM_BUTTONS as usize];
            config.number_of_floors as usize
        ])
    }

    // Convert from HallOrders to Orders
    pub fn from_hall_orders(hall_orders: &HallOrders, config: &config::Config) -> Self {
        let mut orders = Self::new(config);

        // Process up orders
        for order in &hall_orders.up {
            if order.floor < config.number_of_floors {
                orders[order.floor as usize][0] = true; // 0 index for UP
            }
        }

        // Process down orders
        for order in &hall_orders.down {
            if order.floor < config.number_of_floors {
                orders[order.floor as usize][1] = true; // 1 index for DOWN
            }
        }

        orders
    }
}

impl std::ops::Index<usize> for Orders {
    type Output = [bool; config::NUM_BUTTONS as usize];

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Orders {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
