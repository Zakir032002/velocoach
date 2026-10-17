//! The ledger: per (directive x arm x profile), how often cues were complied
//! with and how fast. Beta(1 + complied, 1 + n - complied) over the scorable
//! verdicts; Confounded and Invalid are counted but excluded. A 16-entry ring of
//! L_obs gives the median. Fixed size: no allocation.
use crate::observer::SMOOTHING_LAG_MS;
use crate::{Arm, CueEvent, CueOutcome, Verdict};

/// 8 directives x 4 arms x 3 profiles = 96 possible keys.
pub const MAX_KEYS: usize = 96;
pub const LATENCY_RING: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub directive: u8,
    pub arm: Arm,
    pub profile: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub complied: u32,
    pub partial: u32,
    pub ignored: u32,
    pub reversed: u32,
    pub confounded: u32,
    pub invalid: u32,
    /// Complied with a rider latency (L_obs - lag) under 1 s / under 2 s.
    pub complied_under_1s: u32,
    pub complied_under_2s: u32,
}

impl Counts {
    /// The scorable verdicts: the denominator of every rate.
    pub fn n(&self) -> u32 {
        self.complied + self.partial + self.ignored + self.reversed
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub key: Key,
    pub counts: Counts,
    pub beta_a: f64,
    pub beta_b: f64,
    /// Posterior mean a / (a + b).
    pub compliance: f64,
    pub median_l_obs_ms: Option<i32>,
}

impl Stats {
    pub fn n(&self) -> u32 {
        self.counts.n()
    }

    /// A raw proportion of the scorable cues; None with no data.
    pub fn rate(&self, k: u32) -> Option<f64> {
        let n = self.n();
        (n > 0).then(|| f64::from(k) / f64::from(n))
    }
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    key: Key,
    counts: Counts,
    lat: [i32; LATENCY_RING],
    lat_len: usize,
    lat_next: usize,
}

#[derive(Debug, Clone)]
pub struct Ledger {
    entries: [Option<Entry>; MAX_KEYS],
    total_scored: u32,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_KEYS],
            total_scored: 0,
        }
    }

    /// Every scorable verdict recorded so far (the tuner's "new data" clock).
    pub fn total_scored(&self) -> u32 {
        self.total_scored
    }

    pub fn record(&mut self, ev: &CueEvent, o: &CueOutcome) {
        let key = Key {
            directive: ev.directive,
            arm: if ev.is_sham { Arm::Sham } else { ev.arm },
            profile: ev.profile,
        };
        let Some(e) = self.entry_mut(key) else { return };
        let c = &mut e.counts;
        match o.verdict {
            Verdict::Complied => c.complied += 1,
            Verdict::Partial => c.partial += 1,
            Verdict::Ignored => c.ignored += 1,
            Verdict::Reversed => c.reversed += 1,
            Verdict::Confounded => c.confounded += 1,
            Verdict::Invalid => c.invalid += 1,
        }
        if let (Verdict::Complied, Some(l)) = (o.verdict, o.l_obs_ms) {
            let rider = l - SMOOTHING_LAG_MS;
            if rider < 1_000 {
                c.complied_under_1s += 1;
            }
            if rider < 2_000 {
                c.complied_under_2s += 1;
            }
            if let Some(slot) = e.lat.get_mut(e.lat_next) {
                *slot = l;
            }
            e.lat_next = (e.lat_next + 1) % LATENCY_RING;
            e.lat_len = (e.lat_len + 1).min(LATENCY_RING);
        }
        if matches!(
            o.verdict,
            Verdict::Complied | Verdict::Partial | Verdict::Ignored | Verdict::Reversed
        ) {
            self.total_scored += 1;
        }
    }

    pub fn stats(&self, directive: u8, arm: Arm, profile: u8) -> Option<Stats> {
        let key = Key {
            directive,
            arm,
            profile,
        };
        self.entries
            .iter()
            .flatten()
            .find(|e| e.key == key)
            .map(stats_of)
    }

    pub fn iter(&self) -> impl Iterator<Item = Stats> + '_ {
        self.entries.iter().flatten().map(stats_of)
    }

    fn entry_mut(&mut self, key: Key) -> Option<&mut Entry> {
        let i = match self
            .entries
            .iter()
            .position(|e| e.is_some_and(|e| e.key == key))
        {
            Some(i) => i,
            None => {
                let i = self.entries.iter().position(Option::is_none)?; // full: drop silently (cannot happen: 96 keys)
                self.entries[i] = Some(Entry {
                    key,
                    counts: Counts::default(),
                    lat: [0; LATENCY_RING],
                    lat_len: 0,
                    lat_next: 0,
                });
                i
            }
        };
        self.entries.get_mut(i)?.as_mut()
    }
}

