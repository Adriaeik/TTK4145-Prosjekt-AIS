//! ## Printing Module
//! 
//! This module is onle here to make logging in the terminal easier to read. 
//! It allows to print in appropriate colors depening on the situation.
//! It also provides a nice print-format for the WorldView. 
use crate::{config, network, world_view::{Dirn, ElevatorBehaviour, WorldView}};
use ansi_term::Colour::{self, Green, Red, Yellow, Purple, White};

use prettytable::color::BLUE;
use unicode_width::UnicodeWidthStr;

/// Prints a message in a specified color to the terminal.
///
/// This function uses the `termcolor` crate to print a formatted message with 
/// a given foreground color. If `PRINT_ELSE_ON` is `false`, the message will not be printed.
///
/// ## Parameters
/// - `msg`: The message to print.
/// - `color`: The color to use for the text output.
///
/// ## Example
/// ```
/// use termcolor::{Color, StandardStream, ColorSpec, WriteColor};
/// use elevatorpro::print;
///
/// print::color("Hello, World!".to_string(), Color::Green);
/// ```
///
/// **Note:** This function does not return a value and prints directly to the terminal.
/// If color output is not supported, the text may not appear as expected.
pub fn color(msg: String, color: Colour) {
    let print_stat = config::PRINT_ELSE_ON.lock().unwrap().clone();
    
    if print_stat {
        println!("{}{}\n", color.paint("[CUSTOM]:  "), color.paint(msg));
    }
}

/// Prints an error message in red to the terminal.
///
/// This function uses the `termcolor` crate to print an error message with a red foreground color.
/// If `PRINT_ERR_ON` is `false`, the message will not be printed.
///
/// ## Parameters
/// - `msg`: The error message to print.
/// 
/// ## Terminal output
/// - "\[ERROR\]:   {}", msg
///
/// ## Example
/// ```
/// use elevatorpro::print;
///
/// print::err("Something went wrong!".to_string());
/// print::err(format!("Something went wront: {}", e));
/// ```
///
/// **Note:** This function does not return a value and prints directly to the terminal.
/// If color output is not supported, the error message may not appear in red.
pub fn err(msg: String) {
    let print_stat = config::PRINT_ERR_ON.lock().unwrap().clone();
    
    if print_stat {
        println!("{}{}\n", Red.paint("[ERROR]:   "), Red.paint(msg));
    }
}

/// Prints a warning message in yellow to the terminal.
///
/// This function uses the `termcolor` crate to print a warning message with a yellow foreground color.
/// If `PRINT_WARN_ON` is `false`, the message will not be printed.
///
/// ## Parameters
/// - `msg`: The warning message to print.
///
/// ## Terminal output
/// - "\[WARNING\]: {}", msg
///
/// ## Example
/// ```
/// use elevatorpro::print;
///
/// print::warn("This is a warning.".to_string());
/// ```
///
/// **Note:** This function does not return a value and prints directly to the terminal.
/// If color output is not supported, the warning message may not appear in yellow.
pub fn warn(msg: String) {
    let print_stat = config::PRINT_WARN_ON.lock().unwrap().clone();
    
    if print_stat {
        println!("{}{}\n", Yellow.paint("[WARNING]: "), Yellow.paint(msg));
    }
}

/// Prints a success message in green to the terminal.
///
/// This function uses the `termcolor` crate to print a success message with a green foreground color.
/// If `PRINT_OK_ON` is `false`, the message will not be printed.
///
/// ## Parameters
/// - `msg`: The success message to print.
///
/// ## Terminal output
/// - "\[OK\]:      {}", msg
///
/// ## Example
/// ```
/// use elevatorpro::print;
///
/// print::ok("Operation successful.".to_string());
/// ```
///
/// **Note:** This function does not return a value and prints directly to the terminal.
/// If color output is not supported, the success message may not appear in green.
pub fn ok(msg: String) {
    let print_stat = config::PRINT_OK_ON.lock().unwrap().clone();

    if print_stat {
        println!("{}{}\n", Green.paint("[OK]:      "), Green.paint(msg));
    }
}

