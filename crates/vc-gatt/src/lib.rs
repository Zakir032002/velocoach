//! vc-gatt: Bluetooth SIG byte layouts <-> typed structs.
//! no_std, no heap, never panics on any input.

#![cfg_attr(not(test), no_std)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
#![cfg_attr(test, allow(clippy::indexing_slicing))]

pub mod cpm;
pub mod csc;
mod cursor;
pub mod error;
pub mod hrm;
pub mod location;

pub use csc::{CrankData, Csc, CscWheel, decode_csc, encode_csc};
pub use error::{DecodeError, EncodeError};
pub use heapless;
pub use hrm::{Contact, Hrm, RR_CAP, decode_hrm, encode_hrm};
pub use location::{SensorLocation, decode_sensor_location, encode_sensor_location};

/// Default ATT MTU (23) minus the 3-byte ATT header.
pub const MAX_PAYLOAD: usize = 20;
