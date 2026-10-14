//! velocoach.irp.v1: one JSON object per line, tagged by record_type.
//! Field names follow the plan's §3.3 table. Built from vc-core's plain
//! LedgerRecords; serde stays out of the no_std crates.
use serde::{Deserialize, Serialize};
use vc_core::Directive;
use vc_core::ledger_ring::LedgerRecord;
use vc_core::study::{Fnv1a, NOT_RANDOMISED_Q16};
use vc_efficacy::{Arm, Confound, Mode, OBSERVER_VERSION, Param, Rule};

pub const SCHEMA: &str = "velocoach.irp.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum Line {
    SessionStart(SessionStart),
    Cue(CueLine),
    TunerChange(TunerLine),
    SessionEnd(SessionEnd),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionStart {
    pub schema: String,
    pub observer_version: String,
    pub session_id: String,
    pub rider_id: String,

    pub mode: String,
    pub tuner_frozen: bool,
    pub policy_hash: String,
    pub protocol_hash: Option<String>,
    pub profile: String,
    pub seed: u64,
    pub t_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CueLine {
    pub schema: String,
    pub observer_version: String,
    pub session_id: String,
    pub cue_id: u32,
    pub rider_id: String,
    pub mode: String,
    pub tuner_frozen: bool,
    pub policy_hash: String,
    pub protocol_hash: Option<String>,
    pub profile: String,
    pub directive: String,
    pub priority: String,
    pub aki_class: String,
    pub arm: String,
    pub is_sham: bool,
    pub propensity: f64,
    pub p_arm_given_delivered: Option<f64>,
    pub seed: u64,
    pub draw_index: Option<u32>,
    pub t_decision_ns: u64,
    pub t_cue_onset_ns: u64,
    pub t_audio_end_ns: u64,
    pub onset_source: String,
    pub clock_sync_error_bound_us: u32,
    pub pre_trend_slope_w_per_s: f64,
    pub slope_clamped: bool,
    pub dir: i8,
    pub delta_star_w: i16,
    pub verdict: String,
    /// L_obs, from the audio's end.
    pub latency_ms: Option<i32>,
    pub l_onset_ms: Option<i32>,
    pub l_end_ms: Option<i32>,
    pub coincidental: bool,
    pub confound_flags: Vec<String>,
    pub effect_max_w: f64,
    pub dropped_before: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TunerLine {
    pub schema: String,
    pub session_id: String,
    pub t_ns: u64,
    pub rule: String,
    pub param: String,
    pub from: f64,
    pub to: f64,
    pub n: u32,
    pub evidence: f64,
    pub policy_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEnd {
    pub schema: String,
    pub session_id: String,
    pub t_ns: u64,
    pub cues: u32,
    pub dropped: u32,
}

fn hex(h: u64) -> String {
    format!("{h:016x}")
}

fn profile_name(p: u8) -> &'static str {
    match p {
        0 => "solo",
        1 => "rolling",
        _ => "group",
    }
}

/// Turns LedgerRecords into lines, carrying session context between them.
#[derive(Debug, Clone)]
pub struct Writer {
    pub rider_id: String,
    pub protocol_hash: Option<String>,
    session_id: String,
    mode: Mode,
    seed: u64,
    policy_hash: u64,
    profile: u8,
}

impl Writer {
    pub fn new(rider_id: &str, protocol_hash: Option<String>) -> Self {
        Self {
            rider_id: rider_id.into(),
            protocol_hash,
            session_id: hex(0),
            mode: Mode::Coach,
            seed: 0,
            policy_hash: 0,
            profile: 0,
        }
    }

    pub fn line(&mut self, r: &LedgerRecord) -> Line {
        match *r {
            LedgerRecord::SessionStart {
                t_ns,
                mode,
                seed,
                policy_hash,
                profile,
            } => {
                let mut h = Fnv1a::default();
                h.write_u64(t_ns);
                h.write_u64(seed);
                h.write(self.rider_id.as_bytes());
                (
                    self.session_id,
                    self.mode,
                    self.seed,
                    self.policy_hash,
                    self.profile,
                ) = (hex(h.finish()), mode, seed, policy_hash, profile);
                Line::SessionStart(SessionStart {
                    schema: SCHEMA.into(),
                    observer_version: OBSERVER_VERSION.into(),
                    session_id: self.session_id.clone(),
                    rider_id: self.rider_id.clone(),
                    mode: mode.name().into(),
                    tuner_frozen: mode != Mode::Coach,
                    policy_hash: hex(policy_hash),
                    protocol_hash: self.protocol_hash.clone(),
                    profile: profile_name(profile).into(),
                    seed,
                    t_ns,
                })
            }
            LedgerRecord::Cue {
                event: e,
                outcome: o,
                draw_index,
                onset_source,
                sync_bound_us,
                dropped_before,
            } => {
                let randomised = e.propensity_q16 != NOT_RANDOMISED_Q16;
                Line::Cue(CueLine {
                    schema: SCHEMA.into(),
                    observer_version: OBSERVER_VERSION.into(),
                    session_id: self.session_id.clone(),
                    cue_id: e.cue_id,
                    rider_id: self.rider_id.clone(),
                    mode: self.mode.name().into(),
                    tuner_frozen: self.mode != Mode::Coach,
                    policy_hash: hex(self.policy_hash),
                    protocol_hash: self.protocol_hash.clone(),
                    profile: profile_name(e.profile).into(),
                    directive: Directive::from_code(e.directive)
                        .map_or("UNKNOWN", Directive::name)
                        .into(),
                    priority: format!("P{}", e.priority),
                    aki_class: format!("P{}", e.aki_class),
                    arm: e.arm.name().into(),
                    is_sham: e.is_sham,
                    propensity: if randomised {
                        f64::from(e.propensity_q16) / 65_536.0
                    } else {
                        1.0
                    },
                    // Given that a cue was delivered at all, each phrasing had probability 0.5.
                    p_arm_given_delivered: (randomised
                        && matches!(e.arm, Arm::Direct | Arm::Autonomy))
                    .then_some(0.5),
                    seed: self.seed,
                    draw_index,
                    t_decision_ns: e.t_decision_ns,
                    t_cue_onset_ns: e.t_onset_ns,
                    t_audio_end_ns: e.t_end_ns,
                    onset_source: onset_source.name().into(),
                    clock_sync_error_bound_us: sync_bound_us,
                    pre_trend_slope_w_per_s: o.pre_slope_w_per_s,
                    slope_clamped: o.slope_clamped,
                    dir: e.dir,
                    delta_star_w: e
                        .delta_star_w
                        .round()
                        .clamp(f64::from(i16::MIN), f64::from(i16::MAX))
                        as i16,
                    verdict: o.verdict.name().into(),
                    latency_ms: o.l_obs_ms,
                    l_onset_ms: o.l_onset_ms,
                    l_end_ms: o.l_end_ms,
                    coincidental: o.coincidental,
                    confound_flags: Confound::ALL
                        .iter()
                        .filter(|(c, _)| o.confound.contains(*c))
                        .map(|(_, n)| (*n).into())
                        .collect(),
                    effect_max_w: o.effect_max_w,
                    dropped_before,
                })
            }
            LedgerRecord::TunerChange {
                t_ns,
                change: c,
                policy_hash_after,
            } => {
                self.policy_hash = policy_hash_after;
                Line::TunerChange(TunerLine {
                    schema: SCHEMA.into(),
                    session_id: self.session_id.clone(),
                    t_ns,
                    rule: match c.rule {
                        Rule::T1 => "T1",
                        Rule::T1Prime => "T1'",
                        Rule::T2 => "T2",
                        Rule::T2P3 => "T2-P3",
                        Rule::T3 => "T3",
                        Rule::T4 => "T4",
                    }
                    .into(),
                    param: match c.param {
                        Param::Cooldown => "cooldownBetweenAlertsSec".into(),
                        Param::SurgeLead => "promptSurgeCountdownSec".into(),
                        Param::P3Earcon(code) => format!(
                            "earcon:{}",
                            Directive::from_code(code).map_or("UNKNOWN", Directive::name)
                        ),
                        Param::DefaultVariant(code) => format!(
                            "default_variant:{}",
                            Directive::from_code(code).map_or("UNKNOWN", Directive::name)
                        ),
                    },
                    from: c.from,
                    to: c.to,
                    n: c.n,
                    evidence: c.evidence,
                    policy_hash: hex(policy_hash_after),
                })
            }
            LedgerRecord::SessionEnd {
                t_ns,
                cues,
                dropped,
            } => Line::SessionEnd(SessionEnd {
                schema: SCHEMA.into(),
                session_id: self.session_id.clone(),
                t_ns,
                cues,
                dropped,
            }),
        }
    }
}

/// Serialise one line (no trailing newline).
pub fn to_json(line: &Line) -> String {
    serde_json::to_string(line).unwrap_or_default()
}

/// Read a JSONL file's text; malformed lines are skipped and counted.
pub fn read(text: &str) -> (Vec<Line>, usize) {
    let mut bad = 0;
    let lines = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).map_err(|_| bad += 1).ok())
        .collect();
    (lines, bad)
}
