//! Globale verdier osv
use std::net::Ipv4Addr;

pub static NETWORK_PREFIX: &str = "10.100.23"; //Hardkoda subnet må vel vere greit. DEt er jo ekstra sikkerheit

pub static PN_PORT: u16 = u16::MAX; // Port for TCP mellom mastere
pub static BCU_PORT: u16 = 50000; // Port for TCP mellom lokal master/backup
pub static DUMMY_PORT: u16 = 42069; // Port fro sending / mottak av UDP broadcast

pub static BC_LISTEN_ADDR: &str = "0.0.0.0";
pub static BC_ADDR: &str = "255.255.255.255";
pub static OFFLINE_IP: Ipv4Addr = Ipv4Addr::new(69, 69, 69, 69);

pub const ERROR_ID: u8 = 255;

pub const MASTER_IDX: usize = 1;
pub const KEY_STR: &str = "Gruppe 25";

pub const TCP_TIMEOUT: u64 = 5000; // i millisekunder

