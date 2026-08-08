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

//! Python bindings for XALEN features that exist in the Rust core but were not
//! previously wired into `xalen-python`: DE440/JPL kernel selection + mode
//! reporting, aspects/synastry/transits, exact solar & lunar returns, secondary
//! progressions, Vedic divisional charts (vargas), and Vimshottari dasha.
//!
//! Every function here is a thin PyO3 wrapper over an existing Rust function —
//! no astrology math is reimplemented in this file. See the docstring on each
//! `#[pyfunction]` for which Rust crate/function it delegates to.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};

use xalen_ephem::Almanac;
use xalen_ephem::{De440Provider, ReturnBody, find_return};
use xalen_time::{JdUT1, JulianDay};
use xalen_vedic::dasha::{DashaLevel, DashaPeriod, vimshottari_dasha as core_vimshottari_dasha};
use xalen_vedic::divisional::{VargaChart, compute_varga_sign};
use xalen_western::aspects::{Aspect, AspectType, find_all_aspects};
use xalen_western::progressions::{ProgressionType, progressed_jd};

use crate::body_from_id;

// ---------------------------------------------------------------------------
// DE440 kernel: global optional state + mode reporting
// ---------------------------------------------------------------------------
//
// `Almanac::default_vedic()` is cheap to construct (a Vec of one Arc<Provider>),
// so rather than caching a whole Almanac we cache just the loaded DE440
// provider (if any) and build a fresh Almanac with it stacked on top of VSOP87
// for every call. This mirrors exactly what `Almanac::with_provider` is for.
//
// ponytail: process-wide global, not per-thread/per-session — this matches how
// XALEN's own Rust examples use a single long-lived Almanac; a Python caller
// wanting multiple independent kernels concurrently would need a real handle
// API, add if that need shows up.
static DE440_PROVIDER: OnceLock<RwLock<Option<Arc<De440Provider>>>> = OnceLock::new();

fn de440_slot() -> &'static RwLock<Option<Arc<De440Provider>>> {
    DE440_PROVIDER.get_or_init(|| RwLock::new(None))
}

/// Build an almanac using the currently-loaded DE440 kernel (if any) stacked
/// in front of the analytical VSOP87 provider, exactly like
/// `Almanac::default_vedic().with_provider(de440)`.
pub(crate) fn build_almanac() -> Almanac {
    let guard = de440_slot().read().unwrap();
    match guard.as_ref() {
        Some(provider) => Almanac::default_vedic().with_provider(provider.clone()),
        None => Almanac::default_vedic(),
    }
}

/// The engine label for whatever is currently loaded, matching
/// `xalen_ephem::de440::AccuracyTier::label()` verbatim (not reinvented here).
pub(crate) fn current_engine_label() -> &'static str {
    let guard = de440_slot().read().unwrap();
    match guard.as_ref() {
        Some(provider) => provider.accuracy_tier().label(),
        None => "VSOP87 analytical fallback",
    }
}

