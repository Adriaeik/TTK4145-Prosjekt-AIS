use std::collections::HashSet;

use super::utils::State;

pub fn handle_cab_request_current_floor(cab_queue: &HashSet<u8>, current_floor: u8) -> Option<State> {
    if cab_queue.contains(&current_floor) {
        return Some(State::Door); // Found a valid cab call
    } else {
        return None; // No cab call found
    }
}

pub fn handle_hall_up_request_current_floor(hall_up_queue: &HashSet<u8>, current_floor: u8) -> Option<State> {
    if hall_up_queue.contains(&current_floor) {
        return Some(State::Door); // Found a valid hall call
    } else {
        return None; // No hall call found
    }
}

pub fn handle_hall_down_request_current_floor(hall_down_queue: &HashSet<u8>, current_floor: u8) -> Option<State> {
    if hall_down_queue.contains(&current_floor) {
        return Some(State::Door); // Found a valid hall call
    } else {
        return None; // No hall call found
    }
}

pub fn handle_cab_calls_while_moving(cab_queue: &HashSet<u8>, current_floor: u8, visited_floor: u8) -> Option<State> {
    if cab_queue.contains(&current_floor) && visited_floor != current_floor {
        return Some(State::Door); // Found a valid cab call
    } else {
        return None; // No cab call found
    }
}

pub fn handle_up_requests(up_queue: &HashSet<u8>, current_floor: u8, visited_floor: u8) -> Option<State> {
    if up_queue.contains(&current_floor) && visited_floor != current_floor {
        return Some(State::Door); // Found a valid UP request
    } else {
        return None; // No UP request found
    }
}

pub fn handle_down_requests(down_queue: &HashSet<u8>, current_floor: u8, visited_floor: u8) -> Option<State> {
    if down_queue.contains(&current_floor) && visited_floor != current_floor {
        return Some(State::Door); // Found a valid DOWN request
    } else {
        return None; // No DOWN request found
    }
}

pub fn find_request_above(queue: &HashSet<u8>, current_floor: u8) -> Option<State> {
    if queue.iter().any(|&floor| floor > current_floor) {
        return Some(State::Up); // Found a request above
    } else {
        return None; // No requests above
    }
}

pub fn find_request_below(queue: &HashSet<u8>, current_floor: u8) -> Option<State> {
    if queue.iter().any(|&floor| floor < current_floor) {
        return Some(State::Down); // Found a request below
    } else {
        return None; // No requests below
    }
}

pub fn handle_emergency_stop(stop_button: bool) -> Option<State> {
    if stop_button {
        return Some(State::EmergencyStop);
    } else {
        return None;
    }
}

pub fn find_random_request(cab_queue: &HashSet<u8>, up_queue: &HashSet<u8>, down_queue: &HashSet<u8>, current_floor: u8) -> Option<State> {
    if let Some(&floor) = cab_queue.iter().next() {
        if floor > current_floor {
            return Some(State::Up);
        } else if floor < current_floor {
            return Some(State::Down);
        }
    }

    if let Some(&floor) = up_queue.iter().next() {
        if floor > current_floor {
            return Some(State::Up);
        } else if floor == current_floor {
            return Some(State::Door);
        } else if floor < current_floor {
            return Some(State::Down);
        }
    }

    if let Some(&floor) = down_queue.iter().next() {
        if floor > current_floor {
            return Some(State::Up);
        } else if floor == current_floor {
            return Some(State::Door);
        } else if floor < current_floor {
            return Some(State::Down);
        }
    }

    return None;
}
