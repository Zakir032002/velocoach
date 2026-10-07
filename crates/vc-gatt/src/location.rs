use crate::MAX_PAYLOAD;
use crate::cursor::{Cursor, Writer};
use crate::error::{DecodeError, EncodeError};

pub const SENSOR_LOCATION_UUID16: u16 = 0x2A5D;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorLocation {
    Other,
    TopOfShoe,
    InShoe,
    Hip,
    FrontWheel,
    LeftCrank,
    RightCrank,
    LeftPedal,
    RightPedal,
    FrontHub,
    RearDropout,
    Chainstay,
    RearWheel,
    RearHub,
    Chest,
    Spider,
    ChainRing,
    /// 17-255, reserved for future use. Kept so encoding is lossless.
    Reserved(u8),
}

impl SensorLocation {
    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Other,
            1 => Self::TopOfShoe,
            2 => Self::InShoe,
            3 => Self::Hip,
            4 => Self::FrontWheel,
            5 => Self::LeftCrank,
            6 => Self::RightCrank,
            7 => Self::LeftPedal,
            8 => Self::RightPedal,
            9 => Self::FrontHub,
            10 => Self::RearDropout,
            11 => Self::Chainstay,
            12 => Self::RearWheel,
            13 => Self::RearHub,
            14 => Self::Chest,
            15 => Self::Spider,
            16 => Self::ChainRing,
            other => Self::Reserved(other),
        }
    }

    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Other => 0,
            Self::TopOfShoe => 1,
            Self::InShoe => 2,
            Self::Hip => 3,
            Self::FrontWheel => 4,
            Self::LeftCrank => 5,
            Self::RightCrank => 6,
            Self::LeftPedal => 7,
            Self::RightPedal => 8,
            Self::FrontHub => 9,
            Self::RearDropout => 10,
            Self::Chainstay => 11,
            Self::RearWheel => 12,
            Self::RearHub => 13,
            Self::Chest => 14,
            Self::Spider => 15,
            Self::ChainRing => 16,
            Self::Reserved(v) => v,
        }
    }
}

/// Trailing bytes are ignored.
pub fn decode_sensor_location(b: &[u8]) -> Result<SensorLocation, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }
    let mut c = Cursor::new(b);
    Ok(SensorLocation::from_u8(c.u8()?))
}

pub fn encode_sensor_location(
    loc: SensorLocation,
    out: &mut [u8; MAX_PAYLOAD],
) -> Result<usize, EncodeError> {
    let mut w = Writer::new(out);
    w.u8(loc.to_u8())?;
    Ok(w.written())
}