/// Prints an informational message in light blue to the terminal.
///
/// This function uses the `termcolor` crate to print an informational message with a light blue foreground color.
/// If `PRINT_INFO_ON` is `false`, the message will not be printed.
///
/// ## Parameters
/// - `msg`: The informational message to print.
///
/// ## Terminal output
/// - "\[INFO\]:    {}", msg
///
/// ## Example
/// ```
/// use elevatorpro::print;
///
/// print::info("This is an informational message.".to_string());
/// ```
///
/// **Note:** This function does not return a value and prints directly to the terminal.
/// If color output is not supported, the informational message may not appear in light blue.
pub fn info(msg: String) {
    let print_stat = config::PRINT_INFO_ON.lock().unwrap().clone();
    
    let light_blue = Colour::RGB(102, 178, 255); 
    if print_stat {
        println!("{}{}\n", light_blue.paint("[INFO]:    "), light_blue.paint(msg));
    }
}

/// Prints a master-specific message in pink to the terminal.
///
/// This function uses the `termcolor` crate to print a master-specific message with a pink foreground color.
/// If `PRINT_ELSE_ON` is `false`, the message will not be printed.
///
/// ## Parameters
/// - `msg`: The master-specific message to print.
///
/// ## Terminal output
/// - "\[MASTER\]:  {}", msg
///
/// ## Example
/// ```
/// use elevatorpro::print;
///
/// print::master("Master process initialized.".to_string());
/// ```
///
/// **Note:** This function does not return a value and prints directly to the terminal.
/// If color output is not supported, the master message may not appear in pink.
pub fn master(msg: String) {
    let print_stat = config::PRINT_ELSE_ON.lock().unwrap().clone();
    
    let pink = Colour::RGB(255, 51, 255);
    if print_stat {
        println!("{}[MASTER]:  {}\n", pink.paint(""), pink.paint(msg));
    }

}

/// Prints a slave-specific message in orange to the terminal.
///
/// This function uses the `termcolor` crate to print a slave-specific message with an orange foreground color.
/// If `PRINT_ELSE_ON` is `false`, the message will not be printed.
///
/// ## Parameters
/// - `msg`: The slave-specific message to print.
///
/// ## Terminal output
/// - "\[SLAVE\]:   {}", msg
///
/// ## Example
/// ```
/// use elevatorpro::print;
///
/// print::slave("Slave process running.".to_string());
/// ```
///
/// **Note:** This function does not return a value and prints directly to the terminal.
/// If color output is not supported, the slave message may not appear in orange.
pub fn slave(msg: String) {
    let print_stat = config::PRINT_ELSE_ON.lock().unwrap().clone();
    
    let random = Colour::RGB(153, 76, 0);
    if print_stat {
        println!("{}{}\n", random.paint("[MASTER]:  "), random.paint(msg));
    }
}

/// Prints an error message with a cosmic twist, displaying the message in a rainbow of colors.
///
/// This function prints a message when something happens that is theoretically impossible, 
/// such as a "cosmic ray flipping a bit" scenario. It starts with a red "[ERROR]:" label and 
/// follows with the rest of the message displayed in a rainbow pattern.
///
/// # Parameters
/// - `fun`: The function name or description of the issue that led to this cosmic error. 
///
/// ## Terminal output
/// - "[ERROR]: Cosmic rays flipped a bit! 👽 ⚛️ 🔄 1️⃣ 0️⃣ IN: {fun}"
///   Where `{fun}` is replaced by the provided `fun` parameter, and the rest of the message is displayed in rainbow colors.
///
/// # Example
/// ```
/// use elevatorpro::print;
///
/// print::cosmic_err("Something impossible happened".to_string());
/// ```
///
/// **Note:** This function does not return a value and prints directly to the terminal. The message will be printed in a rainbow of colors.
pub fn cosmic_err(fun: String) {
    // Print "[ERROR]:" in red
    print!("{}", Colour::Red.paint("[ERROR]: "));
    
    // Some colours
    let colors = [
        Colour::Red,
        Colour::Yellow,
        Colour::Green,
        Colour::Cyan,
        Colour::Blue,
        Colour::Purple,
    ];
    
    // Rest of the print in rainbow
    let message = format!("Cosmic rays flipped a bit! 👽 ⚛️ 🔄 1️⃣ 0️⃣  IN: {}", fun);
    for (i, c) in message.chars().enumerate() {
        let color = colors[i % colors.len()];
        print!("{}", color.paint(c.to_string()));
    }
    
    println!();
}
/// Pads the input text to a fixed display width using spaces.
/// 
/// Accounts for characters that may take more than one column width (e.g. Unicode symbols),
/// ensuring aligned text in terminal-based tables or UI output.
/// 
/// # Parameters
/// - `text`: The string to pad.
/// - `width`: The total width the text should occupy (including padding).
///
/// # Returns
/// A `String` with the original text left-aligned and padded with spaces to match the desired width.
fn pad_text(text: &str, width: usize) -> String {
    let visible_width = UnicodeWidthStr::width(text);
    let padding = width.saturating_sub(visible_width);
    format!("{}{}", text, " ".repeat(padding))
}

