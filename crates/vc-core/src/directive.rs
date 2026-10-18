//! The eight directives, named as Aki names them. Each evaluator only offers
//! a candidate; the Policy Engine decides what is spoken.

use crate::config::EngineConfig;
use crate::hysteresis::Hysteresis;
use crate::workout::{StepKind, StepView};

const S: u64 = 1_000_000_000;

/// Our delivery tiers. Declaration order is urgency: P1 first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    /// Safety and step transitions: never muted, may interrupt.
    P1,
    /// Performance.
    P2,
    /// Information.
    P3,
}

/// Aki's P0-P5 classes, recorded beside our tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AkiClass {
    P0,
    P1,
    P2,
    P3,
    P4,
    P5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurgePhase {
    /// "Surge in five": the lead in whole seconds (5-8).
    Countdown(u8),
    /// "Go now" at the interval's first second.
    Go,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Directive {
    WprimeLow = 1,
    SoftPedalWarning = 2,
    GreyZoneDrift = 3,
    CadenceLow = 4,
    CardiacDrift = 16,
    SafetyHr = 32,
    SensorDropout = 33,
    SurgePending = 40,
}

impl Directive {
    pub const COUNT: usize = 8;

    pub const ALL: [Self; Self::COUNT] = [
        Self::SafetyHr,
        Self::SensorDropout,
        Self::SurgePending,
        Self::WprimeLow,
        Self::SoftPedalWarning,
        Self::GreyZoneDrift,
        Self::CadenceLow,
        Self::CardiacDrift,
    ];

    /// Index of this directive's candidate slot.
    pub const fn slot(self) -> usize {
        match self {
            Self::SafetyHr => 0,
            Self::SensorDropout => 1,
            Self::SurgePending => 2,
            Self::WprimeLow => 3,
            Self::SoftPedalWarning => 4,
            Self::GreyZoneDrift => 5,
            Self::CadenceLow => 6,
            Self::CardiacDrift => 7,
        }
    }

    /// Code carried in CUE_PLAYED and the ledger.
    pub const fn code(self) -> u8 {
        self as u8
    }

    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::WprimeLow),
            2 => Some(Self::SoftPedalWarning),
            3 => Some(Self::GreyZoneDrift),
            4 => Some(Self::CadenceLow),
            16 => Some(Self::CardiacDrift),
            32 => Some(Self::SafetyHr),
            33 => Some(Self::SensorDropout),
            40 => Some(Self::SurgePending),
            _ => None,
        }
    }

    /// Aki's spelling, used in golden logs and the ledger.
    pub const fn name(self) -> &'static str {
        match self {
            Self::WprimeLow => "WPRIME_LOW",
            Self::SoftPedalWarning => "SOFT_PEDAL_WARNING",
            Self::GreyZoneDrift => "GREY_ZONE_DRIFT",
            Self::CadenceLow => "CADENCE_LOW",
            Self::CardiacDrift => "CARDIAC_DRIFT",
            Self::SafetyHr => "SAFETY_HR",
            Self::SensorDropout => "SENSOR_DROPOUT",
            Self::SurgePending => "SURGE_PENDING",
        }
    }

    pub const fn priority(self) -> Priority {
        match self {
            Self::SafetyHr | Self::SensorDropout | Self::SurgePending => Priority::P1,
            Self::WprimeLow | Self::SoftPedalWarning | Self::GreyZoneDrift | Self::CadenceLow => {
                Priority::P2
            }
            Self::CardiacDrift => Priority::P3,
        }
    }

    pub const fn aki_class(self) -> AkiClass {
        match self {
            Self::SafetyHr | Self::SensorDropout => AkiClass::P0,
            Self::SurgePending => AkiClass::P1,
            Self::WprimeLow | Self::SoftPedalWarning => AkiClass::P2,
            Self::GreyZoneDrift | Self::CadenceLow => AkiClass::P3,
            Self::CardiacDrift => AkiClass::P4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub directive: Directive,
    pub dir: i8,
    pub delta_star_w: f64,
    pub created_ns: u64,
    pub surge: Option<SurgePhase>,
}

impl Candidate {
    pub const fn new(directive: Directive, dir: i8, delta_star_w: f64, created_ns: u64) -> Self {
        Self {
            directive,
            dir,
            delta_star_w,
            created_ns,
            surge: None,
        }
    }
}

/// One slot per directive; a newer candidate replaces an older one.
/// Fixed size, so offering never allocates.
#[derive(Debug, Clone, Default)]
pub struct Candidates([Option<Candidate>; Directive::COUNT]);

impl Candidates {
    pub fn offer(&mut self, c: Candidate) {
        if let Some(slot) = self.0.get_mut(c.directive.slot()) {
            *slot = Some(c);
        }
    }

    pub fn get(&self, d: Directive) -> Option<Candidate> {
        self.0.get(d.slot()).copied().flatten()
    }

    pub fn remove(&mut self, d: Directive) -> Option<Candidate> {
        self.0.get_mut(d.slot())?.take()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Candidate> {
        self.0.iter().flatten()
    }

    /// P1 before P2 before P3; within a tier, the oldest.
    pub fn most_urgent(&self) -> Option<Candidate> {
        self.iter()
            .copied()
            .min_by_key(|c| (c.directive.priority(), c.created_ns))
    }

    pub fn take_most_urgent(&mut self) -> Option<Candidate> {
        let c = self.most_urgent()?;
        self.remove(c.directive)
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&Candidate) -> bool) {
        for slot in &mut self.0 {
            if slot.is_some_and(|c| !keep(&c)) {
                *slot = None;
            }
        }
    }
}

/// Inputs built fresh by the Engine for each call.
/// A channel that is not Fresh is None: no directive acts on stale data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inputs {
    pub t_ns: u64,
    pub p3_w: Option<f64>,
    pub hr_bpm: Option<f64>,
    pub cadence_rpm: Option<f64>,
    pub wbal_fraction: f64,
    /// Duration power has been non-Fresh.
    /// None while Fresh or before power has ever been Fresh.
    pub power_unfresh_ns: Option<u64>,
    pub paused: bool,
    pub step: Option<StepView>,
}

