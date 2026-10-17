use proptest::prelude::*;
use vc_efficacy::tuner::{Catalogue, TunablePolicy, Tuner};
use vc_efficacy::{Arm, Confound, CueEvent, CueOutcome, Ledger, Mode, Verdict};

const CAT: Catalogue<'static> = Catalogue {
    p2: &[1, 2, 3, 4],
    p3: &[16],
    step_transition: 40,
};

fn verdict() -> impl Strategy<Value = Verdict> {
    prop::sample::select(vec![
        Verdict::Complied,
        Verdict::Partial,
        Verdict::Ignored,
        Verdict::Reversed,
        Verdict::Confounded,
        Verdict::Invalid,
    ])
}

/// (directive, arm, verdict, L_obs)
fn outcome() -> impl Strategy<Value = (u8, Arm, Verdict, i32)> {
    (
        prop::sample::select(vec![1u8, 2, 3, 4, 16, 40]),
        prop::sample::select(vec![Arm::None, Arm::Direct, Arm::Autonomy, Arm::Sham]),
        verdict(),
        -1_000i32..15_000,
    )
}

fn record(l: &mut Ledger, (directive, arm, verdict, l_obs): (u8, Arm, Verdict, i32)) {
    let ev = CueEvent {
        cue_id: 1,
        directive,
        priority: 2,
        aki_class: 2,
        arm,
        propensity_q16: u16::MAX,
        is_sham: arm == Arm::Sham,
        dir: -1,
        delta_star_w: 20.0,
        profile: 0,
        t_decision_ns: 0,
        t_onset_ns: 1,
        t_end_ns: 2,
    };
    let o = CueOutcome {
        cue_id: 1,
        verdict,
        confound: Confound::default(),
        coincidental: false,
        l_obs_ms: Some(l_obs),
        l_onset_ms: None,
        l_end_ms: None,
        pre_slope_w_per_s: 0.0,
        slope_clamped: false,
        effect_max_w: 0.0,
    };
    l.record(&ev, &o);
}

fn policy() -> impl Strategy<Value = TunablePolicy> {
    (15.0f64..=45.0, 5.0f64..=8.0, any::<u64>(), any::<u64>()).prop_map(
        |(cooldown_s, surge_lead_s, p3_earcon, autonomy_default)| TunablePolicy {
            cooldown_s,
            surge_lead_s,
            p3_earcon,
            autonomy_default,
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn e4_tuned_parameters_stay_inside_akis_ranges(start in policy(), batches in prop::collection::vec(prop::collection::vec(outcome(), 0..40), 1..20)) {
        let (mut ledger, mut tuner, mut p) = (Ledger::new(), Tuner::default(), start);
        for batch in batches {
            for o in batch {
                record(&mut ledger, o);
            }
            let before = p;
            tuner.step(&ledger, &mut p, Mode::Coach, 0, &CAT);
            prop_assert!((15.0..=45.0).contains(&p.cooldown_s), "cooldown {}", p.cooldown_s);
            prop_assert!((5.0..=8.0).contains(&p.surge_lead_s), "lead {}", p.surge_lead_s);
            prop_assert!((p.cooldown_s - before.cooldown_s).abs() <= 10.0 + 1e-9);
            prop_assert!((p.surge_lead_s - before.surge_lead_s).abs() <= 1.0 + 1e-9);
        }
    }

    #[test]
    fn study_and_replay_freeze_the_tuner(start in policy(), outcomes in prop::collection::vec(outcome(), 0..200)) {
        let mut ledger = Ledger::new();
        for o in outcomes {
            record(&mut ledger, o);
        }
        for mode in [Mode::Study, Mode::Replay] {
            let mut p = start;
            let report = Tuner::default().step(&ledger, &mut p, mode, 0, &CAT);
            prop_assert!(report.is_empty());
            prop_assert_eq!(p, start);
        }
    }
}

#[test]
fn a_mostly_ignored_p2_cue_lengthens_the_cooldown_by_10_s() {
    let mut ledger = Ledger::new();
    for i in 0..10 {
        record(
            &mut ledger,
            (
                1,
                Arm::Direct,
                if i < 7 {
                    Verdict::Ignored
                } else {
                    Verdict::Complied
                },
                5_000,
            ),
        );
    }
    let mut p = TunablePolicy {
        cooldown_s: 20.0,
        surge_lead_s: 5.0,
        p3_earcon: 0,
        autonomy_default: 0,
    };
    let r = Tuner::default().step(&ledger, &mut p, Mode::Coach, 0, &CAT);
    assert_eq!(p.cooldown_s, 30.0);
    assert_eq!(r.changes[0].map(|c| c.rule), Some(vc_efficacy::Rule::T2));
}

#[test]
fn the_better_phrasing_becomes_the_default() {
    let mut ledger = Ledger::new();
    for i in 0..10 {
        record(
            &mut ledger,
            (
                2,
                Arm::Direct,
                if i < 3 {
                    Verdict::Complied
                } else {
                    Verdict::Ignored
                },
                4_000,
            ),
        ); // 30 %
        record(
            &mut ledger,
            (
                2,
                Arm::Autonomy,
                if i < 8 {
                    Verdict::Complied
                } else {
                    Verdict::Ignored
                },
                4_000,
            ),
        ); // 80 %
    }
    let mut p = TunablePolicy {
        cooldown_s: 45.0,
        surge_lead_s: 5.0,
        p3_earcon: 0,
        autonomy_default: 0,
    };
    Tuner::default().step(&ledger, &mut p, Mode::Coach, 0, &CAT);
    assert_ne!(p.autonomy_default & (1 << 2), 0);
}
