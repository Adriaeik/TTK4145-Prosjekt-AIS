use crate::world_view::world_view::{WorldView, ElevatorContainer, serialize_worldview};
use crate::utils::{print_err, ip2id};
use local_ip_address::local_ip;

pub fn initialize_worldview() -> (Vec<u8>, u8) {
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

    let self_id = ip2id(ip);
    elev_container.elevator_id = self_id;
    worldview.master_id = self_id;
    worldview.add_elev(elev_container);

    (serialize_worldview(&worldview), self_id)
}
