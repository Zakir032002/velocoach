//! The rider, stepped every 250 ms.
//! Models power/noise, true W′, heart-rate lag and drift, cadence and crank data.

use heapless::Deque;
use vc_core::study::Pcg32;
use vc_gatt::CrankData;

pub const STEP_NS: u64 = 250_000_000;
const DT_S: f64 = 0.25;

/// At this grade or steeper downhill, power drops without a coaching cue.
pub const DESCENT_GRADE_PCT: f64 = -3.0;

const LAG_CAP: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiderParams {
    pub cp_w: f64,
    /// The rider's true W′; distinct from the coach's estimate.
    pub wprime_j: f64,
    pub hr_rest: f64,
    pub hr_max: f64,
    pub hr_lag_s: u32,
    pub tau_on_s: f64,
    pub tau_off_s: f64,
    /// Cardiac drift: effective power increases with energy expenditure.
    pub drift_per_mj: f64,
    pub power_noise_frac: f64,
    pub hr_noise_bpm: i32,
    /// Power cap after W′ runs out, expressed as a fraction of CP.
    pub blowup_cap_frac: f64,
    /// W′ fraction required to recover from blow-up.
    pub recover_frac: f64,
    /// Power fraction on a descent.
    pub descent_power_frac: f64,
}

impl RiderParams {
    pub const DEFAULT: Self = Self {
        cp_w: 250.0,
        wprime_j: 20_000.0,
        hr_rest: 55.0,
        hr_max: 190.0,
        hr_lag_s: 15,
        tau_on_s: 35.0,
        tau_off_s: 60.0,
        drift_per_mj: 0.39,
        power_noise_frac: 0.03,
        hr_noise_bpm: 1,
        blowup_cap_frac: 0.92,
        recover_frac: 0.10,
        descent_power_frac: 0.4,
    };

    /// Noise-free, drift-free parameters for exact step-response tests.
    pub const fn clean() -> Self {
        Self {
            drift_per_mj: 0.0,
            power_noise_frac: 0.0,
            hr_noise_bpm: 0,
            ..Self::DEFAULT
        }
    }

    fn hr_target(&self, p_eff: f64) -> f64 {
        let span = 0.92 * self.hr_max - self.hr_rest;
        (self.hr_rest + span * p_eff / self.cp_w).min(self.hr_max)
    }
}

impl Default for RiderParams {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Target for one simulated rider step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effort {
    pub target_w: f64,
    pub target_rpm: f64,
    pub grade_pct: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RiderFaults {
    pub coasting: bool,
    pub hr_redline: bool,
}

/// Ground truth: the real simulated rider state for one step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Truth {
    pub t_ns: u64,
    pub power_w: f64,
    pub hr_bpm: f64,
    pub hr_reported: u16,
    pub cadence_rpm: f64,
    pub wprime_true_j: f64,
    pub blown: bool,
    pub target_w: f64,
    pub grade_pct: f64,
}

/// Exact W′ integration for constant power over this time interval.
/// Deliberately separate from the coach's physiology implementation.
fn wbal_step(w: f64, p: f64, cp: f64, wp: f64, dt: f64) -> f64 {
    if p >= cp {
        w - (p - cp) * dt
    } else {
        wp - (wp - w) * libm::exp(-(cp - p) * dt / wp)
    }
}

#[derive(Debug, Clone)]
pub struct Rider {
    p: RiderParams,
    hr: f64,
    lag: Deque<f64, LAG_CAP>,
    energy_kj: f64,
    w_j: f64,
    blown: bool,
    truth: Truth,
    revs_f: f64,
    revs: u16,
    event_1024: u16,
}

impl Rider {
    pub fn new(p: RiderParams) -> Self {
        let truth = Truth {
            t_ns: 0,
            power_w: 0.0,
            hr_bpm: p.hr_rest,
            hr_reported: report(p.hr_rest, 0),
            cadence_rpm: 0.0,
            wprime_true_j: p.wprime_j,
            blown: false,
            target_w: 0.0,
            grade_pct: 0.0,
        };

        Self {
            p,
            hr: p.hr_rest,
            lag: Deque::new(),
            energy_kj: 0.0,
            w_j: p.wprime_j,
            blown: false,
            truth,
            revs_f: 0.0,
            revs: 0,
            event_1024: 0,
        }
    }

    pub fn params(&self) -> RiderParams {
        self.p
    }

    pub fn truth(&self) -> Truth {
        self.truth
    }

    pub fn crank(&self) -> CrankData {
        CrankData {
            revs: self.revs,
            time_1024: self.event_1024,
        }
    }

    /// True W′ fraction. Sam stops responding below 10% in the persona model.
    pub fn wprime_fraction(&self) -> f64 {
        self.w_j / self.p.wprime_j
    }

    /// Inject a W′ mismatch; carry over the current balance, capped to the new W′.
    pub fn set_true_wprime(&mut self, wprime_j: f64) {
        if wprime_j > 0.0 {
            self.p.wprime_j = wprime_j;
            self.w_j = self.w_j.min(wprime_j);
        }
    }

