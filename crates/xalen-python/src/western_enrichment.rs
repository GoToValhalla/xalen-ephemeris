// Advanced Western bindings for the GoToValhalla maintained XALEN fork.
// Calculation logic remains in Rust core; this module converts typed results
// into stable Python dictionaries/lists.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use xalen_ephem::Almanac;
use xalen_ephem::eclipse::{
    LunarEclipseType, SolarEclipseType, find_lunar_eclipses, find_solar_eclipses,
};
use xalen_ephem::event_search::{find_sign_ingress, find_station};
use xalen_time::JdUT1;
use xalen_western::declination::{
    DeclinationAspect, declination_deg, detect_declination_aspects_from_ecliptic,
};
use xalen_western::dignity::{
    essential_dignity_score, is_detriment, is_domicile, is_exaltation, is_face_ruler, is_fall,
    is_term_ruler, is_triplicity_ruler,
};
use xalen_western::electional::moon_quality;
use xalen_western::hellenistic::{annual_profection, zodiacal_releasing};
use xalen_western::lots::{PlanetPositions, compute_all_lots};
use xalen_western::progressions::{full_profection, solar_arc_directions};
use xalen_western::relationship::{composite_midpoints, davison_midpoint};

use crate::{body_from_id, position_full};

#[pyfunction(name = "annual_profection")]
fn annual_profection_py(py: Python<'_>, natal_asc_sign: usize, age: u32) -> PyResult<Py<PyAny>> {
    if natal_asc_sign > 11 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "natal_asc_sign must be in 0..11",
        ));
    }
    let r = annual_profection(natal_asc_sign, age);
    let full = full_profection(natal_asc_sign, age);
    let d = PyDict::new(py);
    d.set_item("age", r.age)?;
    d.set_item("sign_index", r.sign_index)?;
    d.set_item("time_lord", r.time_lord)?;
    d.set_item("annual_house", full.annual_house)?;
    d.set_item("is_angular", full.is_angular)?;
    d.set_item("monthly_signs", full.monthly_signs.to_vec())?;
    d.set_item("monthly_lords", full.monthly_lords.to_vec())?;
    Ok(d.into())
}

#[pyfunction(name = "zodiacal_releasing", signature = (lot_sign, birth_jd, levels = 2))]
fn zodiacal_releasing_py(
    py: Python<'_>,
    lot_sign: usize,
    birth_jd: f64,
    levels: u8,
) -> PyResult<Vec<Py<PyAny>>> {
    if lot_sign > 11 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "lot_sign must be in 0..11",
        ));
    }
    if !birth_jd.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "birth_jd must be finite",
        ));
    }
    if !(1..=2).contains(&levels) {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "levels must be 1 or 2",
        ));
    }
    zodiacal_releasing(lot_sign, birth_jd, levels)
        .into_iter()
        .map(|p| {
            let d = PyDict::new(py);
            d.set_item("sign_index", p.sign_index)?;
            d.set_item("level", p.level)?;
            d.set_item("years", p.years)?;
            d.set_item("start_jd", p.start_jd)?;
            d.set_item("end_jd", p.end_jd)?;
            Ok(d.into())
        })
        .collect()
}

#[pyfunction(name = "solar_arc_directions")]
fn solar_arc_directions_py(
    py: Python<'_>,
    natal_sun: f64,
    progressed_sun: f64,
    natal_positions: Vec<(String, f64)>,
) -> PyResult<Py<PyAny>> {
    if !natal_sun.is_finite()
        || !progressed_sun.is_finite()
        || natal_positions.iter().any(|(_, v)| !v.is_finite())
    {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "all longitudes must be finite",
        ));
    }
    let borrowed: Vec<(&str, f64)> = natal_positions
        .iter()
        .map(|(n, v)| (n.as_str(), *v))
        .collect();
    let r = solar_arc_directions(natal_sun, progressed_sun, &borrowed);
    let d = PyDict::new(py);
    d.set_item("arc_deg", r.arc)?;
    d.set_item("directed_positions", r.directed_positions)?;
    Ok(d.into())
}

#[pyfunction(name = "declination_aspects", signature = (positions, obliquity_deg = 23.43929111, orb_deg = 1.0))]
fn declination_aspects_py(
    py: Python<'_>,
    positions: Vec<(String, f64, f64)>,
    obliquity_deg: f64,
    orb_deg: f64,
) -> PyResult<Vec<Py<PyAny>>> {
    if orb_deg < 0.0 || !orb_deg.is_finite() || !obliquity_deg.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "orb and obliquity must be finite; orb >= 0",
        ));
    }
    let borrowed: Vec<(&str, f64, f64)> = positions
        .iter()
        .map(|(n, lon, lat)| (n.as_str(), *lon, *lat))
        .collect();
    detect_declination_aspects_from_ecliptic(&borrowed, obliquity_deg, orb_deg)
        .into_iter()
        .map(|c| {
            let d = PyDict::new(py);
            d.set_item(
                "aspect",
                match c.aspect {
                    DeclinationAspect::Parallel => "parallel",
                    DeclinationAspect::Contraparallel => "contraparallel",
                },
            )?;
            d.set_item("body1", c.body1)?;
            d.set_item("body2", c.body2)?;
            d.set_item("declination1_deg", c.dec1_deg)?;
            d.set_item("declination2_deg", c.dec2_deg)?;
            d.set_item("orb_deg", c.orb_deg)?;
            Ok(d.into())
        })
        .collect()
}

