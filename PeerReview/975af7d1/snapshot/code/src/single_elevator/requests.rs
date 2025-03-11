// Crates
use crate::config::ClearRequestVariant;
use crate::single_elevator::elevator;
use crate::types::Direction;
use crate::config::NUM_BUTTONS;

type DirectionBehaviourPair = (Direction, elevator::Behaviour);

pub fn above(e: &elevator::State) -> bool {
    let floor = match e.get_floor() {
        Some(f) => f,
        None => return false,
    };
    (floor + 1..e.config.number_of_floors)
        .any(|f| (0..NUM_BUTTONS)
        .any(|c| e.get_request(f, c.into())))
}

pub fn below(e: &elevator::State) -> bool {
    let floor = match e.get_floor() {
        Some(f) => f,
        None => return false,
    };
    (0..floor)
        .any(|f| (0..NUM_BUTTONS)
        .any(|c| e.get_request(f, c.into())))
}

pub fn here(e: &elevator::State) -> bool {
    let floor = match e.get_floor() {
        Some(f) => f,
        None => return false,
    };
    (0..NUM_BUTTONS)
        .any(|c| e.get_request(floor, c.into()))
}

pub fn choose_direction(e: &elevator::State) -> DirectionBehaviourPair {
    match e.direction {
        Direction::Up => {
            if above(e) {
                (Direction::Up, elevator::Behaviour::Moving)
            } else if here(e) {
                (Direction::Down, elevator::Behaviour::DoorOpen)
            } else if below(e) {
                (Direction::Down, elevator::Behaviour::Moving)
            } else {
                (Direction::Stop, elevator::Behaviour::Idle)
            }
        }
        Direction::Down => {
            if below(e) {
                (Direction::Down, elevator::Behaviour::Moving)
            } else if here(e) {
                (Direction::Up, elevator::Behaviour::DoorOpen)
            } else if above(e) {
                (Direction::Up, elevator::Behaviour::Moving)
            } else {
                (Direction::Stop, elevator::Behaviour::Idle)
            }
        }
        Direction::Stop => {
            if here(e) {
                (Direction::Stop, elevator::Behaviour::DoorOpen)
            } else if above(e) {
                (Direction::Up, elevator::Behaviour::Moving)
            } else if below(e) {
                (Direction::Down, elevator::Behaviour::Moving)
            } else {
                (Direction::Stop, elevator::Behaviour::Idle)
            }
        }
    }
}

pub fn should_stop(e: &elevator::State) -> bool {
    let floor = match e.get_floor() {
        Some(f) => f,
        None => return false,
    };

    match e.direction {
        Direction::Down => {
            (e.get_request(floor, elevator::Button::HallDown))
                || (e.get_request(floor, elevator::Button::Cab))
                || !below(e)
        }
        Direction::Up => {
            (e.get_request(floor, elevator::Button::HallUp))
                || (e.get_request(floor, elevator::Button::Cab))
                || !above(e)
        }
        Direction::Stop => true,
    }
}

pub fn should_clear_immediately(
    e: &elevator::State,
    btn_floor: u8,
    btn_type: elevator::Button,
) -> bool {
    let floor = match e.get_floor() {
        Some(f) => f,
        None => return false,
    };

    match e.config.clear_request_variant {
        ClearRequestVariant::All => floor == btn_floor,
        ClearRequestVariant::InDir => {
            floor == btn_floor
                && (e.direction == Direction::Stop
                    || btn_type == elevator::Button::Cab
                    || (e.direction == Direction::Up
                        && btn_type == elevator::Button::HallUp)
                    || (e.direction == Direction::Down
                        && btn_type == elevator::Button::HallDown))
        }
    }
}

pub fn clear_at_current_floor(e: &mut elevator::State) -> &mut elevator::State {
    let floor = match e.get_floor() {
        Some(f) => f,
        None => return e,
    };

    match e.config.clear_request_variant {
        ClearRequestVariant::All => {
            (0..NUM_BUTTONS).for_each(|c| e.set_request(floor, c.into(), false));
        }
        ClearRequestVariant::InDir => {
            e.set_request(floor, elevator::Button::Cab, false);
            match e.direction {
                Direction::Up => {
                    if !above(&e) && !e.get_request(floor, elevator::Button::HallUp) {
                        e.set_request(floor, elevator::Button::HallDown, false);
                    }
                    e.set_request(floor, elevator::Button::HallUp, false);
                }
                Direction::Down => {
                    if !below(&e) && !e.get_request(floor, elevator::Button::HallDown) {
                        e.set_request(floor, elevator::Button::HallUp, false);
                    }
                    e.set_request(floor, elevator::Button::HallDown, false);
                }
                Direction::Stop => {
                    e.set_request(floor, elevator::Button::HallUp, false);
                    e.set_request(floor, elevator::Button::HallDown, false);
                }
            }
        }
    }
    return e;
}