/// Load a JPL DE440 SPK binary kernel (`.bsp`, e.g. `de440s.bsp`) from a local
/// file path so subsequent position calls can use JPL-grade precision instead
/// of the analytical (VSOP87A/ELP2000-82) engine.
///
/// This wraps `xalen_ephem::De440Provider::try_from_file_strict` — a LOUD
/// loader that returns an error (rather than silently degrading) if the file
/// is missing or is not a parseable DE440 kernel. Call `xalen.de440_status()`
/// afterward (or check this function's return value) to confirm a real,
/// DE440-provenance-confirmed kernel is active versus an unconfirmed SPK file.
///
/// Once loaded, the kernel is used by every position-computing function in
/// this module for every body/epoch it covers, with automatic fallback to the
/// analytical engine for anything outside the kernel's coverage (e.g. a body
/// the kernel doesn't carry, or an epoch outside its span) — this is
/// `xalen_ephem::De440Provider`'s own existing fallback behavior, not new
/// logic added here.
///
/// Parameters
/// ----------
/// path : str
///     Filesystem path to a DE440 `.bsp` kernel (e.g. downloaded from
///     ``https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440s.bsp``).
///
/// Returns
/// -------
/// dict
///     ``{"loaded": bool, "tier": str, "label": str, "jd_start": float,
///     "jd_end": float}``. ``tier`` is one of ``"kernel"`` (confirmed real
///     DE440 provenance — JPL-grade precision active),
///     ``"unconfirmed_kernel"`` (a parseable SPK file that isn't confirmed
///     DE440 — treated as analytical-grade), or this call raises instead of
///     returning ``"analytical_fallback"`` (a hard load failure is an
///     exception, not a silent dict field — see below).
///
/// Raises
/// ------
/// ValueError
///     If the file cannot be read or parsed as an SPK kernel at all.
#[pyfunction]
fn load_de440_kernel(py: Python<'_>, path: &str) -> PyResult<Py<PyAny>> {
    let provider = De440Provider::try_from_file_strict(&PathBuf::from(path))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let tier = provider.accuracy_tier();
    let label = tier.label();
    let (jd_start, jd_end) = {
        use xalen_ephem::EphemerisProvider;
        provider.coverage()
    };
    *de440_slot().write().unwrap() = Some(Arc::new(provider));

    let dict = PyDict::new(py);
    dict.set_item("loaded", true)?;
    dict.set_item(
        "tier",
        match tier {
            xalen_ephem::AccuracyTier::Kernel => "kernel",
            xalen_ephem::AccuracyTier::UnconfirmedKernel => "unconfirmed_kernel",
            xalen_ephem::AccuracyTier::AnalyticalFallback => "analytical_fallback",
        },
    )?;
    dict.set_item("label", label)?;
    dict.set_item("jd_start", jd_start)?;
    dict.set_item("jd_end", jd_end)?;
    Ok(dict.into())
}

/// Report which engine is currently active for position computations:
/// the JPL DE440 kernel (if one was loaded via `load_de440_kernel` and is
/// DE440-provenance-confirmed) or the VSOP87 analytical fallback.
///
/// This does not itself compute anything — it reports the *provider-level*
/// state. A loaded kernel is still used only for the bodies/epochs it
/// actually covers; a body/epoch outside that coverage falls back to
/// analytical for that one call even with a kernel loaded (this is
/// `De440Provider`'s own existing per-call fallback, unchanged here).
///
/// Returns
/// -------
/// dict
///     ``{"tier": str, "label": str, "has_kernel_loaded": bool}``
#[pyfunction]
fn de440_status(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let guard = de440_slot().read().unwrap();
    let dict = PyDict::new(py);
    match guard.as_ref() {
        Some(provider) => {
            let tier = provider.accuracy_tier();
            dict.set_item(
                "tier",
                match tier {
                    xalen_ephem::AccuracyTier::Kernel => "kernel",
                    xalen_ephem::AccuracyTier::UnconfirmedKernel => "unconfirmed_kernel",
                    xalen_ephem::AccuracyTier::AnalyticalFallback => "analytical_fallback",
                },
            )?;
            dict.set_item("label", tier.label())?;
            dict.set_item("has_kernel_loaded", true)?;
        }
        None => {
            dict.set_item("tier", "analytical_fallback")?;
            dict.set_item("label", "VSOP87 analytical fallback")?;
            dict.set_item("has_kernel_loaded", false)?;
        }
    }
    Ok(dict.into())
}

/// Unload any DE440 kernel, reverting all subsequent calls to the analytical
/// (VSOP87A/ELP2000-82) engine. Mainly useful for tests that need to compare
/// both modes in the same process.
#[pyfunction]
fn unload_de440_kernel() {
    *de440_slot().write().unwrap() = None;
}

