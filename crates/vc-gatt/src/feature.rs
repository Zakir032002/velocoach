// - Read ONLY ONCE when your phone connects.
//     - It answers: "What parts were built into this bike at the factory?"
//     - It never changes during the ride
// Before the ride starts, the iPhone asks the bike:
// iPhone: "Before you send me live numbers, let me read your factory sticker (0x2A65). What hardware was built into you?"
// Bike (feature.rs): "I have built-in Cadence (CRANK_REV) and Pedal Balance (PEDAL_BALANCE) hardware!"
#![allow(unused)]
use crate::{
    MAX_PAYLOAD,
    cursor::{Cursor, Writer},
    error::{DecodeError, EncodeError},
};
pub const CP_FEATURE_UUID16: u16 = 0x2A65;
pub const CSC_FEATURE_UUID16: u16 = 0x2A5C;
//here b32 because the bluetooth standard defines 4 bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpFeature(pub u32);

// ================================================================================
//              THE 32-SWITCH SWITCHBOARD (CpFeature / 0x2A65)
// ================================================================================
//  Bit:   ... 11  10   9   8   7   6   5   4   3   2   1   0
//            ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
//  Bulb:     │LOC│MSK│CMP│IND│NRG│DED│ANG│MAG│CRK│WHL│TRQ│BAL│
//            └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
//              ▲                                       ▲   ▲
//              │                                       │   │
//              │                                       │   └── Switch 0: Can measure Left/Right balance
//              │                                       └────── Switch 1: Can measure torque
//              └────────────────────────────────────────────── Switch 11: Sensor can be moved to other bike spots
// ================================================================================

impl CpFeature {
    pub const PEDAL_BALANCE: u32 = 1 << 0;
    pub const ACC_TORQUE: u32 = 1 << 1;
    pub const WHEEL_REV: u32 = 1 << 2;
    pub const CRANK_REV: u32 = 1 << 3;
    pub const EXTREME_MAGNITUDES: u32 = 1 << 4;
    pub const EXTREME_ANGLES: u32 = 1 << 5;
    pub const DEAD_SPOTS: u32 = 1 << 6;
    pub const ACC_ENERGY: u32 = 1 << 7;
    pub const OFFSET_COMP_INDICATOR: u32 = 1 << 8;
    pub const OFFSET_COMP: u32 = 1 << 9;
    pub const CONTENT_MASKING: u32 = 1 << 10;
    pub const MULTIPLE_LOCATIONS: u32 = 1 << 11;
    pub const SIM: Self = Self(Self::PEDAL_BALANCE | Self::CRANK_REV | Self::ACC_ENERGY);
    pub const fn supports(self, bit: u32) -> bool {
        self.0 & bit != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CscFeature(pub u16);

impl CscFeature {
    pub const WHEEL_REV: u16 = 1 << 0;
    pub const CRANK_REV: u16 = 1 << 1;
    pub const MULTIPLE_LOCATIONS: u16 = 1 << 2;

    /// Crank only: avoids the SC Control Point question entirely.
    pub const SIM: Self = Self(Self::CRANK_REV);

    pub const fn supports(self, bit: u16) -> bool {
        self.0 & bit != 0
    }
}

pub fn decode_cp_feature(b: &[u8]) -> Result<CpFeature, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }
    Ok(CpFeature(Cursor::new(b).u32_le()?))
}

pub fn encode_cp_feature(f: CpFeature, out: &mut [u8; MAX_PAYLOAD]) -> Result<usize, EncodeError> {
    let mut w = Writer::new(out);
    w.u32_le(f.0)?;
    Ok(w.written())
}

pub fn decode_csc_feature(b: &[u8]) -> Result<CscFeature, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }

    Ok(CscFeature(Cursor::new(b).u16_le()?))
}

pub fn encode_csc_feature(
    f: CscFeature,
    out: &mut [u8; MAX_PAYLOAD],
) -> Result<usize, EncodeError> {
    let mut w = Writer::new(out);
    w.u16_le(f.0)?;
    Ok(w.written())
}