/// Returns a colored and padded string representation of a boolean value.
/// 
/// Uses green for `true` and red for `false`, and pads the result to a fixed width.
/// Useful for displaying network or status indicators in colored terminal output.
/// 
/// # Parameters
/// - `value`: The boolean value to represent.
/// - `width`: The width to pad the output to.
///
/// # Returns
/// A colored `String` containing "true" or "false", padded to the given width.
fn colored_bool_label(value: bool, width: usize) -> String {
    let raw_text = if value { "true" } else { "false" };
    let padded = pad_text(raw_text, width); // brukar din hjelpefunksjon
    if value {
        Green.paint(padded).to_string()
    } else {
        Red.paint(padded).to_string()
    }
}

/// Computes an ANSI RGB color escape sequence based on packet loss percentage.
/// 
/// The color transitions smoothly:
/// - Green at 0% loss (0,255,0)
/// - Yellow at 50% loss (255,255,0)
/// - Red at 100% loss (255,0,0)
/// 
/// Intended for use in terminal UIs to visually represent loss severity.
///
/// # Parameters
/// - `loss`: Packet loss as a percentage in the range 0–100.
///
/// # Returns
/// An ANSI escape string that sets the foreground color for subsequent text.
fn rgb_color_for_loss(loss: u8) -> String {
    // loss frå 0 → 100 skal gå frå grøn (0,255,0) → gul (255,255,0) → raud (255,0,0)
    let (r, g) = if loss <= 50 {
        let ratio = loss as f32 / 50.0;
        let r = (ratio * 255.0) as u8;
        (r, 255)
    } else {
        let ratio = (loss as f32 - 50.0) / 50.0;
        let g = ((1.0 - ratio) * 255.0) as u8;
        (255, g)
    };
    format!("\x1b[38;2;{};{};0m", r, g)
}

/// Generates a horizontal colored bar representing packet loss visually in the terminal.
/// 
/// The bar is filled proportionally to the loss percentage, with each filled segment
/// colored using a logarithmic gradient between green and red to emphasize early degradation.
/// 
/// # Parameters
/// - `loss`: Packet loss as a percentage from 0 to 100.
/// - `width`: The total width (number of characters) of the bar.
///
/// # Returns
/// A `String` containing the ANSI-colored loss bar.
fn colored_loss_bar(loss: u8, width: usize) -> String {
    let mut filled = (loss as usize * width) / 100;
    if loss == 0 {
        filled = 1;
    }

    let mut bar = String::new();

    let k = 20.0; // juster for "kor bratt" det blir i starten

    for i in 0..width {
        let symbol = if i < filled { "█" } else { " " };

        let x = i as f32 / width as f32; // 0.0 → 1.0
        let intensity = ((1.0 + k * x).ln()) / ((1.0 + k).ln()); // logaritmisk interpolering, 0–1

        let r = (intensity * 255.0) as u8;
        let g = ((1.0 - intensity) * 255.0) as u8;

        let color = format!("\x1b[38;2;{};{};0m", r, g);
        bar.push_str(&format!("{}{}{}", color, symbol, "\x1b[0m"));
    }
    bar
}

