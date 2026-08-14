//! Product astronomical helpers backed by XALEN ephemeris primitives.

use pyo3::prelude::*;
use serde_json::json;
use xalen_ephem::{Almanac, Body};
use xalen_time::{JdUT1, JulianDay};

fn value_error(message: impl Into<String>) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(message.into())
}

#[pyfunction]
fn product_solar_day_json(jd_start: f64, latitude: f64, longitude: f64, elevation_m: f64) -> PyResult<String> {
    crate::check_jd(jd_start)?;
    if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
        return Err(value_error("latitude must be finite and in -90..=90"));
    }
    if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
        return Err(value_error("longitude must be finite and in -180..=180"));
    }
    if !elevation_m.is_finite() {
        return Err(value_error("elevation_m must be finite"));
    }
    let almanac = Almanac::default_vedic();
    let result = xalen_ephem::rise_set::compute(
        &almanac,
        Body::Sun,
        JdUT1(jd_start),
        latitude,
        longitude,
        elevation_m,
    )
    .map_err(|e| value_error(format!("solar rise/set calculation failed: {e}")))?;
    serde_json::to_string(&json!({
        "jd_start": jd_start,
        "rise_jd": result.rise.map(|v| v.as_f64()),
        "transit_jd": result.transit.map(|v| v.as_f64()),
        "set_jd": result.set.map(|v| v.as_f64()),
        "transit_altitude_deg": result.transit_altitude_deg,
        "always_above": result.always_above,
        "always_below": result.always_below,
    }))
    .map_err(|e| value_error(format!("solar day serialization failed: {e}")))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(product_solar_day_json, m)?)?;
    Ok(())
}
