// Library that allows us to use environment variables or command-line arguments to pass variables from terminal to the program directly
use std::env;

// Library for executing terminal commands
use tokio::process::Command;

// Function to execute the algorithm
pub async fn run_cost_algorithm(json_str: String) -> String {
    let algorithm_path = env::current_dir()
        .unwrap()
        .join("Project-resources")
        .join("cost_fns")
        .join("hall_request_assigner")
        .join("hall_request_assigner");

    let output = Command::new(algorithm_path)
        .arg("--input")
        .arg(json_str) // ✅ Use JSON directly as received
        .output()
        .await
        .expect("Failed to start algorithm");

    let output_str = String::from_utf8_lossy(&output.stdout).into_owned();

    return output_str;
}
