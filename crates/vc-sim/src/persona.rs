//! Simulated rider reactions.
//! These are assumptions for testing, not measured rider behaviour.

use vc_core::study::Pcg32;

pub const MIN_LATENCY_S: f64 = 0.6;
pub const MAX_LATENCY_S: f64 = 12.0;
pub const SPONTANEOUS_PER_WINDOW: f64 = 0.05;
pub const WINDOW_S: f64 = 15.0;

const CUE_STREAM_BASE: u64 = 1 << 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Persona {
    Dana,
    Sam,
    Ned,
    Calib,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersonaParams {
    pub comply_direct: f64,
    pub comply_autonomy: f64,
    pub median_s: f64,
    pub sigma: f64,
    pub ignore_below_wprime: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reaction {
    NotApplicable,
    Ignore,
    Respond { latency_s: f64, offset_w: f64 },
}

pub fn spontaneous_per_step() -> f64 {
    1.0 - libm::pow(1.0 - SPONTANEOUS_PER_WINDOW, 0.25 / WINDOW_S)
}

impl Persona {
    pub const ALL: [Self; 4] = [Self::Dana, Self::Sam, Self::Ned, Self::Calib];

    /// LOAD persona code. Zero means the scenario's default.
    pub const fn code(self) -> u8 {
        match self {
            Self::Dana => 1,
            Self::Sam => 2,
            Self::Ned => 3,
            Self::Calib => 4,
        }
    }

    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Dana),
            2 => Some(Self::Sam),
            3 => Some(Self::Ned),
            4 => Some(Self::Calib),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Dana => "dana",
            Self::Sam => "sam",
            Self::Ned => "ned",
            Self::Calib => "calib",
        }
    }

    pub const fn params(self) -> PersonaParams {
        match self {
            Self::Dana => PersonaParams {
                comply_direct: 0.90,
                comply_autonomy: 0.90,
                median_s: 1.8,
                sigma: 0.35,
                ignore_below_wprime: 0.0,
            },
            Self::Sam => PersonaParams {
                comply_direct: 0.30,
                comply_autonomy: 0.80,
                median_s: 2.5,
                sigma: 0.45,
                ignore_below_wprime: 0.10,
            },
            Self::Ned => PersonaParams {
                comply_direct: 0.50,
                comply_autonomy: 0.50,
                median_s: 2.5,
                sigma: 0.40,
                ignore_below_wprime: 0.0,
            },
            Self::Calib => PersonaParams {
                comply_direct: 0.70,
                comply_autonomy: 0.70,
                median_s: 3.0,
                sigma: 0.40,
                ignore_below_wprime: 0.0,
            },
        }
    }

    /// Deterministic reaction to one cue.
    /// arm: 1 = Direct, 2 = Autonomy; other arms are not modelled.
    pub fn react(
        self,
        seed: u64,
        cue_id: u32,
        arm: u8,
        dir: i8,
        delta_w: i16,
        wprime_fraction: f64,
    ) -> Reaction {
        if dir == 0 || delta_w == 0 || !(arm == 1 || arm == 2) {
            return Reaction::NotApplicable;
        }

        let p = self.params();
        let mut rng = Pcg32::new(seed, CUE_STREAM_BASE | u64::from(cue_id));

        // Always draw both values. This cue's reaction is independent
        // of the ride's main random-number stream.
        let u = rng.next_f64();
        let z = rng.normal();

        if wprime_fraction < p.ignore_below_wprime {
            return Reaction::Ignore;
        }

        let comply = if arm == 1 {
            p.comply_direct
        } else {
            p.comply_autonomy
        };

        if u >= comply {
            return Reaction::Ignore;
        }

        let latency_s = (p.median_s * libm::exp(p.sigma * z)).clamp(MIN_LATENCY_S, MAX_LATENCY_S);

        Reaction::Respond {
            latency_s,
            offset_w: f64::from(dir.signum()) * f64::from(delta_w.unsigned_abs()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tally(persona: Persona, arm: u8, n: u32, wprime: f64) -> (f64, f64) {
        let mut latencies = std::vec::Vec::new();

        for id in 0..n {
            if let Reaction::Respond { latency_s, .. } = persona.react(42, id, arm, -1, 40, wprime)
            {
                latencies.push(latency_s);
            }
        }

        let fraction = latencies.len() as f64 / f64::from(n);
        latencies.sort_by(f64::total_cmp);

        let median = latencies.get(latencies.len() / 2).copied().unwrap_or(0.0);

        (fraction, median)
    }

    #[test]
    fn each_persona_matches_its_table() {
        let table = [
            (Persona::Dana, 0.90, 0.90, 1.8),
            (Persona::Sam, 0.30, 0.80, 2.5),
            (Persona::Ned, 0.50, 0.50, 2.5),
            (Persona::Calib, 0.70, 0.70, 3.0),
        ];

        for (persona, direct, autonomy, median) in table {
            let (direct_rate, direct_median) = tally(persona, 1, 4_000, 1.0);
            let (autonomy_rate, _) = tally(persona, 2, 4_000, 1.0);

            assert!(
                (direct_rate - direct).abs() < 0.025,
                "{persona:?} direct {direct_rate}"
            );
            assert!(
                (autonomy_rate - autonomy).abs() < 0.025,
                "{persona:?} autonomy {autonomy_rate}"
            );
            assert!(
                (direct_median - median).abs() < 0.12,
                "{persona:?} median {direct_median}"
            );
        }
    }

    #[test]
    fn sam_ignores_cues_below_ten_percent_wprime() {
        assert_eq!(tally(Persona::Sam, 2, 200, 0.05).0, 0.0);
    }

    #[test]
    fn unsupported_arms_and_empty_asks_get_no_reaction() {
        for (arm, dir, delta) in [(0, -1, 40), (3, -1, 40), (1, 0, 40), (1, -1, 0)] {
            assert_eq!(
                Persona::Dana.react(42, 1, arm, dir, delta, 1.0),
                Reaction::NotApplicable
            );
        }
    }

    #[test]
    fn spontaneous_rate_is_five_percent_per_fifteen_seconds() {
        let q = spontaneous_per_step();
        let per_window = 1.0 - libm::pow(1.0 - q, 60.0);

        assert!((per_window - 0.05).abs() < 1e-12);
    }

    #[test]
    fn persona_codes_round_trip() {
        for persona in Persona::ALL {
            assert_eq!(Persona::from_code(persona.code()), Some(persona));
        }

        assert_eq!(Persona::from_code(0), None);
    }
}
