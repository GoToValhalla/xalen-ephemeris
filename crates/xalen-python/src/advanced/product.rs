//! Product-oriented PyO3 bindings for XALEN capabilities that are useful to
//! interactive applications but were not part of the original compact Python
//! surface.  This module is intentionally thin: every calculation delegates to
//! an existing Rust implementation and only normalises inputs / serialises
//! deterministic facts for Python callers.

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use xalen_ayanamsa::Ayanamsa;
use xalen_chinese::{EarthlyBranch, HeavenlyStem};
use xalen_coords::{ecliptic_to_equatorial, gast_deg, mean_obliquity, nutation_2000b, Planet};
use xalen_ephem::{Almanac, Body};
use xalen_time::JdUT1;

#[derive(Debug, Deserialize)]
struct PositionInput {
    name: String,
    longitude: f64,
}

fn py_value_err(message: impl Into<String>) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(message.into())
}

fn to_json<T: Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(|e| py_value_err(format!("serialization failed: {e}")))
}

fn parse_payload(payload_json: &str) -> PyResult<Value> {
    serde_json::from_str(payload_json)
        .map_err(|e| py_value_err(format!("payload_json must be valid JSON: {e}")))
}

fn required_f64(v: &Value, key: &str) -> PyResult<f64> {
    let value = v
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| py_value_err(format!("missing numeric field '{key}'")))?;
    if !value.is_finite() {
        return Err(py_value_err(format!("field '{key}' must be finite")));
    }
    Ok(value)
}

fn optional_f64(v: &Value, key: &str, default: f64) -> PyResult<f64> {
    match v.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_f64()
            .filter(|x| x.is_finite())
            .ok_or_else(|| py_value_err(format!("field '{key}' must be finite"))),
    }
}

fn required_i64(v: &Value, key: &str) -> PyResult<i64> {
    v.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| py_value_err(format!("missing integer field '{key}'")))
}

fn optional_bool(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn positions_from_value(v: &Value) -> PyResult<Vec<PositionInput>> {
    let raw = v
        .get("positions")
        .ok_or_else(|| py_value_err("missing 'positions' array"))?;
    let positions: Vec<PositionInput> = serde_json::from_value(raw.clone())
        .map_err(|e| py_value_err(format!("invalid positions: {e}")))?;
    if positions.is_empty() {
        return Err(py_value_err("positions must not be empty"));
    }
    if positions.iter().any(|p| !p.longitude.is_finite()) {
        return Err(py_value_err("all position longitudes must be finite"));
    }
    Ok(positions)
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
        _ => Err(py_value_err(format!("unknown planet '{name}'"))),
    }
}

fn planet_positions_from_value(v: &Value, key: &str) -> PyResult<Vec<(Planet, f64)>> {
    let raw = v
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| py_value_err(format!("missing array field '{key}'")))?;
    raw.iter()
        .map(|item| {
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| py_value_err(format!("{key} entry is missing name")))?;
            let longitude = item
                .get("longitude")
                .and_then(Value::as_f64)
                .filter(|x| x.is_finite())
                .ok_or_else(|| py_value_err(format!("{key} entry has invalid longitude")))?;
            Ok((parse_planet(name)?, longitude))
        })
        .collect()
}

