//! vc-report: the ledger schema (velocoach.irp.v1), JSONL, statistics, the HTML
//! report card and the study audit. std only; never on the phone's hot path
//! except through the 1 Hz drain.
pub mod audit;
pub mod html;
pub mod ips;
pub mod schema;
pub mod stats;

pub use schema::{Line, SCHEMA, Writer, read, to_json};
