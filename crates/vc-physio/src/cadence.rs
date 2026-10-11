//! Cadence from cumulative crank revolutions and the time of the last
//! revolution. Both are u16 and wrap (event time every 64 s): wrapping_sub.

/// No new revolution for longer than this means cadence 0.
pub const COAST_AFTER_NS: u64 = 2_500_000_000;

const MAX_RPM: f64 = 250.0;
const GARBAGE_RUN_TO_REBASELINE: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CadenceUpdate {
    /// First packet, or first revolution after a stop or a sensor reset.
    Baseline,
    /// New revolution(s): a fresh cadence value.
    Rpm(f64),
    /// Same counters as last time; not new data.
    Repeat,
    /// Repeats for more than 2.5 s: cadence is now 0.
    Coasting,
    /// Inconsistent or implausible counters; ignored.
    Garbage,
}

#[derive(Debug, Clone, Default)]
pub struct CadenceDeriver {
    /// (revs, time_1024) of the last accepted packet.
    prev: Option<(u16, u16)>,
    last_change_ns: u64,
    rpm: f64,
    garbage_run: u8,
}

impl CadenceDeriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rpm(&self) -> f64 {
        self.rpm
    }

    pub fn push(&mut self, revs: u16, time_1024: u16, now_ns: u64) -> CadenceUpdate {
        let Some((prev_revs, prev_time)) = self.prev else {
            return self.baseline(revs, time_1024, now_ns);
        };

        let since = now_ns.saturating_sub(self.last_change_ns);
        let drev = revs.wrapping_sub(prev_revs);
        let dt = time_1024.wrapping_sub(prev_time);

        if drev == 0 && dt == 0 {
            if since > COAST_AFTER_NS {
                self.rpm = 0.0;
                return CadenceUpdate::Coasting;
            }

            return CadenceUpdate::Repeat;
        }

        if since > COAST_AFTER_NS {
            return self.baseline(revs, time_1024, now_ns);
        }

        let rpm = (drev != 0 && dt != 0).then(|| 60.0 * f64::from(drev) * 1024.0 / f64::from(dt));

        match rpm {
            Some(rpm) if rpm <= MAX_RPM => {
                self.prev = Some((revs, time_1024));
                self.last_change_ns = now_ns;
                self.rpm = rpm;
                self.garbage_run = 0;

                CadenceUpdate::Rpm(rpm)
            }
            _ => {
                self.garbage_run = self.garbage_run.saturating_add(1);

                if self.garbage_run >= GARBAGE_RUN_TO_REBASELINE {
                    // Probably a sensor reset: start over from these counters.
                    self.baseline(revs, time_1024, now_ns);
                }

                CadenceUpdate::Garbage
            }
        }
    }

    /// Call four times per second. Detects silent coasting even when the
    /// sensor stops notifying altogether. Returns true only once per stop.
    pub fn tick(&mut self, now_ns: u64) -> bool {
        let quiet = now_ns.saturating_sub(self.last_change_ns) > COAST_AFTER_NS;

        if self.prev.is_some() && self.rpm > 0.0 && quiet {
            self.rpm = 0.0;
            return true;
        }

        false
    }

    fn baseline(&mut self, revs: u16, time_1024: u16, now_ns: u64) -> CadenceUpdate {
        self.prev = Some((revs, time_1024));
        self.last_change_ns = now_ns;
        self.rpm = 0.0;
        self.garbage_run = 0;

        CadenceUpdate::Baseline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = 1_000_000_000;
    const MS: u64 = 1_000_000;

    #[test]
    fn ninety_rpm() {
        let mut c = CadenceDeriver::new();

        assert_eq!(c.push(1_000, 10_240, 0), CadenceUpdate::Baseline);

        // Three revolutions in 2048 ticks (2 seconds).
        assert_eq!(c.push(1_003, 12_288, 2 * S), CadenceUpdate::Rpm(90.0));
    }

    #[test]
    fn wraps_both_counters() {
        let mut c = CadenceDeriver::new();

        c.push(65_534, 65_000, 0);

        // Revs 65534 -> 1 is 3 revolutions.
        // Time 65000 -> 1512 is 2048 ticks.
        assert_eq!(c.push(1, 1_512, 2 * S), CadenceUpdate::Rpm(90.0));
    }

    #[test]
    fn repeats_then_coasting() {
        let mut c = CadenceDeriver::new();

        c.push(10, 1_024, 0);
        c.push(11, 1_707, 700 * MS);

        assert_eq!(c.push(11, 1_707, 1_500 * MS), CadenceUpdate::Repeat);
        assert!(c.rpm() > 89.0);

        assert_eq!(c.push(11, 1_707, 3_300 * MS), CadenceUpdate::Coasting);
        assert_eq!(c.rpm(), 0.0);
    }

    #[test]
    fn silent_coasting_via_tick() {
        let mut c = CadenceDeriver::new();

        c.push(10, 1_024, 0);
        c.push(11, 1_707, 700 * MS);

        assert!(!c.tick(3_000 * MS)); // 2.3 s quiet
        assert!(c.tick(3_300 * MS)); // 2.6 s quiet
        assert!(!c.tick(3_550 * MS)); // reported once
        assert_eq!(c.rpm(), 0.0);
    }

    #[test]
    fn first_revolution_after_a_stop_is_a_baseline() {
        let mut c = CadenceDeriver::new();

        c.push(10, 1_024, 0);
        c.push(11, 1_707, 700 * MS);

        // This revolution spans a long stop, so don't derive RPM from it.
        assert_eq!(c.push(12, 11_947, 10_700 * MS), CadenceUpdate::Baseline);

        let r = c.push(13, 12_630, 11_400 * MS);

        assert!(
            matches!(r, CadenceUpdate::Rpm(x) if (x - 89.96).abs() < 0.01),
            "{r:?}"
        );
    }

    #[test]
    fn implausible_and_inconsistent_counters_are_garbage() {
        let mut c = CadenceDeriver::new();

        c.push(10, 1_024, 0);

        assert_eq!(c.push(20, 2_048, S), CadenceUpdate::Garbage); // 600 RPM
        assert_eq!(c.push(11, 1_024, 2 * S), CadenceUpdate::Garbage);
        // Revolution count moved, event time did not.
    }

    #[test]
    fn three_garbage_in_a_row_rebaselines() {
        let mut c = CadenceDeriver::new();

        c.push(10, 1_024, 0);

        for k in 1..=3 {
            // Simulate a sensor reset to a new counter origin.
            assert_eq!(c.push(5_000, 40_000, k * 500 * MS), CadenceUpdate::Garbage);
        }

        let r = c.push(5_001, 40_683, 2_200 * MS);

        assert!(
            matches!(r, CadenceUpdate::Rpm(x) if (x - 89.96).abs() < 0.01),
            "{r:?}"
        );
    }
}
