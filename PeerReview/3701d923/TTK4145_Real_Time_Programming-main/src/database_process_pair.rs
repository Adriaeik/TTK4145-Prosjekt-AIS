// * NOTE: Some networking libraries requiring root privileges (for eks for pinging to router to read if we are still on the same network)
// * NOTE: Every new database needs its own unique DATABASE_ID
// *
// * Because of these factors we must run this process as follows:
// * $ sudo -E DATABASE_NETWORK_ID=<ID> ELEVATOR_NETWORK_ID_LIST="[<ID 1>,<ID 2>,...,<ID N>]" NUMBER_FLOORS=<NUMBER FLOORS> cargo run --bin database_process_pair

use std::env;
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::Duration;

fn start_database(database_network_id: &str, elevator_network_id_list: &str, number_floors: &str) -> ExitStatus {
    println!(
        "Starting database process with \n
        DATABASE_NETWORK_ID={} \n
        ELEVATOR_NETWORK_ID_LIST={} \n
        NUMBER_FLOORS={} \n",
        database_network_id, elevator_network_id_list, number_floors
    );

    Command::new("sudo")
        .arg("-E") // Preserve environment
        .env("DATABASE_NETWORK_ID", database_network_id)
        .env("ELEVATOR_NETWORK_ID_LIST", elevator_network_id_list)
        .env("NUMBER_FLOORS", number_floors)
        .arg("cargo")
        .arg("run")
        .arg("--bin")
        .arg("database") // Specify the database binary
        .status()
        .expect("Failed to start database process")
}

fn main() {
    // Ensure DATABASE_NETWORK_ID is passed to the parent process
    let database_network_id = env::var("DATABASE_NETWORK_ID").expect("DATABASE_NETWORK_ID must be set");
    // Retrieve ELEVATOR_NETWORK_ID_LIST (defaulting to "[0]" if not set)
    let elevator_network_id_list = env::var("ELEVATOR_NETWORK_ID_LIST").unwrap_or_else(|_| "[0]".to_string());
    // Retrieve NUMBER_FLOORS (defaulting to "4" if not set)
    let number_floors = env::var("NUMBER_FLOORS").unwrap_or_else(|_| "4".to_string());

    loop {
        let status = start_database(&database_network_id, &elevator_network_id_list, &number_floors);

        if let Some(code) = status.code() {
            println!("Database process exited with code: {}", code);
            if code == 0 {
                println!("Database process exited successfully. Exiting monitor.");
                break;
            }
        } else {
            println!("Database process was terminated by a signal.");
        }

        println!("Restarting database process in 5 seconds...");
        thread::sleep(Duration::from_secs(5));
    }
}
