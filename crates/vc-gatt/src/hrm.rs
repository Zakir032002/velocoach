#![allow(unused)]
use core::default;

use crate::{
    PAYLOAD_MAX,              //20, the biggest packet Bluetooth allows by default.
    cursor::{Cursor, Writer}, //A safe reader. It reads bytes one at a time and returns an error if you run out.A safe writer into a fixed 20-byte box.
    error::DecodeError,
};

//this file does two jobs
//  DECODE:   bytes ───────────────► nice struct   (read the letter)
//            16 9E 00 04              Hrm { bpm: 158, ... }

//  ENCODE:   nice struct ──────────► bytes        (write the letter)
//            Hrm { bpm: 158, ... }    16 9E 00 04

pub const MAX_RR: usize = 9; //9 beacuse its the max bytes rr can have
const FMT_U16: u8 = 1 << 0; // checks if the hr is a u8 if 0 and u16 if 1
const ENERGY: u8 = 1 << 3; // checks if the bluetooth packet has energy or not,,energy is calories burmned or not
const RR: u8 = 1 << 4; // checks if rr is present or not

// THE FLAGS BYTE (8 switches)

//    Bit:      7     6     5     4     3     2     1     0
//           +-----+-----+-----+-----+-----+-----+-----+-----+
//           |  0  |  0  |  0  |  1  |  0  |  1  |  1  |  0  |  = 0x16
//           +-----+-----+-----+-----+-----+-----+-----+-----+
//                                │     │     \_______/     │
//                                │     │         │         └── FMT_U16 (1 << 0)[cite: 95]
//                                │     │         │             0 = u8 BPM
//                                │     │         │             1 = u16 BPM[cite: 29, 95]
//                                │     │         │
//                                │     │         └──────────── Contact (Bits 1-2)[cite: 29, 95]
//                                │     │                       0 or 1 = Not Supported[cite: 29, 95]
//                                │     │                       2 = No Contact[cite: 29, 96]
//                                │     │                       3 = Contact OK[cite: 29, 96]
//                                │     │
//                                │     └────────────────────── ENERGY (1 << 3)[cite: 29, 95]
//                                │                             0 = No energy data[cite: 29, 95]
//                                │                             1 = u16 kJ follows[cite: 29, 95]
//                                │
//                                └──────────────────────────── RR (1 << 4)[cite: 29, 95]
//                                                              0 = No RR list[cite: 29, 95]
//                                                              1 = RR list follows[cite: 29, 95]

//so now reamining one is Contact,,for this we have 4 states which are 0,1,2,3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Contact {
    #[default]
    NotSupported,
    NoContact,
    Contact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hrm {
    pub bpm: u16,               // beats per minute ,, and u16 bcz it can exceed 255
    pub contact: Contact,       //
    pub energy_kj: Option<u16>, // here this is optional so we have used option
    pub rr_len: u8,
    pub rr: [u16; MAX_RR],
}

//now from the struct the only thing thats not fixed is rr,,i mean why do we even need to read the leftovers of rr
impl Hrm {
    pub fn rr(&self) -> &[u16] {
        self.rr.get(..usize::from(self.rr_len)).unwrap_or(&[])
        //get is a safe slice getter
    }
}

pub const fn rr_capacity(wide: bool, energy: bool) -> usize {
    let fixed = 1 + if wide { 2 } else { 1 } + if energy { 2 } else { 0 };
    (PAYLOAD_MAX - fixed) / 2
}

//now lets decode it
//  b ──► empty? ──► flags ──► HR ──► contact ──► energy? ──► build ──► RR? ──► Ok
//         │           │        │                    │                    │
//        Err(Empty)  ?-fail   ?-fail              ?-fail            Err(Truncated)
pub fn decode_hrm(b: &[u8]) -> Result<Hrm, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }
    let mut c = Cursor::new(b);
    let flags = c.u8()?;
    let bpm = if flags & FMT_U16 != 0 {
        c.u16()?
    } else {
        u16::from(c.u8()?)
    };
    let contact = match (flags >> 1) & 0b11 {
        2 => Contact::NoContact,
        3 => Contact::Contact,
        _ => Contact::NotSupported,
    };
    let energy_kj = if flags & ENERGY != 0 {
        Some(c.u16()?)
    } else {
        None
    };
    let mut out = Hrm {
        bpm,
        contact,
        energy_kj,
        ..Hrm::default()
    };
    //filling out rr
    if flags & RR != 0 {
        let rem = c.remaining();

        if rem == 0 || rem % 2 == 1 {
            return Err(DecodeError::Truncated {
                need: 2,
                have: (rem % 2) as u8,
            });
        }

        for slot in out.rr.iter_mut() {
            if c.remaining() < 2 {
                break;
            }

            *slot = c.u16()?;
            out.rr_len += 1;
        }
    }

    Ok(out)
}

pub fn encode_hrm(m: &Hrm, out: &mut [u8; PAYLOAD_MAX]) -> usize {
    let wide = m.bpm > u16::from(u8::MAX); // ensure 2 if heart rate exceeds 255
    let energy = m.energy_kj.is_some(); //if energy is there or that feature is not there
    let rr = m.rr(); // it is the slice of valid rr intervals
    let n_rr = rr.len().min(rr_capacity(wide, energy)); // it is to check how many 2-byte intervals can fit into 20byte payload
    let contact_bits: u8 = match m.contact {
        Contact::NotSupported => 0,
        Contact::Contact => 3,
        Contact::NoContact => 2,
    };
    //now set the flag bits
    let mut flags = contact_bits << 1;
    if wide {
        flags |= FMT_U16;
    }
    if energy {
        flags |= ENERGY;
    }
    if n_rr > 0 {
        flags |= RR;
    }
    //     FINAL ASSEMBLED FLAGS BYTE
    //    Bit:     7   6   5   4   3   2   1   0
    //           +---+---+---+---+---+---+---+---+
    //           | 0 | 0 | 0 | 1 | 0 | 1 | 1 | 0 | = 0x16
    //           +---+---+---+---+---+---+---+---+
    //                         │   │   \___/   │
    //                         │   │     │     └── Bit 0: FMT_U16 (0 = 8-bit BPM)
    //                         │   │     └──────── Bits 1-2: Contact (3 = Contact OK)
    //                         │   └────────────── Bit 3: ENERGY (0 = Not present)
    //                         └────────────────── Bit 4: RR (1 = RR intervals follow)
    let mut w = Writer::new(out);
    w.u8(flags);
    if wide {
        w.u16(m.bpm);
    } else {
        w.u8(u8::try_from(m.bpm).unwrap_or(u8::MAX));
    }

    if let Some(e) = m.energy_kj {
        w.u16(e);
    }
    for v in rr.iter().take(n_rr) {
        w.u16(*v);
    }
    w.written()
}
