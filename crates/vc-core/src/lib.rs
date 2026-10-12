//! vc-core: the brain. Sans-IO and synchronous: platform shells feed it
//! bytes and times; it answers with at most one CueDecision per call.
//! no_std and allocation-free: it does not even link `alloc`.

#![cfg_attr(not(any(test, feature = "testkit")), no_std)]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unused)]
pub mod config;
pub mod decision;
pub mod directive;
pub mod engine;
pub mod hysteresis;
pub mod policy;
pub mod safety;
pub mod smooth;
#[cfg(feature = "testkit")]
pub mod testkit;
pub mod workout;

pub use config::{EngineConfig, Profile, ProfileParams, Verbosity};
pub use decision::CueDecision;
pub use directive::{AkiClass, Directive, Priority, SurgePhase};
pub use engine::{Engine, EngineError, Snapshot, Source};
pub use workout::{Step, StepKind, WorkoutPlan};