/// Fires once while a condition remains true; re-arms when it goes false.
#[derive(Debug, Clone, Copy, Default)]
struct Held {
    since: Option<u64>,
    fired: bool,
}

impl Held {
    fn update(&mut self, t: u64, cond: bool, hold_ns: u64) -> bool {
        if !cond {
            *self = Self::default();
            return false;
        }

        let since = *self.since.get_or_insert(t);

        if !self.fired && t.saturating_sub(since) >= hold_ns {
            self.fired = true;
            return true;
        }

        false
    }
}

fn secs(s: f64) -> u64 {
    (s * 1e9) as u64
}

#[derive(Debug, Clone)]
pub struct Directives {
    wprime: Hysteresis,
    dropout_fired: bool,
    countdown_for: Option<usize>,
    go_for: Option<usize>,
    soft_pedal: Held,
    grey_zone: Held,
    cadence_low: Held,
    cardiac: Held,
    step_index: Option<usize>,
}

impl Directives {
    pub fn new(cfg: &EngineConfig) -> Self {
        Self {
            wprime: Hysteresis::below(cfg.wprime_low_frac, cfg.wprime_rearm_frac),
            dropout_fired: false,
            countdown_for: None,
            go_for: None,
            soft_pedal: Held::default(),
            grey_zone: Held::default(),
            cadence_low: Held::default(),
            cardiac: Held::default(),
            step_index: None,
        }
    }

