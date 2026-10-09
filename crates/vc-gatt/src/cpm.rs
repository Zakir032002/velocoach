#![allow(unused)]
//
// A power meter tell's you how hard your muscles push the pedel right now measured in watts
// ================================================================================
//           THE 16-BIT CPM FLAGS CONTROL PANEL (Bytes 0 & 1 on the Wire)
// ================================================================================
//  Bit:   15 14 13  12  11  10   9   8   7   6   5   4   3   2   1   0
//        ┌──┬──┬──┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐----
//  Name: │  RESERVED  │OFF│NRG│BDS│TDS│ANG│TRQ│FRC│CRK│WHL│SRC│TRQ│REF│BAL│
//        └──┴──┴──┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘----
//          (Ignore)     ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲
//                       │   │   │   │   │   │   │   │   │   │   │   │   │
//                       │   │   │   │   │   │   │   │   │   │   │   │   └── Bit 0: Balance present (1 byte)
//                       │   │   │   │   │   │   │   │   │   │   │   └────── Bit 1: Balance is LEFT foot (0 bytes!) ghost bit, it doest add any bytes to the packet
//                       │   │   │   │   │   │   │   │   │   │   └────────── Bit 2: Torque present (2 bytes)
//                       │   │   │   │   │   │   │   │   │   └────────────── Bit 3: Torque is from CRANK (0 bytes!) ghost bit
//                       │   │   │   │   │   │   │   │   └────────────────── Bit 4: Wheel data present (6 bytes)
//                       │   │   │   │   │   │   │   └────────────────────── Bit 5: Crank data present (4 bytes)
//                       │   │   │   │   │   │   └────────────────────────── Bit 6: Extreme Force (4 bytes raw)
//                       │   │   │   │   │   └────────────────────────────── Bit 7: Extreme Torque (4 bytes raw)
//                       │   │   │   │   └────────────────────────────────── Bit 8: Extreme Angles (3 bytes raw)
//                       │   │   │   └────────────────────────────────────── Bit 9: Top Dead Spot angle (2 bytes)
//                       │   │   └────────────────────────────────────────── Bit 10: Bottom Dead Spot angle (2 bytes)
//                       │   └────────────────────────────────────────────── Bit 11: Accumulated Energy (2 bytes)
//                       └────────────────────────────────────────────────── Bit 12: Offset compensation done (0 bytes!)

//  [ WIRE BYTES ] ────────► decode_cpm() ────────► struct Cpm
//                                                    ├── power_w: i16 (Watts)
//                                                    ├── pedal_balance: Option<u8>
//                                                    ├── balance_ref_left: bool
//                                                    ├── wheel: Option<CpmWheel>
//                                                    │    └── time_2048: u16
//                                                    ├── crank: Option<CrankData>
//                                                    │    └── time_1024: u16
//                                                    ├── raw arrays ([u8; 4], [u8; 3])
//                                                    └── energy_kj: Option<u16>
//                                                          │
//  [ WIRE BYTES ] ◄──────── encode_cpm() ◄─────────────────┘

use crate::{
    MAX_PAYLOAD,
    csc::CrankData,
    cursor::{Cursor, Writer},
    error::{DecodeError, EncodeError},
};

pub const CPS_UUID16: u16 = 0x1818;
pub const CPM_UUID16: u16 = 0x2A63;

pub const CPM_WHEEL_TICKS_PER_S: u32 = 2048;

// Presence bits: each adds a field, in exactly this order.
const PEDAL_BALANCE: u16 = 1 << 0;
const ACC_TORQUE: u16 = 1 << 2;
const WHEEL_REV: u16 = 1 << 4;
const CRANK_REV: u16 = 1 << 5;
const EXTREME_FORCE: u16 = 1 << 6;
const EXTREME_TORQUE: u16 = 1 << 7;
const EXTREME_ANGLES: u16 = 1 << 8;
const TOP_DEAD_SPOT: u16 = 1 << 9;
const BOTTOM_DEAD_SPOT: u16 = 1 << 10;
const ACC_ENERGY: u16 = 1 << 11;

// Descriptive bits: they add NO bytes.
// Treating them as fields shifts every later value.
const BALANCE_REF_LEFT: u16 = 1 << 1;
const TORQUE_SOURCE_CRANK: u16 = 1 << 3;
const OFFSET_COMPENSATION: u16 = 1 << 12;

