//! ## Initialization Module
//!
//! This module is responsible for initializing the elevator system's worldview
//! and handling system arguments. It provides functions to execute necessary startup tasks.
//!
//! ### Key Responsibilities:
//! - **Worldview Initialization**: Constructs an initial worldview for the elevator system 
//!   and attempts to join an existing network if possible.
//! - **Command-line Argument Parsing**: Reads arguments from `cargo run` to control logging 
//!   verbosity and enable debug or backup modes.
//! - **Terminal Command Execution**: Provides platform-specific commands for opening new 
//!   terminal windows.
//! - **Cost Function Build Execution**: Runs a build script for the hall request assigner 
//!   cost function.
//!
//! ### Overview of Functions:
//! - `initialize_worldview` – Creates an initial worldview and merges with the network if possible.
//! - `parse_args` – Parses command-line arguments to configure logging settings and modes.
//! - `get_terminal_command` – Returns the appropriate terminal command for different operating systems.
//! - `build_cost_fn` – Executes a build script for the hall request assigner cost function.

use crate::config; 
use crate::ip_help_functions::ip2id;
use crate::network;
use crate::print; 
use crate::world_view::{self, ElevatorContainer, WorldView};

use std::env;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket; 
use tokio::time::{sleep, timeout, Instant};
use tokio::process::Command;
use socket2::{Domain, Socket, Type};
use local_ip_address::local_ip;

/// ### Initializes the worldview on startup
///
/// This function creates an initial worldview for the elevator system and attempts to join an existing network if possible.
///
/// ## Steps:
/// 1. **Create an empty worldview and elevator container.**
/// 2. **Add an initial placeholder task** to both the task queue and task status list.
/// 3. **Retrieve the local machine's IP address** to determine its unique ID.
/// 4. **Set the elevator ID and master ID** using the extracted IP-based identifier.
/// 5. **Listen for UDP messages** for a brief period to detect other nodes on the network.
/// 6. **If no nodes are found**, return the current worldview as is, with self id as the network master.
/// 7. **If other elevators are detected**, merge their worldview with the local elevator's data.
/// 8. **Check if the master ID should be updated** based on the smallest ID present.
/// 9. **Return the serialized worldview**, ready to be used for network synchronization.
///
/// ## Returns:
/// - A `Vec<u8>` containing the serialized worldview data.
/// 
/// ## Panics:
/// - No internet connection on start-up will result in a panic!
///
/// ## Example Usage:
/// ```rust
/// let worldview_data: Vec<u8> = initialize_worldview().await;
/// let worldview: worldview::WorldView = worldview::deserialize(&worldview_data);
/// ```
pub async fn initialize_worldview(
    self_container : Option<&world_view::ElevatorContainer>
) -> WorldView 
{
    let mut worldview = WorldView::default();
    
    let elev_container: &mut ElevatorContainer = if let Some(container) = self_container 
    {
        &mut container.to_owned()
    } else 
    {
        // Opprett ein standard ElevatorContainer med ein initial placeholder-task
        let container = ElevatorContainer::default();
        &mut container.clone()
    };


    // Retrieve local IP address
    let ip = match local_ip() 
    {
        Ok(ip) => ip,
        Err(e) => 
        {
            print::err(format!("Failed to get local IP at startup: {}", e));
            panic!();
        }
    };

    // Extract self ID from IP address (last segment of IP)
    network::set_self_id(ip2id(ip));
    elev_container.elevator_id = network::read_self_id();
    worldview.master_id = network::read_self_id();
    worldview.add_elev(elev_container.clone());

    // Listen for UDP messages for a short time to detect other elevators
    let mut wv_from_udp = match check_for_udp().await 
    {
        Some(wv) => wv,
        None => 
        {
            print::info("No other elevators detected on the network.".to_string());
            return worldview
        },
    };
    
    // Check if the network has backed up any cab_requests from you, save them if that is the case
    let saved_cab_requests: std::collections::HashMap<u8, Vec<bool>> = wv_from_udp.cab_requests_backup.clone();
    if let Some(saved_requests) = saved_cab_requests.get(&elev_container.elevator_id) 
    {
        elev_container.cab_requests = saved_requests.clone();
    }
    // Add your elevator to the worldview
    wv_from_udp.add_elev(elev_container.clone());

    // Set self as master if the current master has a higher ID
    if wv_from_udp.master_id > network::read_self_id() 
    {
        wv_from_udp.master_id = network::read_self_id();
    }

    // Serialize and return the updated worldview
    wv_from_udp
}



