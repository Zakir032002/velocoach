//! Randomness lives here and only here.
//! Day 7: PCG32 for the simulator.
//! Day 8 adds the Study allocator and policy hash.

const MULT: u64 = 6_364_136_223_846_793_005;

/// PCG32 (XSH RR 64/32).
/// A (seed, stream) pair gives a reproducible independent stream.
#[derive(Debug, Clone)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    /// Same seeding sequence as pcg32_srandom_r(seed, stream).
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut r = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };

        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULT).wrapping_add(self.inc);

        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;

        xorshifted.rotate_right(rot)
    }

    /// Uniform value in [0, 1), constructed with 53 random bits.
    pub fn next_f64(&mut self) -> f64 {
        let hi = u64::from(self.next_u32() >> 5);
        let lo = u64::from(self.next_u32() >> 6);

        ((hi << 26) | lo) as f64 / (1u64 << 53) as f64
    }

    /// Standard normal distribution using Box–Muller.
    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.next_f64();
        let u2 = self.next_f64();

        libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(core::f64::consts::TAU * u2)
    }

    /// Uniform integer in the inclusive range lo..=hi (hi >= lo).
    pub fn int_in(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next_f64() * f64::from(hi - lo + 1)) as i32
    }
}

use vc_efficacy::Arm;

/// Study allocation probabilities in Q16 representation.
pub const P_SHAM_Q16: u16 = 13_107;
pub const P_ARM_Q16: u16 = 26_214;

/// Used when a decision was not randomised.
pub const NOT_RANDOMISED_Q16: u16 = u16::MAX;

const STUDY_STREAM_BASE: u64 = 1 << 48;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Allocation {
    pub arm: Arm,
    pub propensity_q16: u16,
    pub draw_index: u32,
    pub u: f64,
}

#[derive(Debug, Clone)]
pub struct StudyAllocator {
    seed: u64,
    next: u32,
}

impl StudyAllocator {
    pub const fn new(seed: u64) -> Self {
        Self { seed, next: 0 }
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Allocate one eligible P2 decision point.
    pub fn draw(&mut self) -> Allocation {
        let allocation = Self::replay(self.seed, self.next);
        self.next = self.next.wrapping_add(1);
        allocation
    }

    /// Reproduce a draw without replaying previous decision points.
    pub fn replay(seed: u64, draw_index: u32) -> Allocation {
        let u = Pcg32::new(seed, STUDY_STREAM_BASE | u64::from(draw_index)).next_f64();

        let (arm, propensity_q16) = if u < 0.2 {
            (Arm::Sham, P_SHAM_Q16)
        } else if u < 0.6 {
            (Arm::Direct, P_ARM_Q16)
        } else {
            (Arm::Autonomy, P_ARM_Q16)
        };

        Allocation {
            arm,
            propensity_q16,
            draw_index,
            u,
        }
    }
}

/// FNV-1a 64-bit hash over canonical policy bytes.
#[derive(Debug, Clone, Copy)]
pub struct Fnv1a(u64);

impl Default for Fnv1a {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv1a {
    pub fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    pub fn write_u64(&mut self, value: u64) {
        self.write(&value.to_le_bytes());
    }

    pub const fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_reference_implementation() {
        let mut r = Pcg32::new(42, 54);
        let got: Vec<u32> = (0..6).map(|_| r.next_u32()).collect();

        assert_eq!(
            got,
            [
                0xa15c_02b7,
                0x7b47_f409,
                0xba1d_3330,
                0x83d2_f293,
                0xbfa4_784b,
                0xcbed_606e,
            ]
        );
    }

    #[test]
    fn streams_are_independent_and_reproducible() {
        let a: Vec<u32> = {
            let mut r = Pcg32::new(7, 1);
            (0..4).map(|_| r.next_u32()).collect()
        };

        let b: Vec<u32> = {
            let mut r = Pcg32::new(7, 1);
            (0..4).map(|_| r.next_u32()).collect()
        };

        let c: Vec<u32> = {
            let mut r = Pcg32::new(7, 2);
            (0..4).map(|_| r.next_u32()).collect()
        };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn normal_has_mean_zero_and_standard_deviation_one() {
        let mut r = Pcg32::new(3, 0);
        let xs: Vec<f64> = (0..100_000).map(|_| r.normal()).collect();

        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let sd =
            libm::sqrt(xs.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / xs.len() as f64);

        assert!(mean.abs() < 0.02 && (sd - 1.0).abs() < 0.02, "{mean} {sd}");
    }

    use vc_efficacy::Arm;

    #[test]
    fn arm_frequencies_are_within_99_percent_bounds() {
        let mut allocator = StudyAllocator::new(2026);
        let n = 10_000.0;
        let (mut sham, mut direct, mut autonomy, mut none) = (0.0, 0.0, 0.0, 0.0);

        for _ in 0..10_000 {
            match allocator.draw().arm {
                Arm::Sham => sham += 1.0,
                Arm::Direct => direct += 1.0,
                Arm::Autonomy => autonomy += 1.0,
                Arm::None => none += 1.0,
            }
        }

        assert_eq!(none, 0.0, "allocator must never assign Arm::None");

        let bound = |p: f64| 2.576 * libm::sqrt(p * (1.0 - p) / n);

        assert!((sham / n - 0.2).abs() < bound(0.2));
        assert!((direct / n - 0.4).abs() < bound(0.4));
        assert!((autonomy / n - 0.4).abs() < bound(0.4));
    }

    #[test]
    fn every_assignment_replays_from_seed_and_index() {
        let mut allocator = StudyAllocator::new(99);

        for index in 0..500 {
            assert_eq!(allocator.draw(), StudyAllocator::replay(99, index),);
        }
    }

    #[test]
    fn fnv1a_matches_the_published_vector() {
        let mut hash = Fnv1a::default();
        hash.write(b"a");

        assert_eq!(hash.finish(), 0xaf63_dc4c_8601_ec8c);
    }
}
