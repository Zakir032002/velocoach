//! Matching the rig's STAMPs to the packets they describe, and each cue's
//! latency chain T0 -> T1 -> T2 -> T3. Fixed size: no allocation.
use vc_gatt::Channel;

const ARRIVALS: usize = 32; // per channel: 8 s of power at 4 Hz
const CUES: usize = 16;

fn ch(c: Channel) -> usize {
    match c {
        Channel::Hrm => 0,
        Channel::Cpm => 1,
        Channel::Csc => 2,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CueChain {
    pub cue_id: u32,
    /// (channel index, per-channel packet number) of the triggering packet.
    pub trigger: Option<(u8, u16)>,
    /// On the phone's clock.
    pub t0_ns: Option<u64>,
    pub t1_ns: Option<u64>,
    pub t2_ns: Option<u64>,
    pub t3_ns: Option<u64>,
    /// The clock-sync bound when T0 was mapped.
    pub bound_ns: Option<u64>,
    counted: bool,
}

impl CueChain {
    pub fn complete(&self) -> bool {
        self.t0_ns.is_some() && self.t1_ns.is_some() && self.t2_ns.is_some() && self.t3_ns.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Chains {
    seq: [u16; 3],
    arrivals: [[(u16, u64); ARRIVALS]; 3],
    next: [usize; 3],
    cues: [Option<CueChain>; CUES],
    c_next: usize,
    last_stamp: Option<u16>,
    pub stamps: u32,
    pub stamps_lost: u32,
    pub negative_t0t1: u32,
}

impl Default for Chains {
    fn default() -> Self {
        Self {
            seq: [0; 3],
            arrivals: [[(0, 0); ARRIVALS]; 3],
            next: [0; 3],
            cues: [None; CUES],
            c_next: 0,
            last_stamp: None,
            stamps: 0,
            stamps_lost: 0,
            negative_t0t1: 0,
        }
    }
}

impl Chains {
    /// A new connection: packet numbering restarts on both phones.
    pub fn reset(&mut self) {
        let cues = self.cues;
        *self = Self {
            cues,
            ..Self::default()
        }; // cues in flight keep their T1-T3
    }

    /// A sensor notification arrived at `t1`. Returns its per-channel number.
    pub fn on_packet(&mut self, c: Channel, t1: u64) -> u16 {
        let i = ch(c);
        self.seq[i] = self.seq[i].wrapping_add(1);
        self.arrivals[i][self.next[i] % ARRIVALS] = (self.seq[i], t1);
        self.next[i] += 1;
        self.seq[i]
    }

    /// A STAMP. `t0_phone` is the rig's T0 mapped onto the phone clock.
    /// Returns (T1, T0) of the packet it describes, if still remembered.
    pub fn on_stamp(
        &mut self,
        stamp_seq: u16,
        c: Channel,
        char_seq: u16,
        t0_phone: Option<u64>,
        bound: Option<u64>,
    ) -> Option<(u64, u64)> {
        self.stamps += 1;
        if let Some(last) = self.last_stamp {
            let gap = stamp_seq.wrapping_sub(last).wrapping_sub(1);
            if gap < 1_000 {
                self.stamps_lost += u32::from(gap);
            }
        }
        self.last_stamp = Some(stamp_seq);
        let i = ch(c);
        let t0 = t0_phone?;
        let t1 = self.arrivals[i]
            .iter()
            .find(|(s, t)| *s == char_seq && *t > 0)
            .map(|(_, t)| *t)?;
        if t0 > t1 {
            self.negative_t0t1 += 1;
        }
        for cue in self.cues.iter_mut().flatten() {
            if cue.trigger == Some((i as u8, char_seq)) && cue.t0_ns.is_none() {
                cue.t0_ns = Some(t0);
                cue.bound_ns = bound;
            }
        }
        Some((t1, t0))
    }

    pub fn on_decision(&mut self, cue_id: u32, trigger: Option<(Channel, u16)>, t1: u64) {
        self.cues[self.c_next % CUES] = Some(CueChain {
            cue_id,
            trigger: trigger.map(|(c, s)| (ch(c) as u8, s)),
            t1_ns: Some(t1),
            ..CueChain::default()
        });
        self.c_next += 1;
    }

    pub fn on_scheduled(&mut self, cue_id: u32, t2: u64, t3: u64) {
        if let Some(c) = self.cues.iter_mut().flatten().find(|c| c.cue_id == cue_id) {
            c.t2_ns = Some(t2);
            c.t3_ns = Some(t3);
        }
    }

    pub fn get(&self, cue_id: u32) -> Option<CueChain> {
        self.cues
            .iter()
            .flatten()
            .find(|c| c.cue_id == cue_id)
            .copied()
    }

    /// A complete chain not yet counted in the histograms, if any.
    pub fn take_complete(&mut self) -> Option<CueChain> {
        let c = self
            .cues
            .iter_mut()
            .flatten()
            .find(|c| c.complete() && !c.counted)?;
        c.counted = true;
        Some(*c)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_finds_its_packet_and_completes_the_cue() {
        let mut c = Chains::default();
        let s1 = c.on_packet(Channel::Cpm, 1_000);
        let s2 = c.on_packet(Channel::Cpm, 1_250);
        c.on_decision(7, Some((Channel::Cpm, s2)), 1_250);
        c.on_scheduled(7, 1_260, 51_260);
        assert_eq!(
            c.on_stamp(1, Channel::Cpm, s1, Some(990), Some(5)),
            Some((1_000, 990))
        );
        assert!(c.take_complete().is_none()); // the cue's own packet has no T0 yet
        assert_eq!(
            c.on_stamp(2, Channel::Cpm, s2, Some(1_230), Some(5)),
            Some((1_250, 1_230))
        );
        let done = c.take_complete().unwrap();
        assert_eq!((done.t0_ns, done.t3_ns), (Some(1_230), Some(51_260)));
        assert!(c.take_complete().is_none()); // counted once
    }

    #[test]
    fn lost_stamps_and_impossible_t0_are_counted() {
        let mut c = Chains::default();
        let s = c.on_packet(Channel::Hrm, 1_000);
        c.on_stamp(1, Channel::Hrm, s, Some(900), None);
        c.on_stamp(4, Channel::Hrm, 99, Some(900), None); // stamps 2 and 3 missing
        assert_eq!(c.stamps_lost, 2);
        let s = c.on_packet(Channel::Hrm, 2_000);
        c.on_stamp(5, Channel::Hrm, s, Some(2_100), None); // T0 after T1
        assert_eq!(c.negative_t0t1, 1);
    }
}
