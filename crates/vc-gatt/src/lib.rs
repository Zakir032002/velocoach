#![cfg_attr(not(test), no_std)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
pub mod csc;
mod cursor;
pub mod error;
pub mod hrm;
pub mod location;
pub use csc::{Csc, decode_csc, encode_csc};
pub use error::{DecodeError, EncodeError};
pub use hrm::{Contact, Hrm, MAX_RR, decode_hrm, encode_hrm, rr_capacity};
pub use location::{SensorLocation, decode_sensor_location};
pub const PAYLOAD_MAX: usize = 20;

#[cfg(test)]
mod tests {
    #[test]
    fn payload_is_mtu_minus_header() {
        assert_eq!(super::PAYLOAD_MAX, 23 - 3);
    }
}
