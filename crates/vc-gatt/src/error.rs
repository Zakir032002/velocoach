//The important idea is that bad input produces Result::Err instead of crashing the program
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Truncated { need: u8, have: u8 },
    InvalidFlags,
    InvalidValue,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    BufferTooSmall,
}

//              vc-gatt
//                 │
//       ┌─────────┴─────────┐
//       ▼                   ▼
//  DecodeError          EncodeError
//       │                   │
//  "bytes are bad"     "can't fit data"
