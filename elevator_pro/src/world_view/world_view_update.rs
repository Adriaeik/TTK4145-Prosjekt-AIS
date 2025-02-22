use crate::world_view::world_view;
use crate::utils;


pub fn join_wv(mut my_wv: Vec<u8>, master_wv: Vec<u8>) -> Vec<u8> {
    let my_wv_deserialised = world_view::deserialize_worldview(&my_wv);
    let mut master_wv_deserialised = world_view::deserialize_worldview(&master_wv);

    for heis in my_wv_deserialised.heis_spesifikke {
        // if heis.heis_id == self_id {
            master_wv_deserialised.add_elev(heis);
        // }
    }
    my_wv = world_view::serialize_worldview(&master_wv_deserialised);
    utils::print_info(format!("Oppdatert wv fra UDP: {:?}", my_wv));
    my_wv
}