/// ### Listens for a UDP broadcast message for 1 second
///
/// This function listens for incoming UDP broadcasts on a predefined port.
/// It ensures that the received message originates from the expected network before accepting it.
///
/// ## Steps:
/// 1. **Set up a UDP socket** bound to a predefined broadcast address.
/// 2. **Configure socket options** for reuse and broadcasting.
/// 3. **Start a timer** and listen for UDP packets for up to 1 second.
/// 4. **If a message is received**, attempt to decode it as a UTF-8 string.
/// 5. **Filter out messages that do not contain the expected key**.
/// 6. **Extract the relevant data** and convert it into a `Vec<u8>`.
/// 7. **Return the parsed data or an empty vector** if no valid message was received.
///
/// ## Returns:
/// - A `Vec<u8>` containing parsed worldview data if a valid UDP message was received.
/// - An empty vector if no message was received within 1 second.
///
/// ## Example Usage:
/// ```rust
/// let udp_data = check_for_udp().await;
/// if !udp_data.is_empty() {
///     println!("Received worldview data: {:?}", udp_data);
/// } else {
///     println!("No UDP message received within 1 second.");
/// }
/// ```
async fn check_for_udp() -> Option<WorldView> 
{
    // Construct the UDP broadcast listening address
    let broadcast_listen_addr = format!("{}:{}", config::BC_LISTEN_ADDR, config::BROADCAST_PORT);
    let socket_addr: SocketAddr = broadcast_listen_addr.parse().expect("Invalid address");

    // Create a new UDP socket
    let socket_temp = Socket::new(Domain::IPV4, Type::DGRAM, None)
        .expect("Failed to create new socket");

    // Configure socket for address reuse and broadcasting
    socket_temp.set_nonblocking(true).expect("Failed to set non-blocking");
    socket_temp.set_reuse_address(true).expect("Failed to set reuse address");
    socket_temp.set_broadcast(true).expect("Failed to enable broadcast mode");
    socket_temp.bind(&socket_addr.into()).expect("Failed to bind socket");

    // Convert standard socket into an async UDP socket
    let socket = UdpSocket::from_std(socket_temp.into()).expect("Failed to create UDP socket");

    // Buffer for receiving UDP data
    let mut buf = [0; config::UDP_BUFFER];
    let mut read_wv: Option<WorldView>;
    

    // Start the timer for 1-second listening duration
    let time_start = Instant::now();
    let duration = Duration::from_secs(1);

    while Instant::now().duration_since(time_start) < duration 
    {
        // Attempt to receive a UDP packet within the timeout duration
        let recv_result = timeout(duration, socket.recv_from(&mut buf)).await;

        match recv_result 
        {
            Ok(Ok((len, _))) => 
            {
                // Convert the received bytes into a string
                read_wv = network::udp_broadcast::parse_message(&buf[..len]);
            }
            Ok(Err(e)) => 
            {
                // Log errors if receiving fails
                print::err(format!("init.rs, udp_listener(): {}", e));
                continue;
            }
            Err(_) => 
            {
                // Timeout occurred – no data received within 1 second
                print::warn("Timeout - no data received within 1 second.".to_string());
                break;
            }
        }

        match read_wv 
        {
            Some(wv) => 
            {
                return Some(wv);
            },
            None => 
            {
                continue;
            }
        }
    }

    // Drop the socket to free resources
    drop(socket);

    // Return the parsed UDP message data
    None
}


