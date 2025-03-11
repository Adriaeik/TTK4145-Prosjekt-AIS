// This is a special case of the normal driver
// This is made for synchronous processes
// For example peer process

use std::fmt;
use std::io::Write;
use std::net::TcpStream;
use std::process;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Elevator {
    socket: Arc<Mutex<TcpStream>>,
    pub num_floors: u8,
}

pub const DIR_DOWN: u8 = u8::MAX;
pub const DIR_STOP: u8 = 0;
pub const DIR_UP: u8 = 1;

impl Elevator {
    /// Initialize the elevator by connecting to the given address
    pub fn init_sync(addr: &str, num_floors: u8) -> Elevator {
        match TcpStream::connect(addr) {
            Ok(socket) => Self { socket: Arc::new(Mutex::new(socket)), num_floors },
            Err(err) => {
                eprintln!("Failed to connect to the elevator system: {}", err);
                process::exit(1);
            }
        }
    }

    /// Send motor direction command (synchronous version)
    pub fn motor_direction_sync(&self, dirn: u8) {
        let buf = [1, dirn, 0, 0];
        if let Err(err) = self.send_command_sync(&buf) {
            self.log_error_and_exit("Failed to set motor direction", err);
        }
    }

    /// Synchronous helper method to send a command
    fn send_command_sync(&self, buf: &[u8]) -> std::io::Result<()> {
        let mut sock = self.socket.lock().unwrap();
        sock.write_all(buf)?; // Write all bytes synchronously
        Ok(())
    }

    /// Helper method to log an error and terminate the process
    fn log_error_and_exit(&self, msg: &str, err: std::io::Error) {
        eprintln!();
        eprintln!("#============================================================#");
        eprintln!("ERROR: {}: {}", msg, err);
        eprintln!("#============================================================#");
        eprintln!();
        process::exit(1);
    }
}

impl fmt::Display for Elevator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let addr = self.socket.lock().ok().and_then(|sock| sock.peer_addr().ok());
        match addr {
            Some(addr) => write!(f, "Elevator@{}({})", addr, self.num_floors),
            None => write!(f, "Elevator@(unknown)({})", self.num_floors),
        }
    }
}