fn stats_of(e: &Entry) -> Stats {
    let n = e.counts.n();
    let (a, b) = (
        1.0 + f64::from(e.counts.complied),
        1.0 + f64::from(n - e.counts.complied),
    );
    Stats {
        key: e.key,
        counts: e.counts,
        beta_a: a,
        beta_b: b,
        compliance: a / (a + b),
        median_l_obs_ms: median(&e.lat[..e.lat_len]),
    }
}

/// Median of up to 16 values without allocating (insertion sort on a copy).
fn median(values: &[i32]) -> Option<i32> {
    if values.is_empty() {
        return None;
    }
    let mut v = [0i32; LATENCY_RING];
    let n = values.len().min(LATENCY_RING);
    v[..n].copy_from_slice(&values[..n]);
    for i in 1..n {
        let mut j = i;
        while j > 0 && v[j - 1] > v[j] {
            v.swap(j - 1, j);
            j -= 1;
        }
    }
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::Confound;

    fn ev(arm: Arm, sham: bool) -> CueEvent {
        CueEvent {
            cue_id: 1,
            directive: 1,
            priority: 2,
            aki_class: 2,
            arm,
            propensity_q16: u16::MAX,
            is_sham: sham,
            dir: -1,
            delta_star_w: 20.0,
            profile: 0,
            t_decision_ns: 0,
            t_onset_ns: 1,
            t_end_ns: 2,
        }
    }

    fn out(verdict: Verdict, l_obs_ms: Option<i32>) -> CueOutcome {
        CueOutcome {
            cue_id: 1,
            verdict,
            confound: Confound::default(),
            coincidental: false,
            l_obs_ms,
            l_onset_ms: None,
            l_end_ms: None,
            pre_slope_w_per_s: 0.0,
            slope_clamped: false,
            effect_max_w: 0.0,
        }
    }

    #[test]
    fn beta_counts_only_scorable_verdicts() {
        let mut l = Ledger::new();
        for v in [
            Verdict::Complied,
            Verdict::Complied,
            Verdict::Ignored,
            Verdict::Confounded,
            Verdict::Invalid,
        ] {
            l.record(&ev(Arm::Direct, false), &out(v, Some(4_000)));
        }
        let s = l.stats(1, Arm::Direct, 0).unwrap();
        assert_eq!((s.n(), s.counts.confounded, s.counts.invalid), (3, 1, 1));
        assert_eq!((s.beta_a, s.beta_b), (3.0, 2.0)); // Beta(1 + 2, 1 + 3 - 2)
        assert_eq!(l.total_scored(), 3);
    }

    #[test]
    fn shams_and_arms_have_separate_keys() {
        let mut l = Ledger::new();
        l.record(&ev(Arm::Direct, true), &out(Verdict::Complied, Some(3_000)));
        l.record(&ev(Arm::Autonomy, false), &out(Verdict::Ignored, None));
        assert_eq!(l.stats(1, Arm::Sham, 0).map(|s| s.counts.complied), Some(1));
        assert_eq!(
            l.stats(1, Arm::Autonomy, 0).map(|s| s.counts.ignored),
            Some(1)
        );
        assert!(l.stats(1, Arm::Direct, 0).is_none());
    }

    #[test]
    fn the_latency_ring_keeps_the_last_16_and_the_lag_is_subtracted() {
        let mut l = Ledger::new();
        for i in 0..20 {
            l.record(
                &ev(Arm::Direct, false),
                &out(Verdict::Complied, Some(2_000 + 100 * i)),
            ); // 2.0 .. 3.9 s
        }
        let s = l.stats(1, Arm::Direct, 0).unwrap();
        assert_eq!(s.median_l_obs_ms, Some((3_100 + 3_200) / 2)); // last 16: 2.4 .. 3.9 s
        // L_obs minus the derived smoothing lag, under 1 s and under 2 s.
        let fast = |limit: i32| {
            (0..20)
                .filter(|i| 2_000 + 100 * i - SMOOTHING_LAG_MS < limit)
                .count() as u32
        };
        assert_eq!(s.counts.complied_under_1s, fast(1_000));
        assert_eq!(s.counts.complied_under_2s, fast(2_000));
    }
}