#[pyfunction(name = "declination")]
fn declination_py(lon_deg: f64, lat_deg: f64, obliquity_deg: f64) -> PyResult<f64> {
    if !lon_deg.is_finite() || !lat_deg.is_finite() || !obliquity_deg.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "inputs must be finite",
        ));
    }
    Ok(declination_deg(lon_deg, lat_deg, obliquity_deg))
}

#[pyfunction(name = "essential_dignity")]
fn essential_dignity_py(
    py: Python<'_>,
    planet: &str,
    sign_index: usize,
    degree_in_sign: f64,
) -> PyResult<Py<PyAny>> {
    if sign_index > 11 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "sign_index must be in 0..11",
        ));
    }
    if !(0.0..30.0).contains(&degree_in_sign) {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "degree_in_sign must be in [0,30)",
        ));
    }
    let d = PyDict::new(py);
    d.set_item(
        "score",
        essential_dignity_score(planet, sign_index, degree_in_sign),
    )?;
    d.set_item("domicile", is_domicile(planet, sign_index))?;
    d.set_item("exaltation", is_exaltation(planet, sign_index))?;
    d.set_item("detriment", is_detriment(planet, sign_index))?;
    d.set_item("fall", is_fall(planet, sign_index))?;
    d.set_item("triplicity", is_triplicity_ruler(planet, sign_index))?;
    d.set_item("term", is_term_ruler(planet, sign_index, degree_in_sign))?;
    d.set_item("face", is_face_ruler(planet, sign_index, degree_in_sign))?;
    Ok(d.into())
}

#[pyfunction(name = "arabic_lots")]
fn arabic_lots_py(
    py: Python<'_>,
    sun: f64,
    moon: f64,
    mercury: f64,
    venus: f64,
    mars: f64,
    jupiter: f64,
    saturn: f64,
    asc: f64,
    mc: f64,
    is_day: bool,
) -> PyResult<Vec<Py<PyAny>>> {
    if [sun, moon, mercury, venus, mars, jupiter, saturn, asc, mc]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "all longitudes must be finite",
        ));
    }
    let p = PlanetPositions {
        sun,
        moon,
        mercury,
        venus,
        mars,
        jupiter,
        saturn,
        asc,
        mc,
    };
    compute_all_lots(&p, is_day)
        .into_iter()
        .map(|(name, degree)| {
            let d = PyDict::new(py);
            d.set_item("name", name)?;
            d.set_item("longitude_deg", degree)?;
            Ok(d.into())
        })
        .collect()
}

#[pyfunction(name = "composite_midpoints")]
fn composite_midpoints_py(
    py: Python<'_>,
    chart_a: Vec<(String, f64)>,
    chart_b: Vec<(String, f64)>,
) -> PyResult<Vec<Py<PyAny>>> {
    composite_midpoints(&chart_a, &chart_b)
        .map_err(pyo3::exceptions::PyValueError::new_err)?
        .into_iter()
        .map(|p| {
            let d = PyDict::new(py);
            d.set_item("name", p.name)?;
            d.set_item("longitude_deg", p.longitude_deg)?;
            Ok(d.into())
        })
        .collect()
}

#[pyfunction(name = "davison_midpoint")]
fn davison_midpoint_py(
    py: Python<'_>,
    jd_a: f64,
    lat_a: f64,
    lon_a: f64,
    jd_b: f64,
    lat_b: f64,
    lon_b: f64,
) -> PyResult<Py<PyAny>> {
    let r = davison_midpoint(jd_a, lat_a, lon_a, jd_b, lat_b, lon_b)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    let d = PyDict::new(py);
    d.set_item("jd", r.jd)?;
    d.set_item("latitude_deg", r.latitude_deg)?;
    d.set_item("longitude_deg", r.longitude_deg)?;
    Ok(d.into())
}

#[pyfunction(name = "moon_quality")]
fn moon_quality_py(
    py: Python<'_>,
    moon_lon: f64,
    moon_speed: f64,
    planets: Vec<(String, f64, f64)>,
) -> PyResult<Py<PyAny>> {
    if !moon_lon.is_finite()
        || !moon_speed.is_finite()
        || planets
            .iter()
            .any(|(_, l, s)| !l.is_finite() || !s.is_finite())
    {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "positions and speeds must be finite",
        ));
    }
    let q = moon_quality(moon_lon, moon_speed, &planets);
    let d = PyDict::new(py);
    d.set_item("applying_to_benefic", q.applying_to_benefic)?;
    d.set_item("applying_to_malefic", q.applying_to_malefic)?;
    d.set_item("void_of_course", q.void_of_course)?;
    d.set_item("score", q.score)?;
    Ok(d.into())
}

