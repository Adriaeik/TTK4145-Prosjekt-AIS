pub mod config;
pub mod utils;
pub mod init;

pub mod network{
    pub mod udp_broadcast;
    pub mod local_network;
    pub mod tcp_network;
    pub mod tcp_self_elevator;
}

pub mod world_view{
    pub mod world_view_ch;
    pub mod world_view_update;
    pub mod world_view;
}

pub mod elevio {
    pub mod elev;
    pub mod poll;
}

pub mod elevator_logic {
    pub mod task_handler;
    pub mod master {
        pub mod wv_from_slaves;
        pub mod task_allocater;
    }
}
