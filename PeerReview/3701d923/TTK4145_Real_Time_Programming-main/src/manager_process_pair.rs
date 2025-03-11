// * NOTE: Some networking libraries requiring root privileges (for eks for pinging to router to read if we are still on the same network)
// * NOTE: Every new manager needs its own unique MANAGER_ID
// *
// * Because of these factors we must run this process as follows:
// * $ sudo -E MANAGER_ID=<ID> ELEVATOR_NETWORK_ID_LIST="[<ID 1>,<ID 2>,...,<ID N>]" cargo run --bin manager_process_pair

use std::env;
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::Duration;

fn start_manager(manager_id: &str, elevator_network_id_list: &str) -> ExitStatus {
    println!(
        "Starting manager process with \n
        MANAGER_ID={} \n
        ELEVATOR_NETWORK_ID_LIST={} \n",
        manager_id, elevator_network_id_list
    );

    // Start the `cargo run` command with the necessary environment variable
    Command::new("sudo")
        .arg("-E")
        .env("MANAGER_ID", manager_id)
        .env("ELEVATOR_NETWORK_ID_LIST", elevator_network_id_list)
        .arg("cargo")
        .arg("run")
        .arg("--bin")
        .arg("manager") // Specify the database binary
        .status() // Run the command and return the ExitStatus
        .expect("Failed to start manager process") // Handle command failure
}

fn main() {
    // Ensure MANAGER_ID is passed to the parent process
    let manager_id = env::var("MANAGER_ID").expect("MANAGER_ID must be set");
    // Retrieve ELEVATOR_NETWORK_ID_LIST (defaulting to "[0]" if not set)
    let elevator_network_id_list = env::var("ELEVATOR_NETWORK_ID_LIST").unwrap_or_else(|_| "[0]".to_string());

    loop {
        // Start the child process and monitor its exit status
        let status = start_manager(&manager_id, &elevator_network_id_list);

        // Check if the process exited normally
        if let Some(code) = status.code() {
            println!("Manager process exited with code: {}", code);

            // Restart only if it didn't exit with a success code (0)
            if code == 0 {
                println!("Manager process exited successfully. Exiting monitor.");
                break;
            }
        } else {
            println!("Manager process was terminated by a signal.");
        }

        // Delay before restarting to avoid rapid restart loops
        println!("Restarting manager process in 5 seconds...");
        thread::sleep(Duration::from_secs(5));
    }
}
