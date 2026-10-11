//! vc-physio: W′ balance, data quality, cadence.
//! Pure maths over numbers: no_std, no heap, no dependency on byte formats.

#![cfg_attr(not(test), no_std)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
#![allow(unused)]
pub mod wbal;
pub use wbal::{GAP_ZERO_FILL_NS, WbalExact, WbalGc, WbalParams, WbalWaterworth, WprimeBalance};
pub(crate) const NS_PER_S: f64 = 1e9;
