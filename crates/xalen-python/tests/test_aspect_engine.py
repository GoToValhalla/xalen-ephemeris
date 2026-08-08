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

"""Tests for the configurable-orb aspect engine and pattern detection bindings
added in `src/aspect_engine.rs` (Western Enrichment Package 1). The underlying
Rust logic is adapted from Anonyfox/celestine (MIT) -- see
docs/THIRD_PARTY_SOURCES.md. These tests exercise the PYTHON BINDING SURFACE;
the Rust-level behavioral tests (all 14 aspect types, orb boundaries, 0/360
wrap, applying/separating, out-of-sign, all 7 patterns) live in
crates/xalen-western/src/aspects.rs and patterns.rs.
"""

import pytest

xalen = pytest.importorskip("xalen", reason="build the extension first (maturin develop)")


# ---------------------------------------------------------------------------
# Backward compatibility: the pre-existing aspects()/synastry()/transits()
# bindings (added in a prior round) must be completely unaffected.
# ---------------------------------------------------------------------------

def test_legacy_aspects_still_works_unchanged():
    result = xalen.aspects({"Sun": (100.0, 1.0), "Moon": (102.0, 13.0)})
    assert len(result) == 1
    a = result[0]
    assert {a["body1"], a["body2"]} == {"Sun", "Moon"}
    assert a["aspect_type"] == "Conjunction"
    # Legacy shape: "direction" (capitalized enum debug string), no "strength"/"phase".
    assert "direction" in a
    assert "strength" not in a


# ---------------------------------------------------------------------------
# aspect_type_names(): all 14
# ---------------------------------------------------------------------------

def test_aspect_type_names_returns_all_14():
    names = xalen.aspect_type_names()
    assert len(names) == 14
    for expected in [
        "conjunction", "sextile", "square", "trine", "opposition",
        "semi_sextile", "semi_square", "quintile", "sesquiquadrate",
        "biquintile", "quincunx", "septile", "novile", "decile",
    ]:
        assert expected in names, f"missing aspect type: {expected}"


# ---------------------------------------------------------------------------
# aspects_ex(): defaults, configurability, richer result shape
# ---------------------------------------------------------------------------

def test_aspects_ex_default_matches_five_majors():
    # 45deg separation is a semi-square, not a Ptolemaic major -- default
    # aspect_types (5 majors) should find nothing here.
    result = xalen.aspects_ex({"Sun": (0.0, None), "Mars": (45.0, None)})
    assert result == []


def test_aspects_ex_returns_rich_fields():
    result = xalen.aspects_ex({"Sun": (0.0, 1.0), "Mars": (93.0, 0.5)})
    assert len(result) == 1
    a = result[0]
    for key in (
        "body1", "body2", "aspect_type", "angle_deg", "separation_deg",
        "deviation_deg", "orb_deg", "strength", "phase", "is_out_of_sign",
        "is_major", "is_kepler",
    ):
        assert key in a, f"aspects_ex result missing key {key}"
    assert a["aspect_type"] == "square"
    assert a["phase"] == "applying"
    assert 0.0 <= a["strength"] <= 100.0
    assert a["is_major"] is True
    assert a["is_kepler"] is False


def test_aspects_ex_custom_aspect_types_includes_kepler():
    # Septile (360/7 = 51.43deg) is not in the default major set.
    result = xalen.aspects_ex(
        {"A": (0.0, None), "B": (51.43, None)},
        aspect_types=["septile"],
    )
    assert len(result) == 1
    assert result[0]["aspect_type"] == "septile"
    assert result[0]["is_kepler"] is True


def test_aspects_ex_orb_override_widens_detection():
    # 8deg from exact square; default square orb (7deg) excludes it.
    positions = {"Sun": (0.0, None), "Mars": (82.0, None)}
    tight = xalen.aspects_ex(positions, aspect_types=["square"])
    assert tight == []

    wide = xalen.aspects_ex(positions, aspect_types=["square"], orbs={"square": 9.0})
    assert len(wide) == 1
    assert abs(wide[0]["deviation_deg"] - 8.0) < 1e-9


def test_aspects_ex_out_of_sign_flag():
    # 28deg Aries to 2deg Virgo (152deg by construction below): trine-by-degree
    # but Aries/Virgo are not trine signs.
    result = xalen.aspects_ex(
        {"Mars": (28.0, None), "Jupiter": (152.0, None)},
        aspect_types=["trine"],
    )
    assert len(result) == 1
    assert result[0]["is_out_of_sign"] is True


