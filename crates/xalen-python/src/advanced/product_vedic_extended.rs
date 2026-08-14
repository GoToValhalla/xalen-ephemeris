//! Additional Jyotish product bindings. Every calculation delegates to an
//! existing `xalen-vedic` implementation; this module only validates JSON
//! inputs and serialises deterministic Rust results for Python callers.

use std::collections::HashMap;

use pyo3::prelude::*;
use serde_json::{json, Value};
use xalen_coords::Planet;
use xalen_vedic::rashi::Rashi;

fn value_error(message: impl Into<String>) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(message.into())
}

fn payload(raw: &str) -> PyResult<Value> {
    serde_json::from_str(raw).map_err(|e| value_error(format!("invalid JSON payload: {e}")))
}

fn f64_field(value: &Value, key: &str) -> PyResult<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .ok_or_else(|| value_error(format!("missing finite numeric field '{key}'")))
}

fn int_field(value: &Value, key: &str) -> PyResult<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| value_error(format!("missing integer field '{key}'")))
}

fn string_field<'a>(value: &'a Value, key: &str) -> PyResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| value_error(format!("missing string field '{key}'")))
}

fn parse_planet(name: &str) -> PyResult<Planet> {
    match name.to_ascii_lowercase().replace([' ', '-', '_'], "").as_str() {
        "sun" => Ok(Planet::Sun),
        "moon" => Ok(Planet::Moon),
        "mars" => Ok(Planet::Mars),
        "mercury" => Ok(Planet::Mercury),
        "jupiter" => Ok(Planet::Jupiter),
        "venus" => Ok(Planet::Venus),
        "saturn" => Ok(Planet::Saturn),
        "rahu" | "northnode" => Ok(Planet::Rahu),
        "ketu" | "southnode" => Ok(Planet::Ketu),
        "uranus" => Ok(Planet::Uranus),
        "neptune" => Ok(Planet::Neptune),
        "pluto" => Ok(Planet::Pluto),
        _ => Err(value_error(format!("unknown planet '{name}'"))),
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(|e| value_error(format!("serialization failed: {e}")))
}

/// Product bindings for Jyotish techniques not present in the legacy Python
/// API. `mode` selects a Rust technique and `payload_json` supplies only its
/// canonical numerical inputs.
#[pyfunction]
fn product_vedic_extended_json(mode: &str, payload_json: &str) -> PyResult<String> {
    let value = payload(payload_json)?;
    match mode {
        "pushkara" => to_json(&xalen_vedic::pushkara::pushkara_info(f64_field(&value, "degree")?)),
        "mrityu_bhaga" => to_json(&xalen_vedic::mrityu_bhaga::mrityu_bhaga_info(
            string_field(&value, "planet")?,
            f64_field(&value, "degree")?,
        )),
        "upagraha" => {
            let rows: Vec<Value> = xalen_vedic::upagraha::Upagraha::from_sun(f64_field(&value, "sun_sidereal")?)
                .into_iter()
                .map(|(point, longitude)| json!({"point": format!("{point:?}"), "longitude": longitude}))
                .collect();
            to_json(&rows)
        }
        "nadi" => {
            let planet = parse_planet(string_field(&value, "planet")?)?;
            let sign = int_field(&value, "sign")?.rem_euclid(12) as usize;
            to_json(&xalen_vedic::nadi::nadi_indications(planet, sign))
        }
        "sudarshana" => to_json(&xalen_vedic::sudarshana::compute_sudarshana(
            int_field(&value, "lagna_sign")?.rem_euclid(12) as usize,
            int_field(&value, "moon_sign")?.rem_euclid(12) as usize,
            int_field(&value, "sun_sign")?.rem_euclid(12) as usize,
            int_field(&value, "age")?.max(0) as u32,
        )),
        "varshaphal" => to_json(&xalen_vedic::varshaphal::compute_varshaphal(
            f64_field(&value, "birth_jd")?,
            int_field(&value, "natal_asc_sign")?.rem_euclid(12) as usize,
            int_field(&value, "year")?.max(0) as u32,
        )),
        "tajaka" => {
            let asc = f64_field(&value, "asc")?;
            let sun = f64_field(&value, "sun")?;
            let moon = f64_field(&value, "moon")?;
            let age = int_field(&value, "age")?.max(0) as u32;
            let asc_sign = int_field(&value, "asc_sign")?.rem_euclid(12) as usize;
            let is_day = value.get("is_day").and_then(Value::as_bool).unwrap_or(true);
            let mut owned: HashMap<String, f64> = HashMap::new();
            if let Some(rows) = value.get("positions").and_then(Value::as_array) {
                for row in rows {
                    let name = row
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(|| value_error("Tajaka position is missing name"))?;
                    let longitude = row
                        .get("longitude")
                        .and_then(Value::as_f64)
                        .filter(|v| v.is_finite())
                        .ok_or_else(|| value_error("Tajaka position has invalid longitude"))?;
                    owned.insert(name.to_owned(), longitude);
                }
            }
            let borrowed: HashMap<&str, f64> = owned.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            let muntha = xalen_vedic::tajaka::compute_muntha(asc_sign, age);
            to_json(&json!({
                "muntha_sign": muntha,
                "muntha_lord": xalen_vedic::tajaka::muntha_lord(muntha),
                "sahams": xalen_vedic::tajaka::compute_sahams(asc, sun, moon, is_day, &borrowed),
            }))
        }
        "chara_dasha" => {
            let lagna = Rashi::from_index(int_field(&value, "lagna_sign")?.rem_euclid(12) as usize);
            let birth_jd = f64_field(&value, "birth_jd")?;
            let raw = value
                .get("lord_positions")
                .and_then(Value::as_array)
                .ok_or_else(|| value_error("missing lord_positions array"))?;
            let mut positions = Vec::with_capacity(raw.len());
            for pair in raw {
                let parts = pair
                    .as_array()
                    .filter(|p| p.len() == 2)
                    .ok_or_else(|| value_error("lord_positions entries must be [sign, lord_sign]"))?;
                let sign = parts[0]
                    .as_i64()
                    .ok_or_else(|| value_error("invalid sign in lord_positions"))?;
                let lord_sign = parts[1]
                    .as_i64()
                    .ok_or_else(|| value_error("invalid lord sign in lord_positions"))?;
                positions.push((
                    Rashi::from_index(sign.rem_euclid(12) as usize),
                    Rashi::from_index(lord_sign.rem_euclid(12) as usize),
                ));
            }
            to_json(&xalen_vedic::chara_dasha::compute_chara_dasha(lagna, &positions, birth_jd))
        }
        "sthira_dasha" => to_json(&xalen_vedic::chara_dasha::compute_sthira_dasha(
            Rashi::from_index(int_field(&value, "lagna_sign")?.rem_euclid(12) as usize),
            f64_field(&value, "birth_jd")?,
        )),
        other => Err(value_error(format!("unknown extended Vedic product mode '{other}'"))),
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(product_vedic_extended_json, m)?)?;
    Ok(())
}
