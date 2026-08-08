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

//! Thin PyO3 bindings for mature Vedic calculations already implemented in
//! `xalen-vedic`. No Jyotish mathematics is reimplemented here.
//!
//! Validation/reference sources used by the Tarot project for cross-checking
//! these capabilities are recorded in `docs/THIRD_PARTY_SOURCES.md`.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};

use xalen_vedic::ashtakavarga::Ashtakavarga;
use xalen_vedic::ashtottari::ashtottari_dasha as core_ashtottari_dasha;
use xalen_vedic::compatibility::ashtakoota_match;
use xalen_vedic::dasha::DashaPeriod;
use xalen_vedic::dosha::{
    Dosha, detect_kaal_sarpa, detect_mangal_dosha, detect_pitra_dosha,
};
use xalen_vedic::nakshatra::Nakshatra;
use xalen_vedic::rashi::Rashi;
use xalen_vedic::shadbala::{PlanetPosition, ShadBalaInput, Shadbala};
use xalen_vedic::yoga::{
    DetectedYoga, detect_budhaditya, detect_gajakesari, detect_kemadruma,
    detect_pancha_mahapurusha, detect_vipreeta_raja,
};
use xalen_vedic::yogini::yogini_dasha as core_yogini_dasha;

fn dasha_to_dict(py: Python<'_>, p: &DashaPeriod) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("lord", p.lord.to_string())?;
    d.set_item("start_jd", p.start_jd)?;
    d.set_item("end_jd", p.end_jd)?;
    d.set_item("level", format!("{:?}", p.level))?;
    let subs = PyList::empty(py);
    for child in &p.sub_periods {
        subs.append(dasha_to_dict(py, child)?)?;
    }
    d.set_item("sub_periods", subs)?;
    Ok(d.into())
}

fn dasha_list(py: Python<'_>, periods: &[DashaPeriod]) -> PyResult<Py<PyAny>> {
    let list = PyList::empty(py);
    for p in periods {
        list.append(dasha_to_dict(py, p)?)?;
    }
    Ok(list.into())
}

fn dosha_to_dict(py: Python<'_>, dosha: &Dosha) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("name", dosha.name)?;
    d.set_item("present", dosha.present)?;
    d.set_item("severity", format!("{:?}", dosha.severity))?;
    d.set_item("from_chart", dosha.from_chart)?;
    d.set_item("cancellations", dosha.cancellations.clone())?;
    Ok(d.into())
}

fn yoga_to_dict(py: Python<'_>, yoga: &DetectedYoga) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("name", yoga.name)?;
    d.set_item("category", format!("{:?}", yoga.category))?;
    d.set_item("planets_involved", yoga.planets_involved.clone())?;
    d.set_item("strength", format!("{:?}", yoga.strength))?;
    Ok(d.into())
}

fn nakshatra_from_index(index: usize) -> PyResult<Nakshatra> {
    Nakshatra::ALL.get(index).copied().ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "nakshatra index must be in 0..=26, got {index}"
        ))
    })
}

fn rashi_from_index(index: usize) -> PyResult<Rashi> {
    if index >= 12 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "rashi index must be in 0..=11, got {index}"
        )));
    }
    Ok(Rashi::from_longitude_deg(index as f64 * 30.0))
}

/// Full Ashtakavarga from sign indices.
///
/// `planet_signs` must be exactly 9 zero-based signs in this order:
/// Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Lagna.
#[pyfunction]
fn ashtakavarga(py: Python<'_>, planet_signs: Vec<usize>) -> PyResult<Py<PyAny>> {
    if planet_signs.len() != 9 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "planet_signs must contain exactly 9 entries, got {}",
            planet_signs.len()
        )));
    }
    if let Some((i, bad)) = planet_signs.iter().enumerate().find(|(_, s)| **s >= 12) {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "planet_signs[{i}] must be in 0..=11, got {bad}"
        )));
    }
    let arr: [usize; 9] = planet_signs.try_into().expect("length checked above");
    let av = Ashtakavarga::compute(&arr);
    let d = PyDict::new(py);
    d.set_item("bhinna", av.bhinna.map(|row| row.to_vec()).to_vec())?;
    d.set_item("sarva", av.sarva.to_vec())?;
    d.set_item("total", av.total)?;
    let strengths: Vec<&'static str> = (0..12).map(|i| av.sign_strength(i)).collect();
    let favorable: Vec<bool> = (0..12).map(|i| av.transit_favorable(i)).collect();
    d.set_item("sign_strength", strengths)?;
    d.set_item("transit_favorable", favorable)?;
    Ok(d.into())
}

