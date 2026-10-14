//! The report card: one static HTML page with inline SVG. No scripts, no
//! external files, deterministic output for the same input.
use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::ips::{Logged, conservative_gate, snips};
use crate::schema::{CueLine, Line};
use crate::stats::{
    BOOTSTRAP_B, BOOTSTRAP_SEED, Group, Obs, Z95, bootstrap_diff, median_iqr, wilson,
};

const VERDICTS: [(&str, &str); 6] = [
    ("complied", "#2e7d32"),
    ("partial", "#9ccc65"),
    ("ignored", "#bdbdbd"),
    ("reversed", "#c62828"),
    ("confounded", "#ffb300"),
    ("invalid", "#616161"),
];
const ARMS: [&str; 4] = ["direct", "autonomy", "sham", "none"];

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn scorable(c: &CueLine) -> bool {
    matches!(
        c.verdict.as_str(),
        "complied" | "partial" | "ignored" | "reversed"
    )
}

/// Comparison data: people indexed in first-seen order, which is deterministic.
fn observations(
    cues: &[&CueLine],
    a: impl Fn(&CueLine) -> bool,
    b: impl Fn(&CueLine) -> bool,
) -> Vec<Obs> {
    let mut people: BTreeMap<&str, u32> = BTreeMap::new();
    let mut out = Vec::new();
    for c in cues.iter().filter(|c| scorable(c)) {
        let next = people.len() as u32;
        let person = *people.entry(c.rider_id.as_str()).or_insert(next);
        let group = if a(c) {
            Group::A
        } else if b(c) {
            Group::B
        } else {
            continue;
        };
        out.push(Obs {
            person,
            group,
            complied: c.verdict == "complied",
        });
    }
    out
}

fn verdict_bars(cues: &[&CueLine]) -> String {
    let mut svg = String::from(
        r#"<svg viewBox="0 0 640 150" width="640" role="img" aria-label="verdict mix per arm">"#,
    );
    for (row, arm) in ARMS.iter().enumerate() {
        let of_arm: Vec<_> = cues.iter().filter(|c| c.arm == *arm).collect();
        let y = 10 + row * 34;
        let _ = write!(
            svg,
            r#"<text x="0" y="{}" font-size="12">{arm} (n={})</text>"#,
            y + 15,
            of_arm.len()
        );
        if of_arm.is_empty() {
            continue;
        }
        let mut x = 130.0;
        for (v, colour) in VERDICTS {
            let k = of_arm.iter().filter(|c| c.verdict == v).count();
            let w = 500.0 * k as f64 / of_arm.len() as f64;
            if w > 0.0 {
                let _ = write!(
                    svg,
                    r#"<rect x="{x:.1}" y="{y}" width="{w:.1}" height="22" fill="{colour}"><title>{v}: {k}</title></rect>"#
                );
            }
            x += w;
        }
    }
    svg.push_str("</svg>");
    svg
}

fn wilson_rows(cues: &[&CueLine]) -> String {
    let mut html = String::from(
        "<table><tr><th>arm</th><th>complied / scored</th><th>rate</th><th>Wilson 95 %</th><th></th></tr>",
    );
    for arm in ["direct", "autonomy", "sham"] {
        let scored: Vec<_> = cues
            .iter()
            .filter(|c| c.arm == arm && scorable(c))
            .collect();
        let k = scored.iter().filter(|c| c.verdict == "complied").count() as u32;
        let n = scored.len() as u32;
        let Some((lo, hi)) = wilson(k, n, Z95) else {
            continue;
        };
        let rate = f64::from(k) / f64::from(n);
        let bar = format!(
            r##"<svg width="200" height="14"><rect x="0" y="6" width="200" height="2" fill="#eee"/><rect x="{:.1}" y="3" width="{:.1}" height="8" fill="#90caf9"/><rect x="{:.1}" y="1" width="2" height="12" fill="#0d47a1"/></svg>"##,
            lo * 200.0,
            (hi - lo) * 200.0,
            rate * 200.0 - 1.0
        );
        let label = if arm == "sham" {
            "sham (false-positive rate)"
        } else {
            arm
        };
        let _ = write!(
            html,
            "<tr><td>{label}</td><td>{k} / {n}</td><td>{rate:.2}</td><td>{lo:.2}–{hi:.2}</td><td>{bar}</td></tr>"
        );
    }
    html.push_str("</table>");
    html
}

