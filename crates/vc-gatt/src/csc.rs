use crate::MAX_PAYLOAD;
use crate::cursor::{Cursor, Writer};
use crate::error::{DecodeError, EncodeError};

pub const CSC_SERVICE_UUID16: u16 = 0x1816;
pub const CSC_MEASUREMENT_UUID16: u16 = 0x2A5B;

const FLAG_WHEEL: u8 = 1 << 0;
const FLAG_CRANK: u8 = 1 << 1;

/// Wheel data as CSC sends it: event time in 1/1024 s.
/// CPM wheel time is 1/2048 s and gets its own type on Day 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CscWheel {
    pub revs: u32,
    pub time_1024: u16,
}

/// Crank data: 1/1024 s in both CSC and CPM, so CPM reuses this type.
/// Both fields are u16 counters that wrap; deltas use wrapping_sub (Day 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrankData {
    pub revs: u16,
    pub time_1024: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Csc {
    pub wheel: Option<CscWheel>,
    pub crank: Option<CrankData>,
}

pub fn decode_csc(b: &[u8]) -> Result<Csc, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }
    let mut c = Cursor::new(b);
    let flags = c.u8()?; // bits 2-7 reserved, ignored

    let wheel = if flags & FLAG_WHEEL != 0 {
        let revs = c.u32_le()?;
        let time_1024 = c.u16_le()?;
        Some(CscWheel { revs, time_1024 })
    } else {
        None
    };

    let crank = if flags & FLAG_CRANK != 0 {
        let revs = c.u16_le()?;
        let time_1024 = c.u16_le()?;
        Some(CrankData { revs, time_1024 })
    } else {
        None
    };

    Ok(Csc { wheel, crank })
}

pub fn encode_csc(m: &Csc, out: &mut [u8; MAX_PAYLOAD]) -> Result<usize, EncodeError> {
    let mut flags = 0;
    if m.wheel.is_some() {
        flags |= FLAG_WHEEL;
    }
    if m.crank.is_some() {
        flags |= FLAG_CRANK;
    }

    let mut w = Writer::new(out);
    w.u8(flags)?;
    if let Some(wh) = m.wheel {
        w.u32_le(wh.revs)?;
        w.u16_le(wh.time_1024)?;
    }
    if let Some(cr) = m.crank {
        w.u16_le(cr.revs)?;
        w.u16_le(cr.time_1024)?;
    }
    Ok(w.written())
}
