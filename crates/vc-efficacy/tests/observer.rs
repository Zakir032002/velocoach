use proptest::prelude::*;
use vc_efficacy::observer::{ObsContext, Observer, SMOOTHING_LAG_MS};
use vc_efficacy::{Arm, Confound, CueEvent, CueOutcome, P3Ring, Verdict};

const MS: u64 = 1_000_000;

fn event(onset_ms: u64, end_ms: u64, dir: i8, delta_w: f64) -> CueEvent {
    CueEvent {
        cue_id: 7,
        directive: 1, // WPRIME_LOW
        priority: 2,
        aki_class: 2,
        arm: Arm::Direct,
        propensity_q16: u16::MAX,
        is_sham: false,
        dir,
        delta_star_w: delta_w,
        profile: 0,
        t_decision_ns: (onset_ms - 50) * MS,
        t_onset_ns: onset_ms * MS,
        t_end_ns: end_ms * MS,
    }
}

/// Ride at 4 Hz from 0 to 200 s with `power(t_ms)`; open the cue at the audio's
/// end; update on every sample; `tweak` may change the context at any moment.
fn ride_with(
    ev: &CueEvent,
    power: impl Fn(u64) -> Option<f64>,
    tweak: impl Fn(u64, &mut ObsContext<'_>),
) -> Option<CueOutcome> {
    let (mut ring, mut obs, mut opened) = (P3Ring::new(), Observer::new(), false);
    for t_ms in (0..200_000u64).step_by(250) {
        let p = power(t_ms);
        if let Some(w) = p {
            ring.push(t_ms * MS, w);
        }
        if !opened && t_ms * MS >= ev.t_end_ns {
            opened = true;
            if let Some(o) = obs.open(ev, &ring, Some(0.0)) {
                return Some(o);
            }
        }
        if opened {
            let mut ctx = ObsContext::quiet(&ring, p.unwrap_or(0.0));
            ctx.power_fresh = p.is_some();
            ctx.grade_pct = Some(0.0);
            tweak(t_ms, &mut ctx);
            if let Some(o) = obs.update(t_ms * MS, &ctx) {
                return Some(o);
            }
        }
    }
    None
}

fn ride(ev: &CueEvent, power: impl Fn(u64) -> f64) -> CueOutcome {
    ride_with(ev, |t| Some(power(t)), |_, _| {}).expect("the window always closes")
}

/// 200 W, then `after` W from 105.0 s.
fn step_at_105(after: f64) -> impl Fn(u64) -> f64 {
    move |t| if t < 105_000 { 200.0 } else { after }
}

#[test]
fn a_25_w_step_3_s_after_the_audio_is_complied() {
    let o = ride(&event(100_000, 102_000, -1, 25.0), step_at_105(175.0));
    assert_eq!(o.verdict, Verdict::Complied);
    assert_eq!(o.l_obs_ms, Some(5_000));
    assert_eq!(o.l_obs_ms.map(|l| l - SMOOTHING_LAG_MS), Some(3_000)); // the rider's real 3 s
    assert!(o.confound.is_empty());
}

#[test]
fn a_rider_already_falling_at_2_w_per_s_gets_no_credit() {
    let o = ride(&event(100_000, 102_000, -1, 25.0), |t| {
        300.0 - 2.0 * t as f64 / 1_000.0
    });
    assert_eq!(o.verdict, Verdict::Ignored);
    assert!((o.pre_slope_w_per_s + 2.0).abs() < 1e-6);
}

#[test]
fn a_surge_after_an_ease_off_cue_is_reversed() {
    assert_eq!(
        ride(&event(100_000, 102_000, -1, 25.0), step_at_105(225.0)).verdict,
        Verdict::Reversed
    );
}

#[test]
fn a_small_move_is_partial_and_none_is_ignored() {
    assert_eq!(
        ride(&event(100_000, 102_000, -1, 25.0), step_at_105(192.0)).verdict,
        Verdict::Partial
    );
    assert_eq!(
        ride(&event(100_000, 102_000, -1, 25.0), |_| 200.0).verdict,
        Verdict::Ignored
    );
}

#[test]
fn the_mid_phrase_responder_is_complied_with_negative_latency() {
    // Audio 100.0-102.4 s; the rider eases off 24 W at 100.5 s, before the phrase ends.
    let o = ride(&event(100_000, 102_400, -1, 24.0), |t| {
        if t < 100_500 { 200.0 } else { 176.0 }
    });
    assert_eq!(o.verdict, Verdict::Complied);
    assert_eq!(o.l_obs_ms, Some(-400));
}

#[test]
#[allow(clippy::type_complexity)]
fn every_confounder_turns_the_verdict_into_confounded() {
    let cases: [(Confound, fn(&mut ObsContext<'_>)); 7] = [
        (Confound::QUALITY, |c| c.power_fresh = false),
        (Confound::P1_FIRED, |c| c.p1_fired = true),
        (Confound::GRADE, |c| c.grade_pct = Some(-6.0)),
        (Confound::CADENCE_ZERO, |c| c.cadence_rpm = Some(0.0)),
        (Confound::COASTING, |c| c.power_w = 5.0),
        (Confound::OVERLAP, |c| c.overlap = true),
        (Confound::AUDIO_INTERRUPTED, |c| c.audio_interrupted = true),
    ];
    for (flag, set) in cases {
        let ev = event(100_000, 102_000, -1, 25.0);
        let p = step_at_105(175.0);
        let o = ride_with(
            &ev,
            |t| Some(p(t)),
            |t, c| {
                if t == 103_000 {
                    set(c)
                }
            },
        )
        .unwrap();
        assert_eq!(o.verdict, Verdict::Confounded, "{flag:?}");
        assert!(o.confound.contains(flag), "{flag:?}");
    }
}

#[test]
fn a_confounder_after_the_verdict_does_not_count() {
    let ev = event(100_000, 102_000, -1, 25.0);
    let p = step_at_105(175.0);
    let o = ride_with(
        &ev,
        |t| Some(p(t)),
        |t, c| {
            if t == 115_000 {
                c.p1_fired = true
            }
        },
    )
    .unwrap();
    assert_eq!(o.verdict, Verdict::Complied);
}

#[test]
fn a_hole_in_the_window_is_a_quality_confounder() {
    let ev = event(100_000, 102_000, -1, 25.0);
    let o = ride_with(
        &ev,
        |t| (!(104_000..106_000).contains(&t)).then_some(200.0),
        |_, _| {},
    )
    .unwrap();
    assert_eq!(o.verdict, Verdict::Confounded);
    assert!(o.confound.contains(Confound::QUALITY));
}

#[test]
fn cues_that_cannot_be_scored_are_invalid() {
    let p1 = CueEvent {
        priority: 1,
        ..event(100_000, 102_000, -1, 25.0)
    };
    assert_eq!(ride(&p1, |_| 200.0).verdict, Verdict::Invalid);
    assert_eq!(
        ride(&event(100_000, 102_000, 0, 25.0), |_| 200.0).verdict,
        Verdict::Invalid
    );
    // A dropout from 92 to 96 s leaves too few pre-trend points.
    let ev = event(100_000, 102_000, -1, 25.0);
    let o = ride_with(
        &ev,
        |t| (!(92_000..96_000).contains(&t)).then_some(200.0),
        |_, _| {},
    )
    .unwrap();
    assert_eq!(o.verdict, Verdict::Invalid);
}

#[test]
fn a_second_window_is_invalid_and_overlaps_the_first() {
    let mut ring = P3Ring::new();
    for t in (0..103_000u64).step_by(250) {
        ring.push(t * MS, 200.0);
    }
    let mut obs = Observer::new();
    assert!(
        obs.open(&event(100_000, 102_000, -1, 25.0), &ring, None)
            .is_none()
    );
    let second = obs
        .open(
            &CueEvent {
                cue_id: 8,
                ..event(101_000, 103_000, -1, 25.0)
            },
            &ring,
            None,
        )
        .unwrap();
    assert_eq!((second.cue_id, second.verdict), (8, Verdict::Invalid));
    let first = obs.abandon().unwrap();
    assert_eq!(first.cue_id, 7);
}

#[test]
fn human_mode_flags_a_coincidental_first_move() {
    let ev = event(100_000, 102_000, -1, 25.0);
    let p = step_at_105(175.0);
    let o = ride_with(
        &ev,
        |t| Some(p(t)),
        |_, c| c.rider_move_ns = Some(100_200 * MS),
    )
    .unwrap();
    assert!(o.coincidental);
    assert_eq!((o.l_onset_ms, o.l_end_ms), (Some(200), Some(-1_800)));
}

proptest! {
    #[test]
    fn a_constant_power_offset_never_changes_the_verdict(offset in -100.0f64..500.0, after in 150.0f64..250.0) {
        let ev = event(100_000, 102_000, -1, 25.0);
        let base = ride(&ev, step_at_105(after));
        let shifted = ride(&ev, move |t| step_at_105(after)(t) + offset);

        prop_assert_eq!(base.verdict, shifted.verdict);
        prop_assert_eq!(base.l_obs_ms, shifted.l_obs_ms);
    }
}
