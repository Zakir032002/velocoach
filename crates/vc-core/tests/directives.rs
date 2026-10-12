use vc_core::testkit::{Ride, Stream, cpm_bytes};
use vc_core::{
    Directive, EngineConfig, EngineError, Source, Step, StepKind, SurgePhase, WorkoutPlan,
};

fn ride(plan: &[Step]) -> Ride {
    Ride::new(
        EngineConfig::default(),
        WorkoutPlan::from_steps(plan).unwrap(),
    )
    .unwrap()
}

fn warmup_then_interval() -> Vec<Step> {
    vec![
        Step::new(StepKind::Warmup, 60, Some(150.0)),
        Step::new(StepKind::Interval, 60, Some(300.0)),
    ]
}

#[test]
fn safety_fires_120_s_after_hr_turns_fresh() {
    let mut r = ride(&[]);
    r.stream = Stream {
        hr_bpm: Some(188),
        ..Stream::default()
    };

    r.run(130.0);

    // HR becomes Fresh at the third sample (2 s); 120 s later is 122 s.
    assert_eq!(r.times(Directive::SafetyHr), vec![122.0]);
}

#[test]
fn wprime_low_fires_on_the_first_packet_under_20_percent() {
    let mut r = ride(&[]);
    r.stream = Stream {
        power_w: Some(400.0),
        ..Stream::default()
    };

    r.run(120.0);

    // 150 J/s above CP: 3987.5 J remain at 106.75 s;
    // 4025 J remain at 106.5 s.
    assert_eq!(r.times(Directive::WprimeLow), vec![106.75]);
}

#[test]
fn dropout_is_called_5_s_after_power_stops_being_fresh() {
    let mut r = ride(&[]);
    r.run(30.0); // Last power packet at 29.75 s.
    r.stream.power_w = None;
    r.run(20.0);

    // The last sample becomes Stale at the 33.0 s tick,
    // then the 5 s dropout timer runs.
    let t = r.times(Directive::SensorDropout);
    assert_eq!(t.len(), 1);
    assert!((37.75..=38.25).contains(&t[0]), "{t:?}");
}

#[test]
fn the_surge_is_announced_at_the_lead_and_at_the_start() {
    let mut r = ride(&warmup_then_interval());
    r.run(65.0);

    let surge: Vec<(f64, Option<SurgePhase>)> = r
        .log
        .iter()
        .filter(|(_, d)| d.directive == Directive::SurgePending)
        .map(|(t, d)| (*t, d.surge))
        .collect();

    assert_eq!(
        surge,
        vec![
            (55.0, Some(SurgePhase::Countdown(5))),
            (60.0, Some(SurgePhase::Go)),
        ]
    );
}

#[test]
fn auto_pause_holds_the_countdown_back() {
    let mut r = ride(&warmup_then_interval());
    r.run(20.0);

    r.stream = Stream {
        power_w: Some(0.0),
        cadence_rpm: 0.0,
        ..Stream::default()
    };
    r.run(20.0); // Stopped; the auto-pause logic pauses the workout clock.

    r.stream = Stream::default();
    r.run(40.0);

    let t = r.times(Directive::SurgePending);
    assert_eq!(t.len(), 2);
    assert!(t[0] > 62.0 && t[0] < 72.0, "countdown at {} s", t[0]);
}

#[test]
fn garbage_is_counted_and_time_cannot_go_backwards() {
    let mut r = ride(&[]);

    assert!(matches!(
        r.engine.ingest(0, Source::Power, &[]),
        Err(EngineError::Decode(_))
    ));
    assert_eq!(r.engine.snapshot().decode_errors, 1);

    r.engine
        .ingest(5_000_000_000, Source::Power, &cpm_bytes(200.0, None))
        .unwrap();

    assert!(matches!(
        r.engine
            .ingest(4_000_000_000, Source::Power, &cpm_bytes(200.0, None)),
        Err(EngineError::NonMonotonicTime { .. })
    ));
}