    /// Runs every evaluator except SAFETY_HR, which the Engine checks on
    /// each HR sample. Called from both ingest and tick.
    pub fn evaluate(&mut self, inp: &Inputs, cfg: &EngineConfig, out: &mut Candidates) {
        let index = inp.step.map(|s| s.index);

        if index != self.step_index {
            self.step_index = index;

            // Each new workout step restarts its grace windows.
            self.soft_pedal = Held::default();
            self.grey_zone = Held::default();
            self.cadence_low = Held::default();
            self.cardiac = Held::default();
        }

        self.sensor_dropout(inp, cfg, out);
        self.surge_pending(inp, cfg, out);

        if inp.paused {
            return; // A stopped rider gets no performance advice.
        }

        self.wprime_low(inp, cfg, out);
        self.soft_pedal_warning(inp, cfg, out);
        self.grey_zone_drift(inp, cfg, out);
        self.cadence_low(inp, cfg, out);
        self.cardiac_drift(inp, cfg, out);
    }

    fn sensor_dropout(&mut self, inp: &Inputs, cfg: &EngineConfig, out: &mut Candidates) {
        match inp.power_unfresh_ns {
            Some(ns) if ns >= cfg.dropout_cue_ns && !self.dropout_fired => {
                self.dropout_fired = true;
                out.offer(Candidate::new(Directive::SensorDropout, 0, 0.0, inp.t_ns));
            }
            Some(_) => {}
            None => self.dropout_fired = false,
        }
    }

    fn surge_pending(&mut self, inp: &Inputs, cfg: &EngineConfig, out: &mut Candidates) {
        let Some(v) = inp.step else {
            return;
        };

        let lead_s = libm::round(cfg.prompt_surge_countdown_s).clamp(5.0, 8.0) as u8;

        if let Some(n) = v.next
            && n.kind == StepKind::Interval
            && n.starts_in_ns <= u64::from(lead_s) * S
            && self.countdown_for != Some(n.index)
        {
            self.countdown_for = Some(n.index);

            let c = Candidate::new(Directive::SurgePending, 1, 0.0, inp.t_ns);

            out.offer(Candidate {
                surge: Some(SurgePhase::Countdown(lead_s)),
                ..c
            });
        }

        if v.step.kind == StepKind::Interval && v.into_ns < S && self.go_for != Some(v.index) {
            self.go_for = Some(v.index);

            let ask = match (v.step.target_w, inp.p3_w) {
                (Some(target), Some(p3)) => (target - p3).max(10.0),
                _ => 0.0,
            };

            let c = Candidate::new(Directive::SurgePending, 1, ask, inp.t_ns);

            out.offer(Candidate {
                surge: Some(SurgePhase::Go),
                ..c
            });
        }
    }

    fn wprime_low(&mut self, inp: &Inputs, cfg: &EngineConfig, out: &mut Candidates) {
        let Some(p3) = inp.p3_w else {
            return; // Only with Fresh power.
        };

        if self.wprime.update(inp.wbal_fraction) {
            out.offer(Candidate::new(
                Directive::WprimeLow,
                -1,
                (p3 - cfg.cp_w).max(10.0),
                inp.t_ns,
            ));
        }
    }

    fn soft_pedal_warning(&mut self, inp: &Inputs, cfg: &EngineConfig, out: &mut Candidates) {
        let prof = cfg.profile.params();
        let edge = band_edge(inp, StepKind::Recovery, None, prof.power_tolerance_pct);

        let over = match (edge, inp.p3_w) {
            (Some(edge), Some(p3)) => (p3 > edge).then_some((p3, edge)),
            _ => None,
        };

        if self
            .soft_pedal
            .update(inp.t_ns, over.is_some(), secs(prof.soft_pedal_grace_s))
            && let Some((p3, edge)) = over
        {
            out.offer(Candidate::new(
                Directive::SoftPedalWarning,
                -1,
                (p3 - edge).max(10.0),
                inp.t_ns,
            ));
        }
    }

