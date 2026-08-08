import pytest
import xalen


def test_ashtakavarga_shape_and_total():
    result = xalen.ashtakavarga([0, 1, 2, 3, 4, 5, 6, 7, 8])
    assert len(result["bhinna"]) == 8
    assert all(len(row) == 12 for row in result["bhinna"])
    assert len(result["sarva"]) == 12
    assert result["total"] == sum(result["sarva"])
    assert len(result["sign_strength"]) == 12


def test_ashtakavarga_rejects_bad_shape():
    with pytest.raises(ValueError):
        xalen.ashtakavarga([0] * 8)


def test_extra_dasha_cycles():
    ashtottari = xalen.ashtottari_dasha(0.0, 2451545.0)
    yogini = xalen.yogini_dasha(0.0, 2451545.0)
    assert len(ashtottari) == 8
    assert len(yogini) == 8
    assert all(p["end_jd"] > p["start_jd"] for p in ashtottari + yogini)


def test_ashtakoota_contract():
    result = xalen.ashtakoota(0, 3, 0, 1)
    assert 0 <= result["total"] <= result["max_score"] == 36
    assert set(result) >= {
        "varna", "vashya", "tara", "yoni", "graha_maitri", "gana",
        "bhakoot", "nadi", "total", "max_score",
    }


def test_dosha_and_yoga_contracts():
    mangal = xalen.mangal_dosha(1, 3, 5, 4)
    assert set(mangal) == {"name", "present", "severity", "from_chart", "cancellations"}
    kaal = xalen.kaal_sarpa_dosha(1, 7, [1, 2, 3, 4, 5, 6, 7])
    assert "present" in kaal
    yoga = xalen.pancha_mahapurusha_yoga("Mars", 0, 1)
    assert yoga is not None
    assert yoga["name"] == "Ruchaka"


def test_fixed_star_lookup_and_yogatara():
    spica = xalen.fixed_star("Spica", 2026.0)
    assert spica is not None
    assert spica["name"] == "Spica"
    assert 0.0 <= spica["longitude"] < 360.0
    yogatara = xalen.yogatara(13, 2026.0)  # Chitra
    assert yogatara is not None
    assert yogatara["name"] == "Spica"


def test_fixed_star_build_metadata_is_explicit():
    info = xalen.fixed_star_catalog_info()
    assert info["curated_count"] >= 100
    assert info["expanded_count"] >= info["curated_count"]
    assert isinstance(info["hip_catalog_enabled"], bool)
    assert info["commercial_note"]


def test_render_all_three_chart_styles():
    planets = [("Su", 1, 15.0), ("Mo", 4, 100.0), ("Ma", 7, 195.0)]
    cusps = [i * 30.0 for i in range(12)]
    for style in ("western", "north_indian", "south_indian"):
        svg = xalen.render_chart_svg(style, planets, cusps, 0, 24.0)
        assert svg.startswith("<svg")
        assert svg.endswith("</svg>")
        assert "Su" in svg


def test_render_rejects_bad_cusp_count():
    with pytest.raises(ValueError):
        xalen.render_chart_svg("western", [], [0.0] * 11, 0, 0.0)
