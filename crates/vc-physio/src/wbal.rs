//! W′ balance: how much of the anaerobic battery is left.
//!
//! - `WbalExact`: differential model solved exactly per interval. The coach
//!   uses this.
//! - `WbalGc`: the same model stepped at 1 s with Euler, like
//!   GoldenCheetah's WPrime.cpp. Exists to prove parity with GoldenCheetah.
//! - `WbalWaterworth`: Skiba's integral model in Waterworth's running-sum
//!   form, made causal with a running mean. Optional; first on the cut list.
//!
//! A reading at t covers (previous t, t]. Gaps > 10 s are zero-filled.

//! wbal.rs
// │
// ├── GAP_ZERO_FILL_NS
// │   └── Defines the long-gap threshold
// │
// ├── WbalParams
// │   └── Validates and stores CP and W′ capacity
// │
// ├── WprimeBalance
// │   └── Common interface for W′ balance models
// │
// ├── Clock
// │   └── Tracks timestamps and calculates elapsed time
// │
// ├── effective_power()
// │   └── Sanitizes power and applies gap policy
// │
// ├── exact_step()
// │   └── Performs the mathematical calculation
// │
// └── WbalExact
//     ├── Stores model parameters
//     ├── Stores current balance
//     ├── Stores timestamp state
//     └── Connects incoming readings to the calculation
use crate::NS_PER_S;
//the packets dosent arrive on time, this is a threshold
//The program cannot know the power that was produced during the missing interval. This implementation adopts a specific policy: if the interval exceeds ten seconds, it treats the interval's effective power as zero.
pub const GAP_ZERO_FILL_NS: u64 = 10_000_000_000;

//so the rider needs cp_j and w'prime
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WbalParams {
    cp_j: f64,
    wprime_j: f64, // full reserve, not the remaining reserve
}

impl WbalParams {
    pub fn new(cp_j: f64, wprime_j: f64) -> Option<Self> {
        if cp_j.is_finite() && cp_j > 0.0 && wprime_j.is_finite() && wprime_j > 0.0 {
            Some(Self { cp_j, wprime_j })
        } else {
            None
        }
    }
    pub fn cp_w(&self) -> f64 {
        self.cp_j
    }
    pub fn wprime_j(&self) -> f64 {
        self.wprime_j
    }
}

pub trait WprimeBalance {
    fn update(&mut self, t_ns: u64, watts: f64) -> f64;
    fn balance_j(&self) -> f64;
    fn params(&self) -> WbalParams;
    fn fraction(&self) -> f64 {
        (self.balance_j() / self.params().wprime_j()).clamp(0.0, 1.0)
    }
}
//elapsed time
#[derive(Debug, Clone, Copy, Default)]
struct Clock {
    last_t_ns: Option<u64>,
}

impl Clock {
    /// Returns (dt in seconds, is_gap) for a new reading.
    ///
    /// Returns None for the first reading and for duplicate or
    /// out-of-order timestamps, which are ignored.
    fn advance(&mut self, t_ns: u64) -> Option<(f64, bool)> {
        match self.last_t_ns {
            None => {
                self.last_t_ns = Some(t_ns);
                None
            }
            Some(prev) if t_ns <= prev => None,
            Some(prev) => {
                self.last_t_ns = Some(t_ns);

                let dt_ns = t_ns - prev;
                Some((dt_ns as f64 / NS_PER_S, dt_ns > GAP_ZERO_FILL_NS))
            }
        }
    }
}
// now lets implement a case where power can be zero,,its 2 cases,,where gap is more than 10s or power is negetive
fn effective_power(watts: f64, gap: bool) -> f64 {
    if gap || !watts.is_finite() {
        0.0
    } else {
        watts.max(0.0)
    }
}

/// One exact step of the differential model with constant power over dt.
fn exact_step(par: WbalParams, w: f64, p: f64, dt: f64) -> f64 {
    let (cp, wp) = (par.cp_w(), par.wprime_j);

    let next = if p >= cp {
        w - (p - cp) * dt
    } else {
        wp - (wp - w) * libm::exp(-(cp - p) * dt / wp)
    };

    next.min(wp)
}

#[derive(Debug, Clone)]
pub struct WbalExact {
    par: WbalParams,
    w_j: f64, // the amount that got burned from reserve
    clock: Clock,
}

impl WbalExact {
    pub fn starting_at(par: WbalParams, w_j: f64) -> Self {
        Self {
            par,
            w_j: w_j.min(par.wprime_j),
            clock: Clock::default(),
        }
    }
    pub fn new(par: WbalParams) -> Self {
        Self::starting_at(par, par.wprime_j)
    }
}

impl WprimeBalance for WbalExact {
    fn update(&mut self, t_ns: u64, watts: f64) -> f64 {
        if let Some((dt, gap)) = self.clock.advance(t_ns) {
            self.w_j = exact_step(self.par, self.w_j, effective_power(watts, gap), dt);
        }

        self.w_j
    }

    fn balance_j(&self) -> f64 {
        self.w_j
    }

    fn params(&self) -> WbalParams {
        self.par
    }
}

