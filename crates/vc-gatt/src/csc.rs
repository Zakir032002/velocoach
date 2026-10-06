#![allow(unused)]
use crate::{
    PAYLOAD_MAX,
    cursor::{Cursor, Writer},
    error::DecodeError,
};
// to identify the flags byte if they are on or not
const WHEEL: u8 = 1 << 0; // 0000 0001 (Bit 0)
const CRANK: u8 = 1 << 1; // 0000 0010 (Bit 1)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Csc {
    /// (cumulative wheel revolutions, last event time in 1/1024 s)
    pub wheel: Option<(u32, u16)>,

    /// (cumulative crank revolutions, last event time in 1/1024 s)
    pub crank: Option<(u16, u16)>,
}

pub fn decode_csc(b: &[u8]) -> Result<Csc, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }
    let mut c = Cursor::new(b);
    let flags = c.u8()?;
    let wheel = if flags & WHEEL != 0 {
        Some((c.u32()?, c.u16()?))
    } else {
        None
    };
    let crank = if flags & CRANK != 0 {
        Some((c.u16()?, c.u16()?))
    } else {
        None
    };
    Ok(Csc { wheel, crank })
}

pub fn encode_csc(m: &Csc, out: &mut [u8; PAYLOAD_MAX]) -> usize {
    let mut flags = 0u8;

    if m.wheel.is_some() {
        flags |= WHEEL;
    }

    if m.crank.is_some() {
        flags |= CRANK;
    }

    let mut w = Writer::new(out);

    w.u8(flags);

    if let Some((revs, t)) = m.wheel {
        w.u32(revs);
        w.u16(t);
    }

    if let Some((revs, t)) = m.crank {
        w.u16(revs);
        w.u16(t);
    }

    w.written()
}
