//! Off-policy estimates from Study-mode logs (cut-list #1). Each point: the arm
//! logged, whether it complied, and the propensity it was drawn with.
use vc_core::study::Pcg32;

#[derive(Debug, Clone, Copy)]
pub struct Logged<'a> {
    pub arm: &'a str,
    pub complied: bool,
    pub propensity: f64,
}

/// IPS: mean over all points of 1{arm = target} * reward / propensity.
pub fn ips(points: &[Logged<'_>], target: &str) -> Option<f64> {
    (!points.is_empty()).then(|| {
        points
            .iter()
            .filter(|p| p.arm == target)
            .map(|p| f64::from(u8::from(p.complied)) / p.propensity)
            .sum::<f64>()
            / points.len() as f64
    })
}

/// SNIPS: sum(w * reward) / sum(w) over the points whose arm matches.
pub fn snips(points: &[Logged<'_>], target: &str) -> Option<f64> {
    let (num, den) = points
        .iter()
        .filter(|p| p.arm == target)
        .fold((0.0, 0.0), |(n, d), p| {
            (
                n + f64::from(u8::from(p.complied)) / p.propensity,
                d + 1.0 / p.propensity,
            )
        });
    (den > 0.0).then(|| num / den)
}

#[derive(Debug, Clone, Copy)]
pub struct Gate {
    pub estimate: f64,
    pub lo: f64,
    pub adopt: bool,
}

/// Adopt `candidate` only if the bootstrap lower 95 % bound of
/// SNIPS(candidate) - SNIPS(current) is above zero.
pub fn conservative_gate(
    points: &[Logged<'_>],
    current: &str,
    candidate: &str,
    seed: u64,
    b: u32,
) -> Option<Gate> {
    let estimate = snips(points, candidate)? - snips(points, current)?;
    let mut reps = Vec::with_capacity(b as usize);
    let mut sample = Vec::with_capacity(points.len());
    for r in 0..b {
        let mut rng = Pcg32::new(seed, u64::from(r));
        sample.clear();
        for _ in 0..points.len() {
            sample.push(
                points[((rng.next_f64() * points.len() as f64) as usize).min(points.len() - 1)],
            );
        }
        if let (Some(a), Some(c)) = (snips(&sample, candidate), snips(&sample, current)) {
            reps.push(a - c);
        }
    }

    reps.sort_by(f64::total_cmp);
    let lo = *reps.get(reps.len() * 25 / 1000)?;
    Some(Gate {
        estimate,
        lo,
        adopt: lo > 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 200 logged points, each phrasing drawn with probability 0.5 among the
    /// delivered cues, complying with the given true rates.
    fn synthetic(p_direct: f64, p_autonomy: f64, seed: u64) -> Vec<(bool, bool)> {
        let mut rng = Pcg32::new(seed, 1);
        (0..200)
            .map(|_| {
                let autonomy = rng.next_f64() < 0.5;
                let p = if autonomy { p_autonomy } else { p_direct };
                (autonomy, rng.next_f64() < p)
            })
            .collect()
    }

    fn logged(data: &[(bool, bool)]) -> Vec<Logged<'static>> {
        data.iter()
            .map(|&(autonomy, complied)| Logged {
                arm: if autonomy { "autonomy" } else { "direct" },
                complied,
                propensity: 0.5,
            })
            .collect()
    }

    #[test]
    fn a_clearly_better_phrasing_is_adopted() {
        let points = logged(&synthetic(0.3, 0.8, 5));
        let gate = conservative_gate(&points, "direct", "autonomy", 7, 2_000).unwrap();
        assert!(gate.adopt, "{gate:?}");
        assert!(gate.estimate > 0.3, "{gate:?}");
    }

    #[test]
    fn equal_phrasings_keep_the_current_default() {
        let points = logged(&synthetic(0.5, 0.5, 5));
        let gate = conservative_gate(&points, "direct", "autonomy", 7, 2_000).unwrap();
        assert!(!gate.adopt, "{gate:?}");
    }

    #[test]
    fn snips_recovers_the_true_rates() {
        let points = logged(&synthetic(0.3, 0.8, 9));
        let (d, a) = (
            snips(&points, "direct").unwrap(),
            snips(&points, "autonomy").unwrap(),
        );
        assert!((d - 0.3).abs() < 0.12 && (a - 0.8).abs() < 0.12, "{d} {a}");
    }
}