/// Ashtottari (108-year) Mahadasha sequence.
#[pyfunction]
fn ashtottari_dasha(py: Python<'_>, moon_sidereal_deg: f64, birth_jd: f64) -> PyResult<Py<PyAny>> {
    crate::check_jd(birth_jd)?;
    if !moon_sidereal_deg.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err("moon_sidereal_deg must be finite"));
    }
    dasha_list(py, &core_ashtottari_dasha(moon_sidereal_deg, birth_jd))
}

/// Yogini (36-year) Mahadasha sequence.
#[pyfunction]
fn yogini_dasha(py: Python<'_>, moon_sidereal_deg: f64, birth_jd: f64) -> PyResult<Py<PyAny>> {
    crate::check_jd(birth_jd)?;
    if !moon_sidereal_deg.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err("moon_sidereal_deg must be finite"));
    }
    dasha_list(py, &core_yogini_dasha(moon_sidereal_deg, birth_jd))
}

/// Full Ashtakoota/Guna Milan score (maximum 36).
#[pyfunction]
fn ashtakoota(
    py: Python<'_>,
    boy_nakshatra_index: usize,
    girl_nakshatra_index: usize,
    boy_rashi_index: usize,
    girl_rashi_index: usize,
) -> PyResult<Py<PyAny>> {
    let boy_nak = nakshatra_from_index(boy_nakshatra_index)?;
    let girl_nak = nakshatra_from_index(girl_nakshatra_index)?;
    if boy_rashi_index >= 12 || girl_rashi_index >= 12 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "rashi indices must be in 0..=11",
        ));
    }
    let m = ashtakoota_match(boy_nak, girl_nak, boy_rashi_index, girl_rashi_index);
    let d = PyDict::new(py);
    d.set_item("varna", m.varna)?;
    d.set_item("vashya", m.vashya)?;
    d.set_item("tara", m.tara)?;
    d.set_item("yoni", m.yoni)?;
    d.set_item("graha_maitri", m.graha_maitri)?;
    d.set_item("gana", m.gana)?;
    d.set_item("bhakoot", m.bhakoot)?;
    d.set_item("nadi", m.nadi)?;
    d.set_item("total", m.total)?;
    d.set_item("max_score", 36)?;
    Ok(d.into())
}

