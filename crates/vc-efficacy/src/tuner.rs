//! The bounded tuner (plan §3.7). At step boundaries, in Coach mode only, it
//! may move Aki's delivery parameters inside his ranges: at most one change per
//! parameter per boundary, each with its rule and evidence. Safety-critical
//! settings are not in its type, so it cannot reach them.
use crate::ledger::{Counts, Ledger};
use crate::observer::SMOOTHING_LAG_MS;
use crate::{Arm, Mode};

pub const MIN_N: u32 = 8;
pub const COOLDOWN_RANGE_S: (f64, f64) = (15.0, 45.0);
pub const LEAD_RANGE_S: (f64, f64) = (5.0, 8.0);
pub const COOLDOWN_STEP_S: f64 = 10.0;
pub const LEAD_STEP_S: f64 = 1.0;
pub const SWITCH_MARGIN: f64 = 0.15;

/// Everything the tuner may change. Nothing else is reachable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TunablePolicy {
    /// `cooldownBetweenAlertsSec`
    pub cooldown_s: f64,
    /// `promptSurgeCountdownSec`: the step-transition lead
    pub surge_lead_s: f64,
    /// P3 directives that get an earcon (bit = directive code).
    pub p3_earcon: u64,
    /// P2 directives whose default phrasing is Autonomy (bit = directive code).
    pub autonomy_default: u64,
}

/// Which codes are which. vc-efficacy knows no vc-core types, so vc-core passes this in.
#[derive(Debug, Clone, Copy)]
pub struct Catalogue<'a> {
    pub p2: &'a [u8],
    pub p3: &'a [u8],
    /// The scored step-transition cue (the surge's "Go now").
    pub step_transition: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    T1,
    T1Prime,
    T2,
    T2P3,
    T3,
    T4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Param {
    Cooldown,
    SurgeLead,
    P3Earcon(u8),
    DefaultVariant(u8),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    pub rule: Rule,
    pub param: Param,
    pub from: f64,
    pub to: f64,
    /// The cues the rule looked at.
    pub n: u32,
    /// The number that triggered it (a rate or a latency in s).
    pub evidence: f64,
}

/// At most one change per parameter: cooldown, lead, one earcon, one default.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TunerReport {
    pub changes: [Option<Change>; 4],
}

