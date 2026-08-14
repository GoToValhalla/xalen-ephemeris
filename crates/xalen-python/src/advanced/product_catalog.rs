//! Catalog-consistent product bindings for house systems and ayanamsas.
//!
//! `product_capabilities_json()` is a runtime contract: every name it advertises
//! must be accepted by the product calculation surface. The legacy ergonomic
//! `houses_by_name` / `ayanamsa_by_name` parsers predate the expanded catalog
//! and intentionally remain backwards compatible; these product bindings are
//! the canonical catalog-facing entry points.

use pyo3::prelude::*;
use serde_json::json;

use xalen_ayanamsa::Ayanamsa;
use xalen_coords::obliquity::mean_obliquity;
use xalen_houses::{compute_houses, GeoLocation, HouseSystem};
use xalen_time::{DeltaTModel, JdUT1, JulianDay};

fn value_error(message: impl Into<String>) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(message.into())
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !matches!(ch, '-' | '_' | ' ' | '(' | ')' | '/'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn validate_jd(jd: f64) -> PyResult<()> {
    if jd.is_finite() {
        Ok(())
    } else {
        Err(value_error("jd must be finite"))
    }
}

fn validate_geo(jd: f64, latitude: f64, longitude: f64) -> PyResult<()> {
    validate_jd(jd)?;
    if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
        return Err(value_error("latitude must be finite and within [-90, 90]"));
    }
    if !longitude.is_finite() {
        return Err(value_error("longitude must be finite"));
    }
    Ok(())
}

fn house_system_from_catalog_name(name: &str) -> PyResult<HouseSystem> {
    match normalize_name(name).as_str() {
        "wholesign" => Ok(HouseSystem::WholeSign),
        "equal" => Ok(HouseSystem::Equal),
        "placidus" => Ok(HouseSystem::Placidus),
        "koch" => Ok(HouseSystem::Koch),
        "porphyry" => Ok(HouseSystem::Porphyry),
        "regiomontanus" => Ok(HouseSystem::Regiomontanus),
        "campanus" => Ok(HouseSystem::Campanus),
        "morinus" => Ok(HouseSystem::Morinus),
        "alcabitius" => Ok(HouseSystem::Alcabitius),
        "topocentric" | "topocentricpolichpage" => Ok(HouseSystem::Topocentric),
        "meridian" => Ok(HouseSystem::Meridian),
        "vehlow" | "vehlowequal" => Ok(HouseSystem::Vehlow),
        "sripati" => Ok(HouseSystem::Sripati),
        "krusinskipisa" => Ok(HouseSystem::KrusinskiPisa),
        "gauquelin" | "gauquelinsectors" => Ok(HouseSystem::Gauquelin),
        "sunshinemakransky" => Ok(HouseSystem::SunshineMakransky),
        "sunshinetreindl" => Ok(HouseSystem::SunshineTreindl),
        "pullensinusoidaldelta" => Ok(HouseSystem::PullenSinusoidalDelta),
        "pullensinusoidalratio" => Ok(HouseSystem::PullenSinusoidalRatio),
        "carterpoliequatorial" => Ok(HouseSystem::CarterPoliEquatorial),
        "apc" | "apcascendantparallelcircle" => Ok(HouseSystem::APC),
        "zariel" | "axialrotationzariel" => Ok(HouseSystem::Zariel),
        "alcabitiusclassic" => Ok(HouseSystem::AlcabitiusClassic),
        other => Err(value_error(format!("unknown product house system '{other}'"))),
    }
}

fn ayanamsa_from_catalog_name(name: &str) -> PyResult<Ayanamsa> {
    let normalized = normalize_name(name);

    // The catalog itself is generated from `Ayanamsa::all_named()` Debug names.
    // Resolve those names dynamically so adding another named ayanamsa cannot
    // silently make the catalog broader than the product parser again.
    if let Some(value) = Ayanamsa::all_named()
        .iter()
        .find(|candidate| normalize_name(&format!("{candidate:?}")) == normalized)
    {
        return Ok(*value);
    }

    // Human-friendly aliases retained for callers that do not use catalog names.
    match normalized.as_str() {
        "kp" | "krishnamurti" => Ok(Ayanamsa::KPKrishnamurti),
        "chitrapaksha" => Ok(Ayanamsa::TrueChitra),
        "yukteswar" => Ok(Ayanamsa::YukteswarSriSS),
        "galacticcenter" => Ok(Ayanamsa::GalacticCenter0Sag),
        other => Err(value_error(format!("unknown product ayanamsa '{other}'"))),
    }
}

#[pyfunction]
fn product_houses_json(jd: f64, latitude: f64, longitude: f64, system: &str) -> PyResult<String> {
    validate_geo(jd, latitude, longitude)?;
    let system_value = house_system_from_catalog_name(system)?;
    let location = GeoLocation::new(latitude, longitude);
    let t = (jd - 2_451_545.0) / 36_525.0;
    let houses = compute_houses(jd, &location, mean_obliquity(t), system_value);
    let cusps: Vec<f64> = houses
        .cusps
        .iter()
        .map(|value| value.to_degrees().rem_euclid(360.0))
        .collect();
    serde_json::to_string(&json!({
        "system": system,
        "cusps": cusps,
        "ascendant": houses.ascendant.to_degrees().rem_euclid(360.0),
        "mc": houses.mc.to_degrees().rem_euclid(360.0),
        "ic": houses.ic.to_degrees().rem_euclid(360.0),
        "descendant": houses.descendant.to_degrees().rem_euclid(360.0),
        "vertex": houses.vertex.to_degrees().rem_euclid(360.0),
    }))
    .map_err(|error| value_error(format!("house result serialization failed: {error}")))
}

#[pyfunction]
fn product_ayanamsa_json(jd: f64, system: &str) -> PyResult<String> {
    validate_jd(jd)?;
    let ayanamsa = ayanamsa_from_catalog_name(system)?;
    let tt = JdUT1(jd).to_tt(&DeltaTModel::StephensonMorrisonHohenkerk2016);
    let value = ayanamsa.compute_deg(tt.as_f64());
    serde_json::to_string(&json!({
        "system": system,
        "value_deg": value,
    }))
    .map_err(|error| value_error(format!("ayanamsa result serialization failed: {error}")))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(product_houses_json, module)?)?;
    module.add_function(wrap_pyfunction!(product_ayanamsa_json, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_product_house_names_parse() {
        for name in [
            "WholeSign", "Equal", "Placidus", "Koch", "Porphyry", "Regiomontanus",
            "Campanus", "Morinus", "Alcabitius", "Topocentric", "Meridian", "Vehlow",
            "Sripati", "KrusinskiPisa", "Gauquelin", "SunshineMakransky",
            "SunshineTreindl", "PullenSinusoidalDelta", "PullenSinusoidalRatio",
            "CarterPoliEquatorial", "APC", "Zariel", "AlcabitiusClassic",
        ] {
            assert!(house_system_from_catalog_name(name).is_ok(), "catalog house name {name}");
        }
    }

    #[test]
    fn every_named_ayanamsa_debug_name_parses() {
        for value in Ayanamsa::all_named() {
            let name = format!("{value:?}");
            assert!(ayanamsa_from_catalog_name(&name).is_ok(), "catalog ayanamsa {name}");
        }
    }
}
