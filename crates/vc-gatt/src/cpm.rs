#![allow(unused)]
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
//                       │   │   │   │   │   │   │   │   │   │   │   └────── Bit 1: Balance is LEFT foot (0 bytes!)
//                       │   │   │   │   │   │   │   │   │   │   └────────── Bit 2: Torque present (2 bytes)
//                       │   │   │   │   │   │   │   │   │   └────────────── Bit 3: Torque is from CRANK (0 bytes!)
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