#[pyfunction(name = "sign_ingresses", signature = (body_id, jd_start, jd_end, step_days = 0.5))]
fn sign_ingresses_py(
    py: Python<'_>,
    body_id: u8,
    jd_start: f64,
    jd_end: f64,
    step_days: f64,
) -> PyResult<Vec<Py<PyAny>>> {
    let body = body_from_id(body_id)?;
    let almanac = Almanac::default_vedic();
    let events = find_sign_ingress(
        |jd| {
            almanac
                .geocentric_longitude_deg(body, JdUT1(jd))
                .unwrap_or(f64::NAN)
        },
        jd_start,
        jd_end,
        step_days,
    );
    events
        .into_iter()
        .filter(|e| e.value.is_finite())
        .map(|e| {
            let d = PyDict::new(py);
            d.set_item("jd", e.jd)?;
            d.set_item("longitude_deg", e.value.rem_euclid(360.0))?;
            d.set_item(
                "sign_index",
                (e.value.rem_euclid(360.0) / 30.0).floor() as usize,
            )?;
            d.set_item("iterations", e.iterations)?;
            Ok(d.into())
        })
        .collect()
}

#[pyfunction(name = "stations", signature = (body_id, jd_start, jd_end, step_days = 1.0))]
fn stations_py(
    py: Python<'_>,
    body_id: u8,
    jd_start: f64,
    jd_end: f64,
    step_days: f64,
) -> PyResult<Vec<Py<PyAny>>> {
    let body = body_from_id(body_id)?;
    let almanac = Almanac::default_vedic();
    let events = find_station(
        |jd| {
            position_full(&almanac, body, jd, None)
                .map(|p| p.lon_speed)
                .unwrap_or(f64::NAN)
        },
        jd_start,
        jd_end,
        step_days,
    );
    events
        .into_iter()
        .filter(|e| e.value.is_finite())
        .map(|e| {
            let d = PyDict::new(py);
            d.set_item("jd", e.jd)?;
            d.set_item("speed_deg_per_day", e.value)?;
            d.set_item("iterations", e.iterations)?;
            Ok(d.into())
        })
        .collect()
}

#[pyfunction(name = "solar_eclipses")]
fn solar_eclipses_py(py: Python<'_>, jd_start: f64, jd_end: f64) -> PyResult<Vec<Py<PyAny>>> {
    let a = Almanac::default_vedic();
    find_solar_eclipses(&a, jd_start, jd_end)
        .into_iter()
        .map(|e| {
            let d = PyDict::new(py);
            d.set_item("jd_maximum", e.jd_maximum)?;
            d.set_item(
                "type",
                match e.eclipse_type {
                    SolarEclipseType::Partial => "partial",
                    SolarEclipseType::Annular => "annular",
                    SolarEclipseType::Total => "total",
                    SolarEclipseType::Hybrid => "hybrid",
                },
            )?;
            d.set_item("coverage_proxy", e.coverage_proxy)?;
            d.set_item("moon_latitude_deg", e.moon_latitude_deg)?;
            d.set_item("gamma", e.gamma)?;
            Ok(d.into())
        })
        .collect()
}

#[pyfunction(name = "lunar_eclipses")]
fn lunar_eclipses_py(py: Python<'_>, jd_start: f64, jd_end: f64) -> PyResult<Vec<Py<PyAny>>> {
    let a = Almanac::default_vedic();
    find_lunar_eclipses(&a, jd_start, jd_end)
        .into_iter()
        .map(|e| {
            let d = PyDict::new(py);
            d.set_item("jd_maximum", e.jd_maximum)?;
            d.set_item(
                "type",
                match e.eclipse_type {
                    LunarEclipseType::Penumbral => "penumbral",
                    LunarEclipseType::Partial => "partial",
                    LunarEclipseType::Total => "total",
                },
            )?;
            d.set_item("shadow_depth_proxy", e.shadow_depth_proxy)?;
            d.set_item("moon_latitude_deg", e.moon_latitude_deg)?;
            Ok(d.into())
        })
        .collect()
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(annual_profection_py, module)?)?;
    module.add_function(wrap_pyfunction!(zodiacal_releasing_py, module)?)?;
    module.add_function(wrap_pyfunction!(solar_arc_directions_py, module)?)?;
    module.add_function(wrap_pyfunction!(declination_py, module)?)?;
    module.add_function(wrap_pyfunction!(declination_aspects_py, module)?)?;
    module.add_function(wrap_pyfunction!(essential_dignity_py, module)?)?;
    module.add_function(wrap_pyfunction!(arabic_lots_py, module)?)?;
    module.add_function(wrap_pyfunction!(composite_midpoints_py, module)?)?;
    module.add_function(wrap_pyfunction!(davison_midpoint_py, module)?)?;
    module.add_function(wrap_pyfunction!(moon_quality_py, module)?)?;
    module.add_function(wrap_pyfunction!(sign_ingresses_py, module)?)?;
    module.add_function(wrap_pyfunction!(stations_py, module)?)?;
    module.add_function(wrap_pyfunction!(solar_eclipses_py, module)?)?;
    module.add_function(wrap_pyfunction!(lunar_eclipses_py, module)?)?;
    Ok(())
}
