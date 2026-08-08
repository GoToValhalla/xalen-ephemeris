// Copyright 2024-2026 XALEN Technology Pvt Ltd
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Python bindings for the configurable-orb aspect engine and pattern
//! detection added to `xalen-western` in this package
//! (`crates/xalen-western/src/aspects.rs` `AspectConfig`/`AspectResult` API,
//! and `crates/xalen-western/src/patterns.rs` named-body pattern detectors).
//! Portions of the underlying Rust logic are adapted from Anonyfox/celestine
//! (MIT License) — see docs/THIRD_PARTY_SOURCES.md and the provenance
//! comments in the Rust source files themselves. No astrology math is
//! reimplemented in this file; every function here is a thin wrapper.
//!
//! This module is additive alongside `advanced::aspects`/`synastry`/
//! `transits` (the pre-existing Python bindings over
//! `xalen_western::aspects::find_all_aspects`, unchanged) — it does not
//! replace them, and none of their signatures were touched.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};

use xalen_western::aspects::{AspectConfig, AspectResult, AspectType, find_all_aspects_ex};
use xalen_western::patterns::{AspectPatternMatch, find_named_patterns};

/// Body ID -> `AspectType` mapping for the 14 aspects, by canonical lowercase
/// name (matches the naming already used elsewhere in this crate, e.g.
/// `parse_ayanamsa`/`parse_house_system` in `src/lib.rs`).
fn aspect_type_from_name(name: &str) -> PyResult<AspectType> {
    match name.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
        "conjunction" => Ok(AspectType::Conjunction),
        "opposition" => Ok(AspectType::Opposition),
        "trine" => Ok(AspectType::Trine),
        "square" => Ok(AspectType::Square),
        "sextile" => Ok(AspectType::Sextile),
        "semisextile" => Ok(AspectType::SemiSextile),
        "quincunx" => Ok(AspectType::Quincunx),
        "semisquare" => Ok(AspectType::SemiSquare),
        "sesquiquadrate" => Ok(AspectType::Sesquiquadrate),
        "quintile" => Ok(AspectType::Quintile),
        "biquintile" => Ok(AspectType::BiQuintile),
        "septile" => Ok(AspectType::Septile),
        "novile" => Ok(AspectType::Novile),
        "decile" => Ok(AspectType::Decile),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown aspect type '{other}'; expected one of: conjunction, opposition, trine, \
             square, sextile, semi_sextile, quincunx, semi_square, sesquiquadrate, quintile, \
             biquintile, septile, novile, decile"
        ))),
    }
}

fn aspect_type_name(t: AspectType) -> &'static str {
    match t {
        AspectType::Conjunction => "conjunction",
        AspectType::Opposition => "opposition",
        AspectType::Trine => "trine",
        AspectType::Square => "square",
        AspectType::Sextile => "sextile",
        AspectType::SemiSextile => "semi_sextile",
        AspectType::Quincunx => "quincunx",
        AspectType::SemiSquare => "semi_square",
        AspectType::Sesquiquadrate => "sesquiquadrate",
        AspectType::Quintile => "quintile",
        AspectType::BiQuintile => "biquintile",
        AspectType::Septile => "septile",
        AspectType::Novile => "novile",
        AspectType::Decile => "decile",
    }
}

/// Build an `AspectConfig` from the Python-facing keyword arguments shared by
/// `aspects_ex`/`synastry_ex`/`transits_ex`. All parameters are optional and
/// default to exactly what `AspectConfig::default()` gives (5 Ptolemaic
/// majors, XALEN's existing default orbs, out-of-sign reported not filtered)
/// — so omitting every keyword reproduces the same aspect set as the
/// pre-existing `advanced::aspects()` binding's `major_only=True` mode.
#[allow(clippy::too_many_arguments)]
fn build_config(
    aspect_types: Option<Vec<String>>,
    orbs: Option<HashMap<String, f64>>,
    include_out_of_sign: Option<bool>,
    exclude_out_of_sign: Option<bool>,
    out_of_sign_penalty: Option<f64>,
    minimum_strength: Option<f64>,
    include_applying: Option<bool>,
) -> PyResult<AspectConfig> {
    let mut config = AspectConfig::default();

    if let Some(names) = aspect_types {
        let mut types = Vec::with_capacity(names.len());
        for name in &names {
            types.push(aspect_type_from_name(name)?);
        }
        config.aspect_types = types;
    }
    if let Some(orb_map) = orbs {
        for (name, orb) in orb_map {
            let t = aspect_type_from_name(&name)?;
            config.orb_overrides.insert(t, orb);
        }
    }
    if let Some(v) = include_out_of_sign {
        config.include_out_of_sign = v;
    }
    if let Some(v) = exclude_out_of_sign {
        config.exclude_out_of_sign = v;
    }
    if let Some(v) = out_of_sign_penalty {
        config.out_of_sign_penalty = v;
    }
    if let Some(v) = minimum_strength {
        config.minimum_strength = v;
    }
    if let Some(v) = include_applying {
        config.include_applying = v;
    }
    Ok(config)
}

