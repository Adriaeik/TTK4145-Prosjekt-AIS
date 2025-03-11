use std::fmt;
use std::process;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Elevator {
    socket: Arc<Mutex<TcpStream>>,
    pub num_floors: u8,
}

pub const HALL_UP: u8 = 0;
pub const HALL_DOWN: u8 = 1;
pub const CAB: u8 = 2;

pub const DIRN_DOWN: u8 = u8::MAX;
pub const DIRN_STOP: u8 = 0;
pub const DIRN_UP: u8 = 1;

impl Elevator {
    /// Initialize the elevator by connecting to the given address
    pub async fn init(addr: &str, num_floors: u8) -> Elevator {
        match TcpStream::connect(addr).await {
            Ok(socket) => Self { socket: Arc::new(Mutex::new(socket)), num_floors },
            Err(err) => {
                eprintln!("Failed to connect to the elevator system: {}", err);
                process::exit(1);
            }
        }
    }

    /// Reload the elevator command
    pub async fn reload(&self) {
        let buf = [0, 0, 0, 0];
        if let Err(err) = self.send_command(&buf).await {
            self.log_error_and_exit("Failed to reload", err);
        }
    }

    /// Send motor direction command
    pub async fn motor_direction(&self, dirn: u8) {
        let buf = [1, dirn, 0, 0];
        if let Err(err) = self.send_command(&buf).await {
            self.log_error_and_exit("Failed to set motor direction", err);
        }
    }

    /// Set call button light
    pub async fn call_button_light(&self, floor: u8, call: u8, on: bool) {
        let buf = [2, call, floor, on as u8];
        if let Err(err) = self.send_command(&buf).await {
            self.log_error_and_exit("Failed to set call button light", err);
        }
    }

    /// Set floor indicator
    pub async fn floor_indicator(&self, floor: u8) {
        let buf = [3, floor, 0, 0];
        if let Err(err) = self.send_command(&buf).await {
            self.log_error_and_exit("Failed to set floor indicator", err);
        }
    }

    /// Control the door light
    pub async fn door_light(&self, on: bool) {
        let buf = [4, on as u8, 0, 0];
        if let Err(err) = self.send_command(&buf).await {
            self.log_error_and_exit("Failed to control door light", err);
        }
    }

    /// Control the stop button light
    pub async fn stop_button_light(&self, on: bool) {
        let buf = [5, on as u8, 0, 0];
        if let Err(err) = self.send_command(&buf).await {
            self.log_error_and_exit("Failed to control stop button light", err);
        }
    }

    /// Check the status of a call button
    pub async fn call_button(&self, floor: u8, call: u8) -> bool {
        let mut buf = [6, call, floor, 0];
        match self.send_and_receive(&mut buf).await {
            Ok(_) => buf[1] != 0,
            Err(err) => {
                self.log_error_and_exit("Failed to check call button", err);
                false // Unreachable, but required by compiler
            }
        }
    }

    /// Get the current floor sensor value
    pub async fn floor_sensor(&self) -> Option<u8> {
        let mut buf = [7, 0, 0, 0];
        match self.send_and_receive(&mut buf).await {
            Ok(_) => {
                if buf[1] != 0 {
                    Some(buf[2])
                } else {
                    None
                }
            }
            Err(err) => {
                self.log_error_and_exit("Failed to read floor sensor", err);
                None // Unreachable, but required by compiler
            }
        }
    }

    /// Check the status of the stop button
    pub async fn stop_button(&self) -> bool {
        let mut buf = [8, 0, 0, 0];
        match self.send_and_receive(&mut buf).await {
            Ok(_) => buf[1] != 0,
            Err(err) => {
                self.log_error_and_exit("Failed to check stop button", err);
                false // Unreachable, but required by compiler
            }
        }
    }

    /// Check the obstruction sensor
    pub async fn obstruction(&self) -> bool {
        let mut buf = [9, 0, 0, 0];
        match self.send_and_receive(&mut buf).await {
            Ok(_) => buf[1] != 0,
            Err(err) => {
                self.log_error_and_exit("Failed to check obstruction sensor", err);
                false // Unreachable, but required by compiler
            }
        }
    }

    /// Helper method to send a command
    async fn send_command(&self, buf: &[u8]) -> tokio::io::Result<()> {
        let mut sock = self.socket.lock().await;
        sock.write_all(buf).await // `write_all` already returns `Result<()>`
    }

    /// Helper method to send a command and receive a response
    async fn send_and_receive(&self, buf: &mut [u8]) -> tokio::io::Result<()> {
        let mut sock = self.socket.lock().await;
        sock.write_all(buf).await?; // `write_all` ensures all bytes are sent
        sock.read_exact(buf).await.map(|_| ()) // Convert `Result<usize, _>` to `Result<(), _>`
    }

    /// Helper method to log an error and terminate the process
    fn log_error_and_exit(&self, msg: &str, err: tokio::io::Error) {
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
        let addr = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let sock = self.socket.lock().await;
                sock.peer_addr()
            })
        });

        match addr {
            Ok(addr) => write!(f, "Elevator@{}({})", addr, self.num_floors),
            Err(_) => write!(f, "Elevator@(unknown)({})", self.num_floors),
        }
    }
}
