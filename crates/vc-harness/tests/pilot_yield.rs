//! STUDY.md expects 15-20 decision points per 20-minute session. The original `creep`
//! scenario yields about five (its directive re-arms only at a step boundary, and it has four
//! creep blocks); `creep_pilot` is the scenario that meets the plan's figure. In human mode the
//! yield depends on where the rider holds the slider, because the creep is added to the slider's
//! power: the grey-zone floor is 190 W (0.76 x CP), and a 40 s block at 0.8 W/s adds only 32 W.
use vc_core::Mode;
use vc_efficacy::Arm;
use vc_harness::{RunOpts, run_sim};

fn decision_points(scenario: &str, seed: u64, slider: Option<i16>) -> usize {
    let run = run_sim(RunOpts {
        scenario,
        persona: 1,
        seed: Some(seed),
        mode: Mode::Study,
        mode_seed: seed,
        human_permille: slider,
        ..RunOpts::default()
    })
    .expect("the ride runs");
    run.decisions
        .iter()
        .filter(|d| matches!(d.arm, Arm::Sham | Arm::Direct | Arm::Autonomy))
        .count()
}

#[test]
fn creep_pilot_yields_the_planned_number_of_decision_points() {
    for seed in 1_001..1_004 {
        let n = decision_points("creep_pilot", seed, None);
        assert!((15..=20).contains(&n), "seed {seed}: {n} decision points");
    }
}

#[test]
fn the_original_creep_scenario_does_not() {
    for seed in 1_001..1_004 {
        let n = decision_points("creep", seed, None);
        assert!(n < 10, "seed {seed}: {n} decision points");
    }
}

#[test]
fn a_human_holding_70_percent_gets_the_planned_number() {
    for seed in 1_001..1_004 {
        let n = decision_points("creep_pilot", seed, Some(700));
        assert!((15..=20).contains(&n), "seed {seed}: {n} decision points");
    }
}

#[test]
fn a_human_at_60_percent_never_reaches_the_grey_zone() {
    // 0.6 x 250 W + 32 W of creep = 182 W < 190 W: no cue, no decision point. The runbook
    // therefore starts the slider at 70 % for this scenario.
    assert_eq!(decision_points("creep_pilot", 1_001, Some(600)), 0);
}
