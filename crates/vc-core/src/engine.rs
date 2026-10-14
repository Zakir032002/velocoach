//! The Engine: all state for one ride.
//! ingest() handles every BLE packet; tick() runs four times per second.
//! Each returns at most one CueDecision.

use crate::config::Profile;
use crate::policy::Policy;
use crate::safety::RedlineGate;
use vc_efficacy::Mode;
use vc_gatt::{Channel, Telem, decode_telem};
use vc_gatt::{DecodeError, decode_cpm, decode_csc, decode_hrm};
use vc_physio::{
    CadenceDeriver, CadenceUpdate, Quality, QualityTracker, WbalExact, WbalParams, WprimeBalance,
};

use crate::chain::Chains;
use crate::clock::ClockSync;
use crate::config::EngineConfig;
use crate::decision::CueDecision;
use crate::directive::{Candidate, Candidates, Directive, Directives, Inputs};
use crate::efficacy::{Hub, Now, Observed};
use crate::latency::Histogram;
use crate::ledger_ring::{LedgerRecord, LedgerRing, OnsetSource};
use crate::smooth::Smooth3s;
use crate::workout::{Progress, WorkoutPlan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Hr,
    Power,
    Csc,
    Telem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineError {
    Decode(DecodeError),
    NonMonotonicTime { prev: u64, now: u64 },
    Config(&'static str),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioFailure {
    Interrupted,
    RouteLost,
    EngineReset,
    Other,
}

/// One clip's real timing, for the observer (Day 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioEvent {
    pub cue_id: u32,
    pub onset_ns: u64,
    pub end_ns: u64,
    pub callback_ns: u64,
    pub failed: Option<AudioFailure>,
}

impl core::fmt::Display for EngineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(e) => write!(f, "decode: {e}"),
            Self::NonMonotonicTime { prev, now } => {
                write!(f, "time went backwards: {prev} -> {now}")
            }
            Self::Config(reason) => write!(f, "config: {reason}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ch {
    Hr = 0,
    Power = 1,
    Cadence = 2,
}

const CHANNELS: [Ch; 3] = [Ch::Hr, Ch::Power, Ch::Cadence];

/// State copied out for the UI, typically four times per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub power_w: f64,
    pub p3_w: Option<f64>,
    pub hr_bpm: u16,
    pub cadence_rpm: f64,
    pub wbal_fraction: f64,
    pub hr_q: Quality,
    pub power_q: Quality,
    pub cadence_q: Quality,
    pub step_index: Option<usize>,
    pub step_remaining_s: f64,
    pub paused: bool,
    pub decode_errors: u32,
    pub implausible: u32,
    pub last: Option<CueDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    T0T1 = 0,
    T1T2 = 1,
    T1T3 = 2,
    T0T3 = 3,
}

#[derive(Debug, Clone)]
pub struct Engine {
    cfg: EngineConfig,
    plan: WorkoutPlan,
    progress: Progress,
    last_t: u64,
    q: [QualityTracker; 3],
    hr_bpm: u16,
    power_w: f64,
    p3: Smooth3s,
    cadence: CadenceDeriver,
    wbal: WbalExact,
    redline: RedlineGate,
    directives: Directives,
    candidates: Candidates,
    policy: Policy,
    power_unfresh_since: Option<u64>,
    power_ever_fresh: bool,
    decode_errors: u32,
    implausible: u32,
    last: Option<CueDecision>,
    mode: Mode,
    last_audio: Option<AudioEvent>,
    hub: Hub,
    last_step: Option<usize>,
    last_tuning: Option<vc_efficacy::TunerReport>,
    ring: LedgerRing,
    mode_seed: u64,
    route_delay_known: bool,
    clock: ClockSync,
    chain: Chains,
    lat: [Histogram; 4],
    trigger: Option<(Channel, u16)>,
    rider_inputs_unplaced: u32,
}

