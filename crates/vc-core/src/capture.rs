//! The raw capture: every input the Engine received, in order, so a ride
//! recorded on the phone replays bit-for-bit on the Mac.
//! [type u8][len u8][payload], little-endian, after MAGIC. Encoding writes into
//! a caller's buffer; decoding borrows from the input. No allocation.
use vc_efficacy::{Arm, Mode};

use crate::config::Profile;
use crate::engine::{AudioFailure, Source};
use crate::workout::{Step, StepKind};

pub const MAGIC: [u8; 8] = *b"VCRAW01\n";
/// Largest encoded record (an Ingest with a 20-byte packet is 31).
pub const MAX_RECORD: usize = 2 + 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Record<'a> {
    /// The fields the FFI sets; every other EngineConfig field is its default.
    Config {
        cp_w: f64,
        wprime_j: f64,
        hr_max: f64,
        profile: Profile,
        cooldown_s: f64,
        surge_lead_s: f64,
    },
    Step(Step),
    Mode {
        mode: Mode,
        seed: u64,
    },
    Start {
        t_ns: u64,
    },
    Ingest {
        t_ns: u64,
        source: Source,
        bytes: &'a [u8],
    },
    Tick {
        t_ns: u64,
    },
    Played {
        cue_id: u32,
        onset_ns: u64,
        end_ns: u64,
        callback_ns: u64,
    },
    Failed {
        cue_id: u32,
        reason: AudioFailure,
    },
    /// The phone's decision, for comparison. Never fed to the Engine.
    Decision {
        cue_id: u32,
        directive: u8,
        arm: u8,
        decided_ns: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureError {
    Truncated,
    BadType(u8),
    BadValue(&'static str),
}

struct W<'b> {
    buf: &'b mut [u8],
    n: usize,
}

impl W<'_> {
    fn put(&mut self, b: &[u8]) -> Option<()> {
        let end = self.n + b.len();
        self.buf.get_mut(self.n..end)?.copy_from_slice(b);
        self.n = end;
        Some(())
    }
    fn u8(&mut self, v: u8) -> Option<()> {
        self.put(&[v])
    }
    fn u32(&mut self, v: u32) -> Option<()> {
        self.put(&v.to_le_bytes())
    }
    fn u64(&mut self, v: u64) -> Option<()> {
        self.put(&v.to_le_bytes())
    }
    fn f64(&mut self, v: f64) -> Option<()> {
        self.put(&v.to_bits().to_le_bytes())
    }
    fn opt(&mut self, v: Option<f64>) -> Option<()> {
        self.f64(v.unwrap_or(f64::NAN))
    }
}

struct R<'a> {
    b: &'a [u8],
}

impl<'a> R<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], CaptureError> {
        let (head, tail) = self.b.split_at_checked(n).ok_or(CaptureError::Truncated)?;
        self.b = tail;
        Ok(head)
    }
    fn u8(&mut self) -> Result<u8, CaptureError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, CaptureError> {
        let mut a = [0u8; 4];
        a.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(a))
    }
    fn u64(&mut self) -> Result<u64, CaptureError> {
        let mut a = [0u8; 8];
        a.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(a))
    }
    fn f64(&mut self) -> Result<f64, CaptureError> {
        Ok(f64::from_bits(self.u64()?))
    }
    fn opt(&mut self) -> Result<Option<f64>, CaptureError> {
        let v = self.f64()?;
        Ok((!v.is_nan()).then_some(v))
    }
}

const PROFILES: [Profile; 3] = [Profile::Solo, Profile::Rolling, Profile::Group];
const SOURCES: [Source; 4] = [Source::Hr, Source::Power, Source::Csc, Source::Telem];
const MODES: [Mode; 3] = [Mode::Coach, Mode::Study, Mode::Replay];
const KINDS: [StepKind; 5] = [
    StepKind::Warmup,
    StepKind::Interval,
    StepKind::Recovery,
    StepKind::Steady,
    StepKind::Cooldown,
];
const FAILURES: [AudioFailure; 4] = [
    AudioFailure::Interrupted,
    AudioFailure::RouteLost,
    AudioFailure::EngineReset,
    AudioFailure::Other,
];

