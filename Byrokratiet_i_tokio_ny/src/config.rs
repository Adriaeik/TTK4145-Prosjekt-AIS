//! Globale verdier osv

pub static PN_PORT: u16 = 6969; // Port for TCP mellom mastere
pub static BCU_PORT: u16 = 8082; // Port for TCP mellom lokal master/backup
pub static DUMMY_PORT: u16 = 42069; // Port fro sending / mottak av UDP broadcast

pub static BC_LISTEN_ADDR: &str = "0.0.0.0";
pub static BC_ADDR: &str = "255.255.255.255";

pub static ERROR_ID: u8 = 0;