/// Machine-readable catalogue of the broad calculation families exposed by
/// the product binding layer.  Low-level time/coordinate primitives remain
/// internal and are intentionally not advertised as end-user features.
#[pyfunction]
fn product_capabilities_json() -> PyResult<String> {
    let house_systems = [
        "WholeSign",
        "Equal",
        "Placidus",
        "Koch",
        "Porphyry",
        "Regiomontanus",
        "Campanus",
        "Morinus",
        "Alcabitius",
        "Topocentric",
        "Meridian",
        "Vehlow",
        "Sripati",
        "KrusinskiPisa",
        "Gauquelin",
        "SunshineMakransky",
        "SunshineTreindl",
        "PullenSinusoidalDelta",
        "PullenSinusoidalRatio",
        "CarterPoliEquatorial",
        "APC",
        "Zariel",
        "AlcabitiusClassic",
    ];
    let ayanamsas: Vec<String> = Ayanamsa::all_named()
        .iter()
        .map(|a| format!("{a:?}"))
        .collect();
    to_json(&json!({
        "surface_version": "product-v1",
        "house_systems": house_systems,
        "ayanamsas": ayanamsas,
        "families": {
            "western": ["antiscia", "harmonics", "midpoints", "cosmobiology", "uranian", "sabian", "decennials", "firdaria", "electional", "horary"],
            "vedic": ["jaimini", "kp", "gandanta", "muhurta", "prashna"],
            "chinese": ["bazi", "ziwei", "qimen", "flying_star", "ba_zhai"],
            "world": ["mayan", "aztec", "tibetan", "saju", "nine_star_ki", "mahabote", "persian", "egyptian", "celtic"],
            "other": ["iching"],
            "astronomy": ["equatorial_position"]
        }
    }))
}

/// Equatorial position and Greenwich apparent sidereal time for a body.
/// The ephemeris position comes from XALEN; this is the astronomical primitive
/// used by application-level astrocartography instead of a second ephemeris.
#[pyfunction]
fn product_equatorial_json(body: &str, jd: f64) -> PyResult<String> {
    crate::check_jd(jd)?;
    let body: Body = crate::parse_body(body)?;
    let almanac = Almanac::default_vedic();
    let ecl = almanac
        .geocentric_ecliptic(body, JdUT1(jd))
        .map_err(|e| py_value_err(format!("ephemeris failed: {e}")))?;
    let t_tt = (jd - 2_451_545.0) / 36_525.0;
    let nutation = nutation_2000b(t_tt);
    let epsilon_true = mean_obliquity(t_tt) + nutation.delta_epsilon;
    let eq = ecliptic_to_equatorial(&ecl, epsilon_true);
    to_json(&json!({
        "body": body.to_string(),
        "jd": jd,
        "right_ascension_deg": eq.right_ascension.to_degrees().rem_euclid(360.0),
        "right_ascension_hours": eq.ra_hours(),
        "declination_deg": eq.dec_deg(),
        "distance_au": eq.distance,
        "gast_deg": gast_deg(jd, t_tt),
    }))
}