    fn grey_zone_drift(&mut self, inp: &Inputs, cfg: &EngineConfig, out: &mut Candidates) {
        let prof = cfg.profile.params();

        let edge = band_edge(
            inp,
            StepKind::Steady,
            Some(StepKind::Warmup),
            prof.power_tolerance_pct,
        );

        let tempo = cfg.tempo_floor_frac * cfg.cp_w;

        let over = match (edge, inp.p3_w) {
            (Some(edge), Some(p3)) => (p3 > edge && p3 >= tempo).then_some((p3, edge)),
            _ => None,
        };

        if self
            .grey_zone
            .update(inp.t_ns, over.is_some(), secs(prof.grey_zone_grace_s))
            && let Some((p3, edge)) = over
        {
            out.offer(Candidate::new(
                Directive::GreyZoneDrift,
                -1,
                (p3 - edge).max(10.0),
                inp.t_ns,
            ));
        }
    }

    fn cadence_low(&mut self, inp: &Inputs, cfg: &EngineConfig, out: &mut Candidates) {
        let target = inp.step.and_then(|v| v.step.cadence_rpm);

        let low = match (target, inp.cadence_rpm, inp.p3_w) {
            (Some(target), Some(rpm), Some(p3)) => {
                // Zero RPM is coasting, not low cadence.
                // Ignore cadence below the 20 W power filter.
                rpm > 0.0
                    && p3 >= cfg.min_power_for_cadence_w
                    && rpm < target - cfg.cadence_low_margin_rpm
            }
            _ => false,
        };

        if self
            .cadence_low
            .update(inp.t_ns, low, cfg.cadence_low_hold_ns)
        {
            // Not a power ask: the observer must not grade power response.
            out.offer(Candidate::new(Directive::CadenceLow, 0, 0.0, inp.t_ns));
        }
    }

    fn cardiac_drift(&mut self, inp: &Inputs, cfg: &EngineConfig, out: &mut Candidates) {
        let over = match (inp.step, inp.hr_bpm) {
            (Some(v), Some(hr)) => {
                let endurance = matches!(v.step.kind, StepKind::Steady | StepKind::Warmup);

                let cap = v
                    .step
                    .hr_cap_bpm
                    .or(endurance.then_some(cfg.endurance_hr_cap_frac * cfg.hr_max));

                // Freeze HR checks for the first 30 s of each step.
                cap.is_some_and(|cap| v.into_ns >= cfg.hr_freeze_ns && hr > cap)
            }
            _ => false,
        };

        if self
            .cardiac
            .update(inp.t_ns, over, cfg.cardiac_drift_hold_ns)
        {
            let ask = inp.p3_w.map_or(10.0, |p3| (0.05 * p3).max(10.0));

            out.offer(Candidate::new(Directive::CardiacDrift, -1, ask, inp.t_ns));
        }
    }
}

