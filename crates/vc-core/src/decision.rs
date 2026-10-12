//! What the Engine hands to the platform: at most one per call.

use crate::directive::{AkiClass, Directive, Priority, SurgePhase};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CueDecision {
    /// Per-ride counter: cue ID in CUE_PLAYED and the ledger.
    pub id: u32,
    pub directive: Directive,
    pub priority: Priority,
    pub aki_class: AkiClass,
    /// +1 asks for more power, -1 for less, 0 is not a power ask.
    pub dir: i8,
    /// Size of the requested power change in watts.
    pub delta_star_w: f64,
    /// Surge countdown or "go now".
    pub surge: Option<SurgePhase>,
    /// Timestamp of the call that produced this decision.
    pub decided_ns: u64,
}