/// Product-oriented Western calculations that consume deterministic chart
/// positions. Input and output are JSON so Python can preserve the exact Rust
/// result schema without reimplementing the underlying techniques.
#[pyfunction]
fn product_western_json(mode: &str, payload_json: &str) -> PyResult<String> {
    let payload = parse_payload(payload_json)?;
    match mode {
        "antiscia" | "harmonics" | "midpoints" | "cosmobiology" | "uranian_pictures" => {
            let positions = positions_from_value(&payload)?;
            let borrowed: Vec<(&str, f64)> = positions
                .iter()
                .map(|p| (p.name.as_str(), p.longitude))
                .collect();
            match mode {
                "antiscia" => {
                    let orb = optional_f64(&payload, "orb", 1.0)?;
                    to_json(&xalen_western::antiscia::detect_antiscia(&borrowed, orb))
                }
                "harmonics" => {
                    let harmonic = required_i64(&payload, "harmonic")?;
                    if harmonic <= 0 || harmonic > u32::MAX as i64 {
                        return Err(py_value_err("harmonic must be in 1..=u32::MAX"));
                    }
                    to_json(&xalen_western::harmonics::compute_harmonic_chart(
                        &borrowed,
                        harmonic as u32,
                    ))
                }
                "midpoints" => {
                    let orb = optional_f64(&payload, "orb", 1.5)?;
                    to_json(&xalen_western::midpoints::find_activations(&borrowed, orb))
                }
                "cosmobiology" => {
                    let orb = optional_f64(&payload, "orb", 1.0)?;
                    to_json(&xalen_western::cosmobiology::cosmobiology_chart(&borrowed, orb))
                }
                _ => {
                    let orb = optional_f64(&payload, "orb", 1.0)?;
                    to_json(&xalen_western::uranian::scan_planetary_pictures(&borrowed, orb))
                }
            }
        }
        "uranian" => {
            let jd = required_f64(&payload, "jd")?;
            crate::check_jd(jd)?;
            let rows: Vec<Value> = xalen_western::uranian::all_tnp_longitudes(jd)
                .iter()
                .map(|(body, longitude)| json!({
                    "body": body.name(),
                    "abbrev": body.abbrev(),
                    "longitude": longitude,
                    "period_years": body.period_years(),
                    "semi_major_axis_au": body.semi_major_axis_au(),
                }))
                .collect();
            to_json(&rows)
        }
        "sabian" => {
            let degree = required_f64(&payload, "degree")?;
            let symbol = xalen_western::sabian::sabian_for_degree(degree);
            to_json(&symbol)
        }
        "decennials" => {
            let birth_jd = required_f64(&payload, "birth_jd")?;
            crate::check_jd(birth_jd)?;
            let is_day = optional_bool(&payload, "is_day", true);
            let years = required_i64(&payload, "years")?;
            if years <= 0 {
                return Err(py_value_err("years must be positive"));
            }
            to_json(&xalen_western::hellenistic::compute_decennials(
                birth_jd,
                is_day,
                years as usize,
            ))
        }
        "firdaria" => {
            let birth_jd = required_f64(&payload, "birth_jd")?;
            crate::check_jd(birth_jd)?;
            let is_day = optional_bool(&payload, "is_day", true);
            to_json(&xalen_western::hellenistic::compute_firdaria(birth_jd, is_day))
        }
        "electional" => {
            let moon_lon = required_f64(&payload, "moon_lon")?;
            let moon_speed = required_f64(&payload, "moon_speed")?;
            let jd = required_f64(&payload, "jd")?;
            let sunrise_jd = required_f64(&payload, "sunrise_jd")?;
            let sunset_jd = required_f64(&payload, "sunset_jd")?;
            let positions = positions_from_value(&payload)?;
            let borrowed: Vec<(&str, f64)> = positions
                .iter()
                .map(|p| (p.name.as_str(), p.longitude))
                .collect();
            let cusp_values = payload
                .get("cusps")
                .and_then(Value::as_array)
                .ok_or_else(|| py_value_err("missing 'cusps' array"))?;
            if cusp_values.len() != 12 {
                return Err(py_value_err("cusps must contain exactly 12 degrees"));
            }
            let mut cusps = [0.0_f64; 12];
            for (i, value) in cusp_values.iter().enumerate() {
                cusps[i] = value
                    .as_f64()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| py_value_err(format!("cusps[{i}] must be finite")))?;
            }
            to_json(&xalen_western::electional::score_election(
                moon_lon,
                moon_speed,
                &borrowed,
                &cusps,
                jd,
                sunrise_jd,
                sunset_jd,
            ))
        }
        "horary" => {
            let asc = required_f64(&payload, "asc")?;
            let moon_lon = required_f64(&payload, "moon_lon")?;
            let moon_speed = required_f64(&payload, "moon_speed")?;
            let saturn_lon = required_f64(&payload, "saturn_lon")?;
            let positions = positions_from_value(&payload)?;
            let borrowed: Vec<(&str, f64)> = positions
                .iter()
                .map(|p| (p.name.as_str(), p.longitude))
                .collect();
            let cusp_values = payload
                .get("cusps")
                .and_then(Value::as_array)
                .ok_or_else(|| py_value_err("missing 'cusps' array"))?;
            if cusp_values.len() != 12 {
                return Err(py_value_err("cusps must contain exactly 12 degrees"));
            }
            let mut cusps = [0.0_f64; 12];
            for (i, value) in cusp_values.iter().enumerate() {
                cusps[i] = value
                    .as_f64()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| py_value_err(format!("cusps[{i}] must be finite")))?;
            }
            to_json(&xalen_western::horary::considerations(
                asc,
                moon_lon,
                moon_speed,
                saturn_lon,
                &cusps,
                &borrowed,
            ))
        }
        other => Err(py_value_err(format!("unknown Western product mode '{other}'"))),
    }
}

