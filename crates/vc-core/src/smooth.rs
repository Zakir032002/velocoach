//! Aki's 3-second power smoothing: mean of samples in (t - 3 s, t].

use heapless::Deque;

pub const WINDOW_NS: u64 = 3_000_000_000;
const CAP: usize = 32; // 4 Hz × 3 s = 12; room for faster meters.

#[derive(Debug, Clone, Default)]
pub struct Smooth3s {
    buf: Deque<(u64, f64), CAP>,
}

impl Smooth3s {
    pub fn push(&mut self, t_ns: u64, watts: f64) {
        self.evict(t_ns);

        if self.buf.is_full() {
            let _ = self.buf.pop_front();
        }

        let _ = self.buf.push_back((t_ns, watts));
    }

    /// Mean of samples newer than t - 3 s, or None if there are none.
    pub fn mean_at(&self, t_ns: u64) -> Option<f64> {
        let (mut sum, mut n) = (0.0, 0u32);

        for &(t0, w) in self.buf.iter() {
            if t0.saturating_add(WINDOW_NS) > t_ns {
                sum += w;
                n += 1;
            }
        }

        (n > 0).then(|| sum / f64::from(n))
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    fn evict(&mut self, t_ns: u64) {
        while let Some(&(t0, _)) = self.buf.front() {
            if t0.saturating_add(WINDOW_NS) <= t_ns {
                let _ = self.buf.pop_front();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    #[test]
    fn mean_of_the_last_three_seconds_only() {
        let mut s = Smooth3s::default();

        for k in 0..12 {
            s.push(k * 250 * MS, 200.0); // 0.00 .. 2.75 s
        }
        for k in 12..16 {
            s.push(k * 250 * MS, 400.0); // 3.00 .. 3.75 s
        }

        // At 3.75 s: 8 samples at 200 W and 4 at 400 W.
        let m = s.mean_at(3_750 * MS).unwrap();
        assert!((m - (8.0 * 200.0 + 4.0 * 400.0) / 12.0).abs() < 1e-9);
    }

    #[test]
    fn an_old_buffer_reads_as_empty() {
        let mut s = Smooth3s::default();
        s.push(0, 300.0);

        assert_eq!(s.mean_at(3_000 * MS), None);
        assert_eq!(Smooth3s::default().mean_at(0), None);
    }
}
