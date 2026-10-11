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
}
