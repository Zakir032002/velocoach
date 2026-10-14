//! vc-ffi: the UniFFI surface (FFI v0, frozen on Day 8).
//! Thin and copying: no logic lives here, only conversions between
//! vc-core and plain records.
uniffi::setup_scaffolding!();

use std::sync::{Arc, Mutex, MutexGuard};

use vc_core::capture::{self, MAGIC, MAX_RECORD, Record};
use vc_core::latency::Histogram;
use vc_core::workout::{Step, StepKind as CoreKind, WorkoutPlan};
use vc_efficacy::{Arm as CoreArm, Mode as CoreMode};
use vc_gatt::{Channel, Ctrl, MAX_PAYLOAD, Telem, encode_ctrl, encode_telem};

#[cfg(feature = "sim")]
mod sim;
#[cfg(feature = "sim")]
pub use sim::*;

// ---- Errors and enums -------------------------------------------------------

#[derive(Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum VcError {
    Config(String),
    Sim(String),
}

impl std::fmt::Display for VcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(reason) => write!(f, "config: {reason}"),
            Self::Sim(reason) => write!(f, "simulator: {reason}"),
        }
    }
}

impl std::error::Error for VcError {}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum Profile {
    Solo,
    Rolling,
    Group,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum Source {
    Hr,
    Power,
    Csc,
    Telem,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum Mode {
    Coach,
    Study,
    Replay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Arm {
    None,
    Direct,
    Autonomy,
    Sham,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum AudioFailure {
    Interrupted,
    RouteLost,
    EngineReset,
    Other,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum StepKind {
    Warmup,
    Interval,
    Recovery,
    Steady,
    Cooldown,
}

// ---- Records ---------------------------------------------------------------

#[derive(Debug, Clone, uniffi::Record)]
pub struct PlanStep {
    pub kind: StepKind,
    pub dur_s: u32,
    pub target_w: Option<f64>,
    pub cadence_rpm: Option<f64>,
    pub hr_cap_bpm: Option<f64>,
}

/// The coach's settings.
#[derive(Debug, Clone, uniffi::Record)]
pub struct EngineConfig {
    pub cp_w: f64,
    pub wprime_j: f64,
    pub hr_max: f64,
    pub profile: Profile,
    pub cooldown_between_alerts_s: f64,
    pub prompt_surge_countdown_s: f64,
    /// Ignored by `for_scenario`.
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct CueDecision {
    pub id: u32,
    /// Aki's spelling, e.g. "WPRIME_LOW".
    pub directive: String,
    pub directive_code: u8,
    pub priority: u8,
    pub aki_class: u8,
    pub arm: Arm,
    /// File stem of the clip, e.g. "WPL_D". Not played for a sham.
    pub phrase_key: String,
    pub earcon_key: Option<String>,
    pub lead_ms: u16,
    pub interrupt: bool,
    pub dir: i8,
    pub delta_star_w: f64,
    pub decided_ns: u64,
    pub propensity_q16: u16,
    pub draw_index: Option<u32>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Snapshot {
    pub power_w: f64,
    pub p3_w: Option<f64>,
    pub hr_bpm: u16,
    pub cadence_rpm: f64,
    pub wbal_fraction: f64,
    pub hr_quality: String,
    pub power_quality: String,
    pub cadence_quality: String,
    pub step_index: Option<u32>,
    pub step_remaining_s: f64,
    pub paused: bool,
    pub decode_errors: u32,
    pub implausible: u32,
    pub mode: Mode,
    /// Sixteen hexadecimal characters.
    pub policy_hash: String,
}

/// The observer's verdict for the on-screen badge (Day 12).
#[derive(Debug, Clone, uniffi::Record)]
pub struct CueOutcomeFfi {
    pub cue_id: u32,
    pub verdict: String,
    pub l_obs_ms: Option<i32>,
    pub confounds: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ReportCard {
    pub text: String,
    pub cues_scored: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ScenarioInfo {
    pub id: u8,
    pub name: String,
    pub default_persona: String,
    pub minutes: f64,
}

/// Day 11: T1->T2. Day 15 adds T0->T3, T1->T3 and T3->T4.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum LatencySegment {
    T1T2,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LatencySummary {
    pub count: u64,
    pub p50_us: f64,
    pub p90_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
}

fn slot(s: LatencySegment) -> usize {
    match s {
        LatencySegment::T1T2 => 0,
    }
}

// ---- Conversions -----------------------------------------------------------

fn core_profile(p: Profile) -> vc_core::Profile {
    match p {
        Profile::Solo => vc_core::Profile::Solo,
        Profile::Rolling => vc_core::Profile::Rolling,
        Profile::Group => vc_core::Profile::Group,
    }
}

fn core_kind(k: StepKind) -> CoreKind {
    match k {
        StepKind::Warmup => CoreKind::Warmup,
        StepKind::Interval => CoreKind::Interval,
        StepKind::Recovery => CoreKind::Recovery,
        StepKind::Steady => CoreKind::Steady,
        StepKind::Cooldown => CoreKind::Cooldown,
    }
}

fn core_config(c: &EngineConfig) -> vc_core::EngineConfig {
    vc_core::EngineConfig {
        cp_w: c.cp_w,
        wprime_j: c.wprime_j,
        hr_max: c.hr_max,
        profile: core_profile(c.profile),
        cooldown_between_alerts_s: c.cooldown_between_alerts_s,
        prompt_surge_countdown_s: c.prompt_surge_countdown_s,
        ..vc_core::EngineConfig::default()
    }
}

fn plan_from(steps: &[PlanStep]) -> Result<WorkoutPlan, VcError> {
    let steps: Vec<Step> = steps
        .iter()
        .map(|s| Step {
            kind: core_kind(s.kind),
            dur_s: s.dur_s,
            target_w: s.target_w,
            cadence_rpm: s.cadence_rpm,
            hr_cap_bpm: s.hr_cap_bpm,
        })
        .collect();

    WorkoutPlan::from_steps(&steps).map_err(|_| VcError::Config("more than 64 steps".into()))
}

fn ffi_arm(a: CoreArm) -> Arm {
    match a {
        CoreArm::None => Arm::None,
        CoreArm::Direct => Arm::Direct,
        CoreArm::Autonomy => Arm::Autonomy,
        CoreArm::Sham => Arm::Sham,
    }
}

fn ffi_decision(d: vc_core::CueDecision) -> CueDecision {
    CueDecision {
        id: d.id,
        directive: d.directive.name().to_string(),
        directive_code: d.directive.code(),
        priority: d.priority as u8 + 1,
        aki_class: d.aki_class as u8,
        arm: ffi_arm(d.arm),
        phrase_key: vc_core::phrase::def(d.phrase)
            .map_or("?", |p| p.key)
            .to_string(),
        earcon_key: d.earcon.map(|e| e.key().to_string()),
        lead_ms: d.lead_ms,
        interrupt: d.interrupt,
        dir: d.dir,
        delta_star_w: d.delta_star_w,
        decided_ns: d.decided_ns,
        propensity_q16: d.propensity_q16,
        draw_index: d.draw_index,
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ---- Engine ----------------------------------------------------------------

struct Recorder {
    on: bool,
    buf: Vec<u8>,
}

#[derive(uniffi::Object)]
pub struct Engine {
    inner: Mutex<vc_core::Engine>,
    protocol_hash: Mutex<Option<String>>,
    latency: Mutex<[Histogram; 4]>,
    capture: Mutex<Recorder>,
    cfg: vc_core::EngineConfig,
    steps: Vec<Step>,
}

#[uniffi::export]
impl Engine {
    #[uniffi::constructor]
    pub fn new(cfg: EngineConfig) -> Result<Arc<Self>, VcError> {
        Self::build(core_config(&cfg), plan_from(&cfg.steps)?)
    }

    /// Build from a built-in scenario. The scenario defines its workout plan.
    #[uniffi::constructor]
    pub fn for_scenario(cfg: EngineConfig, scenario_id: u8) -> Result<Arc<Self>, VcError> {
        let sc = vc_sim::scenario::by_id(scenario_id)
            .ok_or_else(|| VcError::Config(format!("no scenario {scenario_id}")))?;

        Self::build(core_config(&cfg), sc.plan())
    }

    pub fn start(&self, t_ns: u64) {
        let mut e = lock(&self.inner);
        self.rec(Record::Start { t_ns });
        e.start(t_ns);
    }

    /// Bad bytes and backwards timestamps are counted/ignored, not thrown
    /// through the platform callback.
    ///
    /// Vec<u8> is used because it is a supported owned UniFFI argument.
    pub fn ingest(&self, t_ns: u64, source: Source, bytes: Vec<u8>) -> Option<CueDecision> {
        let src = match source {
            Source::Hr => vc_core::Source::Hr,
            Source::Power => vc_core::Source::Power,
            Source::Csc => vc_core::Source::Csc,
            Source::Telem => vc_core::Source::Telem,
        };

        // Bad bytes are inputs too: record first, then call, under one lock.
        let mut e = lock(&self.inner);
        self.rec(Record::Ingest {
            t_ns,
            source: src,
            bytes: &bytes,
        });
        let decision = e.ingest(t_ns, src, &bytes).ok().flatten();
        if let Some(d) = &decision {
            self.rec_decision(d);
        }
        decision.map(ffi_decision)
    }

    pub fn tick(&self, t_ns: u64) -> Option<CueDecision> {
        let mut e = lock(&self.inner);
        self.rec(Record::Tick { t_ns });
        let decision = e.tick(t_ns);
        if let Some(d) = &decision {
            self.rec_decision(d);
        }
        decision.map(ffi_decision)
    }

    pub fn snapshot(&self) -> Snapshot {
        let e = lock(&self.inner);
        let s = e.snapshot();

        Snapshot {
            power_w: s.power_w,
            p3_w: s.p3_w,
            hr_bpm: s.hr_bpm,
            cadence_rpm: s.cadence_rpm,
            wbal_fraction: s.wbal_fraction,
            hr_quality: format!("{:?}", s.hr_q),
            power_quality: format!("{:?}", s.power_q),
            cadence_quality: format!("{:?}", s.cadence_q),
            step_index: s.step_index.and_then(|i| u32::try_from(i).ok()),
            step_remaining_s: s.step_remaining_s,
            paused: s.paused,
            decode_errors: s.decode_errors,
            implausible: s.implausible,
            mode: match e.mode() {
                CoreMode::Coach => Mode::Coach,
                CoreMode::Study => Mode::Study,
                CoreMode::Replay => Mode::Replay,
            },
            policy_hash: format!("{:016x}", e.policy_hash()),
        }
    }

    pub fn on_cue_played(&self, cue_id: u32, t_onset_ns: u64, t_end_ns: u64, t_callback_ns: u64) {
        let mut e = lock(&self.inner);
        self.rec(Record::Played {
            cue_id,
            onset_ns: t_onset_ns,
            end_ns: t_end_ns,
            callback_ns: t_callback_ns,
        });
        e.on_cue_played(cue_id, t_onset_ns, t_end_ns, t_callback_ns);
    }

    pub fn on_cue_failed(&self, cue_id: u32, reason: AudioFailure) {
        let reason = match reason {
            AudioFailure::Interrupted => vc_core::engine::AudioFailure::Interrupted,
            AudioFailure::RouteLost => vc_core::engine::AudioFailure::RouteLost,
            AudioFailure::EngineReset => vc_core::engine::AudioFailure::EngineReset,
            AudioFailure::Other => vc_core::engine::AudioFailure::Other,
        };

        let mut e = lock(&self.inner);
        self.rec(Record::Failed { cue_id, reason });
        e.on_cue_failed(cue_id, reason);
    }

    /// Day 12: the observer's latest verdict.
    pub fn last_outcome(&self) -> Option<CueOutcomeFfi> {
        None
    }

    /// Day 14: the ledger summary.
    pub fn report_card(&self) -> ReportCard {
        ReportCard {
            text: "No cues scored yet: the observer arrives on Day 12.".into(),
            cues_scored: 0,
        }
    }

    /// Between sessions only. protocol_hash is the STUDY.md commit (Day 15).
    pub fn set_mode(
        &self,
        mode: Mode,
        seed: u64,
        protocol_hash: Option<String>,
    ) -> Result<(), VcError> {
        let mode = match mode {
            Mode::Coach => CoreMode::Coach,
            Mode::Study => CoreMode::Study,
            Mode::Replay => CoreMode::Replay,
        };

        {
            let mut e = lock(&self.inner);
            self.rec(Record::Mode { mode, seed });
            e.set_mode(mode, seed)
                .map_err(|e| VcError::Config(e.to_string()))?;
        }

        *lock(&self.protocol_hash) = protocol_hash;
        Ok(())
    }

    /// Turn capture on before set_mode and start: the header (config + plan) is
    /// written now, and everything after is recorded.
    pub fn set_capture(&self, on: bool) {
        {
            let mut c = lock(&self.capture);
            c.on = on;
            if !on {
                return;
            }
            c.buf.clear();
            c.buf.reserve(1 << 20);
            c.buf.extend_from_slice(&MAGIC);
        }
        let cfg = self.cfg;
        self.rec(Record::Config {
            cp_w: cfg.cp_w,
            wprime_j: cfg.wprime_j,
            hr_max: cfg.hr_max,
            profile: cfg.profile,
            cooldown_s: cfg.cooldown_between_alerts_s,
            surge_lead_s: cfg.prompt_surge_countdown_s,
        });
        for s in &self.steps {
            self.rec(Record::Step(*s));
        }
    }

    /// Everything recorded since the last call. Call about once a second.
    pub fn take_capture(&self) -> Vec<u8> {
        let mut c = lock(&self.capture);
        let out = std::mem::take(&mut c.buf);
        c.buf.reserve(1 << 16);
        out
    }

    pub fn record_latency(&self, segment: LatencySegment, ns: u64) {
        if let Some(h) = lock(&self.latency).get_mut(slot(segment)) {
            h.record(ns);
        }
    }

    pub fn latency_summary(&self, segment: LatencySegment) -> LatencySummary {
        let all = lock(&self.latency);
        let Some(h) = all.get(slot(segment)) else {
            return LatencySummary {
                count: 0,
                p50_us: 0.0,
                p90_us: 0.0,
                p99_us: 0.0,
                max_us: 0.0,
            };
        };
        let us = |ns: u64| ns as f64 / 1_000.0;
        LatencySummary {
            count: h.count(),
            p50_us: us(h.quantile_ns(0.50)),
            p90_us: us(h.quantile_ns(0.90)),
            p99_us: us(h.quantile_ns(0.99)),
            max_us: us(h.max_ns()),
        }
    }

    pub fn reset_latency(&self) {
        for h in lock(&self.latency).iter_mut() {
            h.clear();
        }
    }

    /// Day 14: up to max ledger records as JSONL.
    pub fn drain_ledger_jsonl(&self, max: u32) -> String {
        let _ = max;
        String::new()
    }
}

impl Engine {
    fn rec(&self, r: Record<'_>) {
        let mut c = lock(&self.capture);
        if !c.on {
            return;
        }
        let mut tmp = [0u8; MAX_RECORD];
        if let Some(n) = capture::encode(&r, &mut tmp) {
            c.buf.extend_from_slice(&tmp[..n]);
        }
    }

    fn rec_decision(&self, d: &vc_core::CueDecision) {
        let (cue_id, directive, arm, decided_ns) = capture::key(d);
        self.rec(Record::Decision {
            cue_id,
            directive,
            arm,
            decided_ns,
        });
    }

    fn build(cfg: vc_core::EngineConfig, plan: WorkoutPlan) -> Result<Arc<Self>, VcError> {
        let steps = plan.steps().to_vec();
        let engine = vc_core::Engine::new(cfg, plan).map_err(|e| VcError::Config(e.to_string()))?;

        Ok(Arc::new(Self {
            inner: Mutex::new(engine),
            protocol_hash: Mutex::new(None),
            latency: Mutex::new(Default::default()),
            capture: Mutex::new(Recorder {
                on: false,
                buf: Vec::new(),
            }),
            cfg,
            steps,
        }))
    }
}

// ---- Free functions: protocol bytes and scenario metadata ------------------

fn ctrl_bytes(m: &Ctrl) -> Vec<u8> {
    let mut b = [0u8; MAX_PAYLOAD];
    encode_ctrl(m, &mut b).map_or_else(|_| Vec::new(), |n| b[..n].to_vec())
}

fn telem_bytes(m: &Telem) -> Vec<u8> {
    let mut b = [0u8; MAX_PAYLOAD];
    encode_telem(m, &mut b).map_or_else(|_| Vec::new(), |n| b[..n].to_vec())
}

/// iPhone -> iQOO, written right after a clip is scheduled (never for a sham).
#[uniffi::export]
pub fn encode_cue_played(d: CueDecision, dur_ms: u16) -> Vec<u8> {
    let delta_w = d
        .delta_star_w
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;

    let arm = match d.arm {
        Arm::None => CoreArm::None,
        Arm::Direct => CoreArm::Direct,
        Arm::Autonomy => CoreArm::Autonomy,
        Arm::Sham => CoreArm::Sham,
    };

    ctrl_bytes(&Ctrl::CuePlayed {
        cue_id: d.id,
        kind: d.directive_code,
        dir: d.dir,
        delta_w,
        arm: arm.code(),
        starts_in_ms: d.lead_ms,
        dur_ms,
    })
}

#[uniffi::export]
pub fn encode_load(scenario: u8, persona: u8, seed: u32, human: bool) -> Vec<u8> {
    ctrl_bytes(&Ctrl::Load {
        scenario,
        persona,
        seed,
        human,
    })
}

#[uniffi::export]
pub fn encode_start() -> Vec<u8> {
    ctrl_bytes(&Ctrl::Start)
}

#[uniffi::export]
pub fn encode_stop() -> Vec<u8> {
    ctrl_bytes(&Ctrl::Stop)
}

#[uniffi::export]
pub fn encode_ping(ping_id: u16, t_client_ns: u64) -> Vec<u8> {
    ctrl_bytes(&Ctrl::Ping {
        ping_id,
        t_client_ns,
    })
}

/// iQOO -> iPhone: the reply to PING, for clock sync (Day 15).
#[uniffi::export]
pub fn encode_pong(ping_id: u16, t_rx_ns: u64, t_tx_ns: u64) -> Vec<u8> {
    telem_bytes(&Telem::Pong {
        ping_id,
        t_rx_ns,
        t_tx_ns,
    })
}

/// iQOO -> iPhone in human mode (Day 15).
#[uniffi::export]
pub fn encode_rider_input(seq: u16, t_ns: u64, effort_permille: i16) -> Vec<u8> {
    telem_bytes(&Telem::RiderInput {
        seq,
        t_ns,
        effort_permille,
    })
}

#[uniffi::export]
pub fn scenarios() -> Vec<ScenarioInfo> {
    vc_sim::SCENARIOS
        .iter()
        .map(|s| ScenarioInfo {
            id: s.id,
            name: s.name.to_string(),
            default_persona: s.persona.name().to_string(),
            minutes: s.total_ns() as f64 / 60e9,
        })
        .collect()
}

// Keep the Channel import used even without the sim feature.
#[allow(dead_code)]
const _CHANNELS: [Channel; 3] = [Channel::Hrm, Channel::Cpm, Channel::Csc];
