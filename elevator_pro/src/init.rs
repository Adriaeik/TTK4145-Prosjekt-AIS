
use std::sync::atomic::Ordering;

use crate::world_view::world_view::{WorldView, ElevatorContainer, serialize_worldview};
use crate::utils::{self, ip2id, print_err};
use local_ip_address::local_ip;

pub fn initialize_worldview() -> Vec<u8> {
    let mut worldview = WorldView::default();
    let mut elev_container = ElevatorContainer::default();

    // Hent lokal IP-adresse
    let ip = match local_ip() {
        Ok(ip) => ip,
        Err(e) => {
            print_err(format!("Fant ikke IP i starten av main: {}", e));
            panic!();
        }
    };

    utils::SELF_ID.store(ip2id(ip), Ordering::SeqCst); //🐌 Seigast
    elev_container.elevator_id = utils::SELF_ID.load(Ordering::SeqCst);
    worldview.master_id = utils::SELF_ID.load(Ordering::SeqCst);
    worldview.add_elev(elev_container);

    serialize_worldview(&worldview)
}
