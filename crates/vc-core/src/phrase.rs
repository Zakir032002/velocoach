//! The phrase bank and the earcon before each phrase.
use crate::decision::Arm;
use crate::directive::{AkiClass, Directive, Priority, SurgePhase};
use crate::lint::LintProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhraseId {
    WprimeLowDirect,
    WprimeLowAutonomy,
    SoftPedalDirect,
    SoftPedalAutonomy,
    GreyZoneDirect,
    GreyZoneAutonomy,
    CadenceLowDirect,
    CadenceLowAutonomy,
    CardiacDrift,
    SafetyHr,
    SensorDropout,
    SurgeIn5,
    SurgeIn6,
    SurgeIn7,
    SurgeIn8,
    GoNow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhraseDef {
    pub id: PhraseId,
    /// File stem of the rendered clip.
    pub key: &'static str,
    pub directive: Directive,
    pub arm: Arm,
    pub text: &'static str,
    pub lint: LintProfile,
}

const fn p(
    id: PhraseId,
    key: &'static str,
    directive: Directive,
    arm: Arm,
    text: &'static str,
    lint: LintProfile,
) -> PhraseDef {
    PhraseDef {
        id,
        key,
        directive,
        arm,
        text,
        lint,
    }
}

use Arm::{Autonomy, Direct};
use Directive as D;
use LintProfile::{Strict, Terse};
use PhraseId as P;

pub const PHRASES: [PhraseDef; 16] = [
    p(
        P::WprimeLowDirect,
        "WPL_D",
        D::WprimeLow,
        Direct,
        "Ease off now, your reserve is at twenty",
        Strict,
    ),
    p(
        P::WprimeLowAutonomy,
        "WPL_A",
        D::WprimeLow,
        Autonomy,
        "Ease off now, or save it for later?",
        Strict,
    ),
    p(
        P::SoftPedalDirect,
        "SPW_D",
        D::SoftPedalWarning,
        Direct,
        "Soft-pedal now, drop it by twenty watts",
        Strict,
    ),
    p(
        P::SoftPedalAutonomy,
        "SPW_A",
        D::SoftPedalWarning,
        Autonomy,
        "Soft-pedal now, or keep it if you choose?",
        Strict,
    ),
    p(
        P::GreyZoneDirect,
        "GZD_D",
        D::GreyZoneDrift,
        Direct,
        "Back off now, you're drifting into tempo",
        Strict,
    ),
    p(
        P::GreyZoneAutonomy,
        "GZD_A",
        D::GreyZoneDrift,
        Autonomy,
        "Back off now, or is tempo the plan?",
        Strict,
    ),
    p(
        P::CadenceLowDirect,
        "CAD_D",
        D::CadenceLow,
        Direct,
        "Raise your cadence, target ninety",
        Strict,
    ),
    p(
        P::CadenceLowAutonomy,
        "CAD_A",
        D::CadenceLow,
        Autonomy,
        "Raise your cadence, or keep it if you like?",
        Strict,
    ),
    p(
        P::CardiacDrift,
        "HRD",
        D::CardiacDrift,
        Arm::None,
        "Ease the pace, your heart rate is drifting",
        Strict,
    ),
    p(
        P::SafetyHr,
        "SAF",
        D::SafetyHr,
        Arm::None,
        "Ease right off now, heart rate is too high",
        Strict,
    ),
    p(
        P::SensorDropout,
        "DROP",
        D::SensorDropout,
        Arm::None,
        "Check power meter connection.",
        Strict,
    ),
    p(
        P::SurgeIn5,
        "SRG5",
        D::SurgePending,
        Arm::None,
        "Surge in five",
        Terse,
    ),
    p(
        P::SurgeIn6,
        "SRG6",
        D::SurgePending,
        Arm::None,
        "Surge in six",
        Terse,
    ),
    p(
        P::SurgeIn7,
        "SRG7",
        D::SurgePending,
        Arm::None,
        "Surge in seven",
        Terse,
    ),
    p(
        P::SurgeIn8,
        "SRG8",
        D::SurgePending,
        Arm::None,
        "Surge in eight",
        Terse,
    ),
    p(P::GoNow, "GO", D::SurgePending, Arm::None, "Go now", Terse),
];

pub fn def(id: PhraseId) -> Option<&'static PhraseDef> {
    PHRASES.iter().find(|d| d.id == id)
}