fn primary(cues: &[&CueLine]) -> String {
    let cue_vs_sham = observations(
        cues,
        |c| c.arm == "sham",
        |c| c.arm == "direct" || c.arm == "autonomy",
    );
    let b_vs_a = observations(cues, |c| c.arm == "direct", |c| c.arm == "autonomy");
    let row = |name: &str, obs: &[Obs]| match bootstrap_diff(obs, BOOTSTRAP_SEED, BOOTSTRAP_B) {
        Some(ci) => format!(
            "<tr><td>{name}</td><td>{:+.3}</td><td>{:+.3} to {:+.3}</td><td>{}</td></tr>",
            ci.estimate,
            ci.lo,
            ci.hi,
            obs.len()
        ),
        None => format!(
            "<tr><td>{name}</td><td colspan=3>not enough data (needs both groups for at least one person)</td></tr>"
        ),
    };
    format!(
        "<table><tr><th>comparison</th><th>pooled within-person difference</th><th>bootstrap 95 % CI (B = {BOOTSTRAP_B}, seed {BOOTSTRAP_SEED})</th><th>decision points</th></tr>{}{}</table>",
        row("cue (A ∪ B) − sham", &cue_vs_sham),
        row("autonomy (B) − direct (A)", &b_vs_a)
    )
}

