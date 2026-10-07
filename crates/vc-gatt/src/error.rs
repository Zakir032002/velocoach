use core::fmt;

/// Why a payload could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Zero-length payload.
    Empty,

    /// The flags promised a field the payload is too short to hold.
    Truncated { need: usize, have: usize },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty payload"),
            Self::Truncated { need, have } => {
                write!(f, "truncated: need {need} bytes, have {have}")
            }
        }
    }
}

impl core::error::Error for DecodeError {}

/// Why a struct could not be encoded into the provided buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// `need` is where the first write that did not fit would have ended.
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