impl Engine {
    pub fn new(cfg: EngineConfig, plan: WorkoutPlan) -> Result<Self, EngineError> {
        cfg.validate().map_err(EngineError::Config)?;

        let par = WbalParams::new(cfg.cp_w, cfg.wprime_j)
            .ok_or(EngineError::Config("cp_w and wprime_j must be positive"))?;

        Ok(Self {
            plan,
            progress: Progress::default(),
            last_t: 0,
            q: core::array::from_fn(|_| QualityTracker::new(cfg.quality)),
            hr_bpm: 0,
            power_w: 0.0,
            p3: Smooth3s::default(),
            cadence: CadenceDeriver::new(),
            wbal: WbalExact::new(par),
            redline: RedlineGate::new(&cfg),
            directives: Directives::new(&cfg),
            candidates: Candidates::default(),
            policy: Policy::new(&cfg),
            power_unfresh_since: None,
            power_ever_fresh: false,
            decode_errors: 0,
            implausible: 0,
            last: None,
            cfg,
            mode: Mode::Coach,
            last_audio: None,
            hub: Hub::default(),
            last_step: None,
            last_tuning: None,
            ring: LedgerRing::default(),
            mode_seed: 0,
            route_delay_known: false,
            clock: ClockSync::default(),
            chain: Chains::default(),
            lat: Default::default(),
            trigger: None,
            rider_inputs_unplaced: 0,
        })
    }

    pub fn config(&self) -> &EngineConfig {
        &self.cfg
    }

    pub fn plan(&self) -> &WorkoutPlan {
        &self.plan
    }

    /// Start the workout clock: the plan's second zero.
    pub fn start(&mut self, t_ns: u64) {
        self.progress.start(t_ns);
        self.ring.push(LedgerRecord::SessionStart {
            t_ns,
            mode: self.mode,
            seed: self.mode_seed,
            policy_hash: self.policy.hash(),
            profile: self.profile_code(),
        });
    }

    /// Consume one GATT payload and return at most one decision.
    pub fn ingest(
        &mut self,
        t_ns: u64,
        source: Source,
        bytes: &[u8],
    ) -> Result<Option<CueDecision>, EngineError> {
        self.check_time(t_ns)?;

        match source {
            Source::Hr => self.on_hr(t_ns, bytes)?,
            Source::Power => self.on_power(t_ns, bytes)?,

            // Cadence comes from the power meter's crank data.
            // CSC packets are decoded for validation here.
            Source::Csc => {
                self.trigger = Some((Channel::Csc, self.chain.on_packet(Channel::Csc, t_ns)));
                if let Err(e) = decode_csc(bytes) {
                    return Err(self.bad(e));
                }
            }

            // TELEM carries PONG, STAMP and RIDER_INPUT.
            Source::Telem => {
                self.trigger = None; // not a sensor packet: it cannot trigger a cue
                match decode_telem(bytes).map_err(|e| self.bad(e))? {
                    Telem::Pong {
                        ping_id,
                        t_rx_ns,
                        t_tx_ns,
                    } => {
                        let _ = self.clock.on_pong(ping_id, t_rx_ns, t_tx_ns, t_ns);
                    }
                    Telem::Stamp {
                        seq,
                        channel,
                        char_seq,
                        t0_ns,
                    } => self.on_stamp(seq, channel, char_seq, t0_ns),
                    Telem::RiderInput {
                        t_ns: rig_t,
                        effort_permille,
                        ..
                    } => self.on_rider_input(rig_t, effort_permille),
                    _ => {}
                }
            }
        }

        Ok(self.evaluate_and_decide(t_ns))
    }

    /// Call four times per second for time-based transitions.
    pub fn tick(&mut self, t_ns: u64) -> Option<CueDecision> {
        self.check_time(t_ns).ok()?;
        self.trigger = None; // tick-released: no packet

        for ch in CHANNELS {
            let _ = self.q[ch as usize].on_tick(t_ns);
        }

        // Detect coasting when cadence notifications stop.
        self.cadence.tick(t_ns);

        self.evaluate_and_decide(t_ns)
    }

    /// The audio player reports the actual end of a clip.
    pub fn on_audio_finished(&mut self, cue_id: u32, t_ns: u64) {
        self.on_cue_played(cue_id, t_ns, t_ns, t_ns);
    }

    pub fn snapshot(&self) -> Snapshot {
        let step = self
            .progress
            .elapsed_ns()
            .and_then(|elapsed| self.plan.view(elapsed));

        Snapshot {
            power_w: self.power_w,
            p3_w: self.p3.mean_at(self.last_t),
            hr_bpm: self.hr_bpm,
            cadence_rpm: self.cadence.rpm(),
            wbal_fraction: self.wbal.fraction(),
            hr_q: self.q[Ch::Hr as usize].state(),
            power_q: self.q[Ch::Power as usize].state(),
            cadence_q: self.q[Ch::Cadence as usize].state(),
            step_index: step.map(|s| s.index),
            step_remaining_s: step.map_or(0.0, |s| s.remaining_ns as f64 / 1e9),
            paused: self.progress.paused(),
            decode_errors: self.decode_errors,
            implausible: self.implausible,
            last: self.last,
        }
    }

