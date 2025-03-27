//! # config.rs – Centralized Parameter Store
//!
//! This module holds all static program parameters used throughout the system.
//! Keeping configuration in one place makes tuning, experimentation, and testing easier.
//!
//! ## ⚠️ Maintenance Note:
//! Some of these constants may no longer be used. Consider cleaning up unused values.

use std::net::Ipv4Addr;
use std::sync::Mutex;
use std::time::Duration;
use once_cell::sync::Lazy;

//
// ──────────────────────────────────────────────────────────────
//   1. NETWORK SETTINGS
// ──────────────────────────────────────────────────────────────
//

/// Base network prefix for Sanntidshallen
pub static NETWORK_PREFIX: &str = "10.100.23";

/// Port used for inter-node TCP communication (not active in current design)
pub static PN_PORT: u16 = u16::MAX;

/// Port for local TCP communication with backup client
pub static BCU_PORT: u16 = 50001;

/// Dummy port for UDP broadcast messages
pub static BROADCAST_PORT: u16 = 42069;

/// Port used for direct UDP communication of elevator containers
pub const UDP_CONTAINER_PORT: u16 = 50000;

/// UDP broadcast listen address (bind address)
pub static BC_LISTEN_ADDR: &str = "0.0.0.0";

/// Broadcast address used for system-wide discovery
pub static BC_ADDR: &str = "255.255.255.255";

/// Dummy offline IP for fallback logic (used when disconnected)
pub static OFFLINE_IP: Ipv4Addr = Ipv4Addr::new(69, 69, 69, 69);

/// Localhost address used for visualization tools
pub static LOCAL_ELEV_IP: &str = "localhost:15657";

/// Broadcast key used to filter out invalid worldview messages
pub const KEY_STR: &str = "Gruppe 25";

//
// ──────────────────────────────────────────────────────────────
//   2. SYSTEM & ELEVATOR PARAMETERS
// ──────────────────────────────────────────────────────────────
//

/// Default number of floors in Sanntidshallen setup
pub const DEFAULT_NUM_FLOORS: u8 = 4;

/// Duration between elevator hardware polls
pub const ELEV_POLL: Duration = Duration::from_millis(25);

/// Special error ID used to mark invalid elevators
pub const ERROR_ID: u8 = 255;

/// Position in serialized worldview used to extract the master ID
pub const MASTER_IDX: usize = 1;

//
// ──────────────────────────────────────────────────────────────
//   3. TIMING & TIMEOUTS & INTERVALS
// ──────────────────────────────────────────────────────────────
//

/// Timeout duration for TCP connections
pub const TCP_TIMEOUT: u64 = 5000;

/// Period used for TCP message transmission
pub const TCP_PER_U64: u64 = 10;

/// Time interval between UDP broadcast transmissions
pub const UDP_PERIOD: Duration = Duration::from_millis(5);

/// General polling frequency (10 ms)
pub const POLL_PERIOD: Duration = Duration::from_millis(10);

/// Size of UDP receive buffer in bytes
pub const UDP_BUFFER: usize = u16::MAX as usize;

/// Timeout for individual elevator tasks before being marked failed
pub const TASK_TIMEOUT: u64 = 100;

/// Delay between slave retransmissions
pub const SLAVE_TIMEOUT: Duration = Duration::from_millis(100);

/// Time backup waits before taking over as master
pub const MASTER_TIMEOUT: Duration = Duration::from_millis(50000);

/// Timeout for backup mode takeover (same as above)
pub const BACKUP_TIMEOUT: Duration = Duration::from_millis(50000);

/// How often the backup client receives worldview updates
pub const BACKUP_SEND_INTERVAL: Duration = Duration::from_millis(500);

/// How often the backup refreshes worldview locally
pub const BACKUP_WORLDVIEW_REFRESH_INTERVAL: Duration = Duration::from_millis(500);

/// Time between retry attempts to reconnect to master
pub const BACKUP_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Number of retries before promoting backup to master
pub const BACKUP_FAILOVER_THRESHOLD: u32 = 50;

//
// ──────────────────────────────────────────────────────────────
//   4. PID REDUNDANCY CONTROL
// ──────────────────────────────────────────────────────────────
//

/// Proportional gain for PID redundancy controller
pub const REDUNDANCY_PID_KP: f64 = 60.0;

/// Integral gain for PID redundancy controller
pub const REDUNDANCY_PID_KI: f64 = 14.05;

/// Derivative gain for PID redundancy controller
pub const REDUNDANCY_PID_KD: f64 = 1.01;

/// Minimum clamp value for integral term (anti-windup)
pub const PID_INTEGRAL_MIN: f64 = -20.0;

/// Maximum clamp value for integral term (anti-windup)
pub const PID_INTEGRAL_MAX: f64 = 20.0;

/// Minimum redundancy factor (always send at least 1 packet)
pub const REDUNDANCY_MIN: f64 = 1.0;

/// Maximum redundancy factor (prevent network overload)
pub const REDUNDANCY_MAX: f64 = 300.0;

//
// ──────────────────────────────────────────────────────────────
//   5. LOGGING CONFIGURATION
// ──────────────────────────────────────────────────────────────
//

/// Enable/disable printing of worldview updates
pub static PRINT_WV_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));

/// Enable/disable printing of errors
pub static PRINT_ERR_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));

/// Enable/disable printing of warnings
pub static PRINT_WARN_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));

/// Enable/disable printing of success messages
pub static PRINT_OK_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));

/// Enable/disable printing of general info
pub static PRINT_INFO_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));

/// Enable/disable miscellaneous debug prints
pub static PRINT_ELSE_ON: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));


