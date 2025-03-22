use crate::{config, world_view::{Dirn, ElevatorBehaviour, serial}};
use ansi_term::Colour::{self, Green, Red, Yellow, Purple};

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

/// Hjelpefunksjon for å sikre at kolonner har fast breidde
fn pad_text(text: &str, width: usize) -> String {
    let visible_width = UnicodeWidthStr::width(text);
    let padding = width.saturating_sub(visible_width);
    format!("{}{}", text, " ".repeat(padding))
}

/// Logger `wv` i eit fint tabellformat
pub fn worldview(worldview: Vec<u8>) {
    let print_stat = config::PRINT_WV_ON.lock().unwrap().clone();
    if !print_stat {
        return;
    }

    let wv_deser = serial::deserialize_worldview(&worldview);

    // Overskrift
    println!("{}", Purple.bold().paint("┌────────────────────────────────┐"));
    println!("{}", Purple.bold().paint("│        WORLD VIEW STATUS       │"));
    println!("{}", Purple.bold().paint("└────────────────────────────────┘"));

    // Generell info-tabell
    println!("┌─────────────┬──────────┬────────────────────┐");
    println!("{}", ansi_term::Colour::White.bold().paint("│ Num heiser  │ MasterID │ Pending tasks      │"));
    println!("├─────────────┼──────────┼────────────────────┤");

    println!(
        "│ {:<11} │ {:<8} │                    │",
        wv_deser.get_num_elev(),
        wv_deser.master_id
    );

    for (floor, calls) in wv_deser.hall_request.iter().enumerate().rev() {
        println!(
            "│ floor:{:<5} │          │ {} {}              │",
            floor,
            if calls[1] { "🟢" } else { "🔴" }, // Ned
            if calls[0] { "🟢" } else { "🔴" }  // Opp
        );
    }

    println!("└─────────────┴──────────┴────────────────────┘");

    // Heisstatus-tabell
    println!("┌──────┬──────────┬──────────────┬──────────────┬─────────────┬──────────────────────┬───────────────┐");
    println!("{}", ansi_term::Colour::White.bold().paint("│ ID   │ Dør      │ Obstruksjon  │ Tasks        │ Siste etasje│ Calls (Etg:Call)     │ Elev status   │"));
    println!("├──────┼──────────┼──────────────┼──────────────┼─────────────┼──────────────────────┼───────────────┤");

    for elev in &wv_deser.elevator_containers {
        let id_text = pad_text(&format!("{}", elev.elevator_id), 4);
        let door_text = if elev.behaviour == ElevatorBehaviour::DoorOpen {
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

        let call_list_emoji: Vec<String> = elev.tasks.iter().enumerate().rev()
            .map(|(floor, calls)| format!("{:<2} {} {}", floor, if calls[1] { "🟢" } else { "🔴" }, if calls[0] { "🟢" } else { "🔴" }))
            .collect();

        let task_status = match (elev.dirn, elev.behaviour) {
            (_, ElevatorBehaviour::Idle) => pad_text(&Green.paint("Idle").to_string(), 22),
            (Dirn::Up, ElevatorBehaviour::Moving) => pad_text(&Yellow.paint("⬆️   Moving").to_string(), 23),
            (Dirn::Down, ElevatorBehaviour::Moving) => pad_text(&Yellow.paint("⬇️   Moving").to_string(), 23),
            (Dirn::Stop, ElevatorBehaviour::Moving) => pad_text(&Yellow.paint("Not Moving").to_string(), 22),
            (_, ElevatorBehaviour::DoorOpen) => pad_text(&Purple.paint("Door Open").to_string(), 22),
            (_, ElevatorBehaviour::Error) => pad_text(&Red.paint("Error").to_string(), 22),
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
