import math

import pytest
import xalen


def test_annual_profection_and_zodiacal_releasing():
    p = xalen.annual_profection(0, 25)
    assert p["age"] == 25
    assert 0 <= p["sign_index"] <= 11
    assert len(p["monthly_signs"]) == 12
    periods = xalen.zodiacal_releasing(0, 2451545.0, 1)
    assert periods
    assert periods[0]["level"] == 1
    assert periods[0]["end_jd"] > periods[0]["start_jd"]


def test_solar_arc_and_declination():
    result = xalen.solar_arc_directions(10.0, 11.5, [("Sun", 10.0), ("Moon", 359.0)])
    assert result["arc_deg"] == pytest.approx(1.5)
    assert len(result["directed_positions"]) == 2
    dec = xalen.declination(0.0, 0.0, 23.43929111)
    assert dec == pytest.approx(0.0, abs=1e-12)


def test_declination_aspects_are_structured():
    aspects = xalen.declination_aspects(
        [("A", 0.0, 0.0), ("B", 180.0, 0.0)],
        23.43929111,
        1.0,
    )
    assert isinstance(aspects, list)
    for aspect in aspects:
        assert aspect["aspect"] in {"parallel", "contraparallel"}
        assert aspect["orb_deg"] >= 0.0


def test_essential_dignity_and_arabic_lots():
    dignity = xalen.essential_dignity("Mars", 0, 10.0)
    assert isinstance(dignity["score"], int)
    assert set(dignity) >= {"domicile", "exaltation", "detriment", "fall", "triplicity", "term", "face"}
    lots = xalen.arabic_lots(10, 20, 30, 40, 50, 60, 70, 80, 90, True)
    assert len(lots) >= 7
    assert all(0.0 <= item["longitude_deg"] < 360.0 for item in lots)


def test_composite_midpoint_wrap_and_davison_dateline():
    composite = xalen.composite_midpoints([("Sun", 350.0)], [("Sun", 10.0)])
    assert composite[0]["name"] == "Sun"
    assert min(abs(composite[0]["longitude_deg"]), abs(360.0 - composite[0]["longitude_deg"])) < 1e-9
    d = xalen.davison_midpoint(100.0, 10.0, 170.0, 102.0, 20.0, -170.0)
    assert d["jd"] == pytest.approx(101.0)
    assert d["latitude_deg"] == pytest.approx(15.0)
    assert abs(d["longitude_deg"]) == pytest.approx(180.0)


def test_composite_rejects_mismatched_points():
    with pytest.raises(ValueError):
        xalen.composite_midpoints([("Sun", 1.0)], [("Moon", 1.0)])


def test_voc_and_event_search_surfaces_are_callable():
    q = xalen.moon_quality(10.0, 13.0, [("Jupiter", 120.0, 0.1), ("Saturn", 250.0, 0.1)])
    assert isinstance(q["void_of_course"], bool)
    assert isinstance(q["score"], int)

    # Small deterministic windows are enough to exercise the root-search API;
    # an empty result is valid when no event occurs in the requested interval.
    for value in (
        xalen.sign_ingresses(0, 2451545.0, 2451550.0, 0.5),
        xalen.stations(2, 2451545.0, 2451550.0, 1.0),
        xalen.solar_eclipses(2451545.0, 2451600.0),
        xalen.lunar_eclipses(2451545.0, 2451600.0),
    ):
        assert isinstance(value, list)