    pub fn step(
        &mut self,
        t_ns: u64,
        effort: Effort,
        faults: RiderFaults,
        rng: &mut Pcg32,
    ) -> Truth {
        let p = self.p;

        // Always draw both noise values every step. Ride noise must not shift
        // when other simulator events occur.
        let z = rng.normal();
        let jitter = rng.int_in(-p.hr_noise_bpm, p.hr_noise_bpm);

        // 1. Power: grade effects, W′ blow-up cap, then noise.
        let target_w = if effort.grade_pct <= DESCENT_GRADE_PCT {
            effort.target_w * p.descent_power_frac
        } else {
            effort.target_w
        };

        let cap = if self.blown {
            p.cp_w * p.blowup_cap_frac
        } else {
            f64::INFINITY
        };

        let power = if faults.coasting {
            0.0
        } else {
            (target_w.max(0.0).min(cap) * (1.0 + p.power_noise_frac * z)).max(0.0)
        };

        // 2. Rider's true W′ and blow-up state.
        self.w_j = wbal_step(self.w_j, power, p.cp_w, p.wprime_j, DT_S).min(p.wprime_j);

        if self.w_j <= 0.0 {
            self.blown = true;
        } else if self.blown && self.w_j > p.recover_frac * p.wprime_j {
            self.blown = false;
        }

        // 3. Cardiac drift plus the 15-second dead-time lag.
        self.energy_kj += power * DT_S / 1000.0;
        let p_eff = power * (1.0 + p.drift_per_mj * self.energy_kj / 1000.0);

        if self.lag.is_full() {
            self.lag.pop_front();
        }
        let _ = self.lag.push_back(p_eff);

        let lag_n = (p.hr_lag_s as usize * 4).min(LAG_CAP - 1);
        let delayed = if self.lag.len() > lag_n {
            self.lag.pop_front().unwrap_or(0.0)
        } else {
            0.0
        };

        // 4. First-order heart-rate response, faster rising than falling.
        let target = if faults.hr_redline {
            0.995 * p.hr_max
        } else {
            p.hr_target(delayed)
        };

        let tau = if target > self.hr {
            p.tau_on_s
        } else {
            p.tau_off_s
        };

        self.hr += (target - self.hr) * (1.0 - libm::exp(-DT_S / tau));

        // 5. Cadence and crank counters.
        let cadence = if faults.coasting || effort.target_rpm <= 0.0 {
            0.0
        } else if self.blown {
            0.9 * effort.target_rpm
        } else {
            effort.target_rpm
        };

        self.advance_crank(t_ns as f64 / 1e9, cadence);

        self.truth = Truth {
            t_ns,
            power_w: power,
            hr_bpm: self.hr,
            hr_reported: report(self.hr, jitter),
            cadence_rpm: cadence,
            wprime_true_j: self.w_j,
            blown: self.blown,
            target_w,
            grade_pct: effort.grade_pct,
        };

        self.truth
    }

    fn advance_crank(&mut self, t_end_s: f64, rpm: f64) {
        if rpm <= 0.0 {
            return;
        }

        let rev_per_s = rpm / 60.0;
        let (before, after) = (self.revs_f, self.revs_f + rev_per_s * DT_S);
        let (wb, wa) = (libm::floor(before), libm::floor(after));

        if wa > wb {
            let t_last = t_end_s - DT_S + (wa - before) / rev_per_s;
            self.revs = self.revs.wrapping_add((wa - wb) as u16);
            self.event_1024 = (libm::round(t_last * 1024.0) as u64 % 65_536) as u16;
        }

        self.revs_f = after;
    }
}

fn report(hr: f64, jitter: i32) -> u16 {
    (libm::round(hr) as i32)
        .saturating_add(jitter)
        .clamp(30, 250) as u16
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn effort(w: f64) -> Effort {
        Effort {
            target_w: w,
            target_rpm: 90.0,
            grade_pct: 0.0,
        }
    }

    fn ride(r: &mut Rider, rng: &mut Pcg32, steps: u64, e: Effort) -> Truth {
        let mut t = r.truth();

        for _ in 0..steps {
            t = r.step(t.t_ns + STEP_NS, e, RiderFaults::default(), rng);
        }

        t
    }

    #[test]
    fn hr_waits_15_s_then_rises_first_order() {
        let (mut r, mut rng) = (Rider::new(RiderParams::clean()), Pcg32::new(1, 0));

        assert_eq!(ride(&mut r, &mut rng, 60, effort(200.0)).hr_bpm, 55.0);

        let t = ride(&mut r, &mut rng, 140, effort(200.0));
        let target = 55.0 + (0.92 * 190.0 - 55.0) * 200.0 / 250.0;

        assert!(
            (t.hr_bpm - (target - (target - 55.0) * libm::exp(-1.0))).abs() < 1e-9,
            "{}",
            t.hr_bpm
        );
    }

    #[test]
    fn blow_up_caps_power_until_w_prime_recovers() {
        let p = RiderParams {
            wprime_j: 5_000.0,
            ..RiderParams::clean()
        };

        let (mut r, mut rng) = (Rider::new(p), Pcg32::new(1, 0));

        assert!(!ride(&mut r, &mut rng, 133, effort(400.0)).blown);
        assert!(ride(&mut r, &mut rng, 1, effort(400.0)).blown);
        assert_eq!(
            ride(&mut r, &mut rng, 1, effort(400.0)).power_w,
            250.0 * 0.92
        );
    }

    #[test]
    fn power_drops_on_its_own_on_a_descent() {
        let (mut r, mut rng) = (Rider::new(RiderParams::clean()), Pcg32::new(1, 0));

        let t = ride(
            &mut r,
            &mut rng,
            1,
            Effort {
                grade_pct: -6.0,
                ..effort(300.0)
            },
        );

        assert_eq!((t.power_w, t.grade_pct), (300.0 * 0.4, -6.0));
    }

    #[test]
    fn same_seed_same_ride() {
        let (mut a, mut b) = (
            Rider::new(RiderParams::DEFAULT),
            Rider::new(RiderParams::DEFAULT),
        );

        let (mut ra, mut rb) = (Pcg32::new(42, 0), Pcg32::new(42, 0));

        for k in 1..=2_400_u64 {
            let e = effort(250.0);

            assert_eq!(
                a.step(k * STEP_NS, e, RiderFaults::default(), &mut ra),
                b.step(k * STEP_NS, e, RiderFaults::default(), &mut rb)
            );
        }
    }
}
