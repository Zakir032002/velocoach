//! The Engine's half of the closed loop: which cues are watched, the power
//! ring, the observer, the ledger and the tuner. Fixed size; no allocation.
use vc_efficacy::{
    Arm, Catalogue, Confound, CueEvent, CueOutcome, Ledger, Mode, ObsContext, Observer, P3Ring,
    TunablePolicy, Tuner, TunerReport, Verdict,
};

use crate::decision::CueDecision;
use crate::directive::{Directive, Priority, SurgePhase};

const PENDING: usize = 4;
const P1_LOG: usize = 8;

pub const CATALOGUE: Catalogue<'static> = Catalogue {
    p2: &[
        Directive::WprimeLow.code(),
        Directive::SoftPedalWarning.code(),
        Directive::GreyZoneDrift.code(),
        Directive::CadenceLow.code(),
    ],
    p3: &[Directive::CardiacDrift.code()],
    step_transition: Directive::SurgePending.code(),
};

/// Is this decision scored by the observer?
pub fn observed(d: &CueDecision) -> bool {
    match d.priority {
        Priority::P2 | Priority::P3 => d.dir != 0,
        Priority::P1 => d.directive == Directive::SurgePending && d.surge == Some(SurgePhase::Go),
    }
}

fn event_of(d: &CueDecision, profile: u8) -> CueEvent {
    CueEvent {
        cue_id: d.id,
        directive: d.directive.code(),
        priority: d.priority as u8 + 1,
        aki_class: d.aki_class as u8,
        arm: d.arm,
        propensity_q16: d.propensity_q16,
        is_sham: d.arm == Arm::Sham,
        dir: d.dir,
        delta_star_w: d.delta_star_w,
        profile,
        t_decision_ns: d.decided_ns,
        t_onset_ns: 0,
        t_end_ns: 0,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Observed {
    pub event: CueEvent,
    pub outcome: CueOutcome,
    pub draw_index: Option<u32>,
}

/// What the Engine knows this call, besides the power ring.
#[derive(Debug, Clone, Copy)]
pub struct Now {
    pub t_ns: u64,
    pub power_fresh: bool,
    pub power_w: f64,
    pub cadence_rpm: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Hub {
    ring: P3Ring,
    observer: Observer,
    open_event: Option<(CueEvent, Option<u32>)>,
    ledger: Ledger,
    tuner: Tuner,
    pending: [Option<(CueEvent, Option<u32>)>; PENDING],
    p1_times: [u64; P1_LOG],
    p1_next: usize,
    last_power_bad_ns: Option<u64>,
    p1_flag: bool,
    interrupted_flag: bool,
    grade: Option<f64>,
    last_outcome: Option<CueOutcome>,
}

impl Default for Hub {
    fn default() -> Self {
        Self {
            ring: P3Ring::new(),
            observer: Observer::new(),
            open_event: None,
            ledger: Ledger::new(),
            tuner: Tuner::default(),
            pending: [None; PENDING],
            p1_times: [0; P1_LOG],
            p1_next: 0,
            last_power_bad_ns: None,
            p1_flag: false,
            interrupted_flag: false,
            grade: None,
            last_outcome: None,
        }
    }
}

impl Hub {
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    #[cfg(feature = "testkit")]
    pub fn ledger_mut(&mut self) -> &mut Ledger {
        &mut self.ledger
    }

    pub fn last_outcome(&self) -> Option<CueOutcome> {
        self.last_outcome
    }

    /// Mac and tests only: the true road grade from the simulator.
    pub fn set_grade(&mut self, g: Option<f64>) {
        self.grade = g;
    }

    /// A plausible power sample that arrived while power was Fresh.
    pub fn on_power(&mut self, t_ns: u64, watts: f64) {
        self.ring.push(t_ns, watts);
    }

    pub fn on_decision(&mut self, d: &CueDecision, profile: u8, lead_ns: u64, est_phrase_ns: u64) {
        if d.priority == Priority::P1 && !observed(d) {
            self.p1_times[self.p1_next % P1_LOG] = d.decided_ns;
            self.p1_next += 1;
            self.p1_flag = true;
        }
        if !observed(d) {
            return;
        }
        let mut ev = event_of(d, profile);
        if ev.is_sham {
            ev.t_onset_ns = d.decided_ns + lead_ns; // virtual: nothing plays
            ev.t_end_ns = ev.t_onset_ns + est_phrase_ns;
        }
        let slot = self.pending.iter().position(Option::is_none).unwrap_or(0); // full: the oldest is overwritten
        self.pending[slot] = Some((ev, d.draw_index));
    }

    pub fn on_cue_played(&mut self, cue_id: u32, onset_ns: u64, end_ns: u64) {
        if let Some((ev, _)) = self
            .pending
            .iter_mut()
            .flatten()
            .find(|(e, _)| e.cue_id == cue_id)
        {
            ev.t_onset_ns = onset_ns;
            ev.t_end_ns = end_ns.max(onset_ns);
        }
    }

    pub fn on_cue_failed(&mut self, cue_id: u32) -> Option<Observed> {
        if self.observer.open_cue() == Some(cue_id) {
            self.interrupted_flag = true;
            return None;
        }
        let slot = self
            .pending
            .iter_mut()
            .find(|p| p.is_some_and(|(e, _)| e.cue_id == cue_id))?;
        let (ev, draw) = slot.take()?;
        let o = invalid(&ev);
        Some(self.finish(&ev, o, draw))
    }

    /// After every ingest and tick. Returns an outcome when one is decided.
    pub fn update(&mut self, now: Now) -> Option<Observed> {
        if !now.power_fresh {
            self.last_power_bad_ns = Some(now.t_ns);
        }
        let mut late_quality = false;
        if !self.observer.is_open() {
            let ready = self
                .pending
                .iter()
                .position(|p| p.is_some_and(|(e, _)| e.t_end_ns > 0 && now.t_ns >= e.t_end_ns));
            if let Some((ev, draw)) = ready.and_then(|i| self.pending[i].take()) {
                // Replay what happened between the onset and now.
                self.p1_flag |= self
                    .p1_times
                    .iter()
                    .any(|&t| t >= ev.t_onset_ns && t <= now.t_ns && t > 0);
                late_quality = self.last_power_bad_ns.is_some_and(|t| t >= ev.t_onset_ns);
                match self.observer.open(&ev, &self.ring, self.grade) {
                    Some(o) => return Some(self.finish(&ev, o, draw)),
                    None => self.open_event = Some((ev, draw)),
                }
            }
        }
        let (ev, draw) = self.open_event?;
        let ctx = ObsContext {
            ring: &self.ring,
            power_fresh: now.power_fresh && !late_quality,
            power_w: now.power_w,
            cadence_rpm: now.cadence_rpm,
            grade_pct: self.grade,
            p1_fired: core::mem::take(&mut self.p1_flag),
            overlap: false, // the Policy Engine allows no second P2/P3 cue inside a window
            audio_interrupted: core::mem::take(&mut self.interrupted_flag),
            rider_move_ns: None, // human mode: Day 15
        };
        let o = self.observer.update(now.t_ns, &ctx)?;
        self.open_event = None;
        Some(self.finish(&ev, o, draw))
    }

    /// The ride stopped with a window open: it cannot be scored.
    pub fn abandon(&mut self) -> Option<Observed> {
        let (ev, draw) = self.open_event.take()?;
        let o = self.observer.abandon()?;
        Some(self.finish(&ev, o, draw))
    }

    /// At a step boundary: the tuner (Coach only) over this profile's ledger.
    pub fn boundary(&mut self, mode: Mode, profile: u8, p: &mut TunablePolicy) -> TunerReport {
        self.tuner.step(&self.ledger, p, mode, profile, &CATALOGUE)
    }

    fn finish(&mut self, ev: &CueEvent, o: CueOutcome, draw_index: Option<u32>) -> Observed {
        self.ledger.record(ev, &o);
        self.last_outcome = Some(o);
        Observed {
            event: *ev,
            outcome: o,
            draw_index,
        }
    }
}

fn invalid(ev: &CueEvent) -> CueOutcome {
    CueOutcome {
        cue_id: ev.cue_id,
        verdict: Verdict::Invalid,
        confound: Confound::default(),
        coincidental: false,
        l_obs_ms: None,
        l_onset_ms: None,
        l_end_ms: None,
        pre_slope_w_per_s: 0.0,
        slope_clamped: false,
        effect_max_w: 0.0,
    }
}
