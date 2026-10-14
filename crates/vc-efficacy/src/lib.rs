//! vc-efficacy: the cue-efficacy observer's vocabulary (Day 8) and, from Day
//! 12, the observer, ledger and tuner. no_std, no allocation, no randomness,
//! no clock, no workspace dependencies: time always arrives as an argument.
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod observer;
pub mod p3;

pub use observer::{ObsContext, Observer};
pub use p3::P3Ring;

pub const OBSERVER_VERSION: &str = "vc-efficacy 0.2.0";

/// Coach: tuner on, deterministic round-robin. Study: tuner frozen, arms
/// randomised. Replay: offline re-scoring of a recorded ride.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Coach,
    Study,
    Replay,
}

impl Mode {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Coach => "coach",
            Self::Study => "study",
            Self::Replay => "replay",
        }
    }
}

/// The delivery a decision point got. P1 and P3 have one phrasing (None).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arm {
    None,
    Direct,
    Autonomy,
    /// Study mode's no-cue arm: nothing is played, the cooldown still starts.
    Sham,
}

impl Arm {
    /// The code carried in CUE_PLAYED and the ledger.
    pub const fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Direct => 1,
            Self::Autonomy => 2,
            Self::Sham => 3,
        }
    }

    pub const fn from_code(c: u8) -> Option<Self> {
        match c {
            0 => Some(Self::None),
            1 => Some(Self::Direct),
            2 => Some(Self::Autonomy),
            3 => Some(Self::Sham),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Direct => "direct",
            Self::Autonomy => "autonomy",
            Self::Sham => "sham",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Complied,
    Partial,
    Ignored,
    Reversed,
    /// Something else could explain the change (see `Confound`).
    Confounded,
    /// Not scoreable: no power data, failed audio, a P1 or non-power cue.
    Invalid,
}

impl Verdict {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Complied => "complied",
            Self::Partial => "partial",
            Self::Ignored => "ignored",
            Self::Reversed => "reversed",
            Self::Confounded => "confounded",
            Self::Invalid => "invalid",
        }
    }
}

/// Why a verdict cannot be trusted. Several can hold at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Confound(u8);

impl Confound {
    pub const QUALITY: Self = Self(1 << 0);
    pub const P1_FIRED: Self = Self(1 << 1);
    pub const GRADE: Self = Self(1 << 2);
    pub const CADENCE_ZERO: Self = Self(1 << 3);
    pub const COASTING: Self = Self(1 << 4);
    pub const OVERLAP: Self = Self(1 << 5);
    pub const AUDIO_INTERRUPTED: Self = Self(1 << 6);

    pub const ALL: [(Self, &'static str); 7] = [
        (Self::QUALITY, "quality"),
        (Self::P1_FIRED, "p1_fired"),
        (Self::GRADE, "grade"),
        (Self::CADENCE_ZERO, "cadence_zero"),
        (Self::COASTING, "coasting"),
        (Self::OVERLAP, "overlap"),
        (Self::AUDIO_INTERRUPTED, "audio_interrupted"),
    ];

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// One decision point, as the observer and the ledger see it. Plain data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CueEvent {
    pub cue_id: u32,
    /// `Directive::code()` in vc-core.
    pub directive: u8,
    /// 1, 2 or 3 (our P1/P2/P3).
    pub priority: u8,
    /// 0-5 (Aki's P0-P5).
    pub aki_class: u8,
    pub arm: Arm,
    /// Probability of this arm at this decision point, in 1/65536
    /// (65535 = not randomised).
    pub propensity_q16: u16,
    pub is_sham: bool,
    pub dir: i8,
    pub delta_star_w: f64,
    /// 0 Solo, 1 Rolling, 2 Group.
    pub profile: u8,
    pub t_decision_ns: u64,
    /// At the ear: 0 until the audio callback (or the virtual onset for a sham).
    pub t_onset_ns: u64,
    pub t_end_ns: u64,
}

/// The observer's answer for one cue (Day 12 fills these in).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CueOutcome {
    pub cue_id: u32,
    pub verdict: Verdict,
    pub confound: Confound,
    /// Human mode: the first move came less than 0.35 s after onset.
    pub coincidental: bool,
    /// From the audio's end; may be negative (a mid-phrase responder).
    pub l_obs_ms: Option<i32>,
    pub l_onset_ms: Option<i32>,
    pub l_end_ms: Option<i32>,
    pub pre_slope_w_per_s: f64,
    pub slope_clamped: bool,
    pub effect_max_w: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_codes_round_trip() {
        for arm in [Arm::None, Arm::Direct, Arm::Autonomy, Arm::Sham] {
            assert_eq!(Arm::from_code(arm.code()), Some(arm));
        }

        assert_eq!(Arm::from_code(4), None);
    }

    #[test]
    fn confounds_combine() {
        let mut confound = Confound::default();

        assert!(confound.is_empty());

        confound.insert(Confound::GRADE);
        confound.insert(Confound::P1_FIRED);

        assert!(
            confound.contains(Confound::GRADE)
                && confound.contains(Confound::P1_FIRED)
                && !confound.contains(Confound::QUALITY)
        );
        assert_eq!(confound.bits(), 0b110);
    }

    #[test]
    fn the_types_are_plain_data() {
        fn copy<T: Copy>() {}

        copy::<CueEvent>();
        copy::<CueOutcome>();
    }
}
