use proptest::prelude::*;
use vc_core::testkit::{Ride, Stream};
use vc_core::{Directive, EngineConfig, Step, StepKind, WorkoutPlan};
use vc_efficacy::{Arm, Confound, CueEvent, CueOutcome, Verdict};

fn outcome() -> impl Strategy<Value = (u8, Arm, Verdict, i32)> {
    (
        prop::sample::select(vec![1u8, 2, 3, 4, 16, 40]),
        prop::sample::select(vec![Arm::None, Arm::Direct, Arm::Autonomy]),
        prop::sample::select(vec![
            Verdict::Complied,
            Verdict::Ignored,
            Verdict::Partial,
            Verdict::Reversed,
        ]),
        0i32..12_000,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn e3_tuning_never_reaches_safety(outcomes in prop::collection::vec(outcome(), 0..300)) {
        // Ten 13 s steps: nine boundaries for the tuner, then a long steady step.
        let mut steps: Vec<Step> = (0..10).map(|i| Step::new(if i % 2 == 0 { StepKind::Steady } else { StepKind::Recovery }, 13, Some(150.0))).collect();
        steps.push(Step::new(StepKind::Steady, 600, Some(150.0)));
        let cfg = EngineConfig::default();
        let mut r = Ride::new(cfg, WorkoutPlan::from_steps(&steps).unwrap()).unwrap();
        for (directive, arm, verdict, l) in outcomes {
            let ev = CueEvent { cue_id: 1, directive, priority: 2, aki_class: 2, arm, propensity_q16: u16::MAX, is_sham: false,
                dir: -1, delta_star_w: 20.0, profile: 0, t_decision_ns: 0, t_onset_ns: 1, t_end_ns: 2 };
            let o = CueOutcome { cue_id: 1, verdict, confound: Confound::default(), coincidental: false, l_obs_ms: Some(l),
                l_onset_ms: None, l_end_ms: None, pre_slope_w_per_s: 0.0, slope_clamped: false, effect_max_w: 0.0 };
            r.engine.hub_mut().ledger_mut().record(&ev, &o);
        }
        r.stream = Stream { hr_bpm: Some(188), ..Stream::default() };
        r.run(260.0);
        let after = *r.engine.config();
        prop_assert_eq!((after.redline_frac, after.redline_hold_ns, after.redline_rearm_frac, after.redline_rearm_ns),
            (cfg.redline_frac, cfg.redline_hold_ns, cfg.redline_rearm_frac, cfg.redline_rearm_ns));
        prop_assert_eq!(after.dropout_cue_ns, cfg.dropout_cue_ns);
        prop_assert!((5.0..=8.0).contains(&after.prompt_surge_countdown_s));
        prop_assert!((15.0..=45.0).contains(&after.cooldown_between_alerts_s));
        prop_assert_eq!(r.times(Directive::SafetyHr), vec![122.0]); // still on time, still once
    }
}
