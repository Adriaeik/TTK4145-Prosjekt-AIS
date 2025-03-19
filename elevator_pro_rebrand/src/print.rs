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
    let mut gen_table = Table::new();
    gen_table.set_format(*format::consts::FORMAT_CLEAN);
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);

    // Overskrift i blå feittskrift
    println!("{}", Purple.bold().paint("WORLD VIEW STATUS"));

    //Legg til generell worldview-info
    //Funka ikke når jeg brukte fargene på lik måte som under. gudene vet hvorfor
    gen_table.add_row(Row::new(vec![
        Cell::new("Num heiser").with_style(Attr::ForegroundColor(color::BRIGHT_BLUE)),
        Cell::new("MasterID").with_style(Attr::ForegroundColor(color::BRIGHT_BLUE)),
        Cell::new("Pending tasks").with_style(Attr::ForegroundColor(color::BRIGHT_BLUE)),
    ]));

    let n_text = format!("{}", wv_deser.get_num_elev()); // Fjern ANSI og bruk prettytable farge
    let m_id_text = format!("{}", wv_deser.master_id);
    let task_list = format!("{:?}", wv_deser.hall_request);

    gen_table.add_row(Row::new(vec![
        Cell::new(&n_text).with_style(Attr::ForegroundColor(color::BRIGHT_YELLOW)),
        Cell::new(&m_id_text).with_style(Attr::ForegroundColor(color::BRIGHT_YELLOW)),
        Cell::new(&task_list),
    ]));

    gen_table.printstd();



    // Legg til heis-spesifikke deler
    // Legg til hovudrad (header) med blå feittskrift
    table.add_row(Row::new(vec![
        Cell::new(&Blue.bold().paint("ID").to_string()),
        Cell::new(&Blue.bold().paint("Dør").to_string()),
        Cell::new(&Blue.bold().paint("Obstruksjon").to_string()),
        Cell::new(&Blue.bold().paint("Tasks").to_string()),
        Cell::new(&Blue.bold().paint("Siste etasje").to_string()),
        Cell::new(&Blue.bold().paint("Calls (Etg:Call)").to_string()),
        Cell::new(&Blue.bold().paint("Elev status").to_string()),
    ]));

    // Iterer over alle heisane
    for elev in wv_deser.elevator_containers {
        // Lag ein fargerik streng for ID
        let id_text = Yellow.bold().paint(format!("{}", elev.elevator_id)).to_string();

        // Door og obstruction i grøn/raud
        let door_status = if elev.behaviour == ElevatorBehaviour::DoorOpen {
            Yellow.paint("Åpen").to_string()
        } else {
            Green.paint("Lukket").to_string()
        };

        let obstruction_status = if elev.obstruction {
            Red.paint("Ja").to_string()
        } else {
            Green.paint("Nei").to_string()
        };

        
        // Farge basert på `to_do` Her skal vi printe tildelt noverande task
        let task_list = format!("{:?}", elev.cab_requests);
        // if let Some(task) = elev.cab_requests {
        //     Yellow.paint(format!("{:?}", task.call.floor)).to_string()
        // } else {
        //     Green.paint("None").to_string()
        // };

        let last_floor = Fixed(69).paint(format!("{}", elev.last_floor_sensor));
            

        // Vanleg utskrift av calls
        let call_list = format!("{:?}", elev.tasks);

        let task_stat_list = match (elev.dirn, elev.behaviour) {
            // (Dirn::Up, _) => Blue.paint("Up"),
            // (Dirn::Down, _) => Blue.paint("Down"),
            (_, ElevatorBehaviour::Idle) => Green.paint("Idle"),
            (_, ElevatorBehaviour::Moving) => Yellow.paint("Moving"),
            (_, ElevatorBehaviour::DoorOpen) => Purple.paint("Door Open"),
            (_, ElevatorBehaviour::Error) => Red.paint("Error"),
        };

        table.add_row(Row::new(vec![
            Cell::new(&id_text),
            Cell::new(&door_status),
            Cell::new(&obstruction_status),
            Cell::new(&task_list),
            Cell::new(&last_floor),
            Cell::new(&call_list),
            Cell::new(&task_stat_list),
        ]));
    }

    // Skriv ut tabellen med fargar (ANSI-kodar)
    table.printstd();
    print!("\n\n");
}
