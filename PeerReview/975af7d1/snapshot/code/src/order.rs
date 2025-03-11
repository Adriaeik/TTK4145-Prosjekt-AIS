use crate::types::Direction;

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub floor: u8,
}

#[derive(Debug, Clone)]
pub struct HallOrders {
    pub up: Vec<Order>,
    pub down: Vec<Order>,
}

impl HallOrders {
    pub fn new() -> HallOrders {
        HallOrders {
            up: Vec::new(),
            down: Vec::new(),
        }
    }

    pub fn add_order(&mut self, dir: Direction, floor: u8) {
        let order = Order { floor };
        match dir {
            Direction::Up => self.up.push(order),
            Direction::Down => self.down.push(order),
            _ => (),
        }
    }
}