/// Additional Jyotish capabilities that were previously Rust-only.
#[pyfunction]
fn product_vedic_json(mode: &str, payload_json: &str) -> PyResult<String> {
    let payload = parse_payload(payload_json)?;
    match mode {
        "jaimini" => {
            let positions = positions_from_value(&payload)?;
            let borrowed: Vec<(&str, f64)> = positions
                .iter()
                .map(|p| (p.name.as_str(), p.longitude))
                .collect();
            to_json(&xalen_vedic::jaimini::assign_chara_karakas(&borrowed))
        }
        "kp" => {
            let degree = required_f64(&payload, "degree")?;
            to_json(&xalen_vedic::kp::kp_position(degree))
        }
        "gandanta" => {
            let degree = required_f64(&payload, "degree")?;
            to_json(&xalen_vedic::gandanta::gandanta_info(degree))
        }
        "muhurta" => {
            let sunrise = required_f64(&payload, "sunrise_jd")?;
            let sunset = required_f64(&payload, "sunset_jd")?;
            let next_sunrise = required_f64(&payload, "next_sunrise_jd")?;
            let weekday = required_i64(&payload, "weekday")?.rem_euclid(7) as usize;
            let query_jd = optional_f64(&payload, "query_jd", sunrise)?;
            let choghadiya = xalen_vedic::muhurta::compute_choghadiya(
                sunrise,
                sunset,
                next_sunrise,
                weekday,
            );
            let rahu = xalen_vedic::muhurta::rahu_kalam(sunrise, sunset, weekday);
            let yamagandam = xalen_vedic::muhurta::yamagandam(sunrise, sunset, weekday);
            let gulika = xalen_vedic::muhurta::gulika_kalam(sunrise, sunset, weekday);
            let abhijit = xalen_vedic::muhurta::abhijit_muhurta(sunrise, sunset);
            let brahma = xalen_vedic::muhurta::brahma_muhurta(sunrise);
            let hora = xalen_vedic::muhurta::planetary_hora(
                sunrise,
                sunset,
                next_sunrise,
                weekday,
                query_jd,
            );
            to_json(&json!({
                "choghadiya": choghadiya,
                "rahu_kalam": rahu,
                "yamagandam": yamagandam,
                "gulika_kalam": gulika,
                "abhijit_muhurta": {"start_jd": abhijit.0, "end_jd": abhijit.1},
                "brahma_muhurta": {"start_jd": brahma.0, "end_jd": brahma.1},
                "planetary_hora": hora,
                "disha_shool": format!("{:?}", xalen_vedic::muhurta::disha_shool(weekday)),
            }))
        }
        "prashna" => {
            let query_jd = required_f64(&payload, "query_jd")?;
            let asc_sidereal = required_f64(&payload, "asc_sidereal")?;
            let moon_sidereal = required_f64(&payload, "moon_sidereal")?;
            let arudha = required_i64(&payload, "arudha")?.clamp(1, 12) as u8;
            let planets = if payload.get("planet_positions").is_some() {
                planet_positions_from_value(&payload, "planet_positions")?
            } else {
                Vec::new()
            };
            let chart = xalen_vedic::prashna::PrashnaChart {
                query_jd,
                asc_sidereal,
                moon_sidereal,
                planet_positions: planets,
            };
            to_json(&xalen_vedic::prashna::analyze_prashna(&chart, arudha))
        }
        other => Err(py_value_err(format!("unknown Vedic product mode '{other}'"))),
    }
}

