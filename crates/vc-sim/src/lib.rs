//! vc-sim: a controllable virtual rider that emits BLE sensor bytes
//! and reacts to cues with deterministic seeded behaviour.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod faults;
pub mod packets;
pub mod persona;
pub mod rider;
pub mod scenario;
pub mod sim;

pub use persona::{Persona, PersonaParams, Reaction};
pub use rider::{Effort, Rider, RiderFaults, RiderParams, Truth};
pub use scenario::{SCENARIOS, Scenario, SimStep};
pub use sim::{CtrlOutcome, Outcome, ResponseRecord, SimPacket, Simulator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimError {
    Ctrl(vc_gatt::DecodeError),
    UnknownScenario(u8),
    UnknownPersona(u8),
    NotStarted,
    BadRate,
}

impl From<vc_gatt::DecodeError> for SimError {
    fn from(error: vc_gatt::DecodeError) -> Self {
        Self::Ctrl(error)
    }
}

impl core::fmt::Display for SimError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Ctrl(error) => write!(f, "CTRL message: {error}"),
            Self::UnknownScenario(id) => write!(f, "unknown scenario id {id}"),
            Self::UnknownPersona(persona) => {
                write!(f, "unknown persona code {persona}")
            }
            Self::NotStarted => write!(f, "no ride running: send START first"),
            Self::BadRate => write!(f, "rate must be greater than 0"),
        }
    }
}
