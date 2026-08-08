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

//! Python exposure for XALEN's existing fixed-star and SVG rendering layers.
//! The rendering functions return SVG strings and never perform astrology math.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyDictMethods};

use xalen_chart::{
    ChartData, PlanetPosition, render_north_indian, render_south_indian,
    render_western_wheel,
};
use xalen_stars::{find_by_name, nakshatra_yogatara};

fn star_to_dict(py: Python<'_>, star: &xalen_stars::FixedStar, year: f64) -> PyResult<Py<PyAny>> {
    let (lon, lat) = star.ecliptic_at_epoch(year);
    let d = PyDict::new(py);
    d.set_item("name", star.name)?;
    d.set_item("constellation", star.constellation)?;
    d.set_item("longitude", lon)?;
    d.set_item("latitude", lat)?;
    d.set_item("magnitude", star.magnitude)?;
    d.set_item("nature", star.nature)?;
    d.set_item("epoch_year", year)?;
    Ok(d.into())
}

/// Lookup an astrologically named fixed star, case-insensitively.
#[pyfunction]
#[pyo3(signature = (name, year=2000.0))]
fn fixed_star(py: Python<'_>, name: &str, year: f64) -> PyResult<Option<Py<PyAny>>> {
    if !year.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err("year must be finite"));
    }
    find_by_name(name).map(|s| star_to_dict(py, s, year)).transpose()
}

/// Return the traditional yogatara for a zero-based nakshatra index.
#[pyfunction]
#[pyo3(signature = (nakshatra_index, year=2000.0))]
fn yogatara(py: Python<'_>, nakshatra_index: usize, year: f64) -> PyResult<Option<Py<PyAny>>> {
    if nakshatra_index >= 27 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "nakshatra_index must be in 0..=26",
        ));
    }
    if !year.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err("year must be finite"));
    }
    nakshatra_yogatara(nakshatra_index)
        .map(|s| star_to_dict(py, s, year))
        .transpose()
}

/// Metadata about the fixed-star surface in this build.
#[pyfunction]
fn fixed_star_catalog_info(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("curated_count", xalen_stars::CATALOG.len())?;
    d.set_item("expanded_count", xalen_stars::expanded_star_count())?;
    d.set_item("hip_catalog_enabled", cfg!(feature = "hip-catalog"))?;
    d.set_item(
        "commercial_note",
        if cfg!(feature = "hip-catalog") {
            "Hipparcos-derived expanded catalog is enabled; review its non-commercial data license before commercial distribution"
        } else {
            "Commercial-clean build: only curated fixed-star data is linked"
        },
    )?;
    Ok(d.into())
}

fn build_chart_data(
    planet_positions: Vec<(String, usize, f64)>,
    house_cusps_deg: Vec<f64>,
    ascendant_sign_index: usize,
    ayanamsa_deg: f64,
) -> PyResult<ChartData> {
    if house_cusps_deg.len() != 12 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "house_cusps_deg must contain exactly 12 values, got {}",
            house_cusps_deg.len()
        )));
    }
    if ascendant_sign_index >= 12 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "ascendant_sign_index must be in 0..=11",
        ));
    }
    if !ayanamsa_deg.is_finite() || house_cusps_deg.iter().any(|v| !v.is_finite()) {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "ayanamsa and house cusps must be finite",
        ));
    }
    let mut cusps = [0.0f64; 12];
    cusps.copy_from_slice(&house_cusps_deg);
    let mut positions = Vec::with_capacity(planet_positions.len());
    for (abbreviation, house, longitude_deg) in planet_positions {
        if !(1..=12).contains(&house) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "planet house must be in 1..=12, got {house}"
            )));
        }
        if !longitude_deg.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "planet longitude must be finite",
            ));
        }
        positions.push(PlanetPosition {
            abbreviation,
            house,
            longitude_deg: longitude_deg.rem_euclid(360.0),
        });
    }
    Ok(ChartData {
        planet_positions: positions,
        house_cusps_deg: cusps,
        ascendant_sign_index,
        ayanamsa_deg,
    })
}

/// Render an SVG chart using XALEN's native renderer.
///
/// `style`: `western`, `north_indian`, or `south_indian`.
#[pyfunction]
#[pyo3(signature = (style, planet_positions, house_cusps_deg, ascendant_sign_index, ayanamsa_deg=0.0))]
fn render_chart_svg(
    style: &str,
    planet_positions: Vec<(String, usize, f64)>,
    house_cusps_deg: Vec<f64>,
    ascendant_sign_index: usize,
    ayanamsa_deg: f64,
) -> PyResult<String> {
    let data = build_chart_data(
        planet_positions,
        house_cusps_deg,
        ascendant_sign_index,
        ayanamsa_deg,
    )?;
    match style.to_ascii_lowercase().replace('-', "_").as_str() {
        "western" | "western_wheel" => Ok(render_western_wheel(&data)),
        "north_indian" | "north" => Ok(render_north_indian(&data)),
        "south_indian" | "south" => Ok(render_south_indian(&data)),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown chart style '{other}'; expected western, north_indian, or south_indian"
        ))),
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fixed_star, m)?)?;
    m.add_function(wrap_pyfunction!(yogatara, m)?)?;
    m.add_function(wrap_pyfunction!(fixed_star_catalog_info, m)?)?;
    m.add_function(wrap_pyfunction!(render_chart_svg, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_input_requires_twelve_cusps() {
        let err = build_chart_data(vec![], vec![0.0; 11], 0, 0.0);
        assert!(err.is_err());
    }

    #[test]
    fn western_svg_is_rendered() {
        let svg = render_chart_svg(
            "western",
            vec![("Su".into(), 1, 15.0)],
            (0..12).map(|i| i as f64 * 30.0).collect(),
            0,
            0.0,
        )
        .unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Su"));
    }

    #[test]
    fn known_fixed_star_exists() {
        assert!(find_by_name("Spica").is_some());
    }
}
