import json

import xalen


def _load(raw):
    return json.loads(raw)


def test_every_advertised_house_system_is_callable():
    catalog = _load(xalen.product_capabilities_json())
    for system in catalog["house_systems"]:
        result = _load(xalen.product_houses_json(2451545.0, 48.8566, 2.3522, system))
        assert result["system"] == system
        assert len(result["cusps"]) == 12
        assert all(0.0 <= value < 360.0 for value in result["cusps"])


def test_every_advertised_ayanamsa_is_callable():
    catalog = _load(xalen.product_capabilities_json())
    for system in catalog["ayanamsas"]:
        result = _load(xalen.product_ayanamsa_json(2451545.0, system))
        assert result["system"] == system
        assert isinstance(result["value_deg"], float)
        assert result["value_deg"] == result["value_deg"]