    // ---- Per-packet paths ---------------------------------------------

    fn on_hr(&mut self, t: u64, bytes: &[u8]) -> Result<(), EngineError> {
        self.trigger = Some((Channel::Hrm, self.chain.on_packet(Channel::Hrm, t)));
        let m = match decode_hrm(bytes) {
            Ok(m) => m,
            Err(e) => {
                self.sample(Ch::Hr, t, false);
                return Err(self.bad(e));
            }
        };

        let (lo, hi) = self.cfg.hr_valid_bpm;
        let ok = (lo..=hi).contains(&m.bpm);

        if !ok {
            self.implausible += 1;
        }

        self.sample(Ch::Hr, t, ok);

        if ok {
            self.hr_bpm = m.bpm;

            let fresh = self.fresh(Ch::Hr);

            if self.redline.on_hr(t, f64::from(m.bpm), fresh) {
                self.candidates
                    .offer(Candidate::new(Directive::SafetyHr, -1, 0.0, t));
            }
        }

        Ok(())
    }

    fn on_power(&mut self, t: u64, bytes: &[u8]) -> Result<(), EngineError> {
        self.trigger = Some((Channel::Cpm, self.chain.on_packet(Channel::Cpm, t)));
        let m = match decode_cpm(bytes) {
            Ok(m) => m,
            Err(e) => {
                self.sample(Ch::Power, t, false);
                return Err(self.bad(e));
            }
        };

        let w = f64::from(m.power_w);
        let (lo, hi) = self.cfg.power_valid_w;
        let ok = (lo..=hi).contains(&w);

        if !ok {
            self.implausible += 1;
        }

        self.sample(Ch::Power, t, ok);

        if ok {
            // Valid negative power is clamped to zero for physiology.
            self.power_w = w.max(0.0);
            self.p3.push(t, self.power_w);

            // WbalExact derives elapsed time from the timestamps.
            self.wbal.update(t, self.power_w);
        }

        if ok && self.fresh(Ch::Power) {
            self.hub.on_power(t, self.power_w);
        }

        if let Some(c) = m.crank {
            let good = !matches!(
                self.cadence.push(c.revs, c.time_1024, t),
                CadenceUpdate::Garbage
            );

            self.sample(Ch::Cadence, t, good);
        }

        Ok(())
    }

    // ---- Shared evaluation path ---------------------------------------

    fn evaluate_and_decide(&mut self, t: u64) -> Option<CueDecision> {
        self.update_power_freshness(t);

        // No wheel-speed sensor: missing power is not proof of stopping.
        // Use Fresh low power plus stopped cranks as the auto-pause proxy.
        let stopped = self.fresh(Ch::Power)
            && self.power_w < self.cfg.min_power_for_cadence_w
            && self.cadence.rpm() <= 0.0;

        self.progress.update(t, !stopped, self.cfg.auto_pause_ns);

        let inputs = self.inputs(t);

        self.directives
            .evaluate(&inputs, &self.cfg, &mut self.candidates);

        let decision = self.policy.decide(t, &mut self.candidates);
        if let Some(d) = decision {
            self.last = Some(d);
            self.chain.on_decision(d.id, self.trigger.take(), t);
            let lead_ns = u64::from(d.lead_ms) * 1_000_000;
            self.hub
                .on_decision(&d, self.profile_code(), lead_ns, self.cfg.est_phrase_ns);
        }
        if let Some(obs) = self.hub.update(Now {
            t_ns: t,
            power_fresh: self.fresh(Ch::Power),
            power_w: self.power_w,
            cadence_rpm: self.fresh(Ch::Cadence).then(|| self.cadence.rpm()),
        }) {
            self.push_cue(obs);
        }
        self.at_step_boundary(inputs.step.map(|s| s.index));
        decision
    }

    // 3. the boundary
    fn at_step_boundary(&mut self, step: Option<usize>) {
        if step == self.last_step {
            return;
        }
        let first = self.last_step.is_none();
        self.last_step = step;
        if first {
            return; // the ride's first step is not a boundary
        }
        let mut t = self.policy.tunable(
            self.cfg.cooldown_between_alerts_s,
            self.cfg.prompt_surge_countdown_s,
        );
        let report = self.hub.boundary(self.mode, self.profile_code(), &mut t);
        if !report.is_empty() {
            self.cfg.cooldown_between_alerts_s = t.cooldown_s;
            self.cfg.prompt_surge_countdown_s = t.surge_lead_s; // the directives read it from cfg
            self.policy.apply_tunable(&t);
            self.last_tuning = Some(report);
            for change in report.iter() {
                self.ring.push(LedgerRecord::TunerChange {
                    t_ns: self.last_t,
                    change: *change,
                    policy_hash_after: self.policy.hash(),
                });
            }
        }
    }

