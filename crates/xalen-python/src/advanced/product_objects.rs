//! Product bindings for astrologically useful special objects.

use pyo3::prelude::*;
use serde_json::json;
use xalen_ephem::asteroids::{all_extended_positions, is_retrograde};
use xalen_time::{JdTT, JulianDay};

fn value_error(message: impl Into<String>) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(message.into())
}

#[pyfunction]
fn product_special_objects_json(jd: f64) -> PyResult<String> {
    crate::check_jd(jd)?;
    let jd_tt = JdTT(jd);
    let mean_lilith = xalen_ephem::lilith::mean_lilith(jd_tt).to_degrees().rem_euclid(360.0);
    let true_lilith = xalen_ephem::lilith::true_lilith(jd_tt)
        .map_err(|e| value_error(format!("true Lilith calculation failed: {e}")))?
        .to_degrees()
        .rem_euclid(360.0);
    let asteroids = all_extended_positions(jd_tt)
        .map_err(|e| value_error(format!("asteroid calculation failed: {e}")))?
        .into_iter()
        .map(|(asteroid, position)| {
            let retrograde = is_retrograde(asteroid, jd_tt).unwrap_or(false);
            json!({
                "name": asteroid.name(),
                "number": asteroid.number(),
                "category": asteroid.category(),
                "longitude_deg": position.longitude.to_degrees().rem_euclid(360.0),
                "latitude_deg": position.latitude.to_degrees(),
                "distance_au": position.distance,
                "retrograde": retrograde,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&json!({
        "jd": jd_tt.as_f64(),
        "black_moon_lilith": {
            "mean_longitude_deg": mean_lilith,
            "true_longitude_deg": true_lilith,
            "priapus_longitude_deg": (mean_lilith + 180.0).rem_euclid(360.0),
        },
        "asteroids": asteroids,
    }))
    .map_err(|e| value_error(format!("special-object serialization failed: {e}")))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(product_special_objects_json, m)?)?;
    Ok(())
}
