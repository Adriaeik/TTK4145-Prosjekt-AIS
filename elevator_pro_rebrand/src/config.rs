
use std::net::Ipv4Addr;
use std::time::Duration;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Network prefix: Initialized as the local network prefix in Sanntidshallen
pub static NETWORK_PREFIX: &str = "10.100.23";

/// Port for TCP between nodes
pub static PN_PORT: u16 = u16::MAX;
/// Port for TCP between node and local backup
pub static BCU_PORT: u16 = 50000; 
/// Dummy port. Used for sending/recieving of UDP broadcasts
pub static DUMMY_PORT: u16 = 42069;

/// UDP broadcast listen address
pub static BC_LISTEN_ADDR: &str = "0.0.0.0";
/// UDP broadcast adress
pub static BC_ADDR: &str = "255.255.255.255";
/// Dummy IPv4 address when there is no internet connection (TODO: checking for internet could use an Option)
pub static OFFLINE_IP: Ipv4Addr = Ipv4Addr::new(69, 69, 69, 69);
/// IP to local elevator
pub static LOCAL_ELEV_IP: &str = "localhost:15657";

/// The default number of floors. Used for initializing the elevators in Sanntidshallen
pub const DEFAULT_NUM_FLOORS: u8 = 4;
/// Polling duration for reading from elevator
pub const ELEV_POLL: Duration = Duration::from_millis(25);

/// Error ID (TODO: Could use Some(ID) to identify errors)
pub const ERROR_ID: u8 = 255;

/// Index to ID of the master in a serialized worldview
pub const MASTER_IDX: usize = 1;
/// Key send in front of worldview on UDP broadcast, to filter out irrelevant broadcasts 
pub const KEY_STR: &str = "Gruppe 25";

/// Timeout duration of TCP connections
pub const TCP_TIMEOUT: u64 = 5000; // i millisekunder
/// Probably unneccasary
pub const TCP_PER_U64: u64 = 10; // i millisekunder
/// Period between sending of UDP broadcasts 
pub const UDP_PERIOD: Duration = Duration::from_millis(TCP_PER_U64);
/// Period between sending of TCP messages to master-node
pub const TCP_PERIOD: Duration = Duration::from_millis(TCP_PER_U64);

pub const POLL_PERIOD: Duration = Duration::from_millis(10);

/// Timeout duration of slave-nodes
pub const SLAVE_TIMEOUT: Duration = Duration::from_millis(100);

/// Timeout duration of master-nodes
pub const MASTER_TIMEOUT: Duration = Duration::from_secs(50); // 5 sekunder før failover

/// Timeout duration of backup-nodes
pub const BACKUP_TIMEOUT: Duration = Duration::from_secs(50); // 5 sekunder før failover

/// Size used for buffer when reading UDP broadcasts
pub const UDP_BUFFER: usize = u16::MAX as usize;

/// Time in seconds an elevator has to complete a task before its considered failed by master
pub const TASK_TIMEOUT: u64 = 100;


/// Bool to determine if program should print worldview
pub static PRINT_WV_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));
/// Bool to determine if program should print error's
pub static PRINT_ERR_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));
/// Bool to determine if program should print warnings
pub static PRINT_WARN_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));
/// Bool to determine if program should print ok-messages
pub static PRINT_OK_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));
/// Bool to determine if program should print info-messages
pub static PRINT_INFO_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));
/// Bool to determine if program should print other prints
pub static PRINT_ELSE_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));


/// Penalty for beeing busy
pub const BUSY_PENALTY: u32 = 5;
/// Penalty for going wrong direction
pub const WRONG_DIRECTION_PENALTY: u32 = 10;