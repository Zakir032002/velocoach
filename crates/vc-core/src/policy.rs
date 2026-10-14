//! The Policy Engine: directives in, at most one spoken cue out.
//! This is the only place profiles, cooldowns, and phrasing meet.

use crate::config::{EngineConfig, Profile, ProfileParams};
use crate::decision::CueDecision;
use crate::directive::{Candidate, Candidates, Directive, Priority};
use crate::phrase::{earcon_for, phrase_for};
use crate::study::{Allocation, Fnv1a, NOT_RANDOMISED_Q16, StudyAllocator};
use vc_efficacy::{Arm, Mode};

/// Explore each phrasing this many times per directive before using the default.
pub const EXPLORE_PER_VARIANT: u16 = 8;

/// Parameters that the tuner may change. Safety-critical behavior is hard-coded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolicyParams {
    /// Cooldown in nanoseconds.
    pub cooldown_ns: u64,
    /// P3 directives with earcons enabled; bit = Directive::slot().
    pub p3_earcon_mask: u8,
    /// P2 directives defaulting to Autonomy; bit = Directive::slot().
    pub autonomy_default_mask: u8,
}

#[derive(Debug, Clone, Copy)]
struct Speaking {
    id: u32,
    priority: Priority,
    until_ns: u64,
}

#[derive(Debug, Clone)]
pub struct Policy {
    profile: ProfileParams,
    params: PolicyParams,
    stale_ns: u64,
    est_phrase_ns: u64,
    window_ns: u64,
    lead_ms: u16,
    next_id: u32,
    speaking: Option<Speaking>,
    cooldown_until: u64,
    window_until: u64,
    /// Delivered counts per directive slot: [Direct, Autonomy].
    delivered: [[u16; 2]; Directive::COUNT],
    mode: Mode,
    allocator: Option<StudyAllocator>,
    surge_lead_s: f64,
    profile_code: u8,
}

impl Policy {
    pub fn new(cfg: &EngineConfig) -> Self {
        Self {
            profile: cfg.profile.params(),
            params: PolicyParams {
                cooldown_ns: (cfg.cooldown_between_alerts_s * 1e9) as u64,
                p3_earcon_mask: 0,
                autonomy_default_mask: 0,
            },
            stale_ns: cfg.stale_drop_ns,
            est_phrase_ns: cfg.est_phrase_ns,
            window_ns: cfg.observer_window_ns,
            lead_ms: cfg.audio_lead_ms,
            next_id: 0,
            speaking: None,
            cooldown_until: 0,
            window_until: 0,
            delivered: [[0; 2]; Directive::COUNT],
            mode: Mode::Coach,
            allocator: None,
            surge_lead_s: cfg.prompt_surge_countdown_s,
            profile_code: match cfg.profile {
                Profile::Solo => 0,
                Profile::Rolling => 1,
                Profile::Group => 2,
            },
        }
    }

    pub fn params(&self) -> PolicyParams {
        self.params
    }

    pub fn decide(&mut self, t: u64, cands: &mut Candidates) -> Option<CueDecision> {
        let speaking = self.speaking.filter(|s| t < s.until_ns);

        // 0. Drop stale P2/P3 candidates. P1 safety cues never go stale.
        cands.retain(|c| {
            c.directive.priority() == Priority::P1
                || t.saturating_sub(c.created_ns) <= self.stale_ns
        });

        // 1. P1 bypasses mute and cooldown. It can interrupt P2/P3,
        //    but must wait behind another P1 clip.
        if let Some(c) = cands
            .most_urgent()
            .filter(|c| c.directive.priority() == Priority::P1)
        {
            if speaking.is_some_and(|s| s.priority == Priority::P1) {
                return None;
            }

            cands.remove(c.directive);
            return Some(self.emit(t, c, Arm::None, speaking.is_some()));
        }

        // 2. Group profile suppresses all non-P1 cues.
        if self.profile.mute_p2_p3 {
            cands.retain(|c| c.directive.priority() == Priority::P1);
            return None;
        }

        // 3–5. No overlapping ordinary cues, global cooldown, or
        //      overlapping observer windows.
        if speaking.is_some() || t < self.cooldown_until || t < self.window_until {
            return None;
        }

        // 6–7. Select the most urgent remaining candidate and its phrasing arm.
        let c = cands.take_most_urgent()?;

        if self.mode == Mode::Study
            && c.directive.priority() == Priority::P2
            && let Some(a) = self.allocator.as_mut().map(StudyAllocator::draw)
        {
            if a.arm == Arm::Sham {
                return Some(self.emit_sham(t, c, a));
            }

            let mut d = self.emit(t, c, a.arm, false);
            d.propensity_q16 = a.propensity_q16;
            d.draw_index = Some(a.draw_index);
            return Some(d);
        }

        let arm = self.choose_arm(c.directive);
        Some(self.emit(t, c, arm, false))
    }