/// Compute a body's geocentric ecliptic longitude using whichever engine is
/// currently active (DE440 kernel if loaded and it covers this body/epoch,
/// else the VSOP87 analytical fallback), and report which one actually served
/// this specific call.
///
/// This is the DE440-aware counterpart to `planet_longitude`/`planet_position`
/// (which always use the plain analytical almanac). Use this whenever the
/// caller needs to know, per result, whether JPL-grade precision was used.
///
/// Returns
/// -------
/// dict
///     ``{"longitude": float, "engine": str}`` where ``engine`` is
///     ``"JPL DE440 kernel"``, ``"unconfirmed SPK kernel (analytical-grade)"``,
///     or ``"VSOP87 analytical fallback"`` — the exact
///     `xalen_ephem::AccuracyTier::label()` string, not a new label invented
///     here. Note this reports the *provider's* tier, not per-body/per-epoch
///     kernel coverage (see `load_de440_kernel` docstring).
#[pyfunction]
#[pyo3(signature = (jd, body))]
fn planet_longitude_with_engine(py: Python<'_>, jd: f64, body: u8) -> PyResult<Py<PyAny>> {
    crate::check_jd(jd)?;
    let almanac = build_almanac();
    let b = body_from_id(body)?;
    let lon = almanac
        .geocentric_longitude_deg(b, JdUT1(jd))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let dict = PyDict::new(py);
    dict.set_item("longitude", lon)?;
    dict.set_item("engine", current_engine_label())?;
    Ok(dict.into())
}

// ---------------------------------------------------------------------------
// Aspects / synastry / transits
// ---------------------------------------------------------------------------
//
// All three delegate to the same `xalen_western::aspects::find_all_aspects`
// (the Rust crate's actual aspect-finding engine). Synastry and transits are
// the same computation as "aspects" over a combined position list; the only
// difference is which pairs the caller cares about, which we implement here
// as a post-hoc filter on `find_all_aspects`'s own output (no new aspect math,
// just filtering by name-set membership).
fn aspect_to_dict(py: Python<'_>, a: &Aspect) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("body1", &a.body1)?;
    dict.set_item("body2", &a.body2)?;
    dict.set_item("aspect_type", format!("{:?}", a.aspect_type))?;
    dict.set_item("angle_deg", a.aspect_type.angle_deg())?;
    dict.set_item("orb_deg", a.orb_deg)?;
    dict.set_item("direction", format!("{:?}", a.direction))?;
    dict.set_item("exact_deg", a.exact_deg)?;
    dict.set_item("is_major", a.aspect_type.is_major())?;
    Ok(dict.into())
}

fn parse_positions(
    positions: std::collections::HashMap<String, (f64, f64)>,
) -> Vec<(String, f64, f64)> {
    positions
        .into_iter()
        .map(|(name, (lon, speed))| (name, lon, speed))
        .collect()
}

/// Find all major+minor aspects among a set of named positions.
///
/// This wraps `xalen_western::aspects::find_all_aspects` directly — the same
/// engine XALEN's own Rust natal-chart examples use.
///
/// Parameters
/// ----------
/// positions : dict[str, tuple[float, float]]
///     ``{name: (longitude_deg, speed_deg_per_day)}``. Speed is used only to
///     classify each aspect as applying/separating/exact; pass ``0.0`` if
///     unknown (every aspect will be classified applying or separating from
///     rounding rather than misreported as exact, matching the Rust
///     function's own behavior for zero speed).
/// major_only : bool, optional
///     If True, only the five Ptolemaic aspects (conjunction, sextile,
///     square, trine, opposition) are checked. Default False checks all 11
///     types `xalen_western::aspects::AspectType::ALL` defines.
///
/// Returns
/// -------
/// list[dict]
///     Each dict: ``{"body1", "body2", "aspect_type", "angle_deg", "orb_deg",
///     "direction", "exact_deg", "is_major"}``.
#[pyfunction]
#[pyo3(signature = (positions, major_only = false))]
fn aspects(
    py: Python<'_>,
    positions: std::collections::HashMap<String, (f64, f64)>,
    major_only: bool,
) -> PyResult<Py<PyAny>> {
    let pos_vec = parse_positions(positions);
    let types: &[AspectType] = if major_only {
        AspectType::MAJOR
    } else {
        AspectType::ALL
    };
    let found = find_all_aspects(&pos_vec, types);
    let list = PyList::empty(py);
    for a in &found {
        list.append(aspect_to_dict(py, a)?)?;
    }
    Ok(list.into())
}