/// Full six-fold Shadbala over the existing BPHS-oriented XALEN core.
///
/// `all_planets` items are `(name, sidereal_longitude_deg, speed_deg_per_day)`.
#[pyfunction]
#[pyo3(signature = (planet, lon_deg, house, speed, jd, sun_lon, moon_lon, day_fraction, all_planets))]
#[allow(clippy::too_many_arguments)]
fn shadbala(
    py: Python<'_>,
    planet: &str,
    lon_deg: f64,
    house: usize,
    speed: f64,
    jd: f64,
    sun_lon: f64,
    moon_lon: f64,
    day_fraction: f64,
    all_planets: Vec<(String, f64, f64)>,
) -> PyResult<Py<PyAny>> {
    crate::check_jd(jd)?;
    if !(1..=12).contains(&house) {
        return Err(pyo3::exceptions::PyValueError::new_err("house must be in 1..=12"));
    }
    if !(0.0..=1.0).contains(&day_fraction) || !day_fraction.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "day_fraction must be finite and in 0..=1",
        ));
    }
    let core_positions: Vec<PlanetPosition> = all_planets
        .iter()
        .map(|(name, longitude, pspeed)| PlanetPosition {
            name: Box::leak(name.clone().into_boxed_str()),
            longitude: *longitude,
            speed: *pspeed,
        })
        .collect();
    let input = ShadBalaInput {
        jd,
        sun_lon,
        moon_lon,
        day_fraction,
        all_planets: core_positions,
    };
    let s = Shadbala::compute_full(planet, lon_deg, house, speed, &input);
    let d = PyDict::new(py);
    let sthana = PyDict::new(py);
    sthana.set_item("uchcha_bala", s.sthana_bala.uchcha_bala)?;
    sthana.set_item("saptavargaja_bala", s.sthana_bala.saptavargaja_bala)?;
    sthana.set_item("ojhayugma_bala", s.sthana_bala.ojhayugma_bala)?;
    sthana.set_item("kendra_bala", s.sthana_bala.kendra_bala)?;
    sthana.set_item("drekkana_bala", s.sthana_bala.drekkana_bala)?;
    sthana.set_item("total", s.sthana_bala.total)?;
    let kala = PyDict::new(py);
    kala.set_item("nathonnatha_bala", s.kala_bala.nathonnatha_bala)?;
    kala.set_item("paksha_bala", s.kala_bala.paksha_bala)?;
    kala.set_item("tribhaga_bala", s.kala_bala.tribhaga_bala)?;
    kala.set_item("abda_bala", s.kala_bala.abda_bala)?;
    kala.set_item("masa_bala", s.kala_bala.masa_bala)?;
    kala.set_item("vara_bala", s.kala_bala.vara_bala)?;
    kala.set_item("hora_bala", s.kala_bala.hora_bala)?;
    kala.set_item("total", s.kala_bala.total)?;
    d.set_item("sthana_bala", sthana)?;
    d.set_item("dig_bala", s.dig_bala)?;
    d.set_item("kala_bala", kala)?;
    d.set_item("cheshta_bala", s.cheshta_bala)?;
    d.set_item("naisargika_bala", s.naisargika_bala)?;
    d.set_item("drik_bala", s.drik_bala)?;
    d.set_item("total", s.total)?;
    d.set_item("required_minimum", s.required_minimum)?;
    d.set_item("ratio", s.ratio)?;
    d.set_item("is_strong", s.is_strong())?;
    Ok(d.into())
}

#[pyfunction]
#[pyo3(signature = (mars_house_from_lagna, mars_house_from_moon, mars_house_from_venus, mars_nakshatra_index, jupiter_aspects_mars=false, mars_in_own_sign=false, mars_in_exaltation=false))]
fn mangal_dosha(
    py: Python<'_>,
    mars_house_from_lagna: usize,
    mars_house_from_moon: usize,
    mars_house_from_venus: usize,
    mars_nakshatra_index: usize,
    jupiter_aspects_mars: bool,
    mars_in_own_sign: bool,
    mars_in_exaltation: bool,
) -> PyResult<Py<PyAny>> {
    let d = detect_mangal_dosha(
        mars_house_from_lagna,
        mars_house_from_moon,
        mars_house_from_venus,
        mars_nakshatra_index,
        jupiter_aspects_mars,
        mars_in_own_sign,
        mars_in_exaltation,
    );
    dosha_to_dict(py, &d)
}

#[pyfunction]
fn kaal_sarpa_dosha(
    py: Python<'_>,
    rahu_house: usize,
    ketu_house: usize,
    planet_houses: Vec<usize>,
) -> PyResult<Py<PyAny>> {
    let d = detect_kaal_sarpa(rahu_house, ketu_house, &planet_houses);
    dosha_to_dict(py, &d)
}

