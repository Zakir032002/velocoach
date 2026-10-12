//! The planned workout and where the rider is in it.
//! The workout clock runs only while the rider is moving.

use heapless::Vec;

pub const MAX_STEPS: usize = 64;
const S: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepKind {
    Warmup,
    Interval,
    Recovery,
    Steady,
    Cooldown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    pub kind: StepKind,
    pub dur_s: u32,
    /// Target power in watts; None means free riding.
    pub target_w: Option<f64>,
    pub cadence_rpm: Option<f64>,
    /// HR ceiling for CARDIAC_DRIFT.
    pub hr_cap_bpm: Option<f64>,
}

impl Step {
    pub const fn new(kind: StepKind, dur_s: u32, target_w: Option<f64>) -> Self {
        Self {
            kind,
            dur_s,
            target_w,
            cadence_rpm: None,
            hr_cap_bpm: None,
        }
    }

    pub const fn with_cadence(self, rpm: f64) -> Self {
        Self {
            cadence_rpm: Some(rpm),
            ..self
        }
    }

    pub const fn with_hr_cap(self, bpm: f64) -> Self {
        Self {
            hr_cap_bpm: Some(bpm),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanFull;

#[derive(Debug, Clone, Default)]
pub struct WorkoutPlan {
    steps: Vec<Step, MAX_STEPS>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NextStep {
    pub index: usize,
    pub kind: StepKind,
    pub starts_in_ns: u64,
}

/// The current step, progress through it, and the next step if present.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepView {
    pub index: usize,
    pub step: Step,
    pub into_ns: u64,
    pub remaining_ns: u64,
    pub next: Option<NextStep>,
}

impl WorkoutPlan {
    /// Reject plans that exceed capacity; never truncate silently.
    pub fn from_steps(steps: &[Step]) -> Result<Self, PlanFull> {
        let mut plan = Self::default();

        for step in steps {
            plan.steps.push(*step).map_err(|_| PlanFull)?;
        }

        Ok(plan)
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn total_ns(&self) -> u64 {
        self.steps
            .iter()
            .map(|step| u64::from(step.dur_s) * S)
            .sum()
    }

    /// Return the step at elapsed active-workout time.
    /// Returns None after the workout has ended.
    pub fn view(&self, elapsed_ns: u64) -> Option<StepView> {
        let mut start = 0u64;

        for (i, step) in self.steps.iter().enumerate() {
            let end = start + u64::from(step.dur_s) * S;

            if elapsed_ns < end {
                let next = self.steps.get(i + 1).map(|next| NextStep {
                    index: i + 1,
                    kind: next.kind,
                    starts_in_ns: end - elapsed_ns,
                });

                return Some(StepView {
                    index: i,
                    step: *step,
                    into_ns: elapsed_ns - start,
                    remaining_ns: end - elapsed_ns,
                    next,
                });
            }

            start = end;
        }

        None
    }
}

/// Active time advances only while the workout is not auto-paused.
#[derive(Debug, Clone, Copy, Default)]
pub struct Progress {
    started: bool,
    last_t: u64,
    active_ns: u64,
    paused: bool,
    still_since: Option<u64>,
}

impl Progress {
    pub fn start(&mut self, t_ns: u64) {
        *self = Self {
            started: true,
            last_t: t_ns,
            ..Self::default()
        };
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Active workout time, or None before start.
    pub fn elapsed_ns(&self) -> Option<u64> {
        self.started.then_some(self.active_ns)
    }

    /// Advance the clock. Standing still longer than pause_after_ns pauses
    /// the workout; moving again resumes it.
    pub fn update(&mut self, t_ns: u64, moving: bool, pause_after_ns: u64) {
        if !self.started || t_ns < self.last_t {
            return;
        }

        if !self.paused {
            self.active_ns += t_ns - self.last_t;
        }

        self.last_t = t_ns;

        if moving {
            self.still_since = None;
            self.paused = false;
        } else {
            let since = *self.still_since.get_or_insert(t_ns);

            if t_ns - since > pause_after_ns {
                self.paused = true;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use StepKind::*;

    fn plan() -> WorkoutPlan {
        WorkoutPlan::from_steps(&[
            Step::new(Warmup, 60, Some(150.0)),
            Step::new(Interval, 60, Some(300.0)),
        ])
        .unwrap()
    }

    #[test]
    fn views_respect_boundaries_and_know_the_next_step() {
        let p = plan();

        let v = p.view(55 * S).unwrap();
        assert_eq!((v.index, v.remaining_ns), (0, 5 * S));
        assert_eq!(
            v.next.map(|n| (n.kind, n.starts_in_ns)),
            Some((Interval, 5 * S))
        );

        let v = p.view(60 * S).unwrap();
        assert_eq!((v.index, v.into_ns, v.next), (1, 0, None));

        assert_eq!(p.view(120 * S), None);
        assert_eq!(p.total_ns(), 120 * S);
    }

    #[test]
    fn a_plan_that_does_not_fit_is_rejected_not_truncated() {
        let many = [Step::new(Steady, 10, None); MAX_STEPS + 1];

        assert_eq!(WorkoutPlan::from_steps(&many).err(), Some(PlanFull));
    }

    #[test]
    fn standing_still_for_more_than_3_s_stops_the_clock() {
        let mut p = Progress::default();
        p.start(0);

        for s in 1..=10 {
            p.update(s * S, true, 3 * S);
        }

        for s in 11..=20 {
            p.update(s * S, false, 3 * S);
        }

        assert!(p.paused());
        assert_eq!(p.elapsed_ns(), Some(15 * S));

        p.update(21 * S, true, 3 * S);
        p.update(22 * S, true, 3 * S);

        assert_eq!((p.paused(), p.elapsed_ns()), (false, Some(16 * S)));
    }
}
