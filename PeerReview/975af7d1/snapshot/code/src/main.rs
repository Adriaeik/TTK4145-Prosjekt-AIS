use clap::Parser;
use crossbeam_channel as cbc;
use driver_rust::elevio::elev::Elevator;
use env_logger;
use log::{error, LevelFilter};
use petname::Generator;
use std::thread::spawn;

// Local modules
pub mod config;
pub mod distribute_orders;
pub mod elevator;
pub mod message;
pub mod networking;
pub mod order;
pub mod types;
pub mod single_elevator {
    pub mod elevator;
    pub mod fsm;
    // pub mod main;
    pub mod elevator_controller;
    pub mod requests;
    pub mod timer;
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, short, default_value_t = 15657)]
    server_port: u16,

    #[arg(long, short, default_value_t = 19738)]
    peer_port: u16,

    #[arg(long, short, default_value_t = 19735)]
    message_port: u16,
}

fn main() {
    // Sleep for 1 second to allow the server to start
    // std::thread::sleep(std::time::Duration::from_secs(1));

    std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::new()
        .filter_level(LevelFilter::Trace)
        .init();

    let cli_args = Args::parse();

    let config = config::Config::load().expect("Failed to read config file");
    let elevio_driver = match Elevator::init(
        format!("localhost:{}", cli_args.server_port).as_str(),
        config.number_of_floors,
    ) {
        Ok(driver) => driver,
        Err(e) => {
            error!(
                "Error initializing elevio driver: {}. Did you remember to start the server first?",
                e
            );
            return;
        }
    };

    let alliterations_generator = petname::Alliterations::default();
    let unique_name = alliterations_generator
        .generate_one(3, "-")
        .expect("Failed to generate unique name with alliterations");

    // Initialize network
    let (command_channel_tx, command_channel_rx) = cbc::unbounded::<types::Orders>();
    let network = networking::Network::new(
        config.clone(),
        cli_args.peer_port,
        cli_args.message_port,
        unique_name.clone(),
        command_channel_tx,
    );

    // Start controller for single elevator
    let network_name = network.network_node_name.clone();
    let network_send_tx = network.data_send_tx.clone();
    spawn(move || {
        single_elevator::elevator_controller::run_controller(
            config.clone(),
            elevio_driver,
            network_name.clone(),
            network_send_tx.clone(),
            command_channel_rx,
        )
    });

    network.start_listening();
}
