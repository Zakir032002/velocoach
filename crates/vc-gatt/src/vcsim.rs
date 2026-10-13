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
const OP_RIDER_INPUT: u8 = 0x04;

// The CTRL Opcodes (iPhone -> Android)
const OP_PING: u8 = 0x10; // "Clock sync check"
const OP_LOAD: u8 = 0x20; // "Load a workout scenario"
const OP_START: u8 = 0x21; // "Start pedaling"
const OP_STOP: u8 = 0x22; // "Stop pedaling"
const OP_FAULT: u8 = 0x30; // "Break something on purpose"
const OP_CLEAR: u8 = 0x31; // "Fix the broken sensor"
const OP_RATE: u8 = 0x40; // "Change send speed (Hz)"
const OP_CUE_PLAYED: u8 = 0x50;

/// Persona codes: 0 = scenario default, 1–4 = named personas.
pub const PERSONA_MAX: u8 = 4;

/// Arm codes: 0 none, 1 direct, 2 autonomy, 3 sham (never played).
pub const ARM_MAX: u8 = 3;

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
    Dropout = 1,    //loss of data
    Garbage = 2,    //data is being recieved but corrupted or malformed etc
    Disconnect = 3, //the ble connection is completly severd
    Coast = 4,      // rider has stopped, and powermeter sends 0
    HrRedline = 5,  //exceeded safe limit

    /// param = the rider's true W′ in units of 100 J.
    WprimeMismatch = 6, //calculatuion error regarding w;prime represents their finite anaerobic energy capacity
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
//We've identified the channels and faults. Now we need a way to hold all the information belonging to a report or command.
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

    /// Human mode: slider effort, timestamped on the iQOO clock.
    RiderInput {
        seq: u16,
        t_ns: u64,
        effort_permille: i16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctrl {
    Ping {
        ping_id: u16,
        t_client_ns: u64,
    },

    Load {
        scenario: u8,
        persona: u8,
        seed: u32,
        human: bool,
    },

    /// A cue is scheduled: what it asks and when its audio plays.
    CuePlayed {
        cue_id: u32,
        kind: u8,
        dir: i8,
        delta_w: i16,
        arm: u8,
        starts_in_ms: u16,
        dur_ms: u16,
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

        OP_RIDER_INPUT => Telem::RiderInput {
            seq: c.u16_le()?,
            t_ns: c.u64_le()?,
            effort_permille: c.i16_le()?,
        },

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

        Telem::RiderInput {
            seq,
            t_ns,
            effort_permille,
        } => {
            w.u8(OP_RIDER_INPUT)?;
            w.u16_le(seq)?;
            w.u64_le(t_ns)?;
            w.i16_le(effort_permille)?;
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

        OP_LOAD => {
            let scenario = c.u8()?;
            let persona = c.u8()?;

            if persona > PERSONA_MAX {
                return Err(DecodeError::BadValue {
                    field: "load.persona",
                    value: persona,
                });
            }

            let seed = c.u32_le()?;

            let human = match c.u8()? {
                0 => false,
                1 => true,
                value => {
                    return Err(DecodeError::BadValue {
                        field: "load.human",
                        value,
                    });
                }
            };

            Ctrl::Load {
                scenario,
                persona,
                seed,
                human,
            }
        }

        OP_CUE_PLAYED => {
            let cue_id = c.u32_le()?;
            let kind = c.u8()?;

            if kind == 0 {
                return Err(DecodeError::BadValue {
                    field: "cue.kind",
                    value: 0,
                });
            }

            let dir_byte = c.u8()?;
            let dir = dir_byte as i8;

            if !(-1..=1).contains(&dir) {
                return Err(DecodeError::BadValue {
                    field: "cue.dir",
                    value: dir_byte,
                });
            }

            let delta_w = c.i16_le()?;
            let arm = c.u8()?;

            if arm > ARM_MAX {
                return Err(DecodeError::BadValue {
                    field: "cue.arm",
                    value: arm,
                });
            }

            Ctrl::CuePlayed {
                cue_id,
                kind,
                dir,
                delta_w,
                arm,
                starts_in_ms: c.u16_le()?,
                dur_ms: c.u16_le()?,
            }
        }

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

        Ctrl::Load {
            scenario,
            persona,
            seed,
            human,
        } => {
            w.u8(OP_LOAD)?;
            w.u8(scenario)?;
            w.u8(persona)?;
            w.u32_le(seed)?;
            w.u8(u8::from(human))?;
        }

        Ctrl::CuePlayed {
            cue_id,
            kind,
            dir,
            delta_w,
            arm,
            starts_in_ms,
            dur_ms,
        } => {
            w.u8(OP_CUE_PLAYED)?;
            w.u32_le(cue_id)?;
            w.u8(kind)?;
            w.u8(dir as u8)?;
            w.i16_le(delta_w)?;
            w.u8(arm)?;
            w.u16_le(starts_in_ms)?;
            w.u16_le(dur_ms)?;
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn telem_bytes(m: &Telem) -> std::vec::Vec<u8> {
        let mut buf = [0u8; MAX_PAYLOAD];
        let n = encode_telem(m, &mut buf).unwrap();
        buf[..n].to_vec()
    }

    fn ctrl_bytes(m: &Ctrl) -> std::vec::Vec<u8> {
        let mut buf = [0u8; MAX_PAYLOAD];
        let n = encode_ctrl(m, &mut buf).unwrap();
        buf[..n].to_vec()
    }

    #[test]
    fn rider_input_bytes_exact() {
        let m = Telem::RiderInput {
            seq: 1,
            t_ns: 0x0102_0304_0506_0708,
            effort_permille: 750,
        };

        let want = [
            0x01, 0x04, 0x01, 0x00, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0xEE, 0x02,
        ];

        assert_eq!(telem_bytes(&m), want);
        assert_eq!(decode_telem(&want), Ok(m));
    }

    #[test]
    fn cue_played_bytes_exact() {
        let m = Ctrl::CuePlayed {
            cue_id: 7,
            kind: 1,
            dir: -1,
            delta_w: 150,
            arm: 1,
            starts_in_ms: 50,
            dur_ms: 2_400,
        };

        let want = [
            0x01, 0x50, 0x07, 0x00, 0x00, 0x00, 0x01, 0xFF, 0x96, 0x00, 0x01, 0x32, 0x00, 0x60,
            0x09,
        ];

        assert_eq!(ctrl_bytes(&m), want);
        assert_eq!(decode_ctrl(&want), Ok(m));
    }

    #[test]
    fn load_v2_bytes_exact() {
        let m = Ctrl::Load {
            scenario: 1,
            persona: 2,
            seed: 42,
            human: false,
        };

        let want = [0x01, 0x20, 0x01, 0x02, 0x2A, 0x00, 0x00, 0x00, 0x00];

        assert_eq!(ctrl_bytes(&m), want);
        assert_eq!(decode_ctrl(&want), Ok(m));
    }

    #[test]
    fn new_messages_reject_values_outside_their_sets() {
        use DecodeError::*;

        let mut cue = [
            0x01, 0x50, 0x07, 0, 0, 0, 0x01, 0x02, 0x96, 0x00, 0x01, 0x32, 0x00, 0x60, 0x09,
        ];

        assert_eq!(
            decode_ctrl(&cue),
            Err(BadValue {
                field: "cue.dir",
                value: 2
            })
        );

        cue[7] = 0xFF;
        cue[10] = 4;
        assert_eq!(
            decode_ctrl(&cue),
            Err(BadValue {
                field: "cue.arm",
                value: 4
            })
        );

        cue[10] = 1;
        cue[6] = 0;
        assert_eq!(
            decode_ctrl(&cue),
            Err(BadValue {
                field: "cue.kind",
                value: 0
            })
        );

        assert_eq!(
            decode_ctrl(&[0x01, 0x20, 1, 9, 0, 0, 0, 0, 0]),
            Err(BadValue {
                field: "load.persona",
                value: 9
            })
        );

        assert_eq!(
            decode_ctrl(&[0x01, 0x20, 1, 0, 0, 0, 0, 0, 2]),
            Err(BadValue {
                field: "load.human",
                value: 2
            })
        );
    }
}
