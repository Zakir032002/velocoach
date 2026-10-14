//! P3: Aki's 3-second power smoothing as the observer uses it. Samples land in
//! 1-s bins; P3(k) is the mean of the bin means for seconds k-2, k-1 and k.
//! A fixed 40-bin ring: no allocation.

pub const RING_BINS: usize = 40;
pub const NS_PER_S: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy)]
struct Bin {
    second: u64,
    sum: f64,
    n: u32,
}

const EMPTY: Bin = Bin {
    second: u64::MAX,
    sum: 0.0,
    n: 0,
};

#[derive(Debug, Clone)]
pub struct P3Ring {
    bins: [Bin; RING_BINS],
    newest: Option<u64>,
}

impl Default for P3Ring {
    fn default() -> Self {
        Self::new()
    }
}

impl P3Ring {
    pub const fn new() -> Self {
        Self {
            bins: [EMPTY; RING_BINS],
            newest: None,
        }
    }

    /// One power sample (Fresh and plausible: the caller filters) at `t_ns`.
    pub fn push(&mut self, t_ns: u64, watts: f64) {
        let k = t_ns / NS_PER_S;
        if self.newest.is_some_and(|n| k + RING_BINS as u64 <= n) {
            return; // older than the ring: nothing to update
        }
        if let Some(slot) = self.bins.get_mut((k % RING_BINS as u64) as usize) {
            if slot.second != k {
                *slot = Bin {
                    second: k,
                    sum: 0.0,
                    n: 0,
                };
            }
            slot.sum += watts;
            slot.n += 1;
        }
        self.newest = Some(self.newest.map_or(k, |n| n.max(k)));
    }

    /// The second the newest sample fell in. Every bin before it is complete.
    pub fn newest_second(&self) -> Option<u64> {
        self.newest
    }

    /// The mean of second `k`'s samples, if it has any and is still in the ring.
    pub fn bin_mean(&self, k: u64) -> Option<f64> {
        if self
            .newest
            .is_none_or(|n| k > n || k + RING_BINS as u64 <= n)
        {
            return None;
        }
        let b = self.bins.get((k % RING_BINS as u64) as usize)?;
        (b.second == k && b.n > 0).then(|| b.sum / f64::from(b.n))
    }

    /// P3(k): the mean of the bin means of k-2, k-1, k. None if any is missing.
    pub fn p3(&self, k: u64) -> Option<f64> {
        let a = self.bin_mean(k.checked_sub(2)?)?;
        let b = self.bin_mean(k - 1)?;
        let c = self.bin_mean(k)?;
        Some((a + b + c) / 3.0)
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    /// 4 Hz samples from `from_s` (inclusive) to `to_s` (exclusive) at `w(t_s)`.
    fn fill(r: &mut P3Ring, from_s: f64, to_s: f64, w: impl Fn(f64) -> f64) {
        let mut t = from_s;
        while t < to_s {
            r.push((t * 1e9) as u64, w(t));
            t += 0.25;
        }
    }

    #[test]
    fn p3_is_the_mean_of_three_one_second_bins() {
        let mut r = P3Ring::new();
        fill(&mut r, 0.0, 10.0, |t| if t < 5.5 { 200.0 } else { 160.0 });
        assert_eq!(r.bin_mean(5), Some(180.0)); // half the bin at each level
        assert_eq!(r.p3(4), Some(200.0));
        assert!((r.p3(6).unwrap() - (200.0 + 180.0 + 160.0) / 3.0).abs() < 1e-9);
        assert_eq!(r.p3(7), Some((180.0 + 160.0 + 160.0) / 3.0));
    }

    #[test]
    fn a_missing_second_means_no_p3() {
        let mut r = P3Ring::new();
        fill(&mut r, 0.0, 5.0, |_| 200.0);
        fill(&mut r, 6.0, 10.0, |_| 200.0); // nothing in second 5
        assert_eq!(r.p3(4), Some(200.0));
        assert_eq!(r.p3(5), None);
        assert_eq!(r.p3(7), None); // 5 is in its window
        assert_eq!(r.p3(8), Some(200.0));
    }

    #[test]
    fn the_rate_does_not_matter() {
        let (mut fast, mut slow) = (P3Ring::new(), P3Ring::new());
        fill(&mut fast, 0.0, 10.0, |_| 250.0);
        for s in 0..10 {
            slow.push(s * 1_000 * MS + 500 * MS, 250.0); // 1 Hz, mid-second
        }
        assert_eq!(fast.p3(8), slow.p3(8));
    }

    #[test]
    fn bins_older_than_40_s_are_gone() {
        let mut r = P3Ring::new();
        fill(&mut r, 0.0, 50.0, |_| 200.0);
        assert_eq!(r.bin_mean(5), None);
        assert_eq!(r.bin_mean(10), Some(200.0));
        assert_eq!(r.p3(1), None);
    }
}