/// The upper edge of a power band for the requested step kinds.
fn band_edge(
    inp: &Inputs,
    kind: StepKind,
    also: Option<StepKind>,
    tolerance_pct: f64,
) -> Option<f64> {
    let v = inp.step?;

    if v.step.kind != kind && Some(v.step.kind) != also {
        return None;
    }

    v.step
        .target_w
        .map(|target| target * (1.0 + tolerance_pct / 100.0))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::config::Profile;
    use crate::workout::{Step, WorkoutPlan};

    fn base(t_s: f64) -> Inputs {
        Inputs {
            t_ns: (t_s * 1e9) as u64,
            p3_w: Some(200.0),
            hr_bpm: Some(140.0),
            cadence_rpm: Some(90.0),
            wbal_fraction: 1.0,
            power_unfresh_ns: None,
            paused: false,
            step: None,
        }
    }

    /// A one-step plan's view at `t_s`.
    fn in_step(step: Step, t_s: f64) -> Option<StepView> {
        WorkoutPlan::from_steps(&[step])
            .ok()?
            .view((t_s * 1e9) as u64)
    }

    /// Evaluate at 4 Hz for `secs`; log every offered candidate.
    fn run(cfg: &EngineConfig, secs: f64, f: impl Fn(f64) -> Inputs) -> Vec<(f64, Candidate)> {
        let (mut d, mut out, mut log) = (Directives::new(cfg), Candidates::default(), Vec::new());

        let mut t = 0.0;
        while t <= secs {
            d.evaluate(&f(t), cfg, &mut out);

            for dir in Directive::ALL {
                if let Some(c) = out.remove(dir) {
                    log.push((t, c));
                }
            }

            t += 0.25;
        }

        log
    }

    fn solo() -> EngineConfig {
        EngineConfig::default()
    }

    fn with_profile(p: Profile) -> EngineConfig {
        EngineConfig {
            profile: p,
            ..EngineConfig::default()
        }
    }

    fn times(log: &[(f64, Candidate)], d: Directive) -> Vec<f64> {
        log.iter()
            .filter(|(_, c)| c.directive == d)
            .map(|(t, _)| *t)
            .collect()
    }

    #[test]
    fn surge_counts_down_at_the_lead_then_says_go() {
        let plan = WorkoutPlan::from_steps(&[
            Step::new(StepKind::Warmup, 60, Some(150.0)),
            Step::new(StepKind::Interval, 60, Some(300.0)),
        ])
        .unwrap();

        let at = |t: f64| Inputs {
            step: plan.view((t * 1e9) as u64),
            ..base(t)
        };

        let log = run(&solo(), 70.0, at);

        let surge: Vec<(f64, Option<SurgePhase>)> = log
            .iter()
            .filter(|(_, c)| c.directive == Directive::SurgePending)
            .map(|(t, c)| (*t, c.surge))
            .collect();

        assert_eq!(
            surge,
            vec![
                (55.0, Some(SurgePhase::Countdown(5))),
                (60.0, Some(SurgePhase::Go)),
            ]
        );

        let eight = EngineConfig {
            prompt_surge_countdown_s: 8.0,
            ..solo()
        };

        assert_eq!(
            times(&run(&eight, 70.0, at), Directive::SurgePending),
            vec![52.0, 60.0]
        );
    }

    #[test]
    fn wprime_low_fires_once_below_20_percent() {
        // The crossing has no floating-point ambiguity at 0.25 s intervals.
        let log = run(&solo(), 110.0, |t| Inputs {
            wbal_fraction: 1.0 - t / 128.0,
            ..base(t)
        });

        assert_eq!(times(&log, Directive::WprimeLow), vec![102.5]);
        // P3 below CP: the 10 W floor.
        assert_eq!(log[0].1.delta_star_w, 10.0);
    }

    #[test]
    fn soft_pedal_grace_follows_the_profile() {
        let rec = Step::new(StepKind::Recovery, 600, Some(120.0));

        let at = |t: f64| Inputs {
            p3_w: Some(160.0),
            step: in_step(rec, t),
            ..base(t)
        };

        for (profile, when) in [
            (Profile::Solo, 5.0),
            (Profile::Rolling, 10.0),
            (Profile::Group, 15.0),
        ] {
            let log = run(&with_profile(profile), 30.0, at);
            assert_eq!(
                times(&log, Directive::SoftPedalWarning),
                vec![when],
                "{profile:?}"
            );
        }

        let log = run(&solo(), 10.0, at);
        assert!((log[0].1.delta_star_w - (160.0 - 120.0 * 1.08)).abs() < 1e-9);
    }

    #[test]
    fn grey_zone_needs_both_the_band_and_tempo() {
        let steady = Step::new(StepKind::Steady, 600, Some(150.0));

        let ride = |p3: f64| {
            run(&solo(), 30.0, move |t| Inputs {
                p3_w: Some(p3),
                step: in_step(steady, t),
                ..base(t)
            })
        };

        // Above the band but below 76% of CP.
        assert!(times(&ride(180.0), Directive::GreyZoneDrift).is_empty());
        assert_eq!(times(&ride(200.0), Directive::GreyZoneDrift), vec![8.0]);
    }

    #[test]
    fn cadence_low_ignores_coasting_and_low_power() {
        let s = Step::new(StepKind::Steady, 600, Some(200.0)).with_cadence(90.0);

        let ride = |rpm: f64, p3: f64| {
            run(&solo(), 20.0, move |t| Inputs {
                cadence_rpm: Some(rpm),
                p3_w: Some(p3),
                step: in_step(s, t),
                ..base(t)
            })
        };

        assert_eq!(times(&ride(75.0, 200.0), Directive::CadenceLow), vec![8.0]);

        // Coasting.
        assert!(times(&ride(0.0, 200.0), Directive::CadenceLow).is_empty());

        // Under 20 W.
        assert!(times(&ride(75.0, 15.0), Directive::CadenceLow).is_empty());
    }

    #[test]
    fn cardiac_drift_waits_for_the_30_s_freeze() {
        let s = Step::new(StepKind::Steady, 600, Some(180.0)).with_hr_cap(150.0);

        let log = run(&solo(), 60.0, |t| Inputs {
            hr_bpm: Some(155.0),
            step: in_step(s, t),
            ..base(t)
        });

        // 30 s freeze + 20 s hold.
        assert_eq!(times(&log, Directive::CardiacDrift), vec![50.0]);
    }

    #[test]
    fn dropout_after_5_s_once_and_rearms() {
        let gone = |t: f64| {
            (10.0..30.0)
                .contains(&t)
                .then_some(((t - 10.0) * 1e9) as u64)
        };

        let log = run(&solo(), 60.0, |t| Inputs {
            power_unfresh_ns: gone(t),
            p3_w: None,
            ..base(t)
        });

        assert_eq!(times(&log, Directive::SensorDropout), vec![15.0]);

        let twice = |t: f64| {
            let a = (10.0..20.0)
                .contains(&t)
                .then_some(((t - 10.0) * 1e9) as u64);

            a.or((30.0..40.0)
                .contains(&t)
                .then_some(((t - 30.0) * 1e9) as u64))
        };

        let log = run(&solo(), 60.0, |t| Inputs {
            power_unfresh_ns: twice(t),
            p3_w: None,
            ..base(t)
        });

        assert_eq!(times(&log, Directive::SensorDropout), vec![15.0, 35.0]);
    }

    #[test]
    fn a_paused_rider_gets_p1_only() {
        let rec = Step::new(StepKind::Recovery, 600, Some(120.0));

        let log = run(&solo(), 30.0, |t| Inputs {
            p3_w: Some(160.0),
            paused: true,
            power_unfresh_ns: (t >= 10.0).then_some(((t - 10.0) * 1e9) as u64),
            step: in_step(rec, t),
            ..base(t)
        });

        assert!(times(&log, Directive::SoftPedalWarning).is_empty());
        assert_eq!(times(&log, Directive::SensorDropout), vec![15.0]);
    }

    #[test]
    fn a_new_step_restarts_the_grace_window() {
        let plan = WorkoutPlan::from_steps(&[
            Step::new(StepKind::Recovery, 4, Some(120.0)),
            Step::new(StepKind::Recovery, 60, Some(120.0)),
        ])
        .unwrap();

        let log = run(&solo(), 20.0, |t| Inputs {
            p3_w: Some(160.0),
            step: plan.view((t * 1e9) as u64),
            ..base(t)
        });

        // The second step starts a fresh grace window: 4 s + 5 s.
        assert_eq!(times(&log, Directive::SoftPedalWarning), vec![9.0]);
    }

    #[test]
    fn most_urgent_is_p1_then_oldest() {
        let mut c = Candidates::default();

        c.offer(Candidate::new(Directive::WprimeLow, -1, 10.0, 1));
        c.offer(Candidate::new(Directive::CardiacDrift, -1, 10.0, 0));
        c.offer(Candidate::new(Directive::SensorDropout, 0, 0.0, 5));

        assert_eq!(
            c.take_most_urgent().map(|x| x.directive),
            Some(Directive::SensorDropout)
        );
        assert_eq!(
            c.take_most_urgent().map(|x| x.directive),
            Some(Directive::WprimeLow)
        );
        assert_eq!(
            c.take_most_urgent().map(|x| x.directive),
            Some(Directive::CardiacDrift)
        );
    }
}
