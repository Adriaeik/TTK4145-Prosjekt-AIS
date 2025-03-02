//! Globale verdier osv
use std::net::Ipv4Addr;
use std::time::Duration;

pub static NETWORK_PREFIX: &str = "10.100.23"; //Hardkoda subnet må vel vere greit. DEt er jo ekstra sikkerheit

pub static PN_PORT: u16 = u16::MAX; // Port for TCP mellom mastere
pub static BCU_PORT: u16 = 50000; // Port for TCP mellom lokal master/backup
pub static DUMMY_PORT: u16 = 42069; // Port fro sending / mottak av UDP broadcast

pub static BC_LISTEN_ADDR: &str = "0.0.0.0";
pub static BC_ADDR: &str = "255.255.255.255";
pub static OFFLINE_IP: Ipv4Addr = Ipv4Addr::new(69, 69, 69, 69);

pub static LOCAL_ELEV_IP: &str = "localhost:15657";
pub const DEFAULT_NUM_FLOORS: u8 = 4;
pub const ELEV_POLL: Duration = Duration::from_millis(25);

pub const ERROR_ID: u8 = 255;

pub const MASTER_IDX: usize = 1;
pub const KEY_STR: &str = "Gruppe 25";

pub const TCP_TIMEOUT: u64 = 5000; // i millisekunder
pub const TCP_PER_U64: u64 = 10; // i millisekunder
pub const UDP_PERIOD: Duration = Duration::from_millis(TCP_PER_U64*5);
pub const TCP_PERIOD: Duration = Duration::from_millis(TCP_PER_U64);

pub const PRINT_WV_ON: bool = !false;
