//! S9: The Policy Engine's promises hold for random histories.
use proptest::prelude::*;
use vc_core::directive::{Candidate, Candidates};
use vc_core::policy::Policy;
use vc_core::testkit::{Ride, Stream};
use vc_core::{Directive, EngineConfig, Priority, Profile, WorkoutPlan};

const MS: u64 = 1_000_000;

fn profile() -> impl Strategy<Value = Profile> {
    prop::sample::select(Profile::ALL.to_vec())
}

fn p2_or_p3() -> impl Strategy<Value = Directive> {
    prop::sample::select(
        Directive::ALL
            .into_iter()
            .filter(|d| d.priority() != Priority::P1)
            .collect::<Vec<_>>(),
    )
}

fn any_directive() -> impl Strategy<Value = Directive> {
    prop::sample::select(Directive::ALL.to_vec())
}

fn history<D: Strategy<Value = Directive>>(
    d: D,
) -> impl Strategy<Value = Vec<(u64, Option<Directive>, bool, bool)>> {
    prop::collection::vec(
        (
            0u64..30_000,
            prop::option::of(d),
            any::<bool>(),
            any::<bool>(),
        ),
        0..60,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn p1_is_never_muted(
        profile in profile(),
        steps in history(p2_or_p3())
    ) {
        let cfg = EngineConfig {
            profile,
            ..EngineConfig::default()
        };
        let mut p = Policy::new(&cfg);
        let mut c = Candidates::default();
        let (mut t, mut last_id) = (0u64, None);

        for (dt_ms, offer, decide, finish) in steps {
            t += dt_ms * MS;

            if let Some(d) = offer {
                c.offer(Candidate::new(d, -1, 20.0, t));
            }

            if decide && let Some(dec) = p.decide(t, &mut c) {
                last_id = Some(dec.id);
            }

            if finish && let Some(id) = last_id {
                p.audio_finished(id, t);
            }
        }

        // The generated history never offers a P1, so no P1 clip is active.
        c.offer(Candidate::new(Directive::SafetyHr, -1, 0.0, t));
        let dec = p.decide(t, &mut c);
        prop_assert_eq!(dec.map(|d| d.directive), Some(Directive::SafetyHr));
    }

    #[test]
    fn group_never_speaks_p2_or_p3(
        steps in history(any_directive())
    ) {
        let cfg = EngineConfig {
            profile: Profile::Group,
            ..EngineConfig::default()
        };
        let mut p = Policy::new(&cfg);
        let mut c = Candidates::default();
        let mut t = 0u64;

        for (dt_ms, offer, _, finish) in steps {
            t += dt_ms * MS;

            if let Some(d) = offer {
                c.offer(Candidate::new(d, -1, 20.0, t));
            }

            if let Some(dec) = p.decide(t, &mut c) {
                prop_assert_eq!(dec.priority, Priority::P1, "{:?}", dec.directive);

                if finish {
                    p.audio_finished(dec.id, t);
                }
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn safety_fires_within_one_tick_after_120_s(
        profile in profile(),
        watts in 100.0f64..450.0
    ) {
        let cfg = EngineConfig {
            profile,
            ..EngineConfig::default()
        };
        let mut r = Ride::new(cfg, WorkoutPlan::default()).unwrap();
        r.stream = Stream {
            power_w: Some(watts),
            hr_bpm: Some(188),
            cadence_rpm: 90.0,
        };
        r.run(130.0);

        let t = r.times(Directive::SafetyHr);
        prop_assert_eq!(t.len(), 1);
        prop_assert!(
            (122.0..=122.25).contains(&t[0]),
            "{:?}",
            t
        );
    }
}
