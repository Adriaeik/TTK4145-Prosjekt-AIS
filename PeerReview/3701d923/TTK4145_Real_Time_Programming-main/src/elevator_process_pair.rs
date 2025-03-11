// * NOTE: We don't actually need to run the node in "sudo", however for consistency with other nodes that require "sudo" we have made this node also run in "sudo"
// * NOTE: Every new elevator needs its own unique ELEVATOR_NETWORK_ID and ELEVATOR_HARDWARE_PORT
// *
// * To run this process:
// * $ sudo -E ELEVATOR_NETWORK_ID=<ID> ELEVATOR_HARDWARE_PORT=<PORT> NUMBER_FLOORS=<NUMBER FLOORS> cargo run --bin elevator_process_pair

use std::env;
use std::io::Write;
use std::net::TcpStream;
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::Duration; // For write_all

/// Start the elevator process with the given network ID and hardware port using sudo.
fn start_elevator(network_id: &str, hardware_port: &str, number_floors: &str) -> ExitStatus {
    println!(
        "Starting elevator process with \n
        ELEVATOR_NETWORK_ID={} \n
        ELEVATOR_HARDWARE_PORT={} \n
        NUMBER_FLOORS={} \n",
        network_id, hardware_port, number_floors
    );

    Command::new("sudo")
        .arg("-E") // Preserve environment variables
        .env("ELEVATOR_NETWORK_ID", network_id)
        .env("ELEVATOR_HARDWARE_PORT", hardware_port)
        .env("NUMBER_FLOORS", number_floors)
        .arg("cargo")
        .arg("run")
        .arg("--bin")
        .arg("elevator") // Specify the elevator binary
        .status()
        .expect("Failed to start elevator process")
}

/// Connect to the elevator hardware to send the stop motor command.
fn stop_elevator_motor(hardware_port: &str) {
    let address = format!("localhost:{}", hardware_port);
    match TcpStream::connect(address) {
        Ok(mut socket) => {
            let buf = [1, 0, 0, 0]; // Command to stop the motor.
            if let Err(err) = socket.write_all(&buf) {
                eprintln!("Failed to send stop motor command: {}", err);
            }
        }
        Err(err) => {
            eprintln!("Failed to connect to the elevator system: {}", err);
        }
    }
}

fn main() {
    // Retrieve the environment variables.
    let network_id = env::var("ELEVATOR_NETWORK_ID").unwrap_or_else(|_| "0".to_string());
    let hardware_port = env::var("ELEVATOR_HARDWARE_PORT").unwrap_or_else(|_| "15657".to_string());
    let number_floors = env::var("NUMBER_FLOORS").unwrap_or_else(|_| "4".to_string());

    loop {
        // Start the elevator process.
        let status = start_elevator(&network_id, &hardware_port, &number_floors);

        // For safety, ensure the elevator motor is stopped.
        println!("Ensuring elevator motor is stopped...");
        stop_elevator_motor(&hardware_port);

        // Monitor process exit status.
        if let Some(code) = status.code() {
            println!("Elevator process exited with code: {}", code);
            if code == 0 {
                println!("Elevator process exited successfully. Exiting monitor.");
                break;
            }
        } else {
            println!("Elevator process was terminated by a signal.");
        }

        // Delay before restarting.
        println!("Restarting elevator process in 5 seconds...");
        thread::sleep(Duration::from_secs(5));
    }
}