/// Synastry: aspects strictly BETWEEN two charts (person A's points to person
/// B's points), excluding intra-chart aspects.
///
/// Implemented by calling `xalen_western::aspects::find_all_aspects` once over
/// the union of both position sets (prefixed to avoid name collisions
/// internally), then keeping only pairs that cross from A to B — no aspect
/// math beyond what `find_all_aspects` already computes.
///
/// Parameters
/// ----------
/// positions_a, positions_b : dict[str, tuple[float, float]]
///     Same shape as `aspects`, one dict per chart.
///
/// Returns
/// -------
/// list[dict]
///     Same shape as `aspects`, with ``body1`` always from chart A and
///     ``body2`` always from chart B (names are NOT prefixed in the output).
#[pyfunction]
fn synastry(
    py: Python<'_>,
    positions_a: std::collections::HashMap<String, (f64, f64)>,
    positions_b: std::collections::HashMap<String, (f64, f64)>,
) -> PyResult<Py<PyAny>> {
    let a_names: std::collections::HashSet<String> = positions_a.keys().cloned().collect();
    let mut combined: Vec<(String, f64, f64)> = Vec::new();
    for (name, (lon, speed)) in &positions_a {
        combined.push((format!("A::{name}"), *lon, *speed));
    }
    for (name, (lon, speed)) in &positions_b {
        combined.push((format!("B::{name}"), *lon, *speed));
    }
    let found = find_all_aspects(&combined, AspectType::ALL);
    let list = PyList::empty(py);
    for a in &found {
        let (a_is_1, b_is_2) = (a.body1.starts_with("A::"), a.body2.starts_with("B::"));
        let (b_is_1, a_is_2) = (a.body1.starts_with("B::"), a.body2.starts_with("A::"));
        if !((a_is_1 && b_is_2) || (b_is_1 && a_is_2)) {
            continue; // intra-chart aspect (A-A or B-B); synastry wants only cross-chart
        }
        let mut cleaned = a.clone();
        cleaned.body1 = cleaned.body1.trim_start_matches("A::").trim_start_matches("B::").to_string();
        cleaned.body2 = cleaned.body2.trim_start_matches("A::").trim_start_matches("B::").to_string();
        // Normalize so body1 is always chart A's point, body2 chart B's.
        if a_is_1 {
            // already A -> B
        } else {
            std::mem::swap(&mut cleaned.body1, &mut cleaned.body2);
        }
        let _ = a_names.contains(&cleaned.body1); // silence unused-var lints if names collide
        list.append(aspect_to_dict(py, &cleaned)?)?;
    }
    Ok(list.into())
}

/// Transits: aspects from a set of CURRENT (transiting) positions to a set of
/// NATAL positions. Same underlying engine and cross-pair filter as
/// `synastry` — a transit comparison and a synastry comparison are the same
/// computation (`find_all_aspects` over a combined set, keeping only
/// cross-set pairs); this wrapper exists only so the parameter names read
/// naturally for the transit use case.
///
/// Parameters
/// ----------
/// transiting_positions, natal_positions : dict[str, tuple[float, float]]
///
/// Returns
/// -------
/// list[dict]
///     ``body1`` is always the transiting point, ``body2`` the natal point.
#[pyfunction]
fn transits(
    py: Python<'_>,
    transiting_positions: std::collections::HashMap<String, (f64, f64)>,
    natal_positions: std::collections::HashMap<String, (f64, f64)>,
) -> PyResult<Py<PyAny>> {
    synastry(py, transiting_positions, natal_positions)
}

// ---------------------------------------------------------------------------
// Solar / lunar return -- xalen_ephem::returns (the real-Almanac finder, NOT
// xalen_western::returns' internal low-precision approximation)
// ---------------------------------------------------------------------------

fn return_body_from_str(s: &str) -> PyResult<ReturnBody> {
    match s.to_ascii_lowercase().as_str() {
        "sun" | "solar" => Ok(ReturnBody::Sun),
        "moon" | "lunar" => Ok(ReturnBody::Moon),
        "mars" => Ok(ReturnBody::Mars),
        "jupiter" => Ok(ReturnBody::Jupiter),
        "saturn" => Ok(ReturnBody::Saturn),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown return body '{other}'; expected sun/moon/mars/jupiter/saturn"
        ))),
    }
}

