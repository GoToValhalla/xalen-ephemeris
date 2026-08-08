//! Composite Python binding module for advanced XALEN capabilities.
//!
//! `core` preserves the validated Tarot-project bindings for DE440, aspects,
//! transits, synastry, returns, progressions, vargas and Vimshottari dasha.
//! `vedic_extra` and `presentation` extend that surface without changing the
//! top-level `lib.rs` registration contract (`advanced::register`).

mod core;
mod presentation;
mod vedic_extra;

use pyo3::prelude::*;

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    core::register(m)?;
    vedic_extra::register(m)?;
    presentation::register(m)?;
    Ok(())
}