impl TunerReport {
    pub fn iter(&self) -> impl Iterator<Item = &Change> {
        self.changes.iter().flatten()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.iter().all(Option::is_none)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tuner {
    /// `Ledger::total_scored` when the cooldown / lead last moved.
    cooldown_mark: u32,
    lead_mark: u32,
}

fn bit(code: u8) -> u64 {
    1u64 << (code % 64)
}

/// All arms but Sham, for one directive and profile.
fn by_directive(ledger: &Ledger, code: u8, profile: u8) -> Counts {
    let mut c = Counts::default();
    for s in ledger
        .iter()
        .filter(|s| s.key.directive == code && s.key.profile == profile && s.key.arm != Arm::Sham)
    {
        c.complied += s.counts.complied;
        c.partial += s.counts.partial;
        c.ignored += s.counts.ignored;
        c.reversed += s.counts.reversed;
        c.complied_under_1s += s.counts.complied_under_1s;
        c.complied_under_2s += s.counts.complied_under_2s;
    }
    c
}

fn rate(k: u32, n: u32) -> f64 {
    if n == 0 {
        0.0
    } else {
        f64::from(k) / f64::from(n)
    }
}

impl Tuner {
    pub fn step(
        &mut self,
        ledger: &Ledger,
        p: &mut TunablePolicy,
        mode: Mode,
        profile: u8,
        cat: &Catalogue<'_>,
    ) -> TunerReport {
        let mut report = TunerReport::default();
        if mode != Mode::Coach {
            return report; // frozen in Study and Replay
        }
        let total = ledger.total_scored();

        // Cooldown: T2 (a P2 directive mostly ignored) wins over T4 (P2 fast and compliant).
        if total.saturating_sub(self.cooldown_mark) >= MIN_N {
            let worst = cat
                .p2
                .iter()
                .map(|&code| by_directive(ledger, code, profile))
                .filter(|c| c.n() >= MIN_N)
                .map(|c| (c.n(), rate(c.ignored, c.n())))
                .fold(None, |best: Option<(u32, f64)>, x| {
                    if best.is_none_or(|b| x.1 > b.1) {
                        Some(x)
                    } else {
                        best
                    }
                });
            let mut all = Counts::default();
            for &code in cat.p2 {
                let c = by_directive(ledger, code, profile);
                all.complied += c.complied;
                all.partial += c.partial;
                all.ignored += c.ignored;
                all.reversed += c.reversed;
                all.complied_under_2s += c.complied_under_2s;
            }
            let from = p.cooldown_s;
            let change = match worst {
                Some((n, r)) if r > 0.5 && from < COOLDOWN_RANGE_S.1 => Some((
                    Rule::T2,
                    (from + COOLDOWN_STEP_S).min(COOLDOWN_RANGE_S.1),
                    n,
                    r,
                )),
                _ if all.n() >= MIN_N
                    && rate(all.complied_under_2s, all.n()) > 0.8
                    && from > COOLDOWN_RANGE_S.0 =>
                {
                    Some((
                        Rule::T4,
                        (from - COOLDOWN_STEP_S).max(COOLDOWN_RANGE_S.0),
                        all.n(),
                        rate(all.complied_under_2s, all.n()),
                    ))
                }
                _ => None,
            };
            if let Some((rule, to, n, evidence)) = change {
                p.cooldown_s = to;
                self.cooldown_mark = total;
                report.changes[0] = Some(Change {
                    rule,
                    param: Param::Cooldown,
                    from,
                    to,
                    n,
                    evidence,
                });
            }
        }

        // Countdown lead: T1 / T1' on the step-transition cue.
        if total.saturating_sub(self.lead_mark) >= MIN_N
            && let Some(s) = ledger.stats(cat.step_transition, Arm::None, profile)
            && s.n() >= MIN_N
        {
            let from = p.surge_lead_s;
            let rider_median_s = s
                .median_l_obs_ms
                .map(|m| f64::from(m - SMOOTHING_LAG_MS) / 1000.0);
            let fast = rate(s.counts.complied_under_1s, s.n());
            let change = match rider_median_s {
                Some(m) if m > from - 1.0 && from < LEAD_RANGE_S.1 => {
                    Some((Rule::T1, (from + LEAD_STEP_S).min(LEAD_RANGE_S.1), m))
                }
                _ if fast > 0.8 && from > LEAD_RANGE_S.0 => Some((
                    Rule::T1Prime,
                    (from - LEAD_STEP_S).max(LEAD_RANGE_S.0),
                    fast,
                )),
                _ => None,
            };
            if let Some((rule, to, evidence)) = change {
                p.surge_lead_s = to;
                self.lead_mark = total;
                report.changes[1] = Some(Change {
                    rule,
                    param: Param::SurgeLead,
                    from,
                    to,
                    n: s.n(),
                    evidence,
                });
            }
        }

        // T2-P3: the first P3 directive mostly ignored, without an earcon yet.
        for &code in cat.p3 {
            let c = by_directive(ledger, code, profile);
            if p.p3_earcon & bit(code) == 0 && c.n() >= MIN_N && rate(c.ignored, c.n()) > 0.5 {
                p.p3_earcon |= bit(code);
                report.changes[2] = Some(Change {
                    rule: Rule::T2P3,
                    param: Param::P3Earcon(code),
                    from: 0.0,
                    to: 1.0,
                    n: c.n(),
                    evidence: rate(c.ignored, c.n()),
                });
                break;
            }
        }

        // T3: the first P2 directive whose other phrasing is clearly better.
        for &code in cat.p2 {
            let autonomy_now = p.autonomy_default & bit(code) != 0;
            let (now, other) = if autonomy_now {
                (Arm::Autonomy, Arm::Direct)
            } else {
                (Arm::Direct, Arm::Autonomy)
            };
            let (Some(a), Some(b)) = (
                ledger.stats(code, now, profile),
                ledger.stats(code, other, profile),
            ) else {
                continue;
            };
            if a.n() < MIN_N || b.n() < MIN_N {
                continue;
            }
            let gain = rate(b.counts.complied, b.n()) - rate(a.counts.complied, a.n());
            if gain >= SWITCH_MARGIN {
                p.autonomy_default ^= bit(code);
                let to = if autonomy_now { 0.0 } else { 1.0 }; // 1.0 = Autonomy
                report.changes[3] = Some(Change {
                    rule: Rule::T3,
                    param: Param::DefaultVariant(code),
                    from: 1.0 - to,
                    to,
                    n: a.n() + b.n(),
                    evidence: gain,
                });
                break;
            }
        }
        report
    }
}