/// Chinese metaphysics calculations from `xalen-chinese`.
#[pyfunction]
fn product_chinese_json(system: &str, payload_json: &str) -> PyResult<String> {
    let payload = parse_payload(payload_json)?;
    match system {
        "bazi" => {
            let year = required_i64(&payload, "year")? as i32;
            let jd = required_f64(&payload, "jd")?;
            let hour = required_f64(&payload, "hour")?;
            to_json(&xalen_chinese::compute_bazi(year, jd, hour))
        }
        "ziwei" => {
            let year_stem_index = required_i64(&payload, "year_stem_index")?.rem_euclid(10) as usize;
            let lunar_month = required_i64(&payload, "lunar_month")?.clamp(1, 12) as u32;
            let lunar_day = required_i64(&payload, "lunar_day")?.clamp(1, 30) as u32;
            let hour_branch_index = required_i64(&payload, "hour_branch_index")?.rem_euclid(12) as usize;
            let chart = xalen_chinese::ziwei::compute_chart(
                HeavenlyStem::from_index(year_stem_index),
                lunar_month,
                lunar_day,
                EarthlyBranch::from_index(hour_branch_index),
            );
            to_json(&chart)
        }
        "qimen" => {
            let year = required_i64(&payload, "year")? as i32;
            let month = required_i64(&payload, "month")?.clamp(1, 12) as u32;
            let day = required_i64(&payload, "day")?.clamp(1, 31) as u32;
            let hour = required_i64(&payload, "hour")?.clamp(0, 23) as u32;
            to_json(&xalen_chinese::qimen::compute_qimen(year, month, day, hour))
        }
        "flying_star" => {
            let year = required_i64(&payload, "year")? as i32;
            to_json(&xalen_chinese::fengshui::flying_star_chart(year))
        }
        "ba_zhai" => {
            let year = required_i64(&payload, "year")? as i32;
            let is_male = optional_bool(&payload, "is_male", false);
            to_json(&xalen_chinese::fengshui::ba_zhai(year, is_male))
        }
        other => Err(py_value_err(format!("unknown Chinese product system '{other}'"))),
    }
}

/// I Ching date-based reading from the compiled public-domain Legge corpus.
#[pyfunction]
fn product_iching_json(year: i32, month: u32, day: u32, hour: u32) -> PyResult<String> {
    to_json(&xalen_iching::hexagram_from_date(year, month, day, hour))
}

