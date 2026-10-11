// Imagine you're cycling, and Velocoach is receiving heart rate, power, and cadence measurements from the sensors.
// At first, everything works correctly:
// Heart rate:  145 bpm  ✓
// Power:       280 W   ✓
// Cadence:      90 rpm ✓

// Now the heart-rate strap disconnects, but the application still has the last reading: 145 bpm.
// Two seconds later, the screen might still display 145 bpm. But that value describes the rider at the time of the last successful measurement, not necessarily their current state.
// If the coaching engine blindly trusts that old value, it could make a decision based on outdated information.
// This is the problem quality.rs solves.
//Its responsibility is to track whether each sensor channel currently provides trustworthy data
//    iQOO: simulated sensors
//              |
//              v
//     BLE notifications
//              |
//              v
//           vc-gatt
//     Decode sensor packets
//              |
//              v
//         vc-physio
//       +------+------+
//       |             |
//       v             v
//  W′ balance    QualityTracker
//  calculation   per sensor channel
//       |             |
//       |             v
//       |      Fresh / Stale /
//       |      Lost / Recovering
//       |             |
//       +------+------+
//              |
//              v
//           vc-core
//    Check data quality and
//    coaching / safety rules
//              |
//              v
//       Voice cue decision
//We need to represent one of a finite set of possible states. An enum makes that explicit: a Quality value can be Fresh, Stale, Lost, or Recovering with a counter.
//! Per-channel data quality. Only `Fresh` data may drive coaching cues.

use core::mem::discriminant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Fresh,
    Stale,
    Lost,
    Recovering { good: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualityConfig {
    pub stale_after_ns: u64,
    pub lost_after_ns: u64,
    pub good_to_recover: u8,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            stale_after_ns: 3_000_000_000,
            lost_after_ns: 10_000_000_000,
            good_to_recover: 3,
        }
    }
}

/// A change of state kind, reported once so the engine can queue exactly one cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    pub from: Quality,
    pub to: Quality,
}

#[derive(Debug, Clone)]
pub struct QualityTracker {
    cfg: QualityConfig,
    state: Quality,
    last_good_ns: Option<u64>,
}

impl QualityTracker {
    pub fn new(cfg: QualityConfig) -> Self {
        Self {
            cfg,
            state: Quality::Lost,
            last_good_ns: None,
        }
    }

    pub fn state(&self) -> Quality {
        self.state
    }

    pub fn is_fresh(&self) -> bool {
        self.state == Quality::Fresh
    }

    /// Call four times per second.
    pub fn on_tick(&mut self, now_ns: u64) -> Option<Change> {
        let from = self.state;
        self.age(now_ns);
        self.changed(from)
    }

    /// Call for every packet on this channel.
    /// `ok` means the packet decoded and passed plausibility checks.
    pub fn on_sample(&mut self, now_ns: u64, ok: bool) -> Option<Change> {
        let from = self.state;

        // Catch long silences even if a periodic tick was missed.
        self.age(now_ns);

        if ok {
            self.last_good_ns = Some(now_ns);

            self.state = match self.state {
                Quality::Fresh => Quality::Fresh,
                Quality::Stale | Quality::Lost => self.recovering(1),
                Quality::Recovering { good } => self.recovering(good.saturating_add(1)),
            };
        } else if let Quality::Recovering { .. } = self.state {
            // Garbage breaks the run of consecutive good samples.
            self.state = Quality::Recovering { good: 0 };
        }

        self.changed(from)
    }

    fn recovering(&self, good: u8) -> Quality {
        if good >= self.cfg.good_to_recover {
            Quality::Fresh
        } else {
            Quality::Recovering { good }
        }
    }

    fn age(&mut self, now_ns: u64) {
        let Some(last) = self.last_good_ns else {
            return;
        };

        let age = now_ns.saturating_sub(last);

        self.state = match self.state {
            Quality::Lost => Quality::Lost,
            _ if age > self.cfg.lost_after_ns => Quality::Lost,
            Quality::Fresh | Quality::Recovering { .. } if age > self.cfg.stale_after_ns => {
                Quality::Stale
            }
            state => state,
        };
    }

    fn changed(&self, from: Quality) -> Option<Change> {
        (discriminant(&from) != discriminant(&self.state)).then_some(Change {
            from,
            to: self.state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Quality::*;

    const S: u64 = 1_000_000_000;
    const MS: u64 = 1_000_000;

    fn tracker() -> QualityTracker {
        QualityTracker::new(QualityConfig::default())
    }

    /// Fresh, with the last good sample at t (t >= 2 s).
    fn fresh_at(t: u64) -> QualityTracker {
        let mut q = tracker();

        for k in [t - 2 * S, t - S, t] {
            q.on_sample(k, true);
        }

        assert_eq!(q.state(), Fresh);
        q
    }

    #[test]
    fn starts_lost_and_needs_three_good_samples() {
        let mut q = tracker();

        assert_eq!(q.state(), Lost);
        assert_eq!(
            q.on_sample(0, true).map(|c| c.to),
            Some(Recovering { good: 1 })
        );

        assert_eq!(q.on_sample(S, true), None);
        assert_eq!(q.state(), Recovering { good: 2 });

        assert_eq!(q.on_sample(2 * S, true).map(|c| c.to), Some(Fresh));
    }

    #[test]
    fn stale_strictly_after_3_s_lost_strictly_after_10_s() {
        let mut q = fresh_at(2 * S);

        assert_eq!(q.on_tick(5 * S), None);
        assert_eq!(q.on_tick(5 * S + 1).map(|c| c.to), Some(Stale));

        assert_eq!(q.on_tick(12 * S), None);
        assert_eq!(q.on_tick(12 * S + 1).map(|c| c.to), Some(Lost));
        assert_eq!(q.on_tick(13 * S), None);
    }

    #[test]
    fn garbage_never_refreshes() {
        let mut q = fresh_at(2 * S);

        for k in 1..=8 {
            q.on_sample(2 * S + k * 500 * MS, false);
        }

        assert_eq!(q.state(), Stale);
    }

    #[test]
    fn garbage_breaks_a_recovery_run() {
        let mut q = tracker();

        q.on_sample(0, true);
        q.on_sample(100 * MS, false);
        assert_eq!(q.state(), Recovering { good: 0 });

        q.on_sample(200 * MS, true);
        q.on_sample(300 * MS, true);
        assert_eq!(q.state(), Recovering { good: 2 });

        q.on_sample(400 * MS, true);
        assert_eq!(q.state(), Fresh);
    }

    #[test]
    fn recovering_without_data_goes_stale() {
        let mut q = tracker();

        q.on_sample(0, true);

        assert_eq!(q.on_tick(3 * S + 1).map(|c| c.to), Some(Stale));
    }

    /// HR at 1 Hz, silent from 11 s to 22 s, with ticks at 4 Hz.
    #[test]
    fn hr_dropout_story() {
        let mut q = tracker();
        let mut tos = Vec::new();

        for step in 0..=160u64 {
            let t = step * 250 * MS;
            let second = t / S;

            if t.is_multiple_of(S) && !(11..=22).contains(&second) {
                tos.extend(q.on_sample(t, true).map(|c| c.to));
            }

            tos.extend(q.on_tick(t).map(|c| c.to));
        }

        let r1 = Recovering { good: 1 };

        assert_eq!(tos, vec![r1, Fresh, Stale, Lost, r1, Fresh]);
    }
}
