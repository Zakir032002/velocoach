#![allow(unused)]

use crate::{
    MAX_PAYLOAD,
    cursor::{Cursor, Writer},
    error::{DecodeError, EncodeError},
};
//The actual communication protocol, its a two way walkie-talkie
pub const VCSIM_VERSION: u8 = 1;

// The TELEM Opcodes (Android -> iPhone)
const OP_STAMP: u8 = 0x01; // "I sent a packet at this time"
const OP_PONG: u8 = 0x02; // "Ping reply for clock sync"
const OP_STATUS: u8 = 0x03; // "Simulator health check"

// The CTRL Opcodes (iPhone -> Android)
const OP_PING: u8 = 0x10; // "Clock sync check"
const OP_LOAD: u8 = 0x20; // "Load a workout scenario"
const OP_START: u8 = 0x21; // "Start pedaling"
const OP_STOP: u8 = 0x22; // "Stop pedaling"
const OP_FAULT: u8 = 0x30; // "Break something on purpose"
const OP_CLEAR: u8 = 0x31; // "Fix the broken sensor"
const OP_RATE: u8 = 0x40; // "Change send speed (Hz)"

//which sensor characteristic a message refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Hrm = 1, //heart rate strap
    Cpm = 2, //power meter
    Csc = 3, //speed and cadance
}

impl Channel {
    fn from_u8(value: u8, field: &'static str) -> Result<Self, DecodeError> {
        match value {
            1 => Ok(Self::Hrm),
            2 => Ok(Self::Cpm),
            3 => Ok(Self::Csc),
            value => Err(DecodeError::BadValue { field, value }),
        }
    } //the optional packs channel
    /// 0 = no specific channel (a Disconnect or HrRedline fault).
    fn opt_from_u8(value: u8, field: &'static str) -> Result<Option<Self>, DecodeError> {
        if value == 0 {
            Ok(None)
        } else {
            Self::from_u8(value, field).map(Some)
        }
    }

