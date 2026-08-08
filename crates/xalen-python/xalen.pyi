# Extended XALEN type surface for the GoToValhalla Tarot fork.
# The validated upstream/advanced API is preserved in xalen_base.pyi and
# re-exported here; the declarations below cover the new Vedic, fixed-star,
# and rendering APIs added by this fork.

from typing import Any, Dict, List
from xalen_base import *  # noqa: F401,F403


def ashtakavarga(planet_signs: List[int]) -> Dict[str, Any]: ...
def ashtottari_dasha(moon_sidereal_deg: float, birth_jd: float) -> List[Dict[str, Any]]: ...
def yogini_dasha(moon_sidereal_deg: float, birth_jd: float) -> List[Dict[str, Any]]: ...
def ashtakoota(
    boy_nakshatra_index: int,
    girl_nakshatra_index: int,
    boy_rashi_index: int,
    girl_rashi_index: int,
) -> Dict[str, Any]: ...
def shadbala(
    planet: str,
    lon_deg: float,
    house: int,
    speed: float,
    jd: float,
    sun_lon: float,
    moon_lon: float,
    day_fraction: float,
    all_planets: List[tuple[str, float, float]],
) -> Dict[str, Any]: ...
def mangal_dosha(
    mars_house_from_lagna: int,
    mars_house_from_moon: int,
    mars_house_from_venus: int,
    mars_nakshatra_index: int,
    jupiter_aspects_mars: bool = ...,
    mars_in_own_sign: bool = ...,
    mars_in_exaltation: bool = ...,
) -> Dict[str, Any]: ...
def kaal_sarpa_dosha(rahu_house: int, ketu_house: int, planet_houses: List[int]) -> Dict[str, Any]: ...
def pitra_dosha(
    sun_house: int,
    saturn_house: int,
    rahu_house: int,
    ninth_lord_house: int,
    ninth_lord_debilitated: bool,
) -> Dict[str, Any]: ...
def pancha_mahapurusha_yoga(planet: str, rashi_index: int, house: int) -> Dict[str, Any] | None: ...
def gajakesari_yoga(jupiter_house: int, moon_house: int) -> Dict[str, Any] | None: ...
def budhaditya_yoga(sun_house: int, mercury_house: int) -> Dict[str, Any] | None: ...
def vipreeta_raja_yoga(lord_6_house: int, lord_8_house: int, lord_12_house: int) -> List[Dict[str, Any]]: ...
def kemadruma_yoga(moon_house: int, supporting_planet_houses: List[int]) -> Dict[str, Any] | None: ...

def fixed_star(name: str, year: float = ...) -> Dict[str, Any] | None: ...
def yogatara(nakshatra_index: int, year: float = ...) -> Dict[str, Any] | None: ...
def fixed_star_catalog_info() -> Dict[str, Any]: ...
def render_chart_svg(
    style: str,
    planet_positions: List[tuple[str, int, float]],
    house_cusps_deg: List[float],
    ascendant_sign_index: int,
    ayanamsa_deg: float = ...,
) -> str: ...