/// Cap on 1 s steps per update. Only zero-filled gaps get near it, and
/// after 3600 s at 0 W the battery is full anyway (e^-45).
const GC_MAX_STEPS: u32 = 3600;

fn gc_step_1s(par: WbalParams, w: f64, p: f64) -> f64 {
    let (cp, wp) = (par.cp_w(), par.wprime_j);

    let next = if p < cp {
        w + (cp - p) * (wp - w) / wp
    } else {
        w + (cp - p)
    };

    next.min(wp)
}

/// Feed it 1 Hz data (a GoldenCheetah CSV export) for exact parity.
/// With 4 Hz input it steps once per whole second using the latest reading.
#[derive(Debug, Clone)]
pub struct WbalGc {
    par: WbalParams,
    w_j: f64,
    clock: Clock,
    carry_s: f64,
}

impl WbalGc {
    pub fn new(par: WbalParams) -> Self {
        Self::starting_at(par, par.wprime_j)
    }

    pub fn starting_at(par: WbalParams, w_j: f64) -> Self {
        Self {
            par,
            w_j: w_j.min(par.wprime_j),
            clock: Clock::default(),
            carry_s: 0.0,
        }
    }
}

impl WprimeBalance for WbalGc {
    fn update(&mut self, t_ns: u64, watts: f64) -> f64 {
        if let Some((dt, gap)) = self.clock.advance(t_ns) {
            let p = effective_power(watts, gap);
            self.carry_s += dt;

            let mut steps = 0;

            while self.carry_s >= 1.0 && steps < GC_MAX_STEPS {
                self.w_j = gc_step_1s(self.par, self.w_j, p);
                self.carry_s -= 1.0;
                steps += 1;
            }

            if steps == GC_MAX_STEPS {
                self.carry_s = 0.0;
            }
        }

        self.w_j
    }

    fn balance_j(&self) -> f64 {
        self.w_j
    }

    fn params(&self) -> WbalParams {
        self.par
    }
}

#[derive(Debug, Clone)]
pub struct WbalWaterworth {
    par: WbalParams,
    spent_j: f64,
    clock: Clock,
    below_cp_s: f64,
    below_cp_j: f64,
}

impl WbalWaterworth {
    pub fn new(par: WbalParams) -> Self {
        Self {
            par,
            spent_j: 0.0,
            clock: Clock::default(),
            below_cp_s: 0.0,
            below_cp_j: 0.0,
        }
    }

    /// tau = 546 * exp(-0.01 * D_CP) + 316
    /// D_CP = CP - mean power while below CP.
    pub fn tau_s(&self) -> f64 {
        let mean_below = if self.below_cp_s > 0.0 {
            self.below_cp_j / self.below_cp_s
        } else {
            0.0
        };

        546.0 * libm::exp(-0.01 * (self.par.cp_w() - mean_below)) + 316.0
    }
}

impl WprimeBalance for WbalWaterworth {
    fn update(&mut self, t_ns: u64, watts: f64) -> f64 {
        if let Some((dt, gap)) = self.clock.advance(t_ns) {
            let p = effective_power(watts, gap);

            if p < self.par.cp_w() {
                self.below_cp_s += dt;
                self.below_cp_j += p * dt;
            }

            let tau = self.tau_s();
            let decay = libm::exp(-dt / tau);
            let above = (p - self.par.cp_w()).max(0.0);

            self.spent_j = self.spent_j * decay + above * tau * (1.0 - decay);
        }

        self.balance_j()
    }

    fn balance_j(&self) -> f64 {
        self.par.wprime_j - self.spent_j
    }