    fn push_cue(&mut self, o: Observed) {
        let onset_source = if o.event.is_sham {
            OnsetSource::Virtual
        } else if self.route_delay_known {
            OnsetSource::Loopback
        } else {
            OnsetSource::Scheduled
        };
        let chain = self.chain.get(o.event.cue_id).unwrap_or_default();
        self.ring.push(LedgerRecord::Cue {
            event: o.event,
            outcome: o.outcome,
            draw_index: o.draw_index,
            onset_source,
            sync_bound_us: chain.bound_ns.map_or(0, |b| (b / 1_000) as u32),
            chain,
            dropped_before: 0, // filled in by the ring
        });
    }

    /// The ride is over: close any open window and mark the end.
    pub fn end_session(&mut self, t_ns: u64) {
        if let Some(obs) = self.hub.abandon() {
            self.push_cue(obs);
        }
        self.ring.push(LedgerRecord::SessionEnd {
            t_ns,
            cues: self.ring.cues(),
            dropped: self.ring.dropped(),
        });
    }

    /// Drain up to `out.len()` ledger records. Never allocates.
    pub fn ledger_pop(&mut self, out: &mut [LedgerRecord]) -> usize {
        self.ring.pop_into(out)
    }

    fn profile_code(&self) -> u8 {
        match self.cfg.profile {
            Profile::Solo => 0,
            Profile::Rolling => 1,
            Profile::Group => 2,
        }
    }

    fn inputs(&self, t: u64) -> Inputs {
        Inputs {
            t_ns: t,
            p3_w: if self.fresh(Ch::Power) {
                self.p3.mean_at(t)
            } else {
                None
            },
            hr_bpm: self.fresh(Ch::Hr).then_some(f64::from(self.hr_bpm)),
            cadence_rpm: self.fresh(Ch::Cadence).then(|| self.cadence.rpm()),
            wbal_fraction: self.wbal.fraction(),
            power_unfresh_ns: self.power_unfresh_since.map(|s| t.saturating_sub(s)),
            paused: self.progress.paused(),
            step: self
                .progress
                .elapsed_ns()
                .and_then(|elapsed| self.plan.view(elapsed)),
        }
    }

    fn update_power_freshness(&mut self, t: u64) {
        if self.fresh(Ch::Power) {
            self.power_unfresh_since = None;
            self.power_ever_fresh = true;
        } else if self.power_ever_fresh {
            // A dropout, not the beginning of a ride.
            self.power_unfresh_since.get_or_insert(t);
        }
    }

    fn sample(&mut self, ch: Ch, t: u64, ok: bool) {
        let _ = self.q[ch as usize].on_sample(t, ok);
    }

    fn fresh(&self, ch: Ch) -> bool {
        self.q[ch as usize].is_fresh()
    }

    fn bad(&mut self, e: DecodeError) -> EngineError {
        self.decode_errors += 1;
        EngineError::Decode(e)
    }

