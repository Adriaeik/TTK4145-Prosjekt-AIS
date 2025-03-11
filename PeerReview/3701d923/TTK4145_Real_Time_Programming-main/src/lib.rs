pub mod elevator_io {
    pub mod data;
    pub mod driver;
    pub mod driver_sync;
}

pub mod elevator_logic {
    pub mod state_machine;
    pub mod utils;
}

pub mod distributed_systems {
    pub mod utils;
}

pub mod elevator_algorithm {
    pub mod cost_algorithm;
    pub mod utils;
}