/// Logs the current `WorldView` state to the terminal in a structured and colorized table format.
///
/// This function visually presents the status of the elevator system, including:
/// - Network connection status (internet and elevator mesh)
/// - Packet loss as a colored percentage and visual bar
/// - Hall calls across all floors (up/down requests)
/// - Detailed state of each elevator (ID, door, obstruction, last floor, cab requests, hall tasks, etc.)
///
/// The display uses Unicode symbols and ANSI color codes for clarity:
/// - Green/red circles for active/inactive requests
/// - Directional arrows and colors to indicate elevator movement or door states
///
/// # Parameters
/// - `worldview`: A reference to the current global `WorldView` instance.
/// - `connection`: An optional `ConnectionStatus` containing internet and elevator network status
///                 as well as the current packet loss (0–100%).
///
/// # Behavior
/// - If configured printing is disabled (`config::PRINT_WV_ON` is false), the function exits early.
/// - Displays high-level metadata, followed by per-elevator breakdown.
/// - Pads and aligns all output to fit well in monospaced terminal environments.
///
/// # Notes
/// - This is intended for human-readable debugging and monitoring purposes.
/// - Printing frequency should be limited (e.g., once per 500 ms).
pub fn worldview(worldview: &WorldView, connection: Option<network::ConnectionStatus> ) {
    let print_stat = config::PRINT_WV_ON.lock().unwrap().clone();
    if !print_stat {
        return;
    }
    // Legg til utskrift av nettverksstatus viss det er med
    println!("{}", ansi_term::Colour::Cyan.bold().paint("┌────────────────────────────────┐"));
    println!("{}", ansi_term::Colour::Cyan.bold().paint("│  ELEVATOR NETWORK CONNECTION   │")); 
    println!("{}", ansi_term::Colour::Cyan.bold().paint("└────────────────────────────────┘"));
    match connection {
        Some(status) => {
            let on_net_color = colored_bool_label(status.on_internett, 5);
            let elev_net_color = colored_bool_label(status.connected_on_elevator_network, 5);
    
            let color_prefix = rgb_color_for_loss(status.packet_loss);
            let reset = "\x1b[0m";
            let bar = colored_loss_bar(status.packet_loss, 27);
    
            println!("┌───────────────────────────────┐");
            println!("│ On internett:           {} │", on_net_color);
            println!("│ Elevator network:       {} │", elev_net_color);
            println!("│ Packet loss:        {}{:>8}%{:>2} │", color_prefix, status.packet_loss, reset);
            println!("│ [{}] │", bar);
            println!("└───────────────────────────────┘");
        }
        None => {
            println!("┌───────────────────────────────┐");
            println!("│ Connection status: Not set    │");
            println!("└───────────────────────────────┘");
        }
    }
    

    // Overskrift
    println!("{}", Purple.bold().paint("┌────────────────────────────────┐"));
    println!("{}", Purple.bold().paint("│        WORLD VIEW STATUS       │"));
    println!("{}", Purple.bold().paint("└────────────────────────────────┘"));

    // Generell info-tabell
    println!("┌─────────────┬──────────┬────────────────────┐");
    println!("{}", White.bold().paint("│ Num heiser  │ MasterID │ Pending tasks      │"));
    println!("├─────────────┼──────────┼────────────────────┤");

    println!(
        "│ {:<11} │ {:<8} │                    │",
        worldview.get_num_elev(),
        worldview.master_id
    );

    for (floor, calls) in worldview.hall_request.iter().enumerate().rev() {
        let up = if floor != worldview.hall_request.len() - 1 {
            if calls[0] { "🟢" } else { "🔴" }
        } else {
            "  " // Ingen opp-knapp i øvste etasje
        };

        let down = if floor != 0 {
            if calls[1] { "🟢" } else { "🔴" }
        } else {
            "  " // Ingen ned-knapp i nederste etasje
        };

        println!(
            "│ floor:{:<5} │          │ {} {}              │",
            floor,
            down,
            up
        );
    }

    println!("└─────────────┴──────────┴────────────────────┘");

    // Heisstatus-tabell
    println!("┌──────┬──────────┬──────────────┬──────────────┬─────────────┬──────────────────────┬───────────────┐");
    println!("{}", ansi_term::Colour::White.bold().paint("│ ID   │ Dør      │ Obstruksjon  │ Tasks        │ Siste etasje│ Calls (Etg:Call)     │ Elev status   │"));
    println!("├──────┼──────────┼──────────────┼──────────────┼─────────────┼──────────────────────┼───────────────┤");

    for elev in &worldview.elevator_containers {
        let id_text = pad_text(&format!("{}", elev.elevator_id), 4);
        let door_text = if elev.behaviour == ElevatorBehaviour::DoorOpen || elev.behaviour == ElevatorBehaviour::ObstructionError {
            pad_text(&Yellow.paint("Open").to_string(), 17)
        } else {
            pad_text(&Green.paint("Lukka").to_string(), 17)
        };
        let obstruction_text = if elev.obstruction {
            pad_text(&Red.paint("Ja").to_string(), 21)
        } else {
            pad_text(&Green.paint("Nei").to_string(), 21)
        };
        
        let tasks_emoji: Vec<String> = elev.cab_requests.iter().enumerate().rev()
            .map(|(floor, task)| format!("{:<2} {}", floor, if *task { "🟢" } else { "🔴" }))
            .collect();

        let num_floors = elev.tasks.len();
        let call_list_emoji: Vec<String> = elev.tasks.iter().enumerate().rev()
            .map(|(floor, calls)| {
                let up = if floor != num_floors - 1 {
                    if calls[0] { "🟢" } else { "🔴" }
                } else {
                    "⚫" // ingen opp-knapp i toppetasjen
                };
        
                let down = if floor != 0 {
                    if calls[1] { "🟢" } else { "🔴" }
                } else {
                    "⚫" // ingen ned-knapp i nederste etasje
                };
        
                format!("{:<2} {} {}", floor, down, up)
            })
            .collect();

        let task_status = match (elev.dirn, elev.behaviour) {
            (_, ElevatorBehaviour::Idle) => pad_text(&Green.paint("Idle").to_string(), 22),
            (Dirn::Up, ElevatorBehaviour::Moving) => pad_text(&Yellow.paint("⬆️   Moving").to_string(), 23),
            (Dirn::Down, ElevatorBehaviour::Moving) => pad_text(&Yellow.paint("⬇️   Moving").to_string(), 23),
            (Dirn::Stop, ElevatorBehaviour::Moving) => pad_text(&Yellow.paint("Not Moving").to_string(), 22),
            (_, ElevatorBehaviour::DoorOpen) => pad_text(&Purple.paint("Door Open").to_string(), 22),
            (_, ElevatorBehaviour::ObstructionError) => pad_text(&Red.paint("Obstruction Error").to_string(), 22),
            (_, ElevatorBehaviour::TravelError) => pad_text(&Red.paint("Travel Error").to_string(), 22),
            (_, ElevatorBehaviour::CosmicError) => pad_text(&Red.paint("Cosmic Error?").to_string(), 22),
        };

        let max_rows = std::cmp::max(tasks_emoji.len(), call_list_emoji.len());

        for i in 0..max_rows {
            let task_entry = tasks_emoji.get(i).cloned().unwrap_or_else(|| "  ".to_string());
            let call_entry = call_list_emoji.get(i).cloned().unwrap_or_else(|| "  ".to_string());

            if i == 0 {
                println!(
                    "│ {} │ {} │ {} │ {:<11} │ {:<11} │ {:<18} │ {} │",
                    id_text, door_text, obstruction_text, task_entry, elev.last_floor_sensor, call_entry, task_status
                );
            } else {
                println!(
                    "│      │          │              │ {:<11} │             │ {:<18} │               │",
                    task_entry, call_entry
                );
            }
        }

        println!("├──────┼──────────┼──────────────┼──────────────┼─────────────┼──────────────────────┼───────────────┤");
    }

    println!("└──────┴──────────┴──────────────┴──────────────┴─────────────┴──────────────────────┴───────────────┘");
}