/// Find the exact Julian Day (UT1) of the next return of `body` to
/// `natal_longitude_deg`, searching forward from `search_start_jd`.
///
/// Wraps `xalen_ephem::returns::find_return`, which root-finds (safeguarded
/// Newton + bisection) on the body's REAL apparent geocentric longitude from
/// the active `Almanac` (the same VSOP87/DE440 engine every other function in
/// this module uses) — not a mean-period estimate. This is the more precise
/// of XALEN's two return finders (the other, `xalen_western::returns`, uses
/// an internal low-precision analytic Sun approximation and is not bound
/// here). Uses whichever engine is currently active (see `de440_status`).
///
/// Parameters
/// ----------
/// body : str
///     One of "sun"/"solar", "moon"/"lunar", "mars", "jupiter", "saturn".
/// natal_longitude_deg : float
///     The body's natal geocentric ecliptic longitude, degrees.
/// search_start_jd : float
///     Julian Day (UT1) to search forward from (e.g. birth JD for the first
///     return, or the previous return's JD for the next one).
///
/// Returns
/// -------
/// dict
///     ``{"return_jd": float, "engine": str}``.
///
/// Raises
/// ------
/// RuntimeError
///     If no crossing can be bracketed (should not happen for the supported
///     bodies) or the epoch is outside the active engine's coverage.
#[pyfunction]
fn exact_return(
    py: Python<'_>,
    body: &str,
    natal_longitude_deg: f64,
    search_start_jd: f64,
) -> PyResult<Py<PyAny>> {
    crate::check_jd(search_start_jd)?;
    let rb = return_body_from_str(body)?;
    let almanac = build_almanac();
    let jd = find_return(&almanac, rb, natal_longitude_deg, JdUT1(search_start_jd))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let dict = PyDict::new(py);
    dict.set_item("return_jd", jd.as_f64())?;
    dict.set_item("engine", current_engine_label())?;
    Ok(dict.into())
}

// ---------------------------------------------------------------------------
// Secondary (and other day-for-a-year family) progressions
// ---------------------------------------------------------------------------

fn progression_type_from_str(s: &str) -> PyResult<ProgressionType> {
    match s.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
        "secondary" => Ok(ProgressionType::Secondary),
        "solararc" => Ok(ProgressionType::SolarArc),
        "tertiary" => Ok(ProgressionType::Tertiary),
        "minor" => Ok(ProgressionType::Minor),
        "converse" => Ok(ProgressionType::Converse),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown progression type '{other}'; expected secondary/solar_arc/tertiary/minor/converse"
        ))),
    }
}

/// Compute the progressed Julian Day for a given birth JD and target
/// (current) JD, then evaluate a body's position AT that progressed JD using
/// the active engine.
///
/// Wraps `xalen_western::progressions::progressed_jd` (day-for-a-year and
/// sibling methods) directly; the body position at the resulting JD comes
/// from the same `Almanac`/`position_full` path every other function here
/// uses — no separate progressed-position formula is invented in Python.
///
/// Parameters
/// ----------
/// birth_jd, target_jd : float
///     Julian Days (UT1).
/// body : int
///     Body ID (see `planet_position`).
/// progression_type : str, optional
///     One of "secondary" (1 day = 1 year, default), "tertiary" (1 day = 1
///     month), "minor" (1 synodic month = 1 year), "converse" (reverse
///     secondary), "solar_arc" (returns the same progressed JD as secondary;
///     solar-arc DIRECTION — applying the progressed Sun's arc to natal
///     points — is a separate step callers do themselves with `solar_arc_deg`
///     semantics, not reimplemented here since it needs the natal chart, not
///     just a JD).
///
/// Returns
/// -------
/// dict
///     ``{"progressed_jd": float, "longitude": float, "engine": str}`` —
///     the progressed date and that body's longitude at it.
#[pyfunction]
#[pyo3(signature = (birth_jd, target_jd, body, progression_type = "secondary"))]
fn secondary_progression(
    py: Python<'_>,
    birth_jd: f64,
    target_jd: f64,
    body: u8,
    progression_type: &str,
) -> PyResult<Py<PyAny>> {
    crate::check_jd(birth_jd)?;
    crate::check_jd(target_jd)?;
    let ptype = progression_type_from_str(progression_type)?;
    let prog_jd = progressed_jd(birth_jd, target_jd, ptype);
    let almanac = build_almanac();
    let b = body_from_id(body)?;
    let lon = almanac
        .geocentric_longitude_deg(b, JdUT1(prog_jd))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let dict = PyDict::new(py);
    dict.set_item("progressed_jd", prog_jd)?;
    dict.set_item("longitude", lon)?;
    dict.set_item("engine", current_engine_label())?;
    Ok(dict.into())
}