#[pyfunction]
fn pitra_dosha(
    py: Python<'_>,
    sun_house: usize,
    saturn_house: usize,
    rahu_house: usize,
    ninth_lord_house: usize,
    ninth_lord_debilitated: bool,
) -> PyResult<Py<PyAny>> {
    let d = detect_pitra_dosha(
        sun_house,
        saturn_house,
        rahu_house,
        ninth_lord_house,
        ninth_lord_debilitated,
    );
    dosha_to_dict(py, &d)
}

#[pyfunction]
fn pancha_mahapurusha_yoga(
    py: Python<'_>,
    planet: &str,
    rashi_index: usize,
    house: usize,
) -> PyResult<Option<Py<PyAny>>> {
    let rashi = rashi_from_index(rashi_index)?;
    detect_pancha_mahapurusha(planet, rashi, house)
        .map(|y| yoga_to_dict(py, &y))
        .transpose()
}

#[pyfunction]
fn gajakesari_yoga(py: Python<'_>, jupiter_house: usize, moon_house: usize) -> PyResult<Option<Py<PyAny>>> {
    detect_gajakesari(jupiter_house, moon_house)
        .map(|y| yoga_to_dict(py, &y))
        .transpose()
}

#[pyfunction]
fn budhaditya_yoga(py: Python<'_>, sun_house: usize, mercury_house: usize) -> PyResult<Option<Py<PyAny>>> {
    detect_budhaditya(sun_house, mercury_house)
        .map(|y| yoga_to_dict(py, &y))
        .transpose()
}

#[pyfunction]
fn vipreeta_raja_yoga(
    py: Python<'_>,
    lord_6_house: usize,
    lord_8_house: usize,
    lord_12_house: usize,
) -> PyResult<Py<PyAny>> {
    let list = PyList::empty(py);
    for y in detect_vipreeta_raja(lord_6_house, lord_8_house, lord_12_house) {
        list.append(yoga_to_dict(py, &y)?)?;
    }
    Ok(list.into())
}

#[pyfunction]
fn kemadruma_yoga(py: Python<'_>, moon_house: usize, supporting_planet_houses: Vec<usize>) -> PyResult<Option<Py<PyAny>>> {
    detect_kemadruma(moon_house, &supporting_planet_houses)
        .map(|y| yoga_to_dict(py, &y))
        .transpose()
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(ashtakavarga, m)?)?;
    m.add_function(wrap_pyfunction!(ashtottari_dasha, m)?)?;
    m.add_function(wrap_pyfunction!(yogini_dasha, m)?)?;
    m.add_function(wrap_pyfunction!(ashtakoota, m)?)?;
    m.add_function(wrap_pyfunction!(shadbala, m)?)?;
    m.add_function(wrap_pyfunction!(mangal_dosha, m)?)?;
    m.add_function(wrap_pyfunction!(kaal_sarpa_dosha, m)?)?;
    m.add_function(wrap_pyfunction!(pitra_dosha, m)?)?;
    m.add_function(wrap_pyfunction!(pancha_mahapurusha_yoga, m)?)?;
    m.add_function(wrap_pyfunction!(gajakesari_yoga, m)?)?;
    m.add_function(wrap_pyfunction!(budhaditya_yoga, m)?)?;
    m.add_function(wrap_pyfunction!(vipreeta_raja_yoga, m)?)?;
    m.add_function(wrap_pyfunction!(kemadruma_yoga, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_ashtakavarga_shape() {
        Python::initialize();
        Python::attach(|py| assert!(ashtakavarga(py, vec![0; 8]).is_err()));
    }

    #[test]
    fn ashtottari_has_eight_periods() {
        assert_eq!(core_ashtottari_dasha(0.0, 2_451_545.0).len(), 8);
    }

    #[test]
    fn yogini_has_eight_periods() {
        assert_eq!(core_yogini_dasha(0.0, 2_451_545.0).len(), 8);
    }

    #[test]
    fn nakshatra_index_validation() {
        assert!(nakshatra_from_index(26).is_ok());
        assert!(nakshatra_from_index(27).is_err());
    }
}
