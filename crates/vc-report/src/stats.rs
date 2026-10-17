//! Pre-specified statistics (STUDY.md §Analysis): Wilson intervals, a
//! person-stratified seeded bootstrap of the pooled within-person difference,
//! median and IQR. No p-values anywhere.
use std::collections::BTreeMap;

use vc_core::study::Pcg32;

pub const Z95: f64 = 1.959_963_984_540_054;
pub const BOOTSTRAP_B: u32 = 10_000;
/// Fixed in STUDY.md before the first counted session.
pub const BOOTSTRAP_SEED: u64 = 20_261_013;

/// Wilson score interval for k successes in n trials.
pub fn wilson(k: u32, n: u32, z: f64) -> Option<(f64, f64)> {
    if n == 0 {
        return None;
    }
    let (k, n) = (f64::from(k), f64::from(n));
    let p = k / n;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denom;
    let half = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denom;
    Some(((centre - half).max(0.0), (centre + half).min(1.0)))
}

/// Quantile with linear interpolation between order statistics ("type 7").
fn quantile(sorted: &[f64], q: f64) -> f64 {
    let h = (sorted.len() - 1) as f64 * q;
    let (lo, hi) = (h.floor() as usize, h.ceil() as usize);
    sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
}

/// (median, Q1, Q3), or None for no data.
pub fn median_iqr(values: &[f64]) -> Option<(f64, f64, f64)> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    Some((quantile(&v, 0.5), quantile(&v, 0.25), quantile(&v, 0.75)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    A,
    B,
}

/// One scored decision point.
#[derive(Debug, Clone, Copy)]
pub struct Obs {
    pub person: u32,
    pub group: Group,
    pub complied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub estimate: f64,
    pub lo: f64,
    pub hi: f64,
}

/// rate(B) - rate(A) for one person, if they have both.
fn person_diff<'a>(points: impl Iterator<Item = &'a Obs>) -> Option<f64> {
    let (mut ka, mut na, mut kb, mut nb) = (0u32, 0u32, 0u32, 0u32);
    for o in points {
        match o.group {
            Group::A => {
                na += 1;
                ka += u32::from(o.complied);
            }
            Group::B => {
                nb += 1;
                kb += u32::from(o.complied);
            }
        }
    }
    (na > 0 && nb > 0).then(|| f64::from(kb) / f64::from(nb) - f64::from(ka) / f64::from(na))
}

fn by_person(obs: &[Obs]) -> BTreeMap<u32, Vec<Obs>> {
    let mut m: BTreeMap<u32, Vec<Obs>> = BTreeMap::new();
    for o in obs {
        m.entry(o.person).or_default().push(*o);
    }
    m
}

/// The pooled within-person difference: the mean over people of rate(B) - rate(A).
pub fn pooled_diff(obs: &[Obs]) -> Option<f64> {
    let diffs: Vec<f64> = by_person(obs)
        .values()
        .filter_map(|p| person_diff(p.iter()))
        .collect();
    (!diffs.is_empty()).then(|| diffs.iter().sum::<f64>() / diffs.len() as f64)
}

/// Person-stratified bootstrap of `pooled_diff`: each replicate resamples every
/// person's points within that person. Percentile 95 % interval.
pub fn bootstrap_diff(obs: &[Obs], seed: u64, b: u32) -> Option<Interval> {
    let estimate = pooled_diff(obs)?;
    let people: Vec<Vec<Obs>> = by_person(obs).into_values().collect(); // BTreeMap: a fixed order
    let mut reps = Vec::with_capacity(b as usize);
    let mut sample = Vec::new();
    for r in 0..b {
        let mut rng = Pcg32::new(seed, u64::from(r));
        let mut diffs = Vec::with_capacity(people.len());
        for p in &people {
            sample.clear();
            for _ in 0..p.len() {
                let i = ((rng.next_f64() * p.len() as f64) as usize).min(p.len() - 1);
                sample.push(p[i]);
            }
            if let Some(d) = person_diff(sample.iter()) {
                diffs.push(d);
            }
        }
        if !diffs.is_empty() {
            reps.push(diffs.iter().sum::<f64>() / diffs.len() as f64);
        }
    }
    if reps.is_empty() {
        return None;
    }
    reps.sort_by(f64::total_cmp);
    Some(Interval {
        estimate,
        lo: quantile(&reps, 0.025),
        hi: quantile(&reps, 0.975),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_matches_published_values() {
        let (lo, hi) = wilson(5, 10, Z95).unwrap();
        assert!(
            (lo - 0.2366).abs() < 1e-4 && (hi - 0.7634).abs() < 1e-4,
            "{lo} {hi}"
        );
        let (lo, hi) = wilson(0, 10, Z95).unwrap();
        assert!(lo.abs() < 1e-12 && (hi - 0.2775).abs() < 1e-4, "{lo} {hi}");
        assert_eq!(wilson(0, 0, Z95), None);
    }

    #[test]
    fn median_and_iqr_interpolate() {
        assert_eq!(median_iqr(&[1.0, 2.0, 3.0, 4.0]), Some((2.5, 1.75, 3.25)));
    }

    #[test]
    fn the_bootstrap_is_reproducible_and_brackets_its_estimate() {
        let obs: Vec<Obs> = (0..120)
            .map(|i| Obs {
                person: i % 3,
                group: if i % 2 == 0 { Group::A } else { Group::B },
                complied: i % 5 < 3,
            })
            .collect();
        let a = bootstrap_diff(&obs, 7, 2_000).unwrap();
        assert_eq!(Some(a), bootstrap_diff(&obs, 7, 2_000));
        assert!(a.lo <= a.estimate && a.estimate <= a.hi);
    }
}
