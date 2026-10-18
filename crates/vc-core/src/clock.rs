//! Cristian-style clock sync between the rig (iQOO) and the coach (iPhone).
//! PING sent at a (phone), received at b and answered at c (rig), PONG received
//! at d (phone):  rtt = (d - a) - (c - b),  theta = ((b - a) + (c - d)) / 2.
//! The minimum-rtt sample of the last 16 wins; its error bound is rtt / 2.

pub const WINDOW: usize = 16;
const PENDING: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub rtt_ns: u64,
    /// Rig clock minus phone clock.
    pub theta_ns: i64,
    pub at_ns: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ClockSync {
    samples: [Option<Sample>; WINDOW],
    next: usize,
    pending: [Option<(u16, u64)>; PENDING],
    pending_next: usize,
    last_id: u16,
}

impl ClockSync {
    /// A new PING sent at `a_ns` (phone clock). Returns its id (never 0).
    pub fn make_ping(&mut self, a_ns: u64) -> u16 {
        self.last_id = self.last_id.wrapping_add(1).max(1);
        self.pending[self.pending_next % PENDING] = Some((self.last_id, a_ns));
        self.pending_next += 1;
        self.last_id
    }

    /// A PONG received at `d_ns` (phone clock) for `ping_id`, stamped b and c by the rig.
    pub fn on_pong(&mut self, ping_id: u16, b_ns: u64, c_ns: u64, d_ns: u64) -> Option<Sample> {
        let slot = self
            .pending
            .iter_mut()
            .find(|p| p.is_some_and(|(id, _)| id == ping_id))?;
        let (_, a) = slot.take()?;
        let (a, b, c, d) = (
            i128::from(a),
            i128::from(b_ns),
            i128::from(c_ns),
            i128::from(d_ns),
        );
        let rtt = (d - a) - (c - b);
        if rtt < 0 || c < b || d < a {
            return None; // impossible timings: drop the sample
        }
        let theta = ((b - a) + (c - d)) / 2;
        let s = Sample {
            rtt_ns: rtt as u64,
            theta_ns: theta as i64,
            at_ns: d_ns,
        };
        self.samples[self.next % WINDOW] = Some(s);
        self.next += 1;
        Some(s)
    }

    /// The minimum-rtt sample of the last 16.
    pub fn best(&self) -> Option<Sample> {
        self.samples
            .iter()
            .flatten()
            .copied()
            .min_by_key(|s| s.rtt_ns)
    }

    pub fn bound_ns(&self) -> Option<u64> {
        self.best().map(|s| s.rtt_ns / 2)
    }

    pub fn samples(&self) -> usize {
        self.samples.iter().flatten().count()
    }

    /// A rig timestamp on the phone's clock.
    pub fn to_phone(&self, rig_ns: u64) -> Option<u64> {
        let s = self.best()?;
        u64::try_from(i128::from(rig_ns) - i128::from(s.theta_ns)).ok()
    }

    /// After a reconnection: start over (the rig may have restarted).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;
    const RIG_AHEAD: u64 = 5_000 * MS; // the rig's clock reads 5 s more

    /// One exchange: sent at a (phone), uplink `up`, rig holds it `hold`, downlink `down`.
    fn exchange(c: &mut ClockSync, a: u64, up: u64, hold: u64, down: u64) -> Option<Sample> {
        let id = c.make_ping(a);
        let b = a + up + RIG_AHEAD;
        c.on_pong(id, b, b + hold, a + up + hold + down)
    }

    #[test]
    fn symmetric_legs_give_the_exact_offset() {
        let mut c = ClockSync::default();
        let s = exchange(&mut c, 100 * MS, 10 * MS, 2 * MS, 10 * MS).unwrap();
        assert_eq!((s.rtt_ns, s.theta_ns), (20 * MS, RIG_AHEAD as i64));
        assert_eq!(c.bound_ns(), Some(10 * MS));
        assert_eq!(c.to_phone(RIG_AHEAD + 777 * MS), Some(777 * MS));
    }

    #[test]
    fn the_error_never_exceeds_the_bound() {
        let mut c = ClockSync::default();
        let s = exchange(&mut c, 100 * MS, 4 * MS, MS, 16 * MS).unwrap(); // lopsided
        let error = (s.theta_ns - RIG_AHEAD as i64).unsigned_abs();
        assert!(
            error <= s.rtt_ns / 2,
            "error {error} bound {}",
            s.rtt_ns / 2
        );
    }

    #[test]
    fn the_minimum_rtt_sample_of_the_last_16_wins() {
        let mut c = ClockSync::default();
        exchange(&mut c, 0, 30 * MS, MS, 30 * MS);
        exchange(&mut c, 1_000 * MS, 6 * MS, MS, 8 * MS); // the best
        for k in 2..10 {
            exchange(&mut c, k * 1_000 * MS, 20 * MS, MS, 25 * MS);
        }
        assert_eq!(c.best().map(|s| s.rtt_ns), Some(14 * MS));
        for k in 10..30 {
            exchange(&mut c, k * 1_000 * MS, 20 * MS, MS, 25 * MS); // pushes the best out
        }
        assert_eq!(c.best().map(|s| s.rtt_ns), Some(45 * MS));
    }

    #[test]
    fn unknown_or_impossible_pongs_are_dropped() {
        let mut c = ClockSync::default();
        assert_eq!(c.on_pong(99, 1, 2, 3), None);
        let id = c.make_ping(1_000);
        assert_eq!(c.on_pong(id, 5_000, 4_000, 2_000), None); // c < b
    }
}
