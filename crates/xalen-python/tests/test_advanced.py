# Copyright 2024-2026 XALEN Technology Pvt Ltd
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Tests for the bindings added in `src/advanced.rs`: DE440 kernel control +
mode reporting, aspects/synastry/transits, exact returns, secondary
progressions, vargas, and Vimshottari dasha.

Like test_swe_compat.py, these run against the BUILT extension and are skipped
if it isn't importable. DE440-specific tests additionally skip if no
`de440s.bsp` kernel is found on disk -- this repo does not ship one. To run
those tests, download the public NASA NAIF kernel and either:

  1. place it at `<repo-root>/de440s.bsp` (repo-root-relative default below), or
  2. set the `XALEN_DE440S_PATH` environment variable to wherever you put it.

    curl -o de440s.bsp \\
      https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440s.bsp

Without the kernel present, these tests SKIP cleanly (same convention as the
Rust-side `cargo test -p xalen-ephem --test accuracy_vs_de440`, which looks
for `/tmp/de440s.bsp` and also skips cleanly if absent).
"""

import os
from pathlib import Path

import pytest

xalen = pytest.importorskip("xalen", reason="build the extension first (maturin develop)")

J2000 = 2451545.0

# Repo root is 3 levels up from this file: tests/ -> xalen-python -> crates -> <repo-root>.
_REPO_ROOT = Path(__file__).resolve().parents[3]
_KERNEL_PATH = Path(os.environ.get("XALEN_DE440S_PATH", _REPO_ROOT / "de440s.bsp"))


def _require_kernel():
    if not _KERNEL_PATH.exists():
        pytest.skip(
            f"DE440 kernel not found at {_KERNEL_PATH} -- download de440s.bsp "
            f"(see module docstring) or set XALEN_DE440S_PATH to its location"
        )
    return str(_KERNEL_PATH)


@pytest.fixture(autouse=True)
def _reset_de440_state():
    """Every DE440 test should start from a known (unloaded) state and leave
    it unloaded for the next test, since the loaded kernel is process-global."""
    xalen.unload_de440_kernel()
    yield
    xalen.unload_de440_kernel()


# ---------------------------------------------------------------------------
# DE440 kernel control + mode reporting
# ---------------------------------------------------------------------------

def test_de440_status_defaults_to_analytical_fallback():
    status = xalen.de440_status()
    assert status["tier"] == "analytical_fallback"
    assert status["has_kernel_loaded"] is False
    assert status["label"] == "VSOP87 analytical fallback"


def test_load_de440_kernel_missing_file_raises():
    with pytest.raises(ValueError):
        xalen.load_de440_kernel("/nonexistent/path/de440s.bsp")


def test_load_de440_kernel_reports_kernel_tier():
    path = _require_kernel()
    result = xalen.load_de440_kernel(path)
    assert result["loaded"] is True
    assert result["tier"] == "kernel"
    assert result["label"] == "JPL DE440 kernel"
    assert result["jd_start"] < J2000 < result["jd_end"]

    status = xalen.de440_status()
    assert status["tier"] == "kernel"
    assert status["has_kernel_loaded"] is True


def test_planet_longitude_with_engine_reports_analytical_when_no_kernel():
    result = xalen.planet_longitude_with_engine(J2000, 0)  # Sun
    assert result["engine"] == "VSOP87 analytical fallback"
    assert 0.0 <= result["longitude"] < 360.0


def test_de440_mode_proof_moon_differs_from_analytical_at_expected_scale():
    """PROVES DE440 mode was actually exercised, not just 'didn't error':
    loads the real kernel, computes the Moon's longitude via the DE440-aware
    binding, and checks it against the ORIGINAL (pre-existing, always-analytical)
    `planet_longitude` binding. The two must differ by a small but non-zero
    amount consistent with the documented accuracy of the analytical engine
    (RMS ~2.8" vs the real DE440 kernel per docs/ACCURACY.md; this single
    J2000 sample should be within about an order of magnitude of that, i.e.
    sub-arcminute, and importantly NOT exactly zero --
    zero would mean the kernel path silently fell through to the same
    analytical computation instead of actually using DE440 data).
    """
    path = _require_kernel()
    xalen.load_de440_kernel(path)

    de440_result = xalen.planet_longitude_with_engine(J2000, 1)  # Moon
    assert de440_result["engine"] == "JPL DE440 kernel"

    analytical_lon = xalen.planet_longitude(J2000, 1)  # pre-existing binding, always analytical
    de440_lon = de440_result["longitude"]

    diff_arcsec = abs(((de440_lon - analytical_lon + 180.0) % 360.0) - 180.0) * 3600.0
    # Must be non-zero (proves DE440 data was actually consulted, not a passthrough)...
    assert diff_arcsec > 0.01, "DE440 and analytical Moon longitudes are identical -- kernel not actually used"
    # ...and small (proves it's a genuine precision refinement, not a bug/unit error).
    assert diff_arcsec < 60.0, f"DE440 vs analytical Moon delta implausibly large: {diff_arcsec}\""


def test_unload_de440_kernel_reverts_to_analytical():
    path = _require_kernel()
    xalen.load_de440_kernel(path)
    assert xalen.de440_status()["has_kernel_loaded"] is True

    xalen.unload_de440_kernel()
    status = xalen.de440_status()
    assert status["has_kernel_loaded"] is False
    assert status["tier"] == "analytical_fallback"

    result = xalen.planet_longitude_with_engine(J2000, 0)
    assert result["engine"] == "VSOP87 analytical fallback"


# ---------------------------------------------------------------------------
# Aspects / synastry / transits
# ---------------------------------------------------------------------------

def test_aspects_finds_known_conjunction():
    positions = {"Sun": (100.0, 1.0), "Moon": (102.0, 13.0)}
    found = xalen.aspects(positions)
    assert len(found) == 1
    a = found[0]
    assert {a["body1"], a["body2"]} == {"Sun", "Moon"}
    assert a["aspect_type"] == "Conjunction"
    assert a["angle_deg"] == 0.0
    assert a["orb_deg"] == pytest.approx(2.0, abs=1e-9)


def test_aspects_major_only_excludes_minor_aspects():
    # 30 deg apart is a SemiSextile (minor), not a Ptolemaic major aspect.
    positions = {"Sun": (0.0, 1.0), "Mercury": (30.0, 1.5)}
    all_found = xalen.aspects(positions, major_only=False)
    major_found = xalen.aspects(positions, major_only=True)
    assert any(a["aspect_type"] == "SemiSextile" for a in all_found)
    assert not any(a["aspect_type"] == "SemiSextile" for a in major_found)


def test_synastry_only_returns_cross_chart_pairs():
    a = {"Sun": (0.0, 1.0), "Moon": (10.0, 13.0)}  # Sun-Moon = 10deg apart, not a listed aspect type at that orb boundary
    b = {"Venus": (0.5, 1.2)}
    found = xalen.synastry(a, b)
    # Only A-B pairs should appear; no A-A pair (Sun-Moon) since chart A has
    # no intra-chart aspect requested here, but assert structurally anyway.
    for asp in found:
        assert (asp["body1"] in a and asp["body2"] in b) or (asp["body1"] in b and asp["body2"] in a)
    # Sun (0.0) and Venus (0.5) are in conjunction.
    assert any(
        {asp["body1"], asp["body2"]} == {"Sun", "Venus"} and asp["aspect_type"] == "Conjunction"
        for asp in found
    )


def test_transits_is_synastry_with_natural_naming():
    transiting = {"Jupiter": (10.0, 0.2)}
    natal = {"Sun": (10.0, 1.0)}
    found = xalen.transits(transiting, natal)
    assert len(found) == 1
    assert found[0]["body1"] == "Jupiter"
    assert found[0]["body2"] == "Sun"
    assert found[0]["aspect_type"] == "Conjunction"


# ---------------------------------------------------------------------------
# Exact solar / lunar return (xalen_ephem::returns, real-Almanac root finder)
# ---------------------------------------------------------------------------

def test_exact_solar_return_lands_near_one_year_later():
    natal_jd = xalen.julian_day(1990, 6, 15, 14.5)
    natal_sun = xalen.planet_longitude(natal_jd, 0)
    result = xalen.exact_return("sun", natal_sun, natal_jd + 1.0)  # search from a day after birth
    # First solar return should land ~365.25 days later.
    assert 360.0 < (result["return_jd"] - natal_jd) < 370.0
    assert result["engine"] == "VSOP87 analytical fallback"
    # Verify it actually IS a return: Sun longitude at the found JD should
    # match the natal longitude very closely (this is what "exact" means).
    sun_at_return = xalen.planet_longitude(result["return_jd"], 0)
    diff = abs(((sun_at_return - natal_sun + 180.0) % 360.0) - 180.0)
    assert diff < 1e-4  # degrees; root-finder pins to ~1e-7 deg per Rust docs


def test_exact_lunar_return_lands_near_one_month_later():
    natal_jd = xalen.julian_day(1990, 6, 15, 14.5)
    natal_moon = xalen.planet_longitude(natal_jd, 1)
    result = xalen.exact_return("moon", natal_moon, natal_jd + 0.1)
    assert 25.0 < (result["return_jd"] - natal_jd) < 30.0  # sidereal month ~27.3 days
    moon_at_return = xalen.planet_longitude(result["return_jd"], 1)
    diff = abs(((moon_at_return - natal_moon + 180.0) % 360.0) - 180.0)
    assert diff < 1e-4


def test_exact_return_rejects_unknown_body():
    with pytest.raises(ValueError):
        xalen.exact_return("pluto", 100.0, J2000)


# ---------------------------------------------------------------------------
# Secondary progressions
# ---------------------------------------------------------------------------

def test_secondary_progression_one_day_per_year():
    birth_jd = xalen.julian_day(1990, 6, 15, 14.5)
    # 10 years later in real time -> progressed JD should be ~10 days after birth.
    target_jd = birth_jd + 10 * 365.24219
    result = xalen.secondary_progression(birth_jd, target_jd, body=0)  # Sun
    assert 9.5 < (result["progressed_jd"] - birth_jd) < 10.5
    assert 0.0 <= result["longitude"] < 360.0
    assert result["engine"] == "VSOP87 analytical fallback"


def test_secondary_progression_rejects_unknown_type():
    birth_jd = xalen.julian_day(1990, 6, 15, 14.5)
    with pytest.raises(ValueError):
        xalen.secondary_progression(birth_jd, birth_jd + 100, body=0, progression_type="bogus")


# ---------------------------------------------------------------------------
# Vargas (divisional charts)
# ---------------------------------------------------------------------------

def test_varga_sign_d1_matches_plain_rashi():
    # D1 (Rashi) of 15 deg (mid-Aries) should just be Aries -- same as the
    # existing plain `xalen.rashi()` for D1.
    result = xalen.varga_sign(15.0, 1)
    assert result["division"] == 1
    assert result["varga_name"] == "Rashi"
    assert "Mesha" in result["rashi"] or "Aries" in result["rashi"]


def test_varga_sign_navamsa_d9():
    result = xalen.varga_sign(15.0, 9)
    assert result["division"] == 9
    assert result["varga_name"] == "Navamsa"
    assert isinstance(result["rashi"], str) and len(result["rashi"]) > 0


def test_varga_sign_rejects_unsupported_division():
    with pytest.raises(ValueError):
        xalen.varga_sign(15.0, 11)  # D11 is not one of xalen-vedic's 16 vargas


# ---------------------------------------------------------------------------
# Vimshottari Dasha
# ---------------------------------------------------------------------------

def test_vimshottari_dasha_nine_mahadasha_periods():
    jd = xalen.julian_day(1990, 6, 15, 14.5)
    moon_sid = xalen.planet_longitude(jd, 1, sidereal=True, ayanamsa=0)
    periods = xalen.vimshottari_dasha(moon_sid, jd, depth="mahadasha")
    assert len(periods) == 9
    # Periods must be chronologically contiguous and non-overlapping.
    for i in range(len(periods) - 1):
        assert periods[i]["end_jd"] == pytest.approx(periods[i + 1]["start_jd"], abs=1e-6)
    # Full 120-year cycle, minus whatever fraction of the FIRST lord's period
    # had already elapsed at birth (Vimshottari always starts mid-nakshatra
    # unless birth is exactly at 0 deg of a nakshatra) -- so total is <= 120
    # years, not exactly 120. Bound loosely rather than assume zero elapsed.
    total_days = periods[-1]["end_jd"] - periods[0]["start_jd"]
    assert 0 < total_days <= 120 * 365.25 + 1


def test_vimshottari_dasha_antardasha_has_sub_periods():
    jd = xalen.julian_day(1990, 6, 15, 14.5)
    moon_sid = xalen.planet_longitude(jd, 1, sidereal=True, ayanamsa=0)
    periods = xalen.vimshottari_dasha(moon_sid, jd, depth="antardasha")
    assert len(periods) == 9
    assert all(len(p["sub_periods"]) > 0 for p in periods)


def test_vimshottari_dasha_rejects_unknown_depth():
    with pytest.raises(ValueError):
        xalen.vimshottari_dasha(200.0, J2000, depth="bogus")
