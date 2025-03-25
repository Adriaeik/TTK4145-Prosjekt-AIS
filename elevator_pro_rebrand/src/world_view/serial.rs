//! Serialization and Deserialization for [WorldView] and [ElevatorContainer]

use crate::world_view::{WorldView, ElevatorContainer};
use crate::print;


/// Serializes a `WorldView` into a binary format.
///
/// Uses `bincode` for efficient serialization.
/// If serialization fails, the function logs the error and panics.
///
/// ## Parameters
/// - `worldview`: A reference to the `WorldView` to be serialized.
///
/// ## Returns
/// - A `Vec<u8>` containing the serialized data.
pub fn serialize_worldview(worldview: &WorldView) -> Option<Vec<u8>>{
    let encoded = bincode::serialize(worldview);
    match encoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return Some(serialized_data);
        }
        Err(e) => {
            println!("{:?}", worldview);
            print::err(format!("Serialization failed: {} (world_view.rs, serialize_worldview())", e));
            return None;
        }
    }
}

/// Deserializes a `WorldView` from a binary format.
///
/// Uses `bincode` for deserialization.
/// If deserialization fails, the function logs the error and panics.
///
/// ## Parameters
/// - `data`: A byte slice (`&[u8]`) containing the serialized `WorldView`.
///
/// ## Returns
/// - A `WorldView` instance reconstructed from the binary data.
pub fn deserialize_worldview(data: &[u8]) -> Option<WorldView> {
    let decoded = bincode::deserialize(data);


    match decoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return serialized_data;
        }
        Err(e) => {
            print::err(format!("Serialization failed: {} (world_view.rs, deserialize_worldview())", e));
            return None;
        }
    }
}

/// Serializes an `ElevatorContainer` into a binary format.
///
/// Uses `bincode` for serialization.
/// If serialization fails, the function logs the error and panics.
///
/// ## Parameters
/// - `elev_container`: A reference to the `ElevatorContainer` to be serialized.
///
/// ## Returns
/// - A `Vec<u8>` containing the serialized data.
pub fn serialize_elev_container(elev_container: &ElevatorContainer) -> Option<Vec<u8>> {
    let encoded = bincode::serialize(elev_container);
    match encoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return Some(serialized_data);
        }
        Err(e) => {
            print::err(format!("Serialization failed: {} (world_view.rs, serialize_elev_container())", e));
            return None;
        }
    }
}

/// Deserializes an `ElevatorContainer` from a binary format.
///
/// Uses `bincode` for deserialization.
/// If deserialization fails, the function logs the error and panics.
///
/// ## Parameters
/// - `data`: A byte slice (`&[u8]`) containing the serialized `ElevatorContainer`.
///
/// ## Returns
/// - An `ElevatorContainer` instance reconstructed from the binary data.
pub fn deserialize_elev_container(data: &[u8]) -> Option<ElevatorContainer> {
    let decoded = bincode::deserialize(data);


    match decoded {
        Ok(serialized_data) => {
            // Deserialisere WorldView fra binært format
            return serialized_data;
        }
        Err(e) => {
            print::err(format!("Serialization failed: {} (world_view.rs, deserialize_elev_container())", e));
            return None
        }
    }
}