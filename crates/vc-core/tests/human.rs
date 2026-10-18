use vc_core::testkit::{Ride, Stream};
use vc_core::{Directive, EngineConfig, Profile, Source, Step, StepKind, WorkoutPlan};
use vc_gatt::{MAX_PAYLOAD, Telem, encode_telem};

const MS: u64 = 1_000_000;

fn telem(m: Telem) -> Vec<u8> {
    let mut b = [0u8; MAX_PAYLOAD];
    let n = encode_telem(&m, &mut b).unwrap();
    b[..n].to_vec()
}

fn now(r: &Ride) -> u64 {
    (r.now_s() * 1e9) as u64
}

/// A ride with a clock sample (theta = 0), the slider at 60 %, and the first
/// grey-zone cue's onset. Then the slider moves to `to` at onset + `after_ms`.
fn ride_with_move(after_ms: u64, to: i16) -> vc_efficacy::CueOutcome {
    let cfg = EngineConfig {
        profile: Profile::Rolling,
        ..EngineConfig::default()
    };
    let mut r = Ride::new(
        cfg,
        WorkoutPlan::from_steps(&[Step::new(StepKind::Steady, 600, Some(150.0))]).unwrap(),
    )
    .unwrap();
    r.stream = Stream {
        power_w: Some(200.0),
        ..Stream::default()
    };
    r.run(2.0);
    // Calls between ride steps must sit between the last step (now - 250 ms) and the next (now).
    let a = now(&r) - 200 * MS;
    let id = r.engine.make_ping(a);
    let pong = telem(Telem::Pong {
        ping_id: id,
        t_rx_ns: a + MS,
        t_tx_ns: a + MS,
    });
    r.engine.ingest(a + 2 * MS, Source::Telem, &pong).unwrap(); // theta = 0, bound 1 ms
    let t = now(&r) - 100 * MS;
    let baseline = telem(Telem::RiderInput {
        seq: 1,
        t_ns: t,
        effort_permille: 600,
    });
    r.engine.ingest(t, Source::Telem, &baseline).unwrap();

    while !r
        .log
        .iter()
        .any(|(_, d)| d.directive == Directive::GreyZoneDrift)
    {
        r.run(0.25);
    }
    let decided = r
        .log
        .iter()
        .find(|(_, d)| d.directive == Directive::GreyZoneDrift)
        .map(|(_, d)| d.decided_ns)
        .unwrap();
    let onset = decided + 50 * MS; // the testkit plays at decision + lead
    while now(&r) < onset + after_ms * MS {
        r.run(0.25);
    }
    let mv = telem(Telem::RiderInput {
        seq: 2,
        t_ns: onset + after_ms * MS,
        effort_permille: to,
    });
    r.engine
        .ingest(now(&r) - 10 * MS, Source::Telem, &mv)
        .unwrap();
    r.run(25.0); // until the window is decided
    r.engine.last_observed().map(|o| o.outcome).unwrap()
}

#[test]
fn the_first_5_percent_move_is_l_onset() {
    let o = ride_with_move(1_200, 560);
    assert_eq!(o.l_onset_ms, Some(1_200));
    assert_eq!(o.l_end_ms, Some(1_200 - 2_400));
    assert!(!o.coincidental);
}

#[test]
fn a_move_within_350_ms_of_onset_is_coincidental() {
    assert!(ride_with_move(200, 560).coincidental);
}

#[test]
fn a_small_move_is_not_a_response() {
    assert_eq!(ride_with_move(1_200, 585).l_onset_ms, None); // -2.5 %
}
