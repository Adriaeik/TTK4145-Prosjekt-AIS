//! This module contains some help functions regarding the IP address
//! 
//! Functions
//! - [ip2id]: Generates an ID for the node based on the IP-address.
//! - [get_root_ip]: Extracts the root-ip excluding the ID of the node. 

use std::net::IpAddr;
use std::u8;
use crate::config;
use crate::print;



/// Extracts your ID based on `ip`
/// 
/// ## Example
/// ```
/// use elevatorpro::ip_help_functions::ip2id;
/// use std::net::IpAddr;
/// use std::str::FromStr;
/// 
/// let ip = IpAddr::from_str("192.168.0.1").unwrap();
/// let id = ip2id(ip);
/// 
/// assert_eq!(id, 1);
/// ```
/// 
pub fn ip2id(ip: IpAddr) -> u8 {
    let ip_str = ip.to_string();
    let mut ip_int = config::ERROR_ID;
    let id_str = ip_str.split('.')      // Split on '.'
        .nth(3)                                        // Extract the 4. element
        .and_then(|s| s.split(':')            // Split on ':' if there was a port
            .next())                                   // Only use the part before ':'
        .and_then(|s| s.parse::<u8>().ok());                 // Parse to u8

    match id_str {
        Some(value) => {
            ip_int = value;
        }
        None => {
            print::err(format!("Failed to extract ID from IP"));
        }
    }
    ip_int
}

/// Extracts the root part of an IP address (removes the last segment).
///
/// ## Example
/// ```
/// use std::net::IpAddr;
/// use std::str::FromStr;
/// use elevatorpro::ip_help_functions::get_root_ip;
///
/// let ip = IpAddr::from_str("192.168.0.1").unwrap();
/// let root_ip = get_root_ip(ip);
/// assert_eq!(root_ip, "192.168.0");
/// ```
///
/// Returns a string containing the first three segments of the IP address.
pub fn get_root_ip(ip: IpAddr) -> String {

    match ip {
        IpAddr::V4(addr) => {
            let octets = addr.octets();
            format!("{}.{}.{}", octets[0], octets[1], octets[2])
        }
        IpAddr::V6(addr) => {
            let segments = addr.segments();
            let root_segments = &segments[..segments.len() - 1]; // Remove last element
            root_segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(":")
        }
    }
}
