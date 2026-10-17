//! Fixed-size log-linear latency histograms. No allocation: 16 sub-buckets per
//! power of two, so every percentile is within 6.25 % of the truth; max is exact.

/// 16 exact buckets (0-15 ns), then 16 per power of two up to 2^63.
pub const BUCKETS: usize = 16 + 16 * 60;

#[derive(Debug, Clone)]
pub struct Histogram {
    counts: [u32; BUCKETS],
    n: u64,
    max_ns: u64,
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            counts: [0; BUCKETS],
            n: 0,
            max_ns: 0,
        }
    }
}

fn index(v: u64) -> usize {
    if v < 16 {
        return v as usize;
    }
    let e = 63 - u64::from(v.leading_zeros()); // >= 4
    let sub = (v >> (e - 4)) & 0xF;
    (16 + (e - 4) * 16 + sub).min(BUCKETS as u64 - 1) as usize
}

/// Smallest value that falls in bucket `i`.
fn lower(i: usize) -> u64 {
    if i < 16 {
        return i as u64;
    }
    let e = (i as u64 - 16) / 16 + 4;
    if e >= 64 {
        return u64::MAX; // one past the last bucket
    }
    let sub = (i as u64 - 16) % 16;
    (16 + sub) << (e - 4)
}

impl Histogram {
    pub fn record(&mut self, ns: u64) {
        if let Some(c) = self.counts.get_mut(index(ns)) {
            *c = c.saturating_add(1);
        }
        self.n += 1;
        self.max_ns = self.max_ns.max(ns);
    }

    pub fn count(&self) -> u64 {
        self.n
    }

    pub fn max_ns(&self) -> u64 {
        self.max_ns
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The q-quantile (0 < q <= 1), reported as the top of its bucket (never
    /// optimistic) and capped at the exact max. 0 when empty.
    pub fn quantile_ns(&self, q: f64) -> u64 {
        if self.n == 0 {
            return 0;
        }
        let target = libm::ceil(q.clamp(0.0, 1.0) * self.n as f64).max(1.0) as u64;
        let mut seen = 0u64;
        for (i, c) in self.counts.iter().enumerate() {
            seen += u64::from(*c);
            if seen >= target {
                return (lower(i + 1).saturating_sub(1)).min(self.max_ns);
            }
        }
        self.max_ns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_are_contiguous_and_cover_every_value() {
        for v in [0u64, 15, 16, 17, 31, 32, 33, 1_000, 999_999, 1 << 40] {
            let i = index(v);
            assert!(lower(i) <= v && v < lower(i + 1), "{v}");
        }
    }

    #[test]
    fn percentiles_are_within_one_sixteenth() {
        let mut h = Histogram::default();
        for v in 1..=10_000u64 {
            h.record(v * 100); // 100 ns .. 1 ms, uniform
        }
        for (q, exact) in [(0.5, 500_000.0), (0.9, 900_000.0), (0.99, 990_000.0)] {
            let got = h.quantile_ns(q) as f64;
            assert!(
                got >= exact && got <= exact * (1.0 + 1.0 / 16.0),
                "q{q}: {got}"
            );
        }
        assert_eq!(h.max_ns(), 1_000_000);
    }

    #[test]
    fn empty_reads_zero() {
        assert_eq!(Histogram::default().quantile_ns(0.99), 0);
    }
}
