#![warn(missing_docs)]
//! # This projects library
//!
//! This library manages configuration, network-communication between nodes, synchronization of world view across nodes and internally, elevator logic
//!
//! ## Overview
//! - **config**: Handles configuration settings.
//! - **ip_help_functions**: Various helper functions for the local IP-address.
//! - **init**: System initialization.
//! - **manager**: Allocates available tasks to the connected nodes
//! - **network**: Communication between nodes via UDP and TCP, and updateing the worldview locally via mpsc-channels and watch-channels.
//! - **world_view**: The local WorldView
//! - **elevio**: Interface for elevator I/O.
//! - **elevator_logic**: Task execution and reading from the local elevator.
//! - **backup**: Creating, monitoring and running a backup, ready to overtake if the main program crashes

pub mod config;

pub mod ip_help_functions;

pub mod init;

pub mod print;

pub mod manager;

pub mod network;

pub mod world_view;

pub mod elevio;

pub mod elevator_logic;

pub mod backup;

