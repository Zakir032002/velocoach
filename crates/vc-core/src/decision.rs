//! What the Engine hands to the platform: at most one per call.

use crate::directive::{AkiClass, Directive, Priority, SurgePhase};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CueDecision {
    pub id: u32,
    pub directive: crate::directive::Directive,
    pub priority: crate::directive::Priority,
    pub aki_class: crate::directive::AkiClass,
    pub dir: i8,
    pub delta_star_w: f64,
    pub surge: Option<crate::directive::SurgePhase>,
    pub decided_ns: u64,

    pub arm: Arm,
    pub phrase: crate::phrase::PhraseId,
    pub earcon: Option<crate::phrase::Earcon>,
    pub lead_ms: u16,
    pub interrupt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arm {
    None,
    Direct,
    Autonomy,
    Sham,
}