    /// Replace the estimated end time with the actual audio completion time.
    pub fn audio_finished(&mut self, cue_id: u32, t_ns: u64) {
        let Some(s) = self.speaking.filter(|s| s.id == cue_id) else {
            return;
        };

        self.speaking = None;

        if s.priority != Priority::P1 {
            self.cooldown_until = t_ns.saturating_add(self.params.cooldown_ns);
        }

        if s.priority == Priority::P2 {
            self.window_until = t_ns.saturating_add(self.window_ns);
        }
    }

    /// Coach mode: alternate arms until each has eight deliveries per directive.
    fn choose_arm(&self, d: Directive) -> Arm {
        if d.priority() != Priority::P2 {
            return Arm::None;
        }

        let [direct, autonomy] = self.delivered.get(d.slot()).copied().unwrap_or([0, 0]);

        if direct < EXPLORE_PER_VARIANT || autonomy < EXPLORE_PER_VARIANT {
            return if autonomy < direct {
                Arm::Autonomy
            } else {
                Arm::Direct
            };
        }

        if self.params.autonomy_default_mask & (1 << d.slot()) != 0 {
            Arm::Autonomy
        } else {
            Arm::Direct
        }
    }

    fn emit(&mut self, t: u64, c: Candidate, arm: Arm, interrupt: bool) -> CueDecision {
        self.next_id += 1;

        let priority = c.directive.priority();
        let until = t.saturating_add(self.est_phrase_ns);

        self.speaking = Some(Speaking {
            id: self.next_id,
            priority,
            until_ns: until,
        });

        // Until the real audio callback arrives, estimate the clip end.
        if priority != Priority::P1 {
            self.cooldown_until = until.saturating_add(self.params.cooldown_ns);
        }

        if priority == Priority::P2 {
            self.window_until = until.saturating_add(self.window_ns);
        }

        if let Some(counts) = self.delivered.get_mut(c.directive.slot()) {
            match arm {
                Arm::Direct => counts[0] = counts[0].saturating_add(1),
                Arm::Autonomy => counts[1] = counts[1].saturating_add(1),
                Arm::None | Arm::Sham => {}
            }
        }

        CueDecision {
            id: self.next_id,
            directive: c.directive,
            priority,
            aki_class: c.directive.aki_class(),
            dir: c.dir,
            delta_star_w: c.delta_star_w,
            surge: c.surge,
            decided_ns: t,
            arm,
            phrase: phrase_for(c.directive, arm, c.surge),
            earcon: earcon_for(c.directive, self.params.p3_earcon_mask),
            lead_ms: self.lead_ms,
            interrupt,
            propensity_q16: NOT_RANDOMISED_Q16,
            draw_index: None,
        }
    }
    /// Only between sessions; the Engine enforces that.
    pub fn set_mode(&mut self, mode: Mode, seed: u64) {
        self.mode = mode;
        self.allocator = (mode == Mode::Study).then(|| StudyAllocator::new(seed));
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// M4: every Study session must report the same value.
    pub fn hash(&self) -> u64 {
        let mut h = Fnv1a::default();

        h.write(&[self.profile_code]);
        h.write_u64(self.params.cooldown_ns);
        h.write_u64(libm::round(self.surge_lead_s * 1000.0) as u64);
        h.write(&[
            self.params.p3_earcon_mask,
            self.params.autonomy_default_mask,
        ]);

        for value in [
            self.stale_ns,
            self.est_phrase_ns,
            self.window_ns,
            u64::from(self.lead_ms),
        ] {
            h.write_u64(value);
        }

        h.finish()
    }
    /// Study sham: nothing plays, nothing is sent, but the decision point
    /// uses up the cooldown and opens an observer window from a virtual
    /// onset and length.
    fn emit_sham(&mut self, t: u64, c: Candidate, a: Allocation) -> CueDecision {
        self.next_id += 1;

        let virtual_end = t + u64::from(self.lead_ms) * 1_000_000 + self.est_phrase_ns;

        self.cooldown_until = virtual_end.saturating_add(self.params.cooldown_ns);
        self.window_until = virtual_end.saturating_add(self.window_ns);

        CueDecision {
            id: self.next_id,
            directive: c.directive,
            priority: c.directive.priority(),
            aki_class: c.directive.aki_class(),
            dir: c.dir,
            delta_star_w: c.delta_star_w,
            surge: c.surge,
            decided_ns: t,
            arm: Arm::Sham,
            phrase: phrase_for(c.directive, Arm::Direct, c.surge),
            earcon: None,
            lead_ms: self.lead_ms,
            interrupt: false,
            propensity_q16: a.propensity_q16,
            draw_index: Some(a.draw_index),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::config::Profile;

    const S: u64 = 1_000_000_000;

    fn policy(cfg: EngineConfig) -> Policy {
        Policy::new(&cfg)
    }

    fn offer(c: &mut Candidates, d: Directive, t_s: u64) {
        c.offer(Candidate::new(d, -1, 20.0, t_s * S));
    }

    #[test]
    fn p1_interrupts_a_p2_clip_but_waits_behind_a_p1_clip() {
        let mut p = policy(EngineConfig::default());
        let mut c = Candidates::default();

        offer(&mut c, Directive::WprimeLow, 0);
        assert!(p.decide(0, &mut c).is_some_and(|d| !d.interrupt));

        offer(&mut c, Directive::SafetyHr, 1);
        let d = p.decide(S, &mut c).unwrap();
        assert_eq!(d.directive, Directive::SafetyHr);
        assert!(d.interrupt);

        offer(&mut c, Directive::SensorDropout, 2);
        assert!(p.decide(2 * S, &mut c).is_none());

        let d = p.decide(4 * S, &mut c).unwrap();
        assert_eq!(d.directive, Directive::SensorDropout);
    }

    #[test]
    fn cooldown_runs_from_the_end_of_the_clip() {
        let mut p = policy(EngineConfig::default());
        let mut c = Candidates::default();

        offer(&mut c, Directive::WprimeLow, 0);
        let first = p.decide(0, &mut c).unwrap();
        p.audio_finished(first.id, 3 * S);

        for t in [10, 22] {
            offer(&mut c, Directive::SoftPedalWarning, t);
            assert!(p.decide(t * S, &mut c).is_none(), "{t}");
        }

        offer(&mut c, Directive::SoftPedalWarning, 23);
        assert!(p.decide(23 * S, &mut c).is_some());
    }

    #[test]
    fn the_observer_window_binds_when_the_cooldown_is_short() {
        let cfg = EngineConfig {
            cooldown_between_alerts_s: 15.0,
            observer_window_ns: 30 * S,
            ..EngineConfig::default()
        };
        let mut p = policy(cfg);
        let mut c = Candidates::default();

        offer(&mut c, Directive::WprimeLow, 0);
        let first = p.decide(0, &mut c).unwrap();
        p.audio_finished(first.id, 2 * S);

        offer(&mut c, Directive::CardiacDrift, 20);
        assert!(p.decide(20 * S, &mut c).is_none());

        offer(&mut c, Directive::CardiacDrift, 32);
        assert!(p.decide(32 * S, &mut c).is_some());
    }

    #[test]
    fn group_mutes_p2_and_p3_but_never_p1() {
        let mut p = policy(EngineConfig {
            profile: Profile::Group,
            ..EngineConfig::default()
        });
        let mut c = Candidates::default();

        offer(&mut c, Directive::WprimeLow, 0);
        offer(&mut c, Directive::CardiacDrift, 0);
        assert!(p.decide(0, &mut c).is_none());

        offer(&mut c, Directive::SafetyHr, 1);
        assert_eq!(
            p.decide(S, &mut c).map(|d| d.directive),
            Some(Directive::SafetyHr)
        );
    }

    #[test]
    fn stale_p2_is_dropped_but_p1_is_kept() {
        let mut p = policy(EngineConfig::default());
        let mut c = Candidates::default();

        offer(&mut c, Directive::WprimeLow, 0);
        assert!(p.decide(3 * S, &mut c).is_none());

        offer(&mut c, Directive::SensorDropout, 0);
        assert!(p.decide(10 * S, &mut c).is_some());
    }

    #[test]
    fn coach_mode_round_robins_eight_each_then_keeps_the_default() {
        let mut p = policy(EngineConfig::default());
        let mut c = Candidates::default();
        let mut arms = Vec::new();

        for k in 0..20_u64 {
            let t = k * 60;
            offer(&mut c, Directive::WprimeLow, t);
            arms.push(p.decide(t * S, &mut c).unwrap().arm);
        }

        let explored: Vec<Arm> = (0..16)
            .map(|k| {
                if k % 2 == 0 {
                    Arm::Direct
                } else {
                    Arm::Autonomy
                }
            })
            .collect();

        assert_eq!(&arms[..16], explored.as_slice());
        assert!(arms[16..].iter().all(|a| *a == Arm::Direct));
    }
}