    fn params(&self) -> WbalParams {
        self.par
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const S: u64 = 1_000_000_000;

    fn par() -> WbalParams {
        WbalParams::new(250.0, 20_000.0).unwrap()
    }

    fn start(m: &mut impl WprimeBalance) -> u64 {
        m.update(0, 0.0);
        0
    }

    /// Feed power at 4 Hz for dur_ns, ending exactly at t + dur_ns.
    fn ride(m: &mut impl WprimeBalance, t: &mut u64, watts: f64, dur_ns: u64) {
        const STEP: u64 = 250_000_000;
        let end = *t + dur_ns;

        while *t < end {
            *t = (*t + STEP).min(end);
            m.update(*t, watts);
        }
    }

    #[test]
    fn vector_1_drain_above_cp() {
        let mut m = WbalExact::new(par());
        let mut t = start(&mut m);

        ride(&mut m, &mut t, 350.0, 100 * S);

        assert!((m.balance_j() - 10_000.0).abs() < 1e-6);
    }

    #[test]
    fn vector_2_partial_recovery() {
        let mut m = WbalExact::starting_at(par(), 10_000.0);
        let mut t = start(&mut m);

        ride(&mut m, &mut t, 100.0, 133_333_333_333);

        let want = 20_000.0 - 10_000.0 * (-(150.0 * 133.333_333_333) / 20_000.0_f64).exp();

        assert!(
            (m.balance_j() - want).abs() < 1e-6,
            "{} vs {want}",
            m.balance_j()
        );
        assert!((m.balance_j() - 16_321.2).abs() < 0.1);
    }

    #[test]
    fn vector_3_at_cp_unchanged() {
        let mut m = WbalExact::starting_at(par(), 12_345.0);
        let mut t = start(&mut m);

        ride(&mut m, &mut t, 250.0, 60 * S);

        assert_eq!(m.balance_j(), 12_345.0);
    }

    #[test]
    fn vector_4_recovery_from_empty() {
        let mut m = WbalExact::starting_at(par(), 0.0);
        let mut t = start(&mut m);

        ride(&mut m, &mut t, 0.0, 80 * S);

        let want = 20_000.0 * (1.0 - (-1.0_f64).exp());

        assert!((m.balance_j() - want).abs() < 1e-6);
        assert!((m.balance_j() - 12_642.4).abs() < 0.1);
    }

    #[test]
    fn gap_is_zero_filled() {
        let mut m = WbalExact::starting_at(par(), 0.0);

        m.update(0, 400.0);
        m.update(20 * S, 400.0);

        let want = 20_000.0 * (1.0 - (-0.25_f64).exp());
        assert!((m.balance_j() - want).abs() < 1e-6);
    }

    #[test]
    fn duplicate_and_out_of_order_readings_are_ignored() {
        let mut m = WbalExact::new(par());

        m.update(10 * S, 0.0);
        m.update(11 * S, 400.0);

        let after = m.balance_j();
        assert_eq!(after, 19_850.0);

        m.update(11 * S, 400.0);
        m.update(5 * S, 400.0);

        assert_eq!(m.balance_j(), after);
    }

    #[test]
    fn negative_and_nan_power_count_as_zero() {
        let mut odd = WbalExact::starting_at(par(), 0.0);
        let mut zero = WbalExact::starting_at(par(), 0.0);

        for (k, w) in [(0, 0.0), (1, -50.0), (2, f64::NAN)] {
            odd.update(k * S, w);
            zero.update(k * S, 0.0);
        }

        assert_eq!(odd.balance_j(), zero.balance_j());
    }

    #[test]
    fn rejects_bad_params_and_floors_fraction() {
        assert!(WbalParams::new(0.0, 20_000.0).is_none());
        assert!(WbalParams::new(250.0, f64::NAN).is_none());

        assert_eq!(WbalExact::starting_at(par(), -500.0).fraction(), 0.0);
        assert_eq!(WbalExact::starting_at(par(), 15_000.0).fraction(), 0.75);
    }

    #[test]
    fn gc_euler_overshoots_recovery_slightly() {
        let mut gc = WbalGc::starting_at(par(), 0.0);
        let mut t = start(&mut gc);

        ride(&mut gc, &mut t, 0.0, 80 * S);

        let want = 20_000.0 * (1.0 - (1.0_f64 - 250.0 / 20_000.0).powi(80));

        assert!((gc.balance_j() - want).abs() < 1e-6);
        assert!(gc.balance_j() > 12_642.4);
    }

    /// 20 min @ 200 W, 5 x (3 min @ 330 W + 3 min @ 150 W),
    /// then 10 min @ 150 W. Feed both models at 1 Hz.
    #[test]
    fn exact_and_gc_agree_within_half_a_percent_over_an_hour() {
        let mut exact = WbalExact::new(par());
        let mut gc = WbalGc::new(par());

        let mut plan: Vec<(f64, u64)> = vec![(200.0, 1_200)];

        for _ in 0..5 {
            plan.push((330.0, 180));
            plan.push((150.0, 180));
        }

        plan.push((150.0, 600));

        let mut t = 0;
        exact.update(t, 0.0);
        gc.update(t, 0.0);

        let mut worst = 0.0_f64;

        for (watts, secs) in plan {
            for _ in 0..secs {
                t += S;

                let a = exact.update(t, watts);
                let b = gc.update(t, watts);

                worst = worst.max((a - b).abs());
            }
        }

        assert!(worst < 0.005 * 20_000.0, "worst difference {worst:.1} J");

        std::println!("Exact vs GC worst difference: {worst:.3} J");
    }

    #[test]
    fn waterworth_matches_closed_form_above_cp() {
        let mut m = WbalWaterworth::new(par());
        let mut t = start(&mut m);

        ride(&mut m, &mut t, 350.0, 100 * S);

        let tau = 546.0 * (-0.01_f64 * 250.0).exp() + 316.0;
        let want = 20_000.0 - 100.0 * tau * (1.0 - (-100.0 / tau).exp());

        assert!((m.balance_j() - want).abs() < 1e-6);
        assert!(m.balance_j() > 10_000.0);
    }

    #[test]
    fn waterworth_tau_lengthens_with_easier_recovery() {
        let mut m = WbalWaterworth::new(par());
        let mut t = start(&mut m);

        ride(&mut m, &mut t, 150.0, 60 * S);

        let want = 546.0 * (-1.0_f64).exp() + 316.0;

        assert!((m.tau_s() - want).abs() < 1e-9);
    }
}
