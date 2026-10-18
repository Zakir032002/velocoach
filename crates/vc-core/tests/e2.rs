//! E2: each confounder, introduced on the road, turns the verdict into
//! Confounded through the full Engine (real bytes, real audio callbacks).
use vc_core::testkit::{Ride, Stream};
use vc_core::{Directive, EngineConfig, Profile, Step, StepKind, WorkoutPlan};
use vc_efficacy::{Arm, Confound};

/// Rolling: a 15 s grey-zone grace, so the cue comes after 10 s of history.
fn ride(steps: &[Step]) -> Ride {
    let cfg = EngineConfig {
        profile: Profile::Rolling,
        ..EngineConfig::default()
    };
    let mut r = Ride::new(cfg, WorkoutPlan::from_steps(steps).unwrap()).unwrap();
    r.stream = Stream {
        power_w: Some(200.0),
        ..Stream::default()
    }; // over the 162 W edge, into tempo
    r.engine.set_grade(Some(0.0));
    r
}

fn endurance() -> Vec<Step> {
    vec![Step::new(StepKind::Steady, 600, Some(150.0))]
}

/// The grey-zone cue's ledger entry after the ride.
fn grey(r: &Ride) -> vc_efficacy::Counts {
    // profile 1 = Rolling
    r.engine
        .ledger()
        .stats(Directive::GreyZoneDrift.code(), Arm::Direct, 1)
        .map(|s| s.counts)
        .unwrap_or_default()
}

fn first_grey_id(r: &Ride) -> u32 {
    r.log
        .iter()
        .find(|(_, d)| d.directive == Directive::GreyZoneDrift)
        .map(|(_, d)| d.id)
        .unwrap()
}

fn run(
    inject: impl FnOnce(&mut Ride),
    restore: impl FnOnce(&mut Ride),
    steps: &[Step],
) -> (vc_efficacy::Counts, Option<Confound>) {
    let mut r = ride(steps);
    r.run(19.0);
    inject(&mut r);
    r.run(5.0);
    restore(&mut r);
    r.run(20.0); // the window is decided by about 34 s
    (
        grey(&r),
        r.engine
            .last_outcome()
            .filter(|o| o.cue_id == first_grey_id(&r))
            .map(|o| o.confound),
    )
}

#[test]
fn the_control_ride_is_ignored_not_confounded() {
    let (c, _) = run(|_| {}, |_| {}, &endurance());
    assert_eq!((c.ignored, c.confounded), (1, 0));
}

#[test]
#[allow(clippy::type_complexity)]
fn e2_every_confounder_reaches_the_verdict() {
    let cases: Vec<(
        Confound,
        Box<dyn FnOnce(&mut Ride)>,
        Box<dyn FnOnce(&mut Ride)>,
    )> = vec![
        (
            Confound::QUALITY,
            Box::new(|r| r.stream.power_w = None),
            Box::new(|r| r.stream.power_w = Some(200.0)),
        ),
        (
            Confound::COASTING,
            Box::new(|r| r.stream.power_w = Some(5.0)),
            Box::new(|r| r.stream.power_w = Some(200.0)),
        ),
        (
            Confound::CADENCE_ZERO,
            Box::new(|r| r.stream.cadence_rpm = 0.0),
            Box::new(|r| r.stream.cadence_rpm = 90.0),
        ),
        (
            Confound::GRADE,
            Box::new(|r| r.engine.set_grade(Some(-6.0))),
            Box::new(|_| {}),
        ),
        (
            Confound::AUDIO_INTERRUPTED,
            Box::new(|r| {
                let id = first_grey_id(r);
                r.engine
                    .on_cue_failed(id, vc_core::engine::AudioFailure::Interrupted);
            }),
            Box::new(|_| {}),
        ),
    ];
    for (flag, inject, restore) in cases {
        let (counts, confound) = run(inject, restore, &endurance());
        assert_eq!(counts.confounded, 1, "{flag:?}: {counts:?}");
        assert!(
            confound.is_none_or(|c| c.contains(flag)),
            "{flag:?}: {confound:?}"
        );
    }
}

#[test]
fn e2_a_p1_cue_inside_the_window_confounds_it() {
    let steps = [
        Step::new(StepKind::Steady, 30, Some(150.0)),
        Step::new(StepKind::Interval, 60, Some(300.0)),
        Step::new(StepKind::Steady, 600, Some(150.0)),
    ];
    let (counts, _) = run(|_| {}, |_| {}, &steps);
    assert_eq!(counts.confounded, 1, "{counts:?}"); // the countdown at 25 s
}
