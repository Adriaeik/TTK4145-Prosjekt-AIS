use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::{config, world_view::{Dirn, ElevatorBehaviour, serial}};
use crate::elevio;
use ansi_term::Colour::{Blue, Green, Red, Yellow, Purple, Fixed};
use prettytable::{Table, Row, Cell, format, Attr, color};


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
pub fn color(msg: String, color: Color) {
    let print_stat = config::PRINT_ELSE_ON.lock().unwrap().clone();
    
    if print_stat {        
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(ColorSpec::new().set_fg(Some(color))).unwrap();
        writeln!(&mut stdout, "[CUSTOM]:  {}", msg).unwrap();
        stdout.set_color(&ColorSpec::new()).unwrap();
        println!("\r\n");
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
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
        writeln!(&mut stdout, "[ERROR]:   {}", msg).unwrap();
        stdout.set_color(&ColorSpec::new()).unwrap();
        println!("\r\n");
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
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
        writeln!(&mut stdout, "[WARNING]: {}", msg).unwrap();
        stdout.set_color(&ColorSpec::new()).unwrap();
        println!("\r\n");
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
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
        writeln!(&mut stdout, "[OK]:      {}", msg).unwrap();
        stdout.set_color(&ColorSpec::new()).unwrap();
        println!("\r\n");
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
    
    if print_stat {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(102, 178, 255/*lyseblå*/)))).unwrap();
        writeln!(&mut stdout, "[INFO]:    {}", msg).unwrap();
        stdout.set_color(&ColorSpec::new()).unwrap();
        println!("\r\n");
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
    
    if print_stat {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255, 51, 255/*Rosa*/)))).unwrap();
        writeln!(&mut stdout, "[MASTER]:  {}", msg).unwrap();
        stdout.set_color(&ColorSpec::new()).unwrap();
        println!("\r\n");
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
    
    if print_stat {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(153, 76, 0/*Tilfeldig*/)))).unwrap();
        writeln!(&mut stdout, "[SLAVE]:   {}", msg).unwrap();
        stdout.set_color(&ColorSpec::new()).unwrap();
        println!("\r\n");
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
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    // Skriv ut "[ERROR]:" i rød
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
    write!(&mut stdout, "[ERROR]: ").unwrap();
    // Definer regnbuefargene
    let colors = [
        Color::Red,
        Color::Yellow,
        Color::Green,
        Color::Cyan,
        Color::Blue,
        Color::Magenta,
    ];
    // Resten av meldingen i regnbuefarger
    let message = format!("Cosmic rays flipped a bit! 👽 ⚛️ 🔄 1️⃣ 0️⃣  IN: {}", fun);
    for (i, c) in message.chars().enumerate() {
        let color = colors[i % colors.len()];
        stdout.set_color(ColorSpec::new().set_fg(Some(color))).unwrap();
        write!(&mut stdout, "{}", c).unwrap();
    }
    // Tilbakestill fargen
    stdout.set_color(&ColorSpec::new()).unwrap();
    println!();
}

/// Logs `wv` in a nice format
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
    println!("│ Num heiser  │ MasterID │ Pending tasks      │");
    println!("├─────────────┼──────────┼────────────────────┤");

    println!(
        "│ {:<11} │ {:<8} │                    │",
        wv_deser.get_num_elev(),
        wv_deser.master_id
    );

    for (floor, calls) in wv_deser.hall_request.iter().enumerate() {
        println!(
            "│ {:<11} │          │ {} {} │",
            floor,
            if calls[0] { "✅" } else { "❌" }, // Opp
            if calls[1] { "✅" } else { "❌" }  // Ned
        );
    }

    println!("└─────────────┴──────────┴────────────────────┘");

    // Heisstatus-tabell
    println!("┌──────┬─────────┬──────────────┬──────────────┬─────────────┬──────────────────────┬───────────────┐");
    println!("│ ID   │ Dør     │ Obstruksjon  │ Tasks        │ Siste etasje│ Calls (Etg:Call)     │ Elev status   │");
    println!("├──────┼─────────┼──────────────┼──────────────┼─────────────┼──────────────────────┼───────────────┤");

    for elev in wv_deser.elevator_containers {
        let id_text = format!("│ {:<4} │", elev.elevator_id);
        let door_status = if elev.behaviour == ElevatorBehaviour::DoorOpen {
            format!(" {:<7} │", Yellow.paint("Åpen"))
        } else {
            format!(" {:<7} │", Green.paint("Lukket"))
        };

        let obstruction_status = if elev.obstruction {
            format!(" {:<12} │", Red.paint("Ja"))
        } else {
            format!(" {:<12} │", Green.paint("Nei"))
        };

        // Konverter Tasks til emoji-tabell
        let tasks_emoji = elev.cab_requests
            .iter()
            .enumerate()
            .map(|(floor, task)| format!("{:<2} {}", floor, if *task { "✅" } else { "❌" }))
            .collect::<Vec<String>>();

        // Konverter Calls til emoji-tabell
        let call_list_emoji = elev.tasks
            .iter()
            .enumerate()
            .map(|(floor, calls)| format!(
                "{:<2} {} {}",
                floor,
                if calls[0] { "✅" } else { "❌" }, // Opp
                if calls[1] { "✅" } else { "❌" }  // Ned
            ))
            .collect::<Vec<String>>();

        let task_stat_list = match (elev.dirn, elev.behaviour) {
            (_, ElevatorBehaviour::Idle) => Green.paint("Idle").to_string(),
            (_, ElevatorBehaviour::Moving) => Yellow.paint("Moving").to_string(),
            (_, ElevatorBehaviour::DoorOpen) => Purple.paint("Door Open").to_string(),
            (_, ElevatorBehaviour::Error) => Red.paint("Error").to_string(),
        };

        // Finn max antal rader for Tasks eller Calls
        let max_rows = std::cmp::max(tasks_emoji.len(), call_list_emoji.len());

        for i in 0..max_rows {
            let task_entry = tasks_emoji.get(i).cloned().unwrap_or_else(|| "  ".to_string()); // Legg buffer på tomme rader
            let call_entry = call_list_emoji.get(i).cloned().unwrap_or_else(|| "  ".to_string());

            if i == 0 {
                println!(
                    "{}{}{} {:<15} │ {:<11} │ {:<22} │ {:<13} │",
                    id_text, door_status, obstruction_status, task_entry, elev.last_floor_sensor, call_entry, task_stat_list
                );
            } else {
                println!(
                    "│      │         │              │ {:<15} │             │ {:<22} │               │",
                    task_entry, call_entry
                );
            }
        }

        println!("├──────┼─────────┼──────────────┼──────────────┼─────────────┼──────────────────────┼───────────────┤");
    }

    println!("└──────┴─────────┴──────────────┴──────────────┴─────────────┴──────────────────────┴───────────────┘");
}