def test_aspects_ex_exclude_out_of_sign_filters():
    positions = {"Mars": (28.0, None), "Jupiter": (152.0, None)}
    included = xalen.aspects_ex(positions, aspect_types=["trine"], exclude_out_of_sign=False)
    excluded = xalen.aspects_ex(positions, aspect_types=["trine"], exclude_out_of_sign=True)
    assert len(included) == 1
    assert len(excluded) == 0


def test_aspects_ex_minimum_strength_filters():
    # 7deg deviation on an 8deg-orb conjunction -> low but nonzero strength.
    positions = {"A": (0.0, None), "B": (7.0, None)}
    all_results = xalen.aspects_ex(positions, aspect_types=["conjunction"])
    assert len(all_results) == 1
    strong_only = xalen.aspects_ex(
        positions, aspect_types=["conjunction"], minimum_strength=50.0
    )
    assert strong_only == []


def test_aspects_ex_rejects_unknown_aspect_type():
    with pytest.raises(ValueError):
        xalen.aspects_ex({"A": (0.0, None), "B": (10.0, None)}, aspect_types=["bogus"])


def test_aspects_ex_no_speed_gives_none_phase():
    result = xalen.aspects_ex({"Sun": (0.0, None), "Mars": (90.0, None)})
    assert len(result) == 1
    assert result[0]["phase"] is None


def test_aspects_ex_wrap_across_0_360():
    result = xalen.aspects_ex({"A": (359.0, None), "B": (1.0, None)})
    assert len(result) == 1
    assert result[0]["aspect_type"] == "conjunction"
    assert abs(result[0]["deviation_deg"] - 2.0) < 1e-9


# ---------------------------------------------------------------------------
# aspect_patterns(): all 7 pattern types reachable from Python
# ---------------------------------------------------------------------------

def test_aspect_patterns_grand_trine():
    result = xalen.aspect_patterns(
        {"A": (0.0, None), "B": (120.0, None), "C": (240.0, None)}
    )
    assert any(p["pattern_type"] == "GrandTrine" for p in result)


def test_aspect_patterns_t_square():
    result = xalen.aspect_patterns(
        {"A": (0.0, None), "B": (180.0, None), "C": (90.0, None)}
    )
    assert any(p["pattern_type"] == "TSquare" for p in result)


def test_aspect_patterns_grand_cross():
    result = xalen.aspect_patterns(
        {"A": (0.0, None), "B": (90.0, None), "C": (180.0, None), "D": (270.0, None)}
    )
    assert any(p["pattern_type"] == "GrandCross" for p in result)


def test_aspect_patterns_yod():
    result = xalen.aspect_patterns(
        {"A": (0.0, None), "B": (60.0, None), "C": (210.0, None)}
    )
    assert any(p["pattern_type"] == "Yod" for p in result)


def test_aspect_patterns_kite():
    result = xalen.aspect_patterns(
        {
            "A": (0.0, None), "B": (120.0, None), "C": (240.0, None), "D": (180.0, None),
        }
    )
    assert any(p["pattern_type"] == "Kite" for p in result)


def test_aspect_patterns_mystic_rectangle():
    result = xalen.aspect_patterns(
        {"A": (0.0, None), "B": (60.0, None), "C": (180.0, None), "D": (240.0, None)}
    )
    assert any(p["pattern_type"] == "MysticRectangle" for p in result)


def test_aspect_patterns_stellium():
    result = xalen.aspect_patterns(
        {"A": (0.0, None), "B": (5.0, None), "C": (8.0, None), "D": (3.0, None)}
    )
    assert any(p["pattern_type"] == "Stellium" for p in result)


def test_aspect_patterns_none_for_scattered_bodies():
    result = xalen.aspect_patterns(
        {"A": (10.0, None), "B": (80.0, None), "C": (160.0, None), "D": (230.0, None)}
    )
    assert result == []


def test_aspect_patterns_result_shape():
    result = xalen.aspect_patterns(
        {"A": (0.0, None), "B": (120.0, None), "C": (240.0, None)}
    )
    assert len(result) >= 1
    p = result[0]
    for key in ("pattern_type", "bodies", "description", "aspects"):
        assert key in p
    assert isinstance(p["bodies"], list)
    assert isinstance(p["aspects"], list)
    assert len(p["aspects"]) > 0
    # Nested aspects use the same rich shape as aspects_ex.
    assert "strength" in p["aspects"][0]