    fn opt_to_u8(ch: Option<Self>) -> u8 {
        ch.map_or(0, |c| c as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultKind {
    Dropout = 1,
    Garbage = 2,
    Disconnect = 3,
    Coast = 4,
    HrRedline = 5,

    /// param = the rider's true W′ in units of 100 J.
    WprimeMismatch = 6,
}

impl FaultKind {
    fn from_u8(value: u8) -> Result<Self, DecodeError> {
        match value {
            1 => Ok(Self::Dropout),
            2 => Ok(Self::Garbage),
            3 => Ok(Self::Disconnect),
            4 => Ok(Self::Coast),
            5 => Ok(Self::HrRedline),
            6 => Ok(Self::WprimeMismatch),
            value => Err(DecodeError::BadValue {
                field: "fault.kind",
                value,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub seq: u16,
    pub scenario: u8,
    pub state: u8,

    /// Bit k set = fault kind k active.
    pub faults: u16,

    pub q_drops: u16,
    pub notify_errs: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Telem {
    /// Sent right after each sensor notification: T0 on the iQOO clock.
    Stamp {
        seq: u16,
        channel: Channel,
        char_seq: u16,
        t0_ns: u64,
    },

    /// Clock-sync reply: when the iQOO received the PING (b)
    /// and sent this PONG (c).
    Pong {
        ping_id: u16,
        t_rx_ns: u64,
        t_tx_ns: u64,
    },

    Status(Status),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctrl {
    Ping {
        ping_id: u16,
        t_client_ns: u64,
    },

    Load {
        scenario: u8,
        seed: u32,
    },

    Start,
    Stop,

    Fault {
        kind: FaultKind,
        channel: Option<Channel>,
        dur_ms: u32,
        param: i16,
    },

    /// kind 0xFF clears every fault.
    Clear {
        kind: u8,
    },

    Rate {
        channel: Option<Channel>,
        hz_x10: u8,
    },
}

fn header(c: &mut Cursor<'_>) -> Result<u8, DecodeError> {
    let ver = c.u8()?;

    if ver != VCSIM_VERSION {
        return Err(DecodeError::BadVersion(ver));
    }

    c.u8()
}

// Struct-literal fields are evaluated in the order written,
// so each literal below reads the body in wire order.
pub fn decode_telem(b: &[u8]) -> Result<Telem, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }

    let mut c = Cursor::new(b);

    let msg = match header(&mut c)? {
        OP_STAMP => Telem::Stamp {
            seq: c.u16_le()?,
            channel: Channel::from_u8(c.u8()?, "stamp.channel")?,
            char_seq: c.u16_le()?,
            t0_ns: c.u64_le()?,
        },

        OP_PONG => Telem::Pong {
            ping_id: c.u16_le()?,
            t_rx_ns: c.u64_le()?,
            t_tx_ns: c.u64_le()?,
        },

        OP_STATUS => Telem::Status(Status {
            seq: c.u16_le()?,
            scenario: c.u8()?,
            state: c.u8()?,
            faults: c.u16_le()?,
            q_drops: c.u16_le()?,
            notify_errs: c.u16_le()?,
        }),

        op => return Err(DecodeError::BadOpcode(op)),
    };

    c.finish()?;

    Ok(msg)
}

pub fn encode_telem(m: &Telem, out: &mut [u8; MAX_PAYLOAD]) -> Result<usize, EncodeError> {
    let mut w = Writer::new(out);

    w.u8(VCSIM_VERSION)?;

    match *m {
        Telem::Stamp {
            seq,
            channel,
            char_seq,
            t0_ns,
        } => {
            w.u8(OP_STAMP)?;
            w.u16_le(seq)?;
            w.u8(channel as u8)?;
            w.u16_le(char_seq)?;
            w.u64_le(t0_ns)?;
        }

        Telem::Pong {
            ping_id,
            t_rx_ns,
            t_tx_ns,
        } => {
            w.u8(OP_PONG)?;
            w.u16_le(ping_id)?;
            w.u64_le(t_rx_ns)?;
            w.u64_le(t_tx_ns)?;
        }

        Telem::Status(s) => {
            w.u8(OP_STATUS)?;
            w.u16_le(s.seq)?;
            w.u8(s.scenario)?;
            w.u8(s.state)?;
            w.u16_le(s.faults)?;
            w.u16_le(s.q_drops)?;
            w.u16_le(s.notify_errs)?;
        }
    }

    Ok(w.written())
}

pub fn decode_ctrl(b: &[u8]) -> Result<Ctrl, DecodeError> {
    if b.is_empty() {
        return Err(DecodeError::Empty);
    }

    let mut c = Cursor::new(b);

    let msg = match header(&mut c)? {
        OP_PING => Ctrl::Ping {
            ping_id: c.u16_le()?,
            t_client_ns: c.u64_le()?,
        },

        OP_LOAD => Ctrl::Load {
            scenario: c.u8()?,
            seed: c.u32_le()?,
        },

        OP_START => Ctrl::Start,
        OP_STOP => Ctrl::Stop,

        OP_FAULT => Ctrl::Fault {
            kind: FaultKind::from_u8(c.u8()?)?,
            channel: Channel::opt_from_u8(c.u8()?, "fault.channel")?,
            dur_ms: c.u32_le()?,
            param: c.i16_le()?,
        },

        OP_CLEAR => Ctrl::Clear { kind: c.u8()? },

        OP_RATE => Ctrl::Rate {
            channel: Channel::opt_from_u8(c.u8()?, "rate.channel")?,
            hz_x10: c.u8()?,
        },

        op => return Err(DecodeError::BadOpcode(op)),
    };

    c.finish()?;

    Ok(msg)
}

pub fn encode_ctrl(m: &Ctrl, out: &mut [u8; MAX_PAYLOAD]) -> Result<usize, EncodeError> {
    let mut w = Writer::new(out);

    w.u8(VCSIM_VERSION)?;

    match *m {
        Ctrl::Ping {
            ping_id,
            t_client_ns,
        } => {
            w.u8(OP_PING)?;
            w.u16_le(ping_id)?;
            w.u64_le(t_client_ns)?;
        }

        Ctrl::Load { scenario, seed } => {
            w.u8(OP_LOAD)?;
            w.u8(scenario)?;
            w.u32_le(seed)?;
        }

        Ctrl::Start => w.u8(OP_START)?,
        Ctrl::Stop => w.u8(OP_STOP)?,

        Ctrl::Fault {
            kind,
            channel,
            dur_ms,
            param,
        } => {
            w.u8(OP_FAULT)?;
            w.u8(kind as u8)?;
            w.u8(Channel::opt_to_u8(channel))?;
            w.u32_le(dur_ms)?;
            w.i16_le(param)?;
        }

        Ctrl::Clear { kind } => {
            w.u8(OP_CLEAR)?;
            w.u8(kind)?;
        }

        Ctrl::Rate { channel, hz_x10 } => {
            w.u8(OP_RATE)?;
            w.u8(Channel::opt_to_u8(channel))?;
            w.u8(hz_x10)?;
        }
    }

    Ok(w.written())
}
