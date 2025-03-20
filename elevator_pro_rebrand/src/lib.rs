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

/// Global variables
pub mod config;

/// Help functions
pub mod ip_help_functions;

/// Initialize functions
pub mod init;

/// Print functions with color coding
pub mod print;

/// Responsible for calculating cost and distribute tasks
pub mod manager;

/// Network communication via UDP and TCP.
pub mod network {
    /// Sends and receives messages using UDP broadcast.
    pub mod udp_broadcast;
    /// Handles discovery and management of the local network.
    pub mod local_network;
    /// TCP communication with other nodes.
    pub mod tcp_network;
}

/// Management of the system's world view.
pub mod world_view;

/// Interface for elevator input/output. Only changes are documented here. For source code see: [https://github.com/TTK4145/driver-rust/tree/master/src/elevio]
pub mod elevio;

/// Elevator control logic and task handling.
pub mod elevator_logic;

/// Responsible for creating and running the backup-instnce
pub mod backup;

