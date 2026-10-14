//! The plain-data ledger ring (plan decision 4). Records are pushed inside
//! ingest/tick without allocation; the platform drains them at 1 Hz into std
//! code (vc-report) that writes JSONL. If the ring fills, the oldest record is
//! dropped and the count rides on the next cue record (`dropped_before`).
use heapless::Deque;
use vc_efficacy::{Change, CueEvent, CueOutcome, Mode};

pub const LEDGER_RING: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OnsetSource {
    /// T3 + the configured route delay (0 until Day 15).
    Scheduled,
    /// T3 + the route delay measured by the Day 15 loopback.
    Loopback,
    /// T3 + AVAudio's outputLatency estimate (reserved).
    OutputLatency,
    /// A sham: nothing played; onset and end are virtual.
    Virtual,
}

impl OnsetSource {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Loopback => "loopback",
            Self::OutputLatency => "output_latency",
            Self::Virtual => "virtual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LedgerRecord {
    SessionStart {
        t_ns: u64,
        mode: Mode,
        seed: u64,
        policy_hash: u64,
        profile: u8,
    },
    Cue {
        event: CueEvent,
        outcome: CueOutcome,
        draw_index: Option<u32>,
        onset_source: OnsetSource,
        /// Clock-sync error bound for this cue's T0, +/- rtt_min/2 (0 until Day 15).
        sync_bound_us: u32,
        dropped_before: u32,
    },
    TunerChange {
        t_ns: u64,
        change: Change,
        policy_hash_after: u64,
    },
    SessionEnd {
        t_ns: u64,
        cues: u32,
        dropped: u32,
    },
}

#[derive(Debug, Clone, Default)]
pub struct LedgerRing {
    q: Deque<LedgerRecord, LEDGER_RING>,
    dropped_total: u32,
    dropped_pending: u32,
    cues: u32,
}

impl LedgerRing {
    pub fn push(&mut self, mut r: LedgerRecord) {
        if let LedgerRecord::Cue { dropped_before, .. } = &mut r {
            *dropped_before = core::mem::take(&mut self.dropped_pending);
            self.cues += 1;
        }
        if self.q.is_full() {
            self.q.pop_front();
            self.dropped_total += 1;
            self.dropped_pending += 1;
        }
        let _ = self.q.push_back(r);
    }

    /// Move up to `out.len()` records out, oldest first. Returns how many.
    pub fn pop_into(&mut self, out: &mut [LedgerRecord]) -> usize {
        let mut n = 0;
        for slot in out.iter_mut() {
            match self.q.pop_front() {
                Some(r) => {
                    *slot = r;
                    n += 1;
                }
                None => break,
            }
        }
        n
    }

    pub fn len(&self) -> usize {
        self.q.len()
    }

    pub fn is_empty(&self) -> bool {
        self.q.is_empty()
    }

    pub fn cues(&self) -> u32 {
        self.cues
    }

    pub fn dropped(&self) -> u32 {
        self.dropped_total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end(t: u64) -> LedgerRecord {
        LedgerRecord::SessionEnd {
            t_ns: t,
            cues: 0,
            dropped: 0,
        }
    }

    #[test]
    fn drains_oldest_first_and_counts_overflow() {
        let mut r = LedgerRing::default();
        for t in 0..300u64 {
            r.push(end(t));
        }
        assert_eq!((r.len(), r.dropped()), (256, 44));
        let mut out = [end(0); 10];
        assert_eq!(r.pop_into(&mut out), 10);
        assert_eq!(out[0], end(44));
    }
}