/// ### Reads arguments from `cargo run`
/// 
/// Used to modify what is printed during runtime. Available options:
/// 
/// `print_wv::(true/false)` &rarr; Prints the worldview twice per second  
/// `print_err::(true/false)` &rarr; Prints error messages  
/// `print_wrn::(true/false)` &rarr; Prints warning messages  
/// `print_ok::(true/false)` &rarr; Prints OK messages  
/// `print_info::(true/false)` &rarr; Prints informational messages  
/// `print_else::(true/false)` &rarr; Prints other messages, including master, slave, and color messages  
/// `debug::` &rarr; Disables all prints except error messages  
/// `help` &rarr; Displays all possible arguments without starting the program  
/// 
/// If no arguments are provided, all prints are enabled by default.
/// 
/// Secret options:  
/// `backup` &rarr; Starts the program in backup-mode.
/// 
pub fn parse_args() -> bool 
{
    let args: Vec<String> = env::args().collect();

    // Hvis det ikke finnes argumenter, returner false
    if args.len() <= 0 {return false}

    for arg in &args[1..] 
    {
        let parts: Vec<&str> = arg.split("::").collect();
        if parts.len() == 2 
        {
            let key = parts[0].to_lowercase();
            let value = parts[1].to_lowercase();
            let is_true = value == "true";

           
            match key.as_str() 
            {
                "print_wv" => *config::PRINT_WV_ON.lock().unwrap() = is_true,
                "print_err" => *config::PRINT_ERR_ON.lock().unwrap() = is_true,
                "print_warn" => *config::PRINT_WARN_ON.lock().unwrap() = is_true,
                "print_ok" => *config::PRINT_OK_ON.lock().unwrap() = is_true,
                "print_info" => *config::PRINT_INFO_ON.lock().unwrap() = is_true,
                "print_else" => *config::PRINT_ELSE_ON.lock().unwrap() = is_true,
                "debug" => { // Debug modus: Kun error-meldingar
                    *config::PRINT_WV_ON.lock().unwrap() = false;
                    *config::PRINT_WARN_ON.lock().unwrap() = false;
                    *config::PRINT_OK_ON.lock().unwrap() = false;
                    *config::PRINT_INFO_ON.lock().unwrap() = false;
                    *config::PRINT_ELSE_ON.lock().unwrap() = false;
                }
                _ => {}
            }
            
        } else if arg.to_lowercase() == "help" 
        {
            println!("Tilgjengelige argument:");
            println!("  print_wv::true/false");
            println!("  print_err::true/false");
            println!("  print_warn::true/false");
            println!("  print_ok::true/false");
            println!("  print_info::true/false");
            println!("  print_else::true/false");
            println!("  debug (kun error-meldingar vises)");
            println!("  backup (starter backup-prosess)");
            std::process::exit(0);
        } else if arg.to_lowercase() == "backup" 
        {
            return true;
        }
    }

    // If no arguments was backup, return false
    false
}


/// Returns the terminal command for the corresponding OS.
///
/// # Example
/// ```
/// use elevatorpro::utils::get_terminal_command;
///
/// let (cmd, args) = get_terminal_command();
///
/// if cfg!(target_os = "windows") {
///     assert_eq!(cmd, "cmd");
///     assert_eq!(args, vec!["/C", "start"]);
/// } else {
///     assert_eq!(cmd, "gnome-terminal");
///     assert_eq!(args, vec!["--"]);
/// }
/// ```
pub fn get_terminal_command() -> (String, Vec<String>) 
{
    if cfg!(target_os = "windows") 
    {
        ("cmd".to_string(), vec!["/C".to_string(), "start".to_string()])
    } else 
    {
        ("gnome-terminal".to_string(), vec!["--".to_string()])
    }
}



/// ### Executes the `build.sh` script for the hall_request_assigner cost function in a separate process.
/// 
/// This function asynchronously runs the `build.sh` script located in the
/// `libs/Project_resources/cost_fns/hall_request_assigner` directory using `bash`.
/// 
/// If the build script runs successfully, its stdout and stderr are printed to the console,
/// and the program continues normally. If the script fails, the function will print relevant
/// error output, suggest manual build steps for debugging, and then terminate the process
/// by panicking.
///
/// # Panics
/// Panics if the script fails to execute or if it exits with a non-zero status code.
/// This ensures the caller is alerted early to any build issues.
pub async fn build_cost_fn() 
{
    let output = Command::new("bash")
        .arg("build.sh")
        .current_dir("libs/Project_resources/cost_fns/hall_request_assigner")
        .output()
        .await
        .expect("Failed to run build.sh");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    if output.status.success() 
    {
        println!("build.sh completed successfully.");
    } else 
    {
        eprintln!("build.sh failed. Please try building manually in a new terminal:");
        eprintln!("1. cd libs/Project_resources/cost_fns/hall_request_assigner");
        eprintln!("2. bash build.sh");
        panic!("Failed to build hall_request_assigner.");
    }
    sleep(Duration::from_millis(2000)).await;
}


