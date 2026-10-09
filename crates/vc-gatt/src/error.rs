use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Zero-length payload.
    Empty,

    /// The flags promised a field the payload is too short to hold.
    Truncated { need: usize, have: usize },

    // - TooLong → VC-SIM requires exact-length packets.
    // - BadVersion → VC-SIM is versioned and currently accepts only version 1.
    // - BadOpcode → rejects unknown/wrong-channel operations.
    // - BadValue → rejects invalid enum bytes such as an invalid Channel or FaultKind.
    /// A strict VC-SIM message had bytes left over.
    TooLong { expected: usize, have: usize },

    /// VC-SIM version byte was not 1.
    BadVersion(u8),

    /// Unknown VC-SIM opcode, or a TELEM opcode sent on CTRL (and vice versa).
    BadOpcode(u8),

    /// A byte that must be one of a few known values was not.
    BadValue { field: &'static str, value: u8 },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty payload"),
            Self::Truncated { need, have } => {
                write!(f, "truncated: need {need} bytes, have {have}")
            }
            Self::TooLong { expected, have } => {
                write!(f, "too long: expected {expected} bytes, have {have}")
            }
            Self::BadVersion(v) => {
                write!(f, "unsupported VC-SIM version {v}")
            }
            Self::BadOpcode(op) => {
                write!(f, "unknown opcode 0x{op:02X}")
            }
            Self::BadValue { field, value } => {
                write!(f, "invalid {field}: {value}")
            }
        }
    }
}

impl core::error::Error for DecodeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    BufferTooSmall { need: usize, have: usize },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall { need, have } => {
                write!(f, "buffer too small: need {need} bytes, have {have}")
            }
        }
    }
}

impl core::error::Error for EncodeError {}
