use crate::{cursor::Cursor, error::DecodeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensorLocation(pub u8);

impl SensorLocation {
    pub const OTHER: Self = Self(0);
    pub const LEFT_CRANK: Self = Self(5);
    pub const RIGHT_CRANK: Self = Self(6);
    pub const CHEST: Self = Self(14);

    pub const fn is_reserved(self) -> bool {
        self.0 > 16
    }
}

pub fn decode_sensor_location(b: &[u8]) -> Result<SensorLocation, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }

    Ok(SensorLocation(Cursor::new(b).u8()?))
}