fn result_to_dict(py: Python<'_>, r: &AspectResult) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("body1", &r.body1)?;
    dict.set_item("body2", &r.body2)?;
    dict.set_item("aspect_type", aspect_type_name(r.aspect_type))?;
    dict.set_item("angle_deg", r.aspect_type.angle_deg())?;
    dict.set_item("separation_deg", r.separation_deg)?;
    dict.set_item("deviation_deg", r.deviation_deg)?;
    dict.set_item("orb_deg", r.orb_deg)?;
    dict.set_item("strength", r.strength)?;
    dict.set_item("phase", r.phase.map(|p| format!("{p:?}").to_lowercase()))?;
    dict.set_item("is_out_of_sign", r.is_out_of_sign)?;
    dict.set_item("is_major", r.aspect_type.is_major())?;
    dict.set_item("is_kepler", r.aspect_type.is_kepler())?;
    Ok(dict.into())
}

fn positions_to_tuples(
    positions: HashMap<String, (f64, Option<f64>)>,
) -> Vec<(String, f64, Option<f64>)> {
    positions
        .into_iter()
        .map(|(name, (lon, speed))| (name, lon, speed))
        .collect()
}

/// Find all aspects among a set of named positions, with full configurability
/// over which of the 14 aspect types to check, per-type orb overrides,
/// out-of-sign handling, and a minimum-strength floor.
///
/// This is the configurable counterpart to the pre-existing `aspects()`
/// binding (which is unchanged and keeps working for existing callers).
/// `aspects()` always uses `AspectType::MAJOR`/`AspectType::ALL` (11 types,
/// fixed default orbs, no out-of-sign/strength metadata); `aspects_ex()`
/// exposes the full `AspectConfig` model — all 14 aspect types, configurable
/// orbs, strength, phase, and out-of-sign — added in this package. Wraps
/// `xalen_western::aspects::find_all_aspects_ex`.
///
/// Parameters
/// ----------
/// positions : dict[str, tuple[float, float | None]]
///     ``{name: (longitude_deg, speed_deg_per_day_or_None)}``. Pass ``None``
///     for the speed component when unavailable; `phase` will then be
///     ``None`` for that body's aspects rather than a guessed value.
/// aspect_types : list[str], optional
///     Which of the 14 aspect types to check (by name, e.g. "trine",
///     "semi_sextile", "septile"). Default: the 5 Ptolemaic majors.
/// orbs : dict[str, float], optional
///     Per-aspect-type orb overrides in degrees, keyed by the same names as
///     `aspect_types`. Any type not present uses XALEN's existing default
///     orb for that type.
/// include_out_of_sign : bool, optional
///     Whether to compute the out-of-sign flag. Default True (does not
///     filter results by itself; see `exclude_out_of_sign`).
/// exclude_out_of_sign : bool, optional
///     If True, drop out-of-sign aspects from the results entirely rather
///     than just flagging them. Default False.
/// out_of_sign_penalty : float, optional
///     Strength multiplier penalty (0.0-1.0) applied to out-of-sign aspects.
///     Default 0 (no penalty).
/// minimum_strength : float, optional
///     Minimum strength (0-100) required for an aspect to be included.
///     Default 0.
/// include_applying : bool, optional
///     Whether to compute applying/separating/exact phase (requires speed
///     data). Default True.
///
/// Returns
/// -------
/// list[dict]
///     Each dict: ``{"body1", "body2", "aspect_type", "angle_deg",
///     "separation_deg", "deviation_deg", "orb_deg", "strength", "phase",
///     "is_out_of_sign", "is_major", "is_kepler"}``. ``phase`` is one of
///     ``"applying"``, ``"separating"``, ``"exact"``, or ``None``.
#[pyfunction]
#[pyo3(signature = (
    positions,
    aspect_types = None,
    orbs = None,
    include_out_of_sign = None,
    exclude_out_of_sign = None,
    out_of_sign_penalty = None,
    minimum_strength = None,
    include_applying = None,
))]
#[allow(clippy::too_many_arguments)]
fn aspects_ex(
    py: Python<'_>,
    positions: HashMap<String, (f64, Option<f64>)>,
    aspect_types: Option<Vec<String>>,
    orbs: Option<HashMap<String, f64>>,
    include_out_of_sign: Option<bool>,
    exclude_out_of_sign: Option<bool>,
    out_of_sign_penalty: Option<f64>,
    minimum_strength: Option<f64>,
    include_applying: Option<bool>,
) -> PyResult<Py<PyAny>> {
    let config = build_config(
        aspect_types,
        orbs,
        include_out_of_sign,
        exclude_out_of_sign,
        out_of_sign_penalty,
        minimum_strength,
        include_applying,
    )?;
    let body_tuples = positions_to_tuples(positions);
    let results = find_all_aspects_ex(&body_tuples, &config);
    let list = PyList::empty(py);
    for r in &results {
        list.append(result_to_dict(py, r)?)?;
    }
    Ok(list.into())
}