fn code_of<T: PartialEq>(table: &[T], v: &T) -> u8 {
    table.iter().position(|x| x == v).unwrap_or(0) as u8
}

fn from_code<T: Copy>(table: &[T], c: u8, what: &'static str) -> Result<T, CaptureError> {
    table
        .get(usize::from(c))
        .copied()
        .ok_or(CaptureError::BadValue(what))
}

/// Encode one record into `out`; returns its length, or None if `out` is too small.
pub fn encode(rec: &Record<'_>, out: &mut [u8]) -> Option<usize> {
    let (head, body) = out.split_at_mut_checked(2)?;
    let mut w = W { buf: body, n: 0 };
    let ty = match *rec {
        Record::Config {
            cp_w,
            wprime_j,
            hr_max,
            profile,
            cooldown_s,
            surge_lead_s,
        } => {
            w.f64(cp_w)?;
            w.f64(wprime_j)?;
            w.f64(hr_max)?;
            w.u8(code_of(&PROFILES, &profile))?;
            w.f64(cooldown_s)?;
            w.f64(surge_lead_s)?;
            1
        }
        Record::Step(s) => {
            w.u8(code_of(&KINDS, &s.kind))?;
            w.u32(s.dur_s)?;
            w.opt(s.target_w)?;
            w.opt(s.cadence_rpm)?;
            w.opt(s.hr_cap_bpm)?;
            2
        }
        Record::Mode { mode, seed } => {
            w.u8(code_of(&MODES, &mode))?;
            w.u64(seed)?;
            3
        }
        Record::Start { t_ns } => {
            w.u64(t_ns)?;
            4
        }
        Record::Ingest {
            t_ns,
            source,
            bytes,
        } => {
            w.u64(t_ns)?;
            w.u8(code_of(&SOURCES, &source))?;
            w.put(bytes)?;
            5
        }
        Record::Tick { t_ns } => {
            w.u64(t_ns)?;
            6
        }
        Record::Played {
            cue_id,
            onset_ns,
            end_ns,
            callback_ns,
        } => {
            w.u32(cue_id)?;
            w.u64(onset_ns)?;
            w.u64(end_ns)?;
            w.u64(callback_ns)?;
            7
        }
        Record::Failed { cue_id, reason } => {
            w.u32(cue_id)?;
            w.u8(code_of(&FAILURES, &reason))?;
            8
        }
        Record::Decision {
            cue_id,
            directive,
            arm,
            decided_ns,
        } => {
            w.u32(cue_id)?;
            w.u8(directive)?;
            w.u8(arm)?;
            w.u64(decided_ns)?;
            9
        }
    };
    head[0] = ty;
    head[1] = u8::try_from(w.n).ok()?;
    Some(2 + w.n)
}

/// Decode the record at the start of `buf`: Ok(None) at the end of the data.
pub fn decode(buf: &[u8]) -> Result<Option<(Record<'_>, usize)>, CaptureError> {
    let Some((&ty, rest)) = buf.split_first() else {
        return Ok(None);
    };
    let (&len, rest) = rest.split_first().ok_or(CaptureError::Truncated)?;
    let payload = rest
        .get(..usize::from(len))
        .ok_or(CaptureError::Truncated)?;
    let mut r = R { b: payload };
    let rec = match ty {
        1 => Record::Config {
            cp_w: r.f64()?,
            wprime_j: r.f64()?,
            hr_max: r.f64()?,
            profile: from_code(&PROFILES, r.u8()?, "profile")?,
            cooldown_s: r.f64()?,
            surge_lead_s: r.f64()?,
        },
        2 => Record::Step(Step {
            kind: from_code(&KINDS, r.u8()?, "step kind")?,
            dur_s: r.u32()?,
            target_w: r.opt()?,
            cadence_rpm: r.opt()?,
            hr_cap_bpm: r.opt()?,
        }),
        3 => Record::Mode {
            mode: from_code(&MODES, r.u8()?, "mode")?,
            seed: r.u64()?,
        },
        4 => Record::Start { t_ns: r.u64()? },
        5 => Record::Ingest {
            t_ns: r.u64()?,
            source: from_code(&SOURCES, r.u8()?, "source")?,
            bytes: r.b,
        },
        6 => Record::Tick { t_ns: r.u64()? },
        7 => Record::Played {
            cue_id: r.u32()?,
            onset_ns: r.u64()?,
            end_ns: r.u64()?,
            callback_ns: r.u64()?,
        },
        8 => Record::Failed {
            cue_id: r.u32()?,
            reason: from_code(&FAILURES, r.u8()?, "failure")?,
        },
        9 => Record::Decision {
            cue_id: r.u32()?,
            directive: r.u8()?,
            arm: r.u8()?,
            decided_ns: r.u64()?,
        },
        other => return Err(CaptureError::BadType(other)),
    };
    Ok(Some((rec, 2 + usize::from(len))))
}