// ================================================================================
//                        CPM CODEC PIPELINE (0x2A63)
// ================================================================================

//  [ ANDROID (vc-sim) ]                                  [ iPHONE (vc-core) ]
//    Rider Simulation                                       Coach Decision Engine
//           │                                                         ▲
//           ▼                                                         │
//    struct Cpm { ... }                                     struct Cpm { ... }
//           │                                                         ▲
//           ▼                                                         │
//    encode_cpm()                                           decode_cpm()
//    (Writer: packs fields)                                 (Cursor: unpacks fields)
//           │                                                         ▲
//           ▼                                                         │
//    [u8; 4..20] ────────────► [ BLE 2.4 GHz LINK ] ─────────────► &[u8]
//    Raw BLE Packet             Service: CPS (0x1818)               Raw Bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpmWheel {
    pub revs: u32,
    pub time_2048: u16,
}
//Cpm: The fully unpacked representation of a power packet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cpm {
    pub power_w: i16, // always in every packet
    pub pedal_balance_half_pct: Option<u8>,
    pub balance_ref_left: bool,
    pub acc_torque_32nm: Option<u16>,
    pub torque_source_crank: bool,
    pub wheel: Option<CpmWheel>,
    pub crank: Option<CrankData>,
    pub extreme_force_raw: Option<[u8; 4]>,
    pub extreme_torque_raw: Option<[u8; 4]>,
    pub extreme_angles_raw: Option<[u8; 3]>,
    pub top_dead_spot_deg: Option<u16>,
    pub bottom_dead_spot_deg: Option<u16>,
    pub acc_energy_kj: Option<u16>,
    pub offset_compensation: bool,
}
// ================================================================================
//                     ANATOMY OF struct Cpm (0x2A63)
// ================================================================================
//  struct Cpm
//   ├── MANDATORY READINGS:
//   │    └── power_w: i16                 (Always in every packet: Watts)
//   │
//   ├── PEDAL DYNAMICS (Optional):
//   │    ├── pedal_balance_half_pct: Option<u8>  (Left/Right % in 1/2 %)
//   │    └── balance_ref_left: bool              (Flag: is the % Left or Right?)
//   │
//   ├── TORQUE MEASUREMENT (Optional):
//   │    ├── acc_torque_32nm: Option<u16>        (Rotational force: 1/32 N·m)
//   │    └── torque_source_crank: bool           (Flag: measured at crank or wheel?)
//   │
//   ├── MOTION TRACKERS (Optional):
//   │    ├── wheel: Option<CpmWheel>             (Revolutions + time in 1/2048 s)[cite: 125, 126]
//   │    └── crank: Option<CrankData>            (Revolutions + time in 1/1024 s)[cite: 125, 126]
//   │
//   ├── RAW PEDAL FORCES (Optional):
//   │    ├── extreme_force_raw: Option<[u8; 4]>  (Peak/valley pedal force)[cite: 125, 126]
//   │    ├── extreme_torque_raw: Option<[u8; 4]> (Peak/valley rotational torque)[cite: 125, 126]
//   │    ├── extreme_angles_raw: Option<[u8; 3]> (Angular range of peaks)[cite: 125, 126]
//   │    ├── top_dead_spot_deg: Option<u16>      (Pedal angle at 12 o'clock)[cite: 125, 126]
//   │    └── bottom_dead_spot_deg: Option<u16>   (Pedal angle at 6 o'clock)[cite: 125, 126]
//   │
//   └── TOTAL ENERGY & STATUS (Optional):
//        ├── acc_energy_kj: Option<u16>          (Total work done in kJ)[cite: 125, 126]
//        └── offset_compensation: bool           (Flag: zero-reset applied)[cite: 125, 126]
// ================================================================================

