//! System Information Viewer — shared data layer.
//!
//! Collection and formatting live here so both the GPUI binary and the
//! integration tests drive the exact same code: tests collect from
//! fixture roots (see [`data::fixture`]) through the same formatters and
//! snapshot shape the UI renders.

pub mod data;
pub mod search;
