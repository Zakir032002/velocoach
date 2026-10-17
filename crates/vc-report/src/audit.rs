//! The study audit (M2-M5), computed from the JSONL alone.
use vc_core::study::StudyAllocator;

use crate::schema::{CueLine, Line};

pub struct Check {
    pub id: &'static str,
    pub pass: bool,
    pub detail: String,
}

pub fn audit(lines: &[Line]) -> Vec<Check> {
    let study: Vec<&CueLine> = lines
        .iter()
        .filter_map(|l| if let Line::Cue(c) = l { Some(c) } else { None })
        .filter(|c| c.mode == "study")
        .collect();
    let drawn: Vec<&&CueLine> = study.iter().filter(|c| c.draw_index.is_some()).collect();
    let n = drawn.len() as f64;
    let mut checks = Vec::new();

    let freq_ok = [("sham", 0.2), ("direct", 0.4), ("autonomy", 0.4)]
        .iter()
        .all(|&(arm, p)| {
            let k = drawn.iter().filter(|c| c.arm == arm).count() as f64;
            n == 0.0 || ((k / n) - p).abs() <= 2.576 * (p * (1.0 - p) / n).sqrt()
        });
    checks.push(Check {
        id: "M2 frequencies",
        pass: freq_ok,
        detail: format!("{} drawn decision points", drawn.len()),
    });

    let mismatches = drawn
        .iter()
        .filter(|c| {
            c.draw_index
                .is_some_and(|i| StudyAllocator::replay(c.seed, i).arm.name() != c.arm)
        })
        .count();
    checks.push(Check {
        id: "M2 replay",
        pass: mismatches == 0,
        detail: format!("{mismatches} assignments did not replay"),
    });

    let p1_arms = lines
        .iter()
        .filter(|l| matches!(l, Line::Cue(c) if c.priority == "P1" && c.arm != "none"))
        .count();
    checks.push(Check {
        id: "M3",
        pass: p1_arms == 0,
        detail: format!("{p1_arms} P1 records with an arm"),
    });

    let hashes: std::collections::BTreeSet<&str> = lines
        .iter()
        .filter_map(|l| {
            if let Line::SessionStart(s) = l {
                (s.mode == "study").then_some(s.policy_hash.as_str())
            } else {
                None
            }
        })
        .collect();
    checks.push(Check {
        id: "M4",
        pass: hashes.len() <= 1,
        detail: format!("{} distinct policy_hash values", hashes.len()),
    });

    let invalid = study.iter().filter(|c| c.verdict == "invalid").count();
    let valid_rate = if study.is_empty() {
        1.0
    } else {
        1.0 - invalid as f64 / study.len() as f64
    };
    let mut bounds: Vec<u32> = study.iter().map(|c| c.clock_sync_error_bound_us).collect();
    bounds.sort_unstable();
    let p95 = bounds
        .get((bounds.len() * 95 / 100).min(bounds.len().saturating_sub(1)))
        .copied()
        .unwrap_or(0);
    checks.push(Check {
        id: "M5",
        pass: valid_rate >= 0.90 && p95 <= 15_000,
        detail: format!(
            "{:.0} % not Invalid; p95 sync bound {} µs",
            valid_rate * 100.0,
            p95
        ),
    });
    checks
}
