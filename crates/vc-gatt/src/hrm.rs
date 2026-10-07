#![allow(unused)]
use crate::{DecodeError, EncodeError, MAX_PAYLOAD, cursor::Cursor, cursor::Writer};
use heapless::Vec;
// this file is for decoding the actual incoming bytes and trasforming it into a type struct

//the actual structure of the data thats coming to us is
// ┌─────────────────┬─────────────────┬──────────────────┬──────────────────────┐
//  │ FLAGS           │ HEART RATE      │ ENERGY BURNED    │ R-R INTERVALS        │
//  │ (Mandatory)     │ (Mandatory)     │ (Optional)       │ (Optional)           │
//  │ 1 byte          │ 1 or 2 bytes    │ 0 or 2 bytes     │ 0 to N bytes         │
//  └─────────────────┴─────────────────┴──────────────────┴──────────────────────┘
// Most RR values that fit in 20 bytes:
// flags + u8 HR + 9 x u16.
pub const RR_CAP: usize = 9;
const FLAG_HR_U16: u8 = 1 << 0; //checks if the hr is u8 or u16
const CONTACT_MASK: u8 = 0b0000_0110; // Bits 1-2
const CONTACT_NOT_DETECTED: u8 = 0b0000_0100; // (Value: 4)
const CONTACT_DETECTED: u8 = 0b0000_0110; // (Value: 6)
const FLAG_ENERGY: u8 = 1 << 3; // Bit 3 (Value: 8)
const FLAG_RR: u8 = 1 << 4; // Bit 4 (Value: 16)

// Bit:   7     6     5     4     3     2     1     0
//      +-----+-----+-----+-----+-----+-----+-----+-----+
//      |  0  |  0  |  0  | RR  | EN  |   CONTACT   | HR  |
//      +-----+-----+-----+-----+-----+-----+-----+-----+
//                          │     │      │     │      │
//       RR Intervals ──────┘     │      │     │      │
//       1 = Present              │      │     │      │
//       0 = Missing              │      │     │      │
//                                │      │     │      │
//       Energy Expended ─────────┘      │     │      │
//       1 = Present                     │     │      │
//       0 = Missing                     │     │      │
//                                       │     │      │
//       Sensor Contact (Bits 1 & 2) ────┴─────┘      │
//       00 or 01 = Feature not supported             │
//       10 (Value 4) = Sensor off body               │
//       11 (Value 6) = Sensor on skin                │
//                                                    │
//       Heart Rate Format ───────────────────────────┘
//       1 = 16-bit (For heart rates > 255 bpm)
//       0 = 8-bit (For normal heart rates 0-255)

//enum for contact because we exactly know what type od contact exists
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contact {
    /// Bits 1-2 = 0 or 1.
    NotSupported,
    /// Bits 1-2 = 2: supported, strap not on skin.
    NotDetected,
    /// Bits 1-2 = 3.
    Detected,
}

//now the main data type the incoming packets store into
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hrm {
    pub bpm: u16,
    pub contact: Contact,
    pub energy_kj: Option<u16>,
    pub rr_1024: Vec<u16, RR_CAP>, //here heapless vec means we have to define arr size at upfront not dynamically allocated
}

// / Decodes any payload without panicking.
// /
// / Reserved flag bits are ignored.
// /
// / Packets from a larger MTU with more than `RR_CAP` RR values keep the
// / oldest `RR_CAP`; the coach does not use RR yet
pub fn decode_hrm(b: &[u8]) -> Result<Hrm, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }
    let mut c = Cursor::new(b);
    let flags = c.u8()?;
    let bpm = if flags & FLAG_HR_U16 != 0 {
        c.u16_le()?
    } else {
        u16::from(c.u8()?)
    };
    let contact = match flags & CONTACT_MASK {
        CONTACT_NOT_DETECTED => Contact::NotDetected,
        CONTACT_DETECTED => Contact::Detected,
        _ => Contact::NotSupported,
    };
    let energy_kj = if flags & FLAG_ENERGY != 0 {
        Some(c.u16_le()?)
    } else {
        None
    };
    let mut rr_1024 = Vec::new();
    if flags & FLAG_RR != 0 {
        // RR values are u16s to the end; an odd leftover byte is malformed.
        if !c.remaining().is_multiple_of(2) {
            return Err(DecodeError::Truncated {
                need: b.len().saturating_add(1),
                have: b.len(),
            });
        }
        while c.remaining() >= 2 {
            let rr = c.u16_le()?;
            if rr_1024.push(rr).is_err() {
                break;
            }
        }
    }
    Ok(Hrm {
        bpm,
        contact,
        energy_kj,
        rr_1024,
    })
}

/// Encodes the shortest valid packet: u8 heart rate when it fits.
/// the gerral flow is here the max bt data we can send is 20 bytes, so 1 byte flag, second hr, and remaining energy and rr intervals
pub fn encode_hrm(m: &Hrm, out: &mut [u8; MAX_PAYLOAD]) -> Result<usize, EncodeError> {
    let narrow = u8::try_from(m.bpm).ok();

    let mut flags = match m.contact {
        Contact::NotSupported => 0,
        Contact::NotDetected => CONTACT_NOT_DETECTED,
        Contact::Detected => CONTACT_DETECTED,
    };
    if narrow.is_none() {
        flags |= FLAG_HR_U16;
    }
    if m.energy_kj.is_some() {
        flags |= FLAG_ENERGY;
    }
    if !m.rr_1024.is_empty() {
        flags |= FLAG_RR;
    }

    let mut w = Writer::new(out);
    w.u8(flags)?;
    match narrow {
        Some(b) => w.u8(b)?,
        None => w.u16_le(m.bpm)?,
    }
    if let Some(e) = m.energy_kj {
        w.u16_le(e)?;
    }
    for rr in m.rr_1024.iter() {
        w.u16_le(*rr)?;
    }
    Ok(w.written())
}