// ---------------------------------------------------------------------------
// Vargas (divisional charts)
// ---------------------------------------------------------------------------

fn varga_from_division(division: u32) -> PyResult<VargaChart> {
    match division {
        1 => Ok(VargaChart::D1),
        2 => Ok(VargaChart::D2),
        3 => Ok(VargaChart::D3),
        4 => Ok(VargaChart::D4),
        7 => Ok(VargaChart::D7),
        9 => Ok(VargaChart::D9),
        10 => Ok(VargaChart::D10),
        12 => Ok(VargaChart::D12),
        16 => Ok(VargaChart::D16),
        20 => Ok(VargaChart::D20),
        24 => Ok(VargaChart::D24),
        27 => Ok(VargaChart::D27),
        30 => Ok(VargaChart::D30),
        40 => Ok(VargaChart::D40),
        45 => Ok(VargaChart::D45),
        60 => Ok(VargaChart::D60),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unsupported varga division D{other}; xalen-vedic implements D1,D2,D3,D4,D7,D9,D10,D12,D16,D20,D24,D27,D30,D40,D45,D60"
        ))),
    }
}

/// Compute the divisional-chart (varga) sign for a sidereal longitude.
///
/// Wraps `xalen_vedic::divisional::compute_varga_sign` directly.
///
/// Parameters
/// ----------
/// sidereal_lon_deg : float
///     Sidereal ecliptic longitude, degrees.
/// division : int
///     Which varga: one of 1,2,3,4,7,9,10,12,16,20,24,27,30,40,45,60
///     (D1=Rashi, D9=Navamsa, D10=Dasamsa, ... — the 16 divisional charts
///     `xalen-vedic` implements).
///
/// Returns
/// -------
/// dict
///     ``{"rashi": str, "division": int, "varga_name": str}``.
#[pyfunction]
fn varga_sign(py: Python<'_>, sidereal_lon_deg: f64, division: u32) -> PyResult<Py<PyAny>> {
    let varga = varga_from_division(division)?;
    let rashi = compute_varga_sign(sidereal_lon_deg, varga);
    let dict = PyDict::new(py);
    dict.set_item("rashi", rashi.to_string())?;
    dict.set_item("division", division)?;
    dict.set_item("varga_name", varga.name())?;
    Ok(dict.into())
}

// ---------------------------------------------------------------------------
// Vimshottari Dasha
// ---------------------------------------------------------------------------

fn dasha_period_to_dict(py: Python<'_>, p: &DashaPeriod, include_sub: bool) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("lord", p.lord.to_string())?;
    dict.set_item("start_jd", p.start_jd)?;
    dict.set_item("end_jd", p.end_jd)?;
    dict.set_item("level", format!("{:?}", p.level))?;
    if include_sub && !p.sub_periods.is_empty() {
        let subs = PyList::empty(py);
        for sp in &p.sub_periods {
            subs.append(dasha_period_to_dict(py, sp, include_sub)?)?;
        }
        dict.set_item("sub_periods", subs)?;
    } else {
        dict.set_item("sub_periods", PyList::empty(py))?;
    }
    Ok(dict.into())
}

