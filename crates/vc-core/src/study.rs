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
}