/// World-tradition calculations. Approximate systems remain approximate in the
/// returned facts; this wrapper deliberately does not upgrade their maturity.
#[pyfunction]
fn product_world_json(system: &str, payload_json: &str) -> PyResult<String> {
    let payload = parse_payload(payload_json)?;
    match system {
        "mayan" => {
            let jd = required_f64(&payload, "jd")?;
            to_json(&xalen_world::mayan::calendar_round(jd))
        }
        "aztec" => {
            let year = required_i64(&payload, "year")? as i32;
            let month = required_i64(&payload, "month")?.clamp(1, 12) as u32;
            let day = required_i64(&payload, "day")?.clamp(1, 31) as u32;
            to_json(&xalen_world::aztec::tonalpohualli_from_date(year, month, day))
        }
        "tibetan" => {
            let year = required_i64(&payload, "year")? as i32;
            to_json(&xalen_world::tibetan::tibetan_year(year))
        }
        "saju" => {
            let year = required_i64(&payload, "year")? as i32;
            let month = required_i64(&payload, "month")?.clamp(1, 12) as u32;
            let day = required_i64(&payload, "day")?.clamp(1, 31) as u32;
            let hour = required_i64(&payload, "hour")?.clamp(0, 23) as u32;
            to_json(&xalen_world::saju::compute_saju(year, month, day, hour))
        }
        "nine_star_ki" => {
            let year = required_i64(&payload, "year")? as i32;
            let month = required_i64(&payload, "month")?.clamp(1, 12) as u32;
            to_json(&xalen_world::nine_star_ki::nine_star_ki(year, month))
        }
        "mahabote" => {
            let jd = required_f64(&payload, "jd")?;
            let profile = xalen_world::mahabote::mahabote_from_jd(jd);
            let square = xalen_world::mahabote::mahabote_house_square_from_jd(jd);
            to_json(&json!({"profile": profile, "house_square": square}))
        }
        "persian" => {
            let birth_year = required_f64(&payload, "birth_year")?;
            let starting_planet = payload
                .get("starting_planet")
                .and_then(Value::as_str)
                .ok_or_else(|| py_value_err("missing string field 'starting_planet'"))?;
            to_json(&xalen_world::persian::compute_jarbakhtar(birth_year, starting_planet))
        }
        "egyptian" => {
            let degree = required_f64(&payload, "degree")?;
            to_json(xalen_world::egyptian::decan_for_degree(degree))
        }
        "celtic" => {
            let month = required_i64(&payload, "month")?.clamp(1, 12) as u32;
            let day = required_i64(&payload, "day")?.clamp(1, 31) as u32;
            let year = required_i64(&payload, "year")? as i32;
            let tree = xalen_world::celtic::celtic_tree(month, day);
            let year_tree = xalen_world::celtic::celtic_year_tree(year);
            to_json(&json!({"birth_tree": tree, "year_tree": year_tree}))
        }
        other => Err(py_value_err(format!("unknown world product system '{other}'"))),
    }
}

/// Lal Kitab house-based analysis. The caller supplies the already calculated
/// house placements; XALEN remains responsible for Lal Kitab classification.
#[pyfunction]
fn product_lalkitab_json(payload_json: &str) -> PyResult<String> {
    let payload = parse_payload(payload_json)?;
    let placements = payload
        .get("placements")
        .and_then(Value::as_array)
        .ok_or_else(|| py_value_err("missing placements array"))?;
    let mut chart = xalen_lalkitab::LalKitabChart::empty();
    for item in placements {
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| py_value_err("placement is missing name"))?;
        let house = item
            .get("house")
            .and_then(Value::as_u64)
            .ok_or_else(|| py_value_err("placement is missing house"))?;
        if !(1..=12).contains(&house) {
            return Err(py_value_err("Lal Kitab house must be in 1..=12"));
        }
        chart.place(parse_planet(name)?, house as usize);
    }
    let debts = xalen_lalkitab::detect_debts(&chart);
    let mut rows = Vec::new();
    for (house_idx, planets) in chart.placements.iter().enumerate() {
        for planet in planets {
            let house = house_idx + 1;
            rows.push(json!({
                "planet": format!("{planet:?}"),
                "house": house,
                "effect": format!("{:?}", xalen_lalkitab::planet_effect(*planet, house)),
                "dormant": xalen_lalkitab::is_dormant(*planet, &chart),
                "remedy": xalen_lalkitab::remedy_lookup(*planet, house),
            }));
        }
    }
    to_json(&json!({"placements": rows, "debts": debts}))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(product_capabilities_json, m)?)?;
    m.add_function(wrap_pyfunction!(product_equatorial_json, m)?)?;
    m.add_function(wrap_pyfunction!(product_western_json, m)?)?;
    m.add_function(wrap_pyfunction!(product_vedic_json, m)?)?;
    m.add_function(wrap_pyfunction!(product_chinese_json, m)?)?;
    m.add_function(wrap_pyfunction!(product_iching_json, m)?)?;
    m.add_function(wrap_pyfunction!(product_world_json, m)?)?;
    m.add_function(wrap_pyfunction!(product_lalkitab_json, m)?)?;
    Ok(())
}
