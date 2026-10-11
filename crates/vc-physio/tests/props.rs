use proptest::prelude::*;
use vc_physio::*;

fn par() -> WbalParams {
    WbalParams::new(250.0, 20_000.0).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    /// Never above W′, never NaN, for any spacing and any power.
    #[test]
    fn wbal_stays_bounded(
        steps in proptest::collection::vec(
            (1u64..2_000, 0.0f64..1_500.0),
            1..400
        )
    ) {
        let mut exact = WbalExact::new(par());
        let mut ww = WbalWaterworth::new(par());

        let mut t = 0;
        exact.update(t, 0.0);
        ww.update(t, 0.0);

        for (dt_ms, watts) in steps {
            t += dt_ms * 1_000_000;

            let a = exact.update(t, watts);
            let b = ww.update(t, watts);

            prop_assert!(a.is_finite() && a <= 20_000.0);
            prop_assert!(b.is_finite() && b <= 20_000.0);
        }
    }

    /// Constant-power results must not depend on how the interval is sliced.
    #[test]
    fn exact_is_step_invariant(
        start_j in -5_000.0f64..20_000.0,
        watts in 0.0f64..800.0,
        total_ms in 1u64..10_000,
        slices in 1u64..40,
    ) {
        let mut one = WbalExact::starting_at(par(), start_j);
        let mut many = WbalExact::starting_at(par(), start_j);

        one.update(0, 0.0);
        many.update(0, 0.0);

        let total_ns = total_ms * 1_000_000;
        one.update(total_ns, watts);

        for k in 1..=slices {
            many.update(total_ns * k / slices, watts);
        }

        prop_assert!((one.balance_j() - many.balance_j()).abs() < 1e-6);
    }
    /// Any start point, including just before both counters wrap, should
/// reproduce the true cadence within 0.5 RPM.
#[test]
fn cadence_tracks_truth(
    revs0 in any::<u16>(),
    time0 in any::<u16>(),
    rpm in 40.0f64..180.0,
    drev in 1u16..=4,
) {
    let dt_ticks =
        (f64::from(drev) * 60.0 * 1024.0 / rpm).round() as u16;

    let dt_ns =
        (f64::from(dt_ticks) / 1024.0 * 1e9) as u64;

    prop_assume!(dt_ns <= 2_400_000_000);

    let mut c = CadenceDeriver::new();
    c.push(revs0, time0, 0);

    let got = c.push(
        revs0.wrapping_add(drev),
        time0.wrapping_add(dt_ticks),
        dt_ns,
    );

    match got {
        CadenceUpdate::Rpm(r) => {
            prop_assert!((r - rpm).abs() < 0.5, "{r} vs {rpm}");
        }
        other => prop_assert!(false, "unexpected {other:?}"),
    }
}
}