    fn check_time(&mut self, t: u64) -> Result<(), EngineError> {
        if t < self.last_t {
            return Err(EngineError::NonMonotonicTime {
                prev: self.last_t,
                now: t,
            });
        }

        self.last_t = t;
        Ok(())
    }
    /// Between sessions only. Study fixes the profile to Solo and starts
    /// the allocator.
    pub fn set_mode(&mut self, mode: Mode, seed: u64) -> Result<(), EngineError> {
        if self.progress.elapsed_ns().is_some() {
            return Err(EngineError::Config(
                "set_mode is only allowed between sessions",
            ));
        }

        if mode == Mode::Study {
            self.cfg.profile = Profile::Solo;
            self.policy = Policy::new(&self.cfg);
        }

        self.policy.set_mode(mode, seed);
        self.mode = mode;
        self.mode_seed = seed;
        Ok(())
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn policy_hash(&self) -> u64 {
        self.policy.hash()
    }

    /// The audio player reports a clip's real onset and end (both at the ear).
    pub fn on_cue_played(
        &mut self,
        cue_id: u32,
        t_onset_ns: u64,
        t_end_ns: u64,
        t_callback_ns: u64,
    ) {
        self.last_audio = Some(AudioEvent {
            cue_id,
            onset_ns: t_onset_ns,
            end_ns: t_end_ns,
            callback_ns: t_callback_ns,
            failed: None,
        });

        self.policy.audio_finished(cue_id, t_end_ns);
        self.hub.on_cue_played(cue_id, t_onset_ns, t_end_ns);
    }

    /// The clip did not play to the end: the observer will mark it Invalid.
    pub fn on_cue_failed(&mut self, cue_id: u32, reason: AudioFailure) {
        let t = self.last_t;

        self.last_audio = Some(AudioEvent {
            cue_id,
            onset_ns: 0,
            end_ns: t,
            callback_ns: t,
            failed: Some(reason),
        });

        self.policy.audio_finished(cue_id, t);
        if let Some(obs) = self.hub.on_cue_failed(cue_id) {
            self.push_cue(obs);
        }
    }

    pub fn last_audio(&self) -> Option<AudioEvent> {
        self.last_audio
    }

    /// A PING is about to be written at `t_ns`. Returns its id for encode_ping.
    pub fn make_ping(&mut self, t_ns: u64) -> u16 {
        self.clock.make_ping(t_ns)
    }

    /// A (re)connection: new clock samples, new packet counters.
    pub fn reset_link(&mut self) {
        self.clock.reset();
        self.chain.reset();
    }

    pub fn clock(&self) -> &ClockSync {
        &self.clock
    }

    fn on_stamp(&mut self, stamp_seq: u16, channel: Channel, char_seq: u16, t0_rig: u64) {
        let t0 = self.clock.to_phone(t0_rig);
        let bound = self.clock.bound_ns();
        if let Some((t1, t0)) = self.chain.on_stamp(stamp_seq, channel, char_seq, t0, bound) {
            self.lat[Segment::T0T1 as usize].record(t1.saturating_sub(t0));
        }
        self.count_complete();
    }

    /// The platform scheduled the clip: T2 (ingest/tick returned) and T3 (render start).
    pub fn on_cue_scheduled(&mut self, cue_id: u32, t2_ns: u64, t3_ns: u64) {
        self.chain.on_scheduled(cue_id, t2_ns, t3_ns);
        if let Some(c) = self.chain.get(cue_id)
            && let Some(t1) = c.t1_ns
        {
            self.lat[Segment::T1T3 as usize].record(t3_ns.saturating_sub(t1));
        }
        self.count_complete();
    }

    fn count_complete(&mut self) {
        while let Some(c) = self.chain.take_complete() {
            if let (Some(t0), Some(t3)) = (c.t0_ns, c.t3_ns) {
                self.lat[Segment::T0T3 as usize].record(t3.saturating_sub(t0));
            }
        }
    }

    pub fn record_latency(&mut self, s: Segment, ns: u64) {
        self.lat[s as usize].record(ns);
    }

    pub fn latency(&self, s: Segment) -> &Histogram {
        &self.lat[s as usize]
    }

    pub fn reset_latency(&mut self) {
        for h in &mut self.lat {
            h.clear();
        }
    }

    pub fn chain(&self) -> &Chains {
        &self.chain
    }

    /// The platform set the route delay from the loopback: onsets are now `loopback`.
    pub fn set_route_delay_known(&mut self, known: bool) {
        self.route_delay_known = known;
    }

    /// A slider input from the rig (RIDER_INPUT), placed in time by the clock sync.
    fn on_rider_input(&mut self, rig_t_ns: u64, effort_permille: i16) {
        match self.clock.to_phone(rig_t_ns) {
            Some(t) => self.hub.on_rider_input(t, effort_permille),
            None => self.rider_inputs_unplaced += 1, // no clock sample yet: cannot place it in time
        }
    }

    /// RIDER_INPUTs that arrived before the clock had a sample.
    pub fn rider_inputs_unplaced(&self) -> u32 {
        self.rider_inputs_unplaced
    }

    // 5. accessors
    pub fn last_outcome(&self) -> Option<vc_efficacy::CueOutcome> {
        self.hub.last_outcome()
    }

    /// The last observed cue with its event (for the badge).
    pub fn last_observed(&self) -> Option<Observed> {
        self.hub.last_observed()
    }

    pub fn ledger(&self) -> &vc_efficacy::Ledger {
        self.hub.ledger()
    }

    pub fn last_tuning(&self) -> Option<vc_efficacy::TunerReport> {
        self.last_tuning
    }

    pub fn set_grade(&mut self, grade_pct: Option<f64>) {
        self.hub.set_grade(grade_pct)
    }

    #[cfg(feature = "testkit")]
    pub fn hub_mut(&mut self) -> &mut Hub {
        &mut self.hub
    }
}
