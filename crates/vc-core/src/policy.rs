//! Day 5 stub: emits the most urgent candidate and nothing else.
//! Day 6 replaces this file with the full Policy Engine.

use crate::config::EngineConfig;
use crate::decision::CueDecision;
use crate::directive::Candidates;

#[derive(Debug, Clone)]
pub struct Policy {
    next_id: u32,
}

impl Policy {
    pub fn new(_cfg: &EngineConfig) -> Self {
        Self { next_id: 0 }
    }

    pub fn decide(&mut self, t_ns: u64, cands: &mut Candidates) -> Option<CueDecision> {
        let c = cands.take_most_urgent()?;
        self.next_id += 1;

        Some(CueDecision {
            id: self.next_id,
            directive: c.directive,
            priority: c.directive.priority(),
            aki_class: c.directive.aki_class(),
            dir: c.dir,
            delta_star_w: c.delta_star_w,
            surge: c.surge,
            decided_ns: t_ns,
        })
    }

    pub fn audio_finished(&mut self, _cue_id: u32, _t_ns: u64) {}
}