fn latency_histogram(cues: &[&CueLine]) -> String {
    let mut svg = String::from(
        r#"<svg viewBox="0 0 640 180" width="640" role="img" aria-label="latency histogram">"#,
    );
    let mut summary = String::new();
    for (row, (arm, colour)) in [("direct", "#1565c0"), ("autonomy", "#6a1b9a")]
        .iter()
        .enumerate()
    {
        let l: Vec<f64> = cues
            .iter()
            .filter(|c| c.arm == *arm && c.verdict == "complied")
            .filter_map(|c| c.latency_ms)
            .map(|ms| f64::from(ms) / 1000.0)
            .collect();
        let mut bins = [0u32; 34]; // -2 .. 15 s in 0.5 s
        for v in &l {
            let i = (((v + 2.0) / 0.5).floor().max(0.0) as usize).min(33);
            bins[i] += 1;
        }
        let top = bins.iter().copied().max().unwrap_or(1).max(1);
        for (i, k) in bins.iter().enumerate() {
            let h = 70.0 * f64::from(*k) / f64::from(top);
            let x = 40.0 + i as f64 * 17.0 + row as f64 * 8.0;
            let _ = write!(
                svg,
                r#"<rect x="{x:.1}" y="{:.1}" width="8" height="{h:.1}" fill="{colour}"/>"#,
                150.0 - h
            );
        }
        if let Some((m, q1, q3)) = median_iqr(&l) {
            let _ = write!(
                summary,
                "<li>{arm}: median L_obs {m:.2} s (IQR {q1:.2}–{q3:.2}), n = {}</li>",
                l.len()
            );
        }
    }
    svg.push_str(r#"<text x="40" y="170" font-size="11">-2 s</text><text x="590" y="170" font-size="11">15 s</text></svg>"#);
    format!(
        "{svg}<ul>{summary}</ul><p>L_obs is measured from the end of the phrase and includes about 2.0 s of 3-s smoothing lag.</p>"
    )
}

fn timeline(cues: &[&CueLine]) -> String {
    let mut by_session: BTreeMap<&str, Vec<&&CueLine>> = BTreeMap::new();
    for c in cues {
        by_session.entry(c.session_id.as_str()).or_default().push(c);
    }
    let h = 20 * by_session.len().max(1) + 10;
    let mut svg =
        format!(r#"<svg viewBox="0 0 640 {h}" width="640" role="img" aria-label="timeline">"#);
    for (row, (session, list)) in by_session.iter().enumerate() {
        let t0 = list.iter().map(|c| c.t_decision_ns).min().unwrap_or(0);
        let span = list
            .iter()
            .map(|c| c.t_decision_ns - t0)
            .max()
            .unwrap_or(1)
            .max(1) as f64;
        let y = 15 + row * 20;
        let _ = write!(
            svg,
            r#"<text x="0" y="{}" font-size="10">{}</text>"#,
            y + 4,
            &session[..6.min(session.len())]
        );
        for c in list {
            let x = 60.0 + 560.0 * (c.t_decision_ns - t0) as f64 / span;
            let colour = VERDICTS
                .iter()
                .find(|(v, _)| *v == c.verdict)
                .map_or("#000", |(_, col)| col);
            let _ = write!(
                svg,
                r#"<circle cx="{x:.1}" cy="{y}" r="4" fill="{colour}"><title>cue {} {} {}</title></circle>"#,
                c.cue_id,
                esc(&c.directive),
                c.verdict
            );
        }
    }
    svg.push_str("</svg>");
    svg
}

/// "What if every eligible decision point had used one phrasing?" Study data only:
/// SNIPS for Direct and Autonomy, and the conservative gate's answer.
fn what_if(cues: &[&CueLine]) -> String {
    let points: Vec<Logged<'_>> = cues
        .iter()
        .filter(|c| c.mode == "study" && c.draw_index.is_some() && scorable(c))
        .filter(|c| c.arm == "direct" || c.arm == "autonomy")
        .map(|c| Logged {
            arm: c.arm.as_str(),
            complied: c.verdict == "complied",
            propensity: c.propensity,
        })
        .collect();
    if points.is_empty() {
        return String::new();
    }
    let est = |arm: &str| snips(&points, arm).map_or("n/a".to_string(), |v| format!("{v:.2}"));
    let gate = match conservative_gate(&points, "direct", "autonomy", BOOTSTRAP_SEED, BOOTSTRAP_B) {
        Some(g) if g.adopt => format!(
            "adopt autonomy: the lower 95 % bound of SNIPS(autonomy) − SNIPS(direct) is {:+.3}, above zero",
            g.lo
        ),
        Some(g) => format!(
            "keep the current default: the lower 95 % bound of SNIPS(autonomy) − SNIPS(direct) is {:+.3}, not above zero",
            g.lo
        ),
        None => "not enough data for both phrasings".to_string(),
    };
    format!(
        "<h2>What if?</h2><p>Off-policy (SNIPS) compliance if every eligible decision point had used one phrasing: direct {}, autonomy {} (n = {} logged points).</p><p>Conservative gate: {gate}.</p>",
        est("direct"),
        est("autonomy"),
        points.len()
    )
}

pub fn render(lines: &[Line], title: &str) -> String {
    let cues: Vec<&CueLine> = lines
        .iter()
        .filter_map(|l| if let Line::Cue(c) = l { Some(c) } else { None })
        .collect();
    let riders: std::collections::BTreeSet<&str> =
        cues.iter().map(|c| c.rider_id.as_str()).collect();
    let sessions: std::collections::BTreeSet<&str> =
        cues.iter().map(|c| c.session_id.as_str()).collect();
    let hashes: std::collections::BTreeSet<&str> =
        cues.iter().map(|c| c.policy_hash.as_str()).collect();
    let excluded = cues.iter().filter(|c| !scorable(c)).count();
    let mut tuner = String::from(
        "<table><tr><th>t (s)</th><th>rule</th><th>parameter</th><th>from → to</th><th>n</th><th>evidence</th></tr>",
    );
    for l in lines {
        if let Line::TunerChange(t) = l {
            let _ = write!(
                tuner,
                "<tr><td>{:.0}</td><td>{}</td><td>{}</td><td>{} → {}</td><td>{}</td><td>{:.3}</td></tr>",
                t.t_ns as f64 / 1e9,
                t.rule,
                esc(&t.param),
                t.from,
                t.to,
                t.n,
                t.evidence
            );
        }
    }
    tuner.push_str("</table>");
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>{title}</title>
<style>body{{font:14px system-ui,sans-serif;max-width:720px;margin:24px auto;padding:0 16px}}table{{border-collapse:collapse}}td,th{{padding:3px 8px;border-bottom:1px solid #ddd;text-align:left}}</style></head><body>
<h1>{title}</h1>
<p><strong>Feasibility pilot · slider proxy, not cycling · personas are assumptions.</strong> n = {n} decision points from {people} people in {s} sessions; {excluded} excluded (Confounded or Invalid). Not powered to detect differences under about 0.2. No p-values are reported.</p>
<p>Policy hashes: {hashes}</p>
<h2>Verdict mix per arm</h2>{bars}
<h2>Compliance per arm (Wilson 95 %)</h2>{wilson}
<h2>Primary outcomes</h2>{primary}
{what_if}<h2>Latency</h2>{latency}
<h2>Timeline</h2>{timeline}
<h2>Tuner changes</h2>{tuner}
</body></html>"#,
        title = esc(title),
        n = cues.len(),
        people = riders.len(),
        s = sessions.len(),
        hashes = hashes.into_iter().collect::<Vec<_>>().join(", "),
        bars = verdict_bars(&cues),
        wilson = wilson_rows(&cues),
        primary = primary(&cues),
        what_if = what_if(&cues),
        latency = latency_histogram(&cues),
        timeline = timeline(&cues),
    )
}