/// Selects the phrase using directive, arm, and surge phase.
pub const fn phrase_for(d: Directive, arm: Arm, surge: Option<SurgePhase>) -> PhraseId {
    let autonomy = matches!(arm, Arm::Autonomy);

    match d {
        D::WprimeLow if autonomy => P::WprimeLowAutonomy,
        D::WprimeLow => P::WprimeLowDirect,
        D::SoftPedalWarning if autonomy => P::SoftPedalAutonomy,
        D::SoftPedalWarning => P::SoftPedalDirect,
        D::GreyZoneDrift if autonomy => P::GreyZoneAutonomy,
        D::GreyZoneDrift => P::GreyZoneDirect,
        D::CadenceLow if autonomy => P::CadenceLowAutonomy,
        D::CadenceLow => P::CadenceLowDirect,
        D::CardiacDrift => P::CardiacDrift,
        D::SafetyHr => P::SafetyHr,
        D::SensorDropout => P::SensorDropout,
        D::SurgePending => match surge {
            Some(SurgePhase::Go) => P::GoNow,
            Some(SurgePhase::Countdown(6)) => P::SurgeIn6,
            Some(SurgePhase::Countdown(7)) => P::SurgeIn7,
            Some(SurgePhase::Countdown(8)) => P::SurgeIn8,
            _ => P::SurgeIn5,
        },
    }
}

/// Aki's earcon table, by directive class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Earcon {
    /// 440 Hz low double beep
    P0DoubleBeep,
    /// 587 Hz soft chime
    P1Chime,
    /// 300 -> 200 Hz dual-tone
    P2DualTone,
    /// 880 -> 660 Hz (falling: back off)
    P3High,
    /// 660 -> 880 Hz (rising: raise)
    P3Low,
    /// 1000 Hz click
    P4Click,
    /// 523 Hz bell
    P5Bell,
}

impl Earcon {
    /// File stem of the generated tone.
    pub const fn key(self) -> &'static str {
        match self {
            Self::P0DoubleBeep => "ear_p0",
            Self::P1Chime => "ear_p1",
            Self::P2DualTone => "ear_p2",
            Self::P3High => "ear_p3_high",
            Self::P3Low => "ear_p3_low",
            Self::P4Click => "ear_p4",
            Self::P5Bell => "ear_p5",
        }
    }
}

/// P3 is speech-only unless enabled by the directive's tuner bit.
pub fn earcon_for(d: Directive, p3_earcon_mask: u8) -> Option<Earcon> {
    if d.priority() == Priority::P3 && p3_earcon_mask & (1 << d.slot()) == 0 {
        return None;
    }

    Some(match d.aki_class() {
        AkiClass::P0 => Earcon::P0DoubleBeep,
        AkiClass::P1 => Earcon::P1Chime,
        AkiClass::P2 => Earcon::P2DualTone,
        AkiClass::P3 if d == D::CadenceLow => Earcon::P3Low,
        AkiClass::P3 => Earcon::P3High,
        AkiClass::P4 => Earcon::P4Click,
        AkiClass::P5 => Earcon::P5Bell,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::lint;

    #[test]
    fn lint_every_phrase_passes_its_profile() {
        for phrase in PHRASES {
            assert!(
                lint(phrase.text, phrase.lint).is_ok(),
                "{}: {:?} -> {:?}",
                phrase.key,
                phrase.text,
                lint(phrase.text, phrase.lint)
            );
        }
    }

    #[test]
    fn keys_are_unique_and_every_phrase_is_reachable() {
        let mut keys: Vec<&str> = PHRASES.iter().map(|d| d.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), PHRASES.len());

        for phrase in PHRASES {
            let surge = match phrase.id {
                P::GoNow => Some(SurgePhase::Go),
                P::SurgeIn6 => Some(SurgePhase::Countdown(6)),
                P::SurgeIn7 => Some(SurgePhase::Countdown(7)),
                P::SurgeIn8 => Some(SurgePhase::Countdown(8)),
                _ => Some(SurgePhase::Countdown(5)),
            };
            assert_eq!(
                phrase_for(phrase.directive, phrase.arm, surge),
                phrase.id,
                "{}",
                phrase.key
            );
        }
    }

    #[test]
    fn every_p2_directive_has_both_arms() {
        for directive in Directive::ALL
            .into_iter()
            .filter(|d| d.priority() == Priority::P2)
        {
            for arm in [Direct, Autonomy] {
                assert!(
                    PHRASES
                        .iter()
                        .any(|p| p.directive == directive && p.arm == arm),
                    "{directive:?} {arm:?}"
                );
            }
        }
    }

    #[test]
    fn p3_is_speech_only_until_the_tuner_enables_an_earcon() {
        assert_eq!(earcon_for(D::CardiacDrift, 0), None);
        assert_eq!(
            earcon_for(D::CardiacDrift, 1 << D::CardiacDrift.slot()),
            Some(Earcon::P4Click)
        );
        assert_eq!(earcon_for(D::SafetyHr, 0), Some(Earcon::P0DoubleBeep));
        assert_eq!(earcon_for(D::WprimeLow, 0), Some(Earcon::P2DualTone));
    }
}