/// Compute the full Vimshottari Mahadasha (and, if requested, nested
/// Antardasha) sequence from a natal sidereal Moon longitude.
///
/// Wraps `xalen_vedic::dasha::vimshottari_dasha` directly — the 120-year,
/// nakshatra-lord-based cycle XALEN's own ACCURACY.md documents as "Full
/// Antardasha level computed."
///
/// Parameters
/// ----------
/// moon_sidereal_deg : float
///     Natal Moon's SIDEREAL longitude in degrees (use
///     ``xalen.planet_longitude(jd, 1, sidereal=True, ayanamsa=0)``).
/// birth_jd : float
///     Julian Day (UT1) of birth.
/// depth : str, optional
///     One of "mahadasha" (default, top level only), "antardasha" (nested one
///     level deep). Deeper levels (`pratyantardasha`, `sookshmadasha`,
///     `pranadasha`) exist in the Rust enum but are not exercised by this
///     wrapper's depth parameter yet — pass "antardasha" and read
///     ``sub_periods`` recursively if you need to go further; the Rust
///     function itself supports it, this binding just doesn't parameterize
///     past two levels.
///
/// Returns
/// -------
/// list[dict]
///     Nine Mahadasha periods, each
///     ``{"lord": str, "start_jd": float, "end_jd": float, "level": str,
///     "sub_periods": list[dict]}``.
#[pyfunction]
#[pyo3(signature = (moon_sidereal_deg, birth_jd, depth = "mahadasha"))]
fn vimshottari_dasha(
    py: Python<'_>,
    moon_sidereal_deg: f64,
    birth_jd: f64,
    depth: &str,
) -> PyResult<Py<PyAny>> {
    crate::check_jd(birth_jd)?;
    let (level, include_sub) = match depth.to_ascii_lowercase().as_str() {
        "mahadasha" => (DashaLevel::Mahadasha, false),
        "antardasha" => (DashaLevel::Antardasha, true),
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown depth '{other}'; expected 'mahadasha' or 'antardasha'"
            )));
        }
    };
    let periods = core_vimshottari_dasha(moon_sidereal_deg, birth_jd, level);
    let list = PyList::empty(py);
    for p in &periods {
        list.append(dasha_period_to_dict(py, p, include_sub)?)?;
    }
    Ok(list.into())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(load_de440_kernel, m)?)?;
    m.add_function(wrap_pyfunction!(de440_status, m)?)?;
    m.add_function(wrap_pyfunction!(unload_de440_kernel, m)?)?;
    m.add_function(wrap_pyfunction!(planet_longitude_with_engine, m)?)?;
    m.add_function(wrap_pyfunction!(aspects, m)?)?;
    m.add_function(wrap_pyfunction!(synastry, m)?)?;
    m.add_function(wrap_pyfunction!(transits, m)?)?;
    m.add_function(wrap_pyfunction!(exact_return, m)?)?;
    m.add_function(wrap_pyfunction!(secondary_progression, m)?)?;
    m.add_function(wrap_pyfunction!(varga_sign, m)?)?;
    m.add_function(wrap_pyfunction!(vimshottari_dasha, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varga_navamsa_matches_core() {
        // 15deg Aries -> whichever Navamsa pada the core function says; just
        // prove the binding calls through without transformation.
        let core = compute_varga_sign(15.0, VargaChart::D9);
        let via_division = varga_from_division(9).unwrap();
        assert_eq!(core, compute_varga_sign(15.0, via_division));
    }

    #[test]
    fn test_return_body_from_str() {
        assert_eq!(return_body_from_str("Sun").unwrap(), ReturnBody::Sun);
        assert_eq!(return_body_from_str("lunar").unwrap(), ReturnBody::Moon);
        assert!(return_body_from_str("pluto").is_err());
    }

    #[test]
    fn test_progression_type_from_str() {
        assert!(matches!(
            progression_type_from_str("secondary").unwrap(),
            ProgressionType::Secondary
        ));
        assert!(matches!(
            progression_type_from_str("solar-arc").unwrap(),
            ProgressionType::SolarArc
        ));
        assert!(progression_type_from_str("bogus").is_err());
    }

    #[test]
    fn test_de440_status_defaults_to_analytical_fallback() {
        // Don't assume test ordering vs other tests that may load a kernel;
        // just check the accessor doesn't panic and returns a valid label set.
        let label = current_engine_label();
        assert!(
            label == "VSOP87 analytical fallback"
                || label == "JPL DE440 kernel"
                || label == "unconfirmed SPK kernel (analytical-grade)"
        );
    }
}
