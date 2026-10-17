//! vc-ffi: the UniFFI surface (FFI v0, frozen on Day 8).
//! Thin and copying: no logic lives here, only conversions between
//! vc-core and plain records.
uniffi::setup_scaffolding!();

use std::sync::{Arc, Mutex, MutexGuard};

use vc_core::capture::{self, MAGIC, MAX_RECORD, Record};
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

/// The observer's verdict for the on-screen badge.
#[derive(Debug, Clone, uniffi::Record)]
pub struct CueOutcomeFfi {
    pub cue_id: u32,
    pub directive: String,
    pub arm: String,
    pub verdict: String,
    pub l_obs_ms: Option<i32>,
    pub l_onset_ms: Option<i32>,
    pub coincidental: bool,
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

/// T1->T2 (Day 11); Day 15 adds T0->T1, T1->T3 and T0->T3.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum LatencySegment {
    T1T2,
    T0T1,
    T1T3,
    T0T3,
}

fn segment(s: LatencySegment) -> vc_core::engine::Segment {
    use vc_core::engine::Segment as S;
    match s {
        LatencySegment::T1T2 => S::T1T2,
        LatencySegment::T0T1 => S::T0T1,
        LatencySegment::T1T3 => S::T1T3,
        LatencySegment::T0T3 => S::T0T3,
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LatencySummary {
    pub count: u64,
    pub p50_us: f64,
    pub p90_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LinkStats {
    pub stamps: u32,
    pub stamps_lost: u32,
    pub negative_t0t1: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ClockStatus {
    pub samples: u32,
    pub rtt_min_us: Option<f64>,
    /// +/- rtt_min / 2
    pub bound_us: Option<f64>,
    /// Rig clock minus phone clock.
    pub theta_ms: Option<f64>,
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
    capture: Mutex<Recorder>,
    cfg: vc_core::EngineConfig,
    steps: Vec<Step>,
    writer: Mutex<vc_report::Writer>,
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

    /// The observer's latest verdict, with its directive and arm, for the badge.
    pub fn last_outcome(&self) -> Option<CueOutcomeFfi> {
        let o = lock(&self.inner).last_observed()?;
        Some(CueOutcomeFfi {
            cue_id: o.outcome.cue_id,
            directive: vc_core::Directive::from_code(o.event.directive)
                .map_or("?", vc_core::Directive::name)
                .to_string(),
            arm: o.event.arm.name().to_string(),
            verdict: o.outcome.verdict.name().to_string(),
            l_obs_ms: o.outcome.l_obs_ms,
            l_onset_ms: o.outcome.l_onset_ms,
            coincidental: o.outcome.coincidental,
            confounds: vc_efficacy::Confound::ALL
                .iter()
                .filter(|(c, _)| o.outcome.confound.contains(*c))
                .map(|(_, n)| (*n).to_string())
                .collect(),
        })
    }

    /// The ledger as text: the phone shows it as-is.
    pub fn report_card(&self) -> ReportCard {
        let e = lock(&self.inner);
        let (mut scored, mut complied, mut confounded, mut invalid) = (0u32, 0u32, 0u32, 0u32);
        let mut rows = Vec::new();
        for s in e.ledger().iter() {
            let c = s.counts;
            scored += c.n();
            complied += c.complied;
            confounded += c.confounded;
            invalid += c.invalid;
            let name = vc_core::Directive::from_code(s.key.directive)
                .map_or("?", vc_core::Directive::name);
            let median = s.median_l_obs_ms.map_or(String::new(), |m| {
                format!(", median L_obs {:.1} s", f64::from(m) / 1000.0)
            });
            rows.push(format!(
                "{name} · {}: {}/{} complied{median}",
                s.key.arm.name(),
                c.complied,
                c.n()
            ));
        }
        rows.sort();
        let pct = if scored > 0 {
            100.0 * f64::from(complied) / f64::from(scored)
        } else {
            0.0
        };
        let tuning = match e.last_tuning() {
            Some(r) if !r.is_empty() => {
                format!(
                    "\nLast tuning: {} change(s) at a step boundary.",
                    r.iter().count()
                )
            }
            _ => String::new(),
        };
        ReportCard {
            text: format!(
                "{scored} cues scored, {complied} complied ({pct:.0} %). Excluded: {confounded} confounded, {invalid} invalid.\n{}{tuning}",
                rows.join("\n")
            ),
            cues_scored: scored,
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

        lock(&self.writer).protocol_hash = protocol_hash.clone();
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
        lock(&self.inner).record_latency(crate::segment(segment), ns);
    }

    pub fn latency_summary(&self, segment: LatencySegment) -> LatencySummary {
        let e = lock(&self.inner);
        let h = e.latency(crate::segment(segment));
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
        lock(&self.inner).reset_latency();
    }

    /// A PING is about to be written at `t_ns`. Returns its id for encode_ping.
    pub fn make_ping(&self, t_ns: u64) -> u16 {
        let mut e = lock(&self.inner);
        self.rec(Record::Ping { t_ns });
        e.make_ping(t_ns)
    }

    /// A (re)connection: new clock samples, new packet counters.
    pub fn reset_link(&self) {
        let mut e = lock(&self.inner);
        self.rec(Record::ResetLink);
        e.reset_link();
    }

    pub fn clock_status(&self) -> ClockStatus {
        let e = lock(&self.inner);
        let c = e.clock();
        let best = c.best();
        ClockStatus {
            samples: c.samples() as u32,
            rtt_min_us: best.map(|s| s.rtt_ns as f64 / 1e3),
            bound_us: best.map(|s| s.rtt_ns as f64 / 2e3),
            theta_ms: best.map(|s| s.theta_ns as f64 / 1e6),
        }
    }

    /// The platform scheduled the clip: T2 (ingest/tick returned) and T3 (render start).
    pub fn on_cue_scheduled(&self, cue_id: u32, t2_ns: u64, t3_ns: u64) {
        let mut e = lock(&self.inner);
        self.rec(Record::Scheduled {
            cue_id,
            t2_ns,
            t3_ns,
        });
        e.on_cue_scheduled(cue_id, t2_ns, t3_ns);
    }

    pub fn link_stats(&self) -> LinkStats {
        let e = lock(&self.inner);
        let c = e.chain();
        LinkStats {
            stamps: c.stamps,
            stamps_lost: c.stamps_lost,
            negative_t0t1: c.negative_t0t1,
        }
    }

    /// After the d_route loopback: onsets are `loopback`, not `scheduled`.
    pub fn set_route_delay_known(&self, known: bool) {
        lock(&self.inner).set_route_delay_known(known);
    }

    /// Up to `max` ledger records as JSONL (one per line). Allocates: call it at
    /// about 1 Hz, never from inside ingest.
    pub fn drain_ledger_jsonl(&self, max: u32) -> String {
        let mut buf = [vc_core::LedgerRecord::SessionEnd {
            t_ns: 0,
            cues: 0,
            dropped: 0,
        }; 32];
        let mut out = String::new();
        let mut left = max as usize;
        let mut writer = lock(&self.writer);
        while left > 0 {
            let n = lock(&self.inner).ledger_pop(&mut buf[..left.min(32)]);
            if n == 0 {
                break;
            }
            for r in &buf[..n] {
                out.push_str(&vc_report::to_json(&writer.line(r)));
                out.push('\n');
            }
            left -= n;
        }
        out
    }

    /// The pseudonym written on every record ("R01" ... "R03").
    pub fn set_rider(&self, rider_id: String) {
        lock(&self.writer).rider_id = rider_id;
    }

    /// The provenance written on the next session_start record and printed on vc-report's
    /// banner, e.g. "simulated persona=calib seed=42 link=ble". Set before `start`.
    pub fn set_data_label(&self, label: String) {
        lock(&self.writer).data = Some(label);
    }

    pub fn end_session(&self, t_ns: u64) {
        lock(&self.inner).end_session(t_ns);
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
            capture: Mutex::new(Recorder {
                on: false,
                buf: Vec::new(),
            }),
            cfg,
            steps,
            writer: Mutex::new(vc_report::Writer::new("R01", None)),
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

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum RateChannel {
    All,
    Hrm,
    Cpm,
    Csc,
}

#[uniffi::export]
pub fn encode_rate(channel: RateChannel, hz_x10: u8) -> Vec<u8> {
    let channel = match channel {
        RateChannel::All => None,
        RateChannel::Hrm => Some(Channel::Hrm),
        RateChannel::Cpm => Some(Channel::Cpm),
        RateChannel::Csc => Some(Channel::Csc),
    };
    ctrl_bytes(&Ctrl::Rate { channel, hz_x10 })
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum FaultKind {
    Dropout,
    Garbage,
    Disconnect,
    Coast,
    HrRedline,
    WprimeMismatch,
}

/// iPhone -> iQOO: inject a fault (the debug "HR redline at __ s" field uses it).
#[uniffi::export]
pub fn encode_fault(kind: FaultKind, channel: RateChannel, dur_ms: u32, param: i16) -> Vec<u8> {
    let kind = match kind {
        FaultKind::Dropout => vc_gatt::FaultKind::Dropout,
        FaultKind::Garbage => vc_gatt::FaultKind::Garbage,
        FaultKind::Disconnect => vc_gatt::FaultKind::Disconnect,
        FaultKind::Coast => vc_gatt::FaultKind::Coast,
        FaultKind::HrRedline => vc_gatt::FaultKind::HrRedline,
        FaultKind::WprimeMismatch => vc_gatt::FaultKind::WprimeMismatch,
    };
    let channel = match channel {
        RateChannel::All => None,
        RateChannel::Hrm => Some(Channel::Hrm),
        RateChannel::Cpm => Some(Channel::Cpm),
        RateChannel::Csc => Some(Channel::Csc),
    };
    ctrl_bytes(&Ctrl::Fault {
        kind,
        channel,
        dur_ms,
        param,
    })
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