pub fn decode_cpm(b: &[u8]) -> Result<Cpm, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }

    let mut c = Cursor::new(b);

    let flags = c.u16_le()?;
    let power_w = c.i16_le()?;

    let has = |bit: u16| flags & bit != 0;

    // Read strictly in bit order; every branch is a real field.
    let pedal_balance_half_pct = if has(PEDAL_BALANCE) {
        Some(c.u8()?)
    } else {
        None
    };

    let acc_torque_32nm = if has(ACC_TORQUE) {
        Some(c.u16_le()?)
    } else {
        None
    };

    let wheel = if has(WHEEL_REV) {
        let revs = c.u32_le()?;
        let time_2048 = c.u16_le()?;

        Some(CpmWheel { revs, time_2048 })
    } else {
        None
    };

    let crank = if has(CRANK_REV) {
        let revs = c.u16_le()?;
        let time_1024 = c.u16_le()?;

        Some(CrankData { revs, time_1024 })
    } else {
        None
    };

    let extreme_force_raw = if has(EXTREME_FORCE) {
        Some(c.take::<4>()?)
    } else {
        None
    };

    let extreme_torque_raw = if has(EXTREME_TORQUE) {
        Some(c.take::<4>()?)
    } else {
        None
    };

    let extreme_angles_raw = if has(EXTREME_ANGLES) {
        Some(c.take::<3>()?)
    } else {
        None
    };

    let top_dead_spot_deg = if has(TOP_DEAD_SPOT) {
        Some(c.u16_le()?)
    } else {
        None
    };

    let bottom_dead_spot_deg = if has(BOTTOM_DEAD_SPOT) {
        Some(c.u16_le()?)
    } else {
        None
    };

    let acc_energy_kj = if has(ACC_ENERGY) {
        Some(c.u16_le()?)
    } else {
        None
    };

    Ok(Cpm {
        power_w,
        pedal_balance_half_pct,
        balance_ref_left: has(BALANCE_REF_LEFT),
        acc_torque_32nm,
        torque_source_crank: has(TORQUE_SOURCE_CRANK),
        wheel,
        crank,
        extreme_force_raw,
        extreme_torque_raw,
        extreme_angles_raw,
        top_dead_spot_deg,
        bottom_dead_spot_deg,
        acc_energy_kj,
        offset_compensation: has(OFFSET_COMPENSATION),
    })
}

pub fn encode_cpm(m: &Cpm, out: &mut [u8; MAX_PAYLOAD]) -> Result<usize, EncodeError> {
    let bits = [
        (m.pedal_balance_half_pct.is_some(), PEDAL_BALANCE),
        (m.balance_ref_left, BALANCE_REF_LEFT),
        (m.acc_torque_32nm.is_some(), ACC_TORQUE),
        (m.torque_source_crank, TORQUE_SOURCE_CRANK),
        (m.wheel.is_some(), WHEEL_REV),
        (m.crank.is_some(), CRANK_REV),
        (m.extreme_force_raw.is_some(), EXTREME_FORCE),
        (m.extreme_torque_raw.is_some(), EXTREME_TORQUE),
        (m.extreme_angles_raw.is_some(), EXTREME_ANGLES),
        (m.top_dead_spot_deg.is_some(), TOP_DEAD_SPOT),
        (m.bottom_dead_spot_deg.is_some(), BOTTOM_DEAD_SPOT),
        (m.acc_energy_kj.is_some(), ACC_ENERGY),
        (m.offset_compensation, OFFSET_COMPENSATION),
    ];

    let flags = bits
        .iter()
        .fold(0u16, |f, &(on, bit)| if on { f | bit } else { f });

    let mut w = Writer::new(out);

    w.u16_le(flags)?;
    w.i16_le(m.power_w)?;

    if let Some(v) = m.pedal_balance_half_pct {
        w.u8(v)?;
    }

    if let Some(v) = m.acc_torque_32nm {
        w.u16_le(v)?;
    }

    if let Some(wh) = m.wheel {
        w.u32_le(wh.revs)?;
        w.u16_le(wh.time_2048)?;
    }

    if let Some(cr) = m.crank {
        w.u16_le(cr.revs)?;
        w.u16_le(cr.time_1024)?;
    }

    if let Some(v) = m.extreme_force_raw {
        w.bytes(&v)?;
    }

    if let Some(v) = m.extreme_torque_raw {
        w.bytes(&v)?;
    }

    if let Some(v) = m.extreme_angles_raw {
        w.bytes(&v)?;
    }

    if let Some(v) = m.top_dead_spot_deg {
        w.u16_le(v)?;
    }

    if let Some(v) = m.bottom_dead_spot_deg {
        w.u16_le(v)?;
    }

    if let Some(v) = m.acc_energy_kj {
        w.u16_le(v)?;
    }

    Ok(w.written())
}
