//! Composite Python binding module for advanced XALEN capabilities.
//!
//! `core` preserves the validated Tarot-project bindings for DE440, aspects,
//! transits, synastry, returns, progressions, vargas and Vimshottari dasha.
//! `vedic_extra` and `presentation` extend that surface without changing the
//! top-level `lib.rs` registration contract (`advanced::register`).

// The original validated advanced binding file predates the repository's
// current rustfmt output. Keep it byte-identical while we compose it here.
#[rustfmt::skip]
mod core;
#[rustfmt::skip]
mod presentation;
#[rustfmt::skip]
mod vedic_extra;

use pyo3::prelude::*;

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    core::register(m)?;
    vedic_extra::register(m)?;
    presentation::register(m)?;
    Ok(())
}