/// (cue id, directive code, arm code, decided ns): what "identical" compares.
pub type DecisionKey = (u32, u8, u8, u64);

pub fn key(d: &crate::CueDecision) -> DecisionKey {
    (d.id, d.directive.code(), Arm::code(d.arm), d.decided_ns)
}

#[cfg(feature = "testkit")]
pub struct Replay {
    pub phone: Vec<DecisionKey>,
    pub replayed: Vec<DecisionKey>,
}

/// Feed every recorded input to a fresh Engine; collect both decision lists.
#[cfg(feature = "testkit")]
pub fn replay(data: &[u8]) -> Result<Replay, String> {
    use crate::{Engine, EngineConfig, WorkoutPlan};

    let mut rest = data
        .strip_prefix(&MAGIC[..])
        .ok_or("not a velocoach capture")?;
    let (mut cfg, mut steps) = (EngineConfig::default(), Vec::new());
    let mut engine: Option<Engine> = None;
    let (mut phone, mut replayed) = (Vec::new(), Vec::new());
    while let Some((rec, n)) = decode(rest).map_err(|e| format!("{e:?}"))? {
        rest = &rest[n..];
        match rec {
            Record::Config {
                cp_w,
                wprime_j,
                hr_max,
                profile,
                cooldown_s,
                surge_lead_s,
            } => {
                cfg = EngineConfig {
                    cp_w,
                    wprime_j,
                    hr_max,
                    profile,
                    cooldown_between_alerts_s: cooldown_s,
                    prompt_surge_countdown_s: surge_lead_s,
                    ..EngineConfig::default()
                };
            }
            Record::Step(s) => steps.push(s),
            Record::Decision {
                cue_id,
                directive,
                arm,
                decided_ns,
            } => phone.push((cue_id, directive, arm, decided_ns)),
            input => {
                if engine.is_none() {
                    let plan = WorkoutPlan::from_steps(&steps).map_err(|_| "plan too long")?;
                    engine = Some(Engine::new(cfg, plan).map_err(|e| e.to_string())?);
                }
                let Some(e) = engine.as_mut() else { continue };
                let decision = match input {
                    Record::Mode { mode, seed } => {
                        e.set_mode(mode, seed).map_err(|x| x.to_string())?;
                        None
                    }
                    Record::Start { t_ns } => {
                        e.start(t_ns);
                        None
                    }
                    Record::Ingest {
                        t_ns,
                        source,
                        bytes,
                    } => e.ingest(t_ns, source, bytes).ok().flatten(),
                    Record::Tick { t_ns } => e.tick(t_ns),
                    Record::Played {
                        cue_id,
                        onset_ns,
                        end_ns,
                        callback_ns,
                    } => {
                        e.on_cue_played(cue_id, onset_ns, end_ns, callback_ns);
                        None
                    }
                    Record::Failed { cue_id, reason } => {
                        e.on_cue_failed(cue_id, reason);
                        None
                    }
                    _ => None,
                };
                if let Some(d) = decision {
                    replayed.push(key(&d));
                }
            }
        }
    }
    Ok(Replay { phone, replayed })
}
