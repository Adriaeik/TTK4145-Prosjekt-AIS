

use std::{sync::atomic::Ordering, net::SocketAddr, time::Duration, borrow::Cow, env};
use tokio::{time::{Instant, timeout}, net::UdpSocket};
use socket2::{Domain, Socket, Type};
use local_ip_address::local_ip;
use crate::{config, elevio, /*manager::task_allocator::Task,*/ network::local_network, print, ip_help_functions::ip2id, world_view::{self, serial, ElevatorContainer, WorldView}};


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
/// let worldview: worldview::WorldView = worldview::serial::deserialize_worldview(&worldview_data);
/// ```
pub async fn initialize_worldview(self_container : Option< world_view::ElevatorContainer>) -> Vec<u8> {
    let mut worldview = WorldView::default();
    
    let mut elev_container = if let Some(container) = self_container {
        container
    } else {
        // Opprett ein standard ElevatorContainer med ein initial placeholder-task
        let mut container = ElevatorContainer::default();
        container.cab_requests[0] = true;
        container
    };


    // Retrieve local IP address
    let ip = match local_ip() {
        Ok(ip) => ip,
        Err(e) => {
            print::err(format!("Failed to get local IP at startup: {}", e));
            panic!();
        }
    };

    // Extract self ID from IP address (last segment of IP)
    local_network::SELF_ID.store(ip2id(ip), Ordering::SeqCst);
    elev_container.elevator_id = local_network::SELF_ID.load(Ordering::SeqCst);
    worldview.master_id = local_network::SELF_ID.load(Ordering::SeqCst);
    worldview.add_elev(elev_container.clone());

    // Listen for UDP messages for a short time to detect other elevators
    let wv_from_udp = check_for_udp().await;
    if wv_from_udp.is_empty() {
        print::info("No other elevators detected on the network.".to_string());
        return serial::serialize_worldview(&worldview);
    }

    // If other elevators are found, merge worldview and add the local elevator
    let mut wv_from_udp_deser = serial::deserialize_worldview(&wv_from_udp);
    wv_from_udp_deser.add_elev(elev_container.clone());

    // Set self as master if the current master has a higher ID
    if wv_from_udp_deser.master_id > local_network::SELF_ID.load(Ordering::SeqCst) {
        wv_from_udp_deser.master_id = local_network::SELF_ID.load(Ordering::SeqCst);
    }

    // Serialize and return the updated worldview
    serial::serialize_worldview(&wv_from_udp_deser)
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
pub async fn check_for_udp() -> Vec<u8> {
    // Construct the UDP broadcast listening address
    let broadcast_listen_addr = format!("{}:{}", config::BC_LISTEN_ADDR, config::DUMMY_PORT);
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
    let mut read_wv: Vec<u8> = Vec::new();
    
    // Placeholder for received message
    let mut message: Cow<'_, str>;

    // Start the timer for 1-second listening duration
    let time_start = Instant::now();
    let duration = Duration::from_secs(1);

    while Instant::now().duration_since(time_start) < duration {
        // Attempt to receive a UDP packet within the timeout duration
        let recv_result = timeout(duration, socket.recv_from(&mut buf)).await;

        match recv_result {
            Ok(Ok((len, _))) => {
                // Convert the received bytes into a string
                message = String::from_utf8_lossy(&buf[..len]).into_owned().into();
            }
            Ok(Err(e)) => {
                // Log errors if receiving fails
                print::err(format!("init.rs, udp_listener(): {}", e));
                continue;
            }
            Err(_) => {
                // Timeout occurred – no data received within 1 second
                print::warn("Timeout - no data received within 1 second.".to_string());
                break;
            }
        }

        // Verify that the UDP message is from our expected network
        if &message[1..config::KEY_STR.len() + 1] == config::KEY_STR {
            // Extract and clean the message by removing the key and surrounding characters
            let clean_message = &message[config::KEY_STR.len() + 3..message.len() - 1];

            // Parse the message as a comma-separated list of u8 values
            read_wv = clean_message
                .split(", ") // Split on ", "
                .filter_map(|s| s.parse::<u8>().ok()) // Convert to u8, ignore errors
                .collect(); // Collect into a Vec<u8>

            break; // Exit loop as a valid message was received
        }
    }

    // Drop the socket to free resources
    drop(socket);

    // Return the parsed UDP message data
    read_wv
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
pub fn parse_args() -> bool {
    let args: Vec<String> = env::args().collect();

    // Hvis det ikke finnes argumenter, returner false
    if args.len() <= 0 {
        return false;
    }

    for arg in &args[1..] {
        let parts: Vec<&str> = arg.split("::").collect();
        if parts.len() == 2 {
            let key = parts[0].to_lowercase();
            let value = parts[1].to_lowercase();
            let is_true = value == "true";

           
            match key.as_str() {
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
            
        } else if arg.to_lowercase() == "help" {
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
        } else if arg.to_lowercase() == "backup" {
            return true;
        }
    }

    // Hvis ingen av argumentene matcher "backup", returner false
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
pub fn get_terminal_command() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        ("cmd".to_string(), vec!["/C".to_string(), "start".to_string()])
    } else {
        ("gnome-terminal".to_string(), vec!["--".to_string()])
    }
}