/// List of the 14 aspect-type names accepted by `aspects_ex`'s `aspect_types`
/// and `orbs` parameters, in canonical (Conjunction-first) order.
///
/// Returns
/// -------
/// list[str]
#[pyfunction]
fn aspect_type_names(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let list = PyList::empty(py);
    for t in xalen_western::aspects::AspectType::ALL_14 {
        list.append(aspect_type_name(*t))?;
    }
    Ok(list.into())
}

fn pattern_to_dict(py: Python<'_>, p: &AspectPatternMatch) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("pattern_type", format!("{:?}", p.pattern_type))?;
    dict.set_item("bodies", p.bodies.clone())?;
    dict.set_item("description", &p.description)?;
    let aspects_list = PyList::empty(py);
    for a in &p.aspects {
        aspects_list.append(result_to_dict(py, a)?)?;
    }
    dict.set_item("aspects", aspects_list)?;
    Ok(dict.into())
}

/// Detect aspect patterns (T-Square, Grand Trine, Grand Cross, Yod, Kite,
/// Mystic Rectangle, Stellium) among a set of named positions.
///
/// This computes aspects internally via `aspects_ex`'s same configuration
/// model (all 14 types available), then runs
/// `xalen_western::patterns::find_named_patterns` over the result — pattern
/// detection is not re-implemented here, it operates purely on the
/// already-calculated aspect list, matching the Rust API's own design.
///
/// Parameters
/// ----------
/// positions : dict[str, tuple[float, float | None]]
///     Same shape as `aspects_ex`.
/// aspect_types : list[str], optional
///     Defaults to all 14 aspect types (patterns like Yod/Kite need
///     quincunx/sextile in addition to the Ptolemaic majors, so — unlike
///     `aspects_ex`'s 5-major default — this defaults wide).
/// orbs : dict[str, float], optional
///     Same as `aspects_ex`.
///
/// Returns
/// -------
/// list[dict]
///     Each dict: ``{"pattern_type", "bodies", "description", "aspects"}``,
///     where ``aspects`` is a list of the same dicts `aspects_ex` returns.
#[pyfunction]
#[pyo3(signature = (positions, aspect_types = None, orbs = None))]
fn aspect_patterns(
    py: Python<'_>,
    positions: HashMap<String, (f64, Option<f64>)>,
    aspect_types: Option<Vec<String>>,
    orbs: Option<HashMap<String, f64>>,
) -> PyResult<Py<PyAny>> {
    let mut config = build_config(aspect_types, orbs, None, None, None, None, None)?;
    // Patterns need more than the 5-major default (Yod/Kite need
    // quincunx+sextile, Mystic Rectangle needs trine+sextile+opposition) —
    // widen to all 14 unless the caller explicitly requested specific types.
    if config.aspect_types == xalen_western::aspects::AspectType::MAJOR.to_vec() {
        config.aspect_types = xalen_western::aspects::AspectType::ALL_14.to_vec();
    }
    let body_tuples = positions_to_tuples(positions);
    let aspect_results = find_all_aspects_ex(&body_tuples, &config);
    let patterns = find_named_patterns(&aspect_results);
    let list = PyList::empty(py);
    for p in &patterns {
        list.append(pattern_to_dict(py, p)?)?;
    }
    Ok(list.into())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(aspects_ex, m)?)?;
    m.add_function(wrap_pyfunction!(aspect_type_names, m)?)?;
    m.add_function(wrap_pyfunction!(aspect_patterns, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aspect_type_from_name_roundtrip() {
        for t in xalen_western::aspects::AspectType::ALL_14 {
            let name = aspect_type_name(*t);
            let parsed = aspect_type_from_name(name).unwrap();
            assert_eq!(parsed, *t);
        }
    }

    #[test]
    fn test_aspect_type_from_name_rejects_unknown() {
        assert!(aspect_type_from_name("bogus").is_err());
    }

    #[test]
    fn test_build_config_defaults_match_major_only() {
        let config = build_config(None, None, None, None, None, None, None).unwrap();
        assert_eq!(
            config.aspect_types,
            xalen_western::aspects::AspectType::MAJOR.to_vec()
        );
    }
}
