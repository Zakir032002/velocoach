//! Deterministic safety gate. Fixed behavior, outside the tuner's reach.

use crate::config::EngineConfig;

#[derive(Debug, Clone)]
pub struct RedlineGate {
    fire_bpm: f64,
    rearm_bpm: f64,
    hold_ns: u64,
    rearm_ns: u64,
    armed: bool,
    above_since: Option<u64>,
    below_since: Option<u64>,
}

impl RedlineGate {
    pub fn new(cfg: &EngineConfig) -> Self {
        Self {
            fire_bpm: cfg.redline_frac * cfg.hr_max,
            rearm_bpm: cfg.redline_rearm_frac * cfg.hr_max,
            hold_ns: cfg.redline_hold_ns,
            rearm_ns: cfg.redline_rearm_ns,
            armed: true,
            above_since: None,
            below_since: None,
        }
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Call with every plausible HR sample. `fresh` means the HR channel
    /// is Fresh. Returns true exactly once per redline episode.
    pub fn on_hr(&mut self, t_ns: u64, bpm: f64, fresh: bool) -> bool {
        if !fresh {
            self.above_since = None;
            self.below_since = None;
            return false;
        }

        if self.armed {
            if bpm > self.fire_bpm {
                let since = *self.above_since.get_or_insert(t_ns);

                if t_ns.saturating_sub(since) >= self.hold_ns {
                    self.armed = false;
                    self.above_since = None;
                    return true;
                }
            } else {
                // "In a row" means every considered sample must stay above.
                self.above_since = None;
            }
        } else if bpm < self.rearm_bpm {
            let since = *self.below_since.get_or_insert(t_ns);

            if t_ns.saturating_sub(since) >= self.rearm_ns {
                self.armed = true;
                self.below_since = None;
            }
        } else {
            self.below_since = None;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = 1_000_000_000;

    /// Returns the seconds at which the gate fired, for 1 Hz input.
    fn feed(g: &mut RedlineGate, from_s: u64, samples: &[(f64, bool)]) -> std::vec::Vec<u64> {
        samples
            .iter()
            .enumerate()
            .filter_map(|(k, &(bpm, fresh))| {
                let t = from_s + k as u64;
                g.on_hr(t * S, bpm, fresh).then_some(t)
            })
            .collect()
    }

    fn gate() -> RedlineGate {
        RedlineGate::new(&EngineConfig::default())
        // Redline > 186.2 bpm; re-arm below 180.5 bpm.
    }

    #[test]
    fn fires_once_after_120_s_above() {
        assert_eq!(feed(&mut gate(), 0, &[(187.0, true); 200]), vec![120]);
    }

    #[test]
    fn one_sample_below_restarts_the_count() {
        let mut s = vec![(187.0, true); 101];
        s.push((185.0, true));
        s.extend([(187.0, true); 130]);

        assert_eq!(feed(&mut gate(), 0, &s), vec![222]);
    }

    #[test]
    fn stale_data_breaks_the_run() {
        let mut s = vec![(187.0, true); 60];
        s.push((187.0, false));
        s.extend([(187.0, true); 125]);

        assert_eq!(feed(&mut gate(), 0, &s), vec![181]);
    }

    #[test]
    fn rearms_only_after_30_s_below_95_percent() {
        let mut g = gate();

        assert_eq!(feed(&mut g, 0, &[(187.0, true); 121]), vec![120]);

        assert!(feed(&mut g, 121, &[(183.0, true); 300]).is_empty());
        // 96% HRmax is below the fire threshold but above the re-arm threshold.

        feed(&mut g, 421, &[(179.0, true); 30]);
        assert!(!g.is_armed()); // Thirty samples span only 29 seconds.

        feed(&mut g, 451, &[(179.0, true); 1]);
        assert!(g.is_armed());
    }
}
