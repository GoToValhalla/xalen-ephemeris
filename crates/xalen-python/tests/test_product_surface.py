import json

import xalen


def _load(value):
    parsed = json.loads(value)
    assert parsed is not None
    return parsed


def test_product_catalog_exposes_complete_house_and_ayanamsa_sets():
    catalog = _load(xalen.product_capabilities_json())
    assert catalog["surface_version"] == "product-v1"
    assert len(catalog["house_systems"]) == 23
    assert len(catalog["ayanamsas"]) >= 50
    assert "bazi" in catalog["families"]["chinese"]
    assert "iching" in catalog["families"]["other"]


def test_equatorial_product_binding_is_deterministic_and_finite():
    first = _load(xalen.product_equatorial_json("Sun", 2451545.0))
    second = _load(xalen.product_equatorial_json("Sun", 2451545.0))
    assert first == second
    assert 0.0 <= first["right_ascension_deg"] < 360.0
    assert -90.0 <= first["declination_deg"] <= 90.0
    assert 0.0 <= first["gast_deg"] < 360.0


def test_solar_day_product_binding_returns_ordered_real_events():
    result = _load(xalen.product_solar_day_json(2460676.5, 48.8566, 2.3522, 35.0))
    assert result["always_above"] is False
    assert result["always_below"] is False
    assert result["rise_jd"] is not None
    assert result["transit_jd"] is not None
    assert result["set_jd"] is not None
    assert result["rise_jd"] < result["transit_jd"] < result["set_jd"]


def test_product_world_and_chinese_surfaces_return_real_results():
    bazi = _load(xalen.product_chinese_json("bazi", json.dumps({
        "year": 2024, "jd": 2460370.5, "hour": 12.0
    })))
    assert set(bazi) >= {"year", "month", "day", "hour", "day_master"}

    aztec = _load(xalen.product_world_json("aztec", json.dumps({
        "year": 2024, "month": 1, "day": 1
    })))
    assert "day_sign" in aztec

    iching = _load(xalen.product_iching_json(2024, 1, 1, 12))
    assert iching


def test_product_western_and_vedic_surfaces_are_deterministic():
    positions = {
        "positions": [
            {"name": "Sun", "longitude": 10.0},
            {"name": "Moon", "longitude": 100.0},
            {"name": "Mars", "longitude": 190.0},
            {"name": "Jupiter", "longitude": 250.0},
        ],
        "orb": 2.0,
    }
    first = _load(xalen.product_western_json("antiscia", json.dumps(positions)))
    second = _load(xalen.product_western_json("antiscia", json.dumps(positions)))
    assert first == second

    kp = _load(xalen.product_vedic_json("kp", json.dumps({"degree": 123.45})))
    assert 1 <= kp["kp_number"] <= 249
    gandanta = _load(xalen.product_vedic_json("gandanta", json.dumps({"degree": 120.5})))
    assert gandanta["in_gandanta"] is True


def test_extended_jyotish_product_surface_is_callable_and_deterministic():
    payload = json.dumps({"degree": 83.25})
    first = _load(xalen.product_vedic_extended_json("pushkara", payload))
    second = _load(xalen.product_vedic_extended_json("pushkara", payload))
    assert first == second

    upagraha = _load(xalen.product_vedic_extended_json("upagraha", json.dumps({"sun_sidereal": 42.0})))
    assert isinstance(upagraha, list)
    assert len(upagraha) > 0
