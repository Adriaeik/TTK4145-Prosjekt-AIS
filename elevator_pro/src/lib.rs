#![warn(missing_docs)]
//! # This projects library
//!
//! This library manages configuration, network-communication between nodes, synchronization of world view across nodes and internally, elevator logic
//!
//! ## Overview
//! - **Config**: Handles configuration settings.
//! - **Utils**: Various helper functions.
//! - **Init**: System initialization.
//! - **Network**: Communication via UDP and TCP.
//! - **World View**: Managing and updating the world view.
//! - **Elevio**: Interface for elevator I/O.
//! - **Elevator Logic**: Task management and control logic for elevators.
//! 
//! 
pub mod config;
pub mod utils;
pub mod init;

/// Network communication via UDP and TCP.
pub mod network {
    /// Sends and receives messages using UDP broadcast.
    pub mod udp_broadcast;
    /// Handles discovery and management of the local network.
    pub mod local_network;
    /// TCP communication with other nodes.
    pub mod tcp_network;
    /// TCP communication for the local elevator.
    pub mod tcp_self_elevator;
}

/// Management of the system's world view.
pub mod world_view {
    /// Handles communication between elevators regarding the world view.
    pub mod world_view_ch;
    /// Updates the world view.
    pub mod world_view_update;
    /// Manages the world view.
    pub mod world_view;
}

/// Interface for elevator input/output.
pub mod elevio {
    /// Controls the elevator.
    pub mod elev;
    /// Listens for events from the elevator.
    pub mod poll;
}

/// Elevator control logic and task handling.
pub mod elevator_logic {
    /// Handles elevator task management.
    pub mod task_handler;
    /// Logic for the master elevator.
    pub mod master {
        /// Handles world view data from slave elevators.
        pub mod wv_from_slaves;
        /// Allocates tasks to elevators.
        pub mod task_allocater;
    }
}