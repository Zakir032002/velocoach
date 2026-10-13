//! The six scenarios as constant data.
//! Each step separates what the coach sees from the rider's true road and effort.

use heapless::Vec;
use vc_core::workout::MAX_STEPS;
use vc_core::{Profile, Step, StepKind, WorkoutPlan};
use vc_gatt::{Channel, FaultKind};

use crate::persona::Persona;
use crate::rider::RiderParams;

const S: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimStep {
    pub step: Step,
    pub grade_pct: f64,
    /// Deliberate rider deviation from the target in watts.
    pub offset_w: f64,
    /// Power creep in watts per second.
    pub creep_w_per_s: f64,
}

const fn s(step: Step) -> SimStep {
    SimStep {
        step,
        grade_pct: 0.0,
        offset_w: 0.0,
        creep_w_per_s: 0.0,
    }
}

impl SimStep {
    const fn grade(self, pct: f64) -> Self {
        Self {
            grade_pct: pct,
            ..self
        }
    }

    const fn offset(self, watts: f64) -> Self {
        Self {
            offset_w: watts,
            ..self
        }
    }

    const fn creep(self, watts_per_second: f64) -> Self {
        Self {
            creep_w_per_s: watts_per_second,
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScheduledFault {
    pub at_s: u32,
    pub kind: FaultKind,
    pub channel: Option<Channel>,
    pub dur_ms: u32,
    pub param: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scenario {
    pub id: u8,
    pub name: &'static str,
    pub persona: Persona,
    pub profile: Profile,
    pub seed: u64,
    pub rider: RiderParams,
    pub steps: &'static [SimStep],
    pub faults: &'static [ScheduledFault],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct At {
    pub sim: SimStep,
    pub coach_index: usize,
    pub into_coach_ns: u64,
}

fn same_for_coach(a: &Step, b: &Step) -> bool {
    a.kind == b.kind
        && a.target_w == b.target_w
        && a.cadence_rpm == b.cadence_rpm
        && a.hr_cap_bpm == b.hr_cap_bpm
}

impl Scenario {
    pub fn total_ns(&self) -> u64 {
        self.steps.iter().map(|s| u64::from(s.step.dur_s) * S).sum()
    }

    /// Build the coach's plan, merging adjacent steps that look identical
    /// to the coach even when the simulated road changes.
    pub fn plan(&self) -> WorkoutPlan {
        let mut merged: Vec<Step, MAX_STEPS> = Vec::new();

        for sim_step in self.steps {
            match merged.last_mut() {
                Some(last) if same_for_coach(last, &sim_step.step) => {
                    last.dur_s += sim_step.step.dur_s;
                }
                _ => {
                    let _ = merged.push(sim_step.step);
                }
            }
        }

        WorkoutPlan::from_steps(&merged).unwrap_or_default()
    }

    /// Return the rider step active at time t, and the matching coach step.
    pub fn at(&self, t_ns: u64) -> Option<At> {
        let (mut start, mut coach_start, mut coach_index) = (0u64, 0u64, 0usize);

        for (i, sim_step) in self.steps.iter().enumerate() {
            if i > 0
                && !self
                    .steps
                    .get(i - 1)
                    .is_some_and(|previous| same_for_coach(&previous.step, &sim_step.step))
            {
                coach_index += 1;
                coach_start = start;
            }

            let end = start + u64::from(sim_step.step.dur_s) * S;

            if t_ns < end {
                return Some(At {
                    sim: *sim_step,
                    coach_index,
                    into_coach_ns: t_ns - coach_start,
                });
            }

            start = end;
        }

        None
    }
}

use StepKind::*;

const NONE: &[ScheduledFault] = &[];

const CLIMB_SURGE: &[SimStep] = &[
    s(Step::new(Warmup, 300, Some(150.0)).with_cadence(90.0)),
    s(Step::new(Interval, 240, Some(320.0)).with_cadence(85.0)).grade(4.0),
    s(Step::new(Recovery, 180, Some(130.0)).with_cadence(90.0)),
    s(Step::new(Interval, 240, Some(320.0)).with_cadence(85.0)).grade(4.0),
    s(Step::new(Cooldown, 180, Some(120.0)).with_cadence(90.0)),
];

const RECOVERY_DRIFT: &[SimStep] = &[
    s(Step::new(Warmup, 300, Some(150.0)).with_cadence(90.0)),
    s(Step::new(Interval, 180, Some(300.0)).with_cadence(95.0)),
    s(Step::new(Recovery, 240, Some(120.0)).with_cadence(90.0)).offset(45.0),
    s(Step::new(Steady, 600, Some(165.0)).with_cadence(90.0)).creep(0.2),
    s(Step::new(Cooldown, 180, Some(120.0)).with_cadence(90.0)),
];

const STEADY_RIDE: &[SimStep] = &[
    s(Step::new(Warmup, 300, Some(150.0)).with_cadence(90.0)),
    s(Step::new(Steady, 900, Some(200.0)).with_cadence(90.0)),
    s(Step::new(Cooldown, 180, Some(120.0)).with_cadence(90.0)),
];

const REDLINE: &[ScheduledFault] = &[ScheduledFault {
    at_s: 600,
    kind: FaultKind::HrRedline,
    channel: None,
    dur_ms: 240_000,
    param: 0,
}];

const DROPOUTS: &[ScheduledFault] = &[
    ScheduledFault {
        at_s: 600,
        kind: FaultKind::Dropout,
        channel: Some(Channel::Cpm),
        dur_ms: 12_000,
        param: 0,
    },
    ScheduledFault {
        at_s: 900,
        kind: FaultKind::Dropout,
        channel: Some(Channel::Hrm),
        dur_ms: 20_000,
        param: 0,
    },
];

const GRADE_CONFOUND: &[SimStep] = &[
    s(Step::new(Warmup, 300, Some(150.0)).with_cadence(90.0)),
    s(Step::new(Steady, 10, Some(160.0)).with_cadence(90.0)).offset(40.0),
    s(Step::new(Steady, 120, Some(160.0)).with_cadence(90.0))
        .offset(40.0)
        .grade(-6.0),
    s(Step::new(Steady, 470, Some(160.0)).with_cadence(90.0)).offset(40.0),
    s(Step::new(Cooldown, 180, Some(120.0)).with_cadence(90.0)),
];

const CREEP: &[SimStep] = &[
    s(Step::new(Warmup, 180, Some(150.0)).with_cadence(90.0)),
    s(Step::new(Steady, 180, Some(180.0)).with_cadence(90.0)).creep(0.8),
    s(Step::new(Steady, 60, Some(160.0)).with_cadence(90.0)),
    s(Step::new(Steady, 180, Some(180.0)).with_cadence(90.0)).creep(0.8),
    s(Step::new(Steady, 60, Some(160.0)).with_cadence(90.0)),
    s(Step::new(Steady, 180, Some(180.0)).with_cadence(90.0)).creep(0.8),
    s(Step::new(Steady, 60, Some(160.0)).with_cadence(90.0)),
    s(Step::new(Steady, 180, Some(180.0)).with_cadence(90.0)).creep(0.8),
    s(Step::new(Steady, 60, Some(160.0)).with_cadence(90.0)),
    s(Step::new(Cooldown, 60, Some(120.0)).with_cadence(90.0)),
];

const fn scenario(
    id: u8,
    name: &'static str,
    persona: Persona,
    seed: u64,
    steps: &'static [SimStep],
    faults: &'static [ScheduledFault],
) -> Scenario {
    Scenario {
        id,
        name,
        persona,
        profile: Profile::Solo,
        seed,
        rider: RiderParams::DEFAULT,
        steps,
        faults,
    }
}

pub const SCENARIOS: [Scenario; 6] = [
    scenario(1, "climb_surge", Persona::Dana, 42, CLIMB_SURGE, NONE),
    scenario(2, "recovery_drift", Persona::Dana, 42, RECOVERY_DRIFT, NONE),
    scenario(3, "safety_hr", Persona::Dana, 42, STEADY_RIDE, REDLINE),
    scenario(4, "dropout", Persona::Dana, 42, STEADY_RIDE, DROPOUTS),
    scenario(5, "grade_confound", Persona::Dana, 42, GRADE_CONFOUND, NONE),
    scenario(6, "creep", Persona::Calib, 7, CREEP, NONE),
];

pub fn by_id(id: u8) -> Option<&'static Scenario> {
    SCENARIOS.iter().find(|scenario| scenario.id == id)
}

pub fn by_name(name: &str) -> Option<&'static Scenario> {
    SCENARIOS.iter().find(|scenario| scenario.name == name)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_names_are_unique_and_every_plan_builds() {
        for (i, scenario) in SCENARIOS.iter().enumerate() {
            assert!(
                SCENARIOS
                    .iter()
                    .skip(i + 1)
                    .all(|other| { other.id != scenario.id && other.name != scenario.name })
            );
            assert_eq!(
                scenario.plan().total_ns(),
                scenario.total_ns(),
                "{}",
                scenario.name
            );
        }
    }

    #[test]
    fn the_coach_sees_one_step_where_the_rider_descends() {
        let Some(scenario) = by_name("grade_confound") else {
            panic!("scenario missing");
        };

        let plan = scenario.plan();
        assert_eq!(plan.steps().len(), 3);
        assert_eq!(plan.steps()[1].dur_s, 600);

        let Some(at) = scenario.at(320 * S) else {
            panic!("scenario time missing");
        };

        assert_eq!(
            (at.coach_index, at.into_coach_ns, at.sim.grade_pct),
            (1, 20 * S, -6.0)
        );
    }

    #[test]
    fn creep_scenario_lasts_twenty_minutes() {
        let Some(scenario) = by_name("creep") else {
            panic!("scenario missing");
        };

        assert_eq!(scenario.total_ns(), 1_200 * S);
    }
}
