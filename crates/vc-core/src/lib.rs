//! vc-core: the brain. Sans-IO and synchronous: platform shells feed it
//! bytes and times; it answers with at most one CueDecision per call.
//! no_std and allocation-free: it does not even link `alloc`.

#![cfg_attr(not(any(test, feature = "testkit")), no_std)]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unused)]
pub mod capture;
pub mod chain;
pub mod clock;
pub mod config;
pub mod decision;
pub mod directive;
pub mod efficacy;
pub mod engine;
pub mod golden;
pub mod hysteresis;
pub mod latency;
pub mod ledger_ring;
pub mod lint;
pub mod phrase;
pub mod policy;
pub mod safety;
pub mod smooth;
pub mod study;
#[cfg(feature = "testkit")]
pub mod testkit;
pub mod workout;

pub use config::{EngineConfig, Profile, ProfileParams, Verbosity};
pub use decision::CueDecision;
pub use directive::{AkiClass, Directive, Priority, SurgePhase};
pub use engine::{Engine, EngineError, Snapshot, Source};
pub use ledger_ring::{LedgerRecord, LedgerRing, OnsetSource};
pub use phrase::{Earcon, PhraseId};
pub use vc_efficacy::{Arm, Confound, CueEvent, CueOutcome, Mode, Verdict};
pub use workout::{Step, StepKind, WorkoutPlan};
