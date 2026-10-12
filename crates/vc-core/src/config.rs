//! Every threshold in one place. Where Aki published a name, the field's
//! doc comment carries it; where he published a value, it is the default.

use vc_physio::QualityConfig;

const S: u64 = 1_000_000_000;

/// Aki's context profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    Solo,
    Rolling,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verbosity {
    Standard,
    Minimal,
    /// Zone-1 explanatory wording. Off by default; never used in Study mode.
    StandardExplanatory,
}

/// One row of Aki's profile table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileParams {
    /// `powerTolerancePercent`
    pub power_tolerance_pct: f64,
    /// `softPedalGracePeriodSec`
    pub soft_pedal_grace_s: f64,
    /// `greyZoneGraceSec`
    pub grey_zone_grace_s: f64,
    pub verbosity: Verbosity,
    /// P2 and P3 cues muted in Group profile.
    pub mute_p2_p3: bool,
}

impl Profile {
    pub const ALL: [Self; 3] = [Self::Solo, Self::Rolling, Self::Group];

    /// Published profile values: Solo ±8% / 5 s / 8 s;
    /// Rolling ±15% / 10 s / 15 s; Group ±20% / 15 s / 20 s.
    pub const fn params(self) -> ProfileParams {
        match self {
            Self::Solo => ProfileParams {
                power_tolerance_pct: 8.0,
                soft_pedal_grace_s: 5.0,
                grey_zone_grace_s: 8.0,
                verbosity: Verbosity::Standard,
                mute_p2_p3: false,
            },
            Self::Rolling => ProfileParams {
                power_tolerance_pct: 15.0,
                soft_pedal_grace_s: 10.0,
                grey_zone_grace_s: 15.0,
                verbosity: Verbosity::Minimal,
                mute_p2_p3: false,
            },
            Self::Group => ProfileParams {
                power_tolerance_pct: 20.0,
                soft_pedal_grace_s: 15.0,
                grey_zone_grace_s: 20.0,
                verbosity: Verbosity::Minimal,
                mute_p2_p3: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    /// The coach's estimate of the rider; simulator truth may differ.
    pub cp_w: f64,
    pub wprime_j: f64,
    pub hr_max: f64,
    pub profile: Profile,

    /// `cooldownBetweenAlertsSec`. 20 s is our choice within Aki's 15–45 s.
    pub cooldown_between_alerts_s: f64,
    /// `promptSurgeCountdownSec`. Tunable only within 5–8 s.
    pub prompt_surge_countdown_s: f64,

    /// HR redline and re-arm settings.
    pub redline_frac: f64,
    pub redline_hold_ns: u64,
    pub redline_rearm_frac: f64,
    pub redline_rearm_ns: u64,

    /// Rule 5: power non-Fresh this long triggers the dropout cue.
    pub dropout_cue_ns: u64,

    /// WPRIME_LOW trigger and re-arm fractions.
    pub wprime_low_frac: f64,
    pub wprime_rearm_frac: f64,

    /// GREY_ZONE_DRIFT: tempo begins at this fraction of CP.
    pub tempo_floor_frac: f64,

    /// CADENCE_LOW margin and hold time.
    pub cadence_low_margin_rpm: f64,
    pub cadence_low_hold_ns: u64,

    /// Cadence is ignored below this power.
    pub min_power_for_cadence_w: f64,

    /// CARDIAC_DRIFT timing and default endurance HR cap.
    pub hr_freeze_ns: u64,
    pub cardiac_drift_hold_ns: u64,
    pub endurance_hr_cap_frac: f64,

    /// No movement for this long stops the workout clock.
    pub auto_pause_ns: u64,

    /// Delivery bookkeeping.
    pub stale_drop_ns: u64,
    pub est_phrase_ns: u64,
    pub observer_window_ns: u64,
    pub audio_lead_ms: u16,

    /// Plausible input ranges.
    pub hr_valid_bpm: (u16, u16),
    pub power_valid_w: (f64, f64),
    pub quality: QualityConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            cp_w: 250.0,
            wprime_j: 20_000.0,
            hr_max: 190.0,
            profile: Profile::Solo,

            cooldown_between_alerts_s: 20.0,
            prompt_surge_countdown_s: 5.0,

            redline_frac: 0.98,
            redline_hold_ns: 120 * S,
            redline_rearm_frac: 0.95,
            redline_rearm_ns: 30 * S,
            dropout_cue_ns: 5 * S,

            wprime_low_frac: 0.20,
            wprime_rearm_frac: 0.30,
            tempo_floor_frac: 0.76,

            cadence_low_margin_rpm: 10.0,
            cadence_low_hold_ns: 8 * S,
            min_power_for_cadence_w: 20.0,

            hr_freeze_ns: 30 * S,
            cardiac_drift_hold_ns: 20 * S,
            endurance_hr_cap_frac: 0.80,

            auto_pause_ns: 3 * S,

            stale_drop_ns: 2 * S,
            est_phrase_ns: 2_500_000_000,
            observer_window_ns: 15 * S,
            audio_lead_ms: 50,

            hr_valid_bpm: (30, 230),
            power_valid_w: (-50.0, 2500.0),
            quality: QualityConfig::default(),
        }
    }
}

/// Aki's published ranges; the tuner will be clamped to these on Day 13.
pub const COOLDOWN_RANGE_S: (f64, f64) = (15.0, 45.0);
pub const SURGE_LEAD_RANGE_S: (f64, f64) = (5.0, 8.0);

impl EngineConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(self.cp_w > 0.0 && self.wprime_j > 0.0 && self.hr_max > 0.0) {
            return Err("cp_w, wprime_j and hr_max must be positive");
        }

        if !(self.wprime_low_frac < self.wprime_rearm_frac
            && self.redline_rearm_frac < self.redline_frac)
        {
            return Err("each re-arm threshold must sit on the safe side of its trigger");
        }

        let within = |v: f64, (lo, hi): (f64, f64)| (lo..=hi).contains(&v);

        if !within(self.cooldown_between_alerts_s, COOLDOWN_RANGE_S) {
            return Err("cooldownBetweenAlertsSec must be within 15-45 s");
        }

        if !within(self.prompt_surge_countdown_s, SURGE_LEAD_RANGE_S) {
            return Err("promptSurgeCountdownSec must be within 5-8 s");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_follow_the_published_table() {
        assert_eq!(EngineConfig::default().validate(), Ok(()));

        let solo = Profile::Solo.params();
        assert_eq!(
            (
                solo.power_tolerance_pct,
                solo.soft_pedal_grace_s,
                solo.grey_zone_grace_s
            ),
            (8.0, 5.0, 8.0)
        );

        assert!(Profile::Group.params().mute_p2_p3);
        assert!(!Profile::Rolling.params().mute_p2_p3);
    }

    #[test]
    fn values_outside_akis_ranges_are_rejected() {
        let c = EngineConfig {
            cooldown_between_alerts_s: 60.0,
            ..EngineConfig::default()
        };
        assert!(c.validate().is_err());

        let c = EngineConfig {
            prompt_surge_countdown_s: 4.0,
            ..EngineConfig::default()
        };
        assert!(c.validate().is_err());
    }
}
