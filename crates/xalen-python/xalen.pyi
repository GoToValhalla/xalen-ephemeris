# Type stubs for the `xalen` extension module (PyO3 / xalen-python).
#
# Hand-written to mirror the #[pyfunction] surface in src/lib.rs so that
# pyswisseph migrants and native users get autocomplete + mypy checking.
# The `xalen.swe` pyswisseph drop-in submodule is stubbed in xalen/swe.pyi.
#
# [Modified] Stubs appended for the DE440/aspects/synastry/transits/returns/
# progressions/vargas/dasha bindings in src/advanced.rs. Per Apache-2.0
# Section 4(b), this notice states that this file was changed from its
# original upstream form.

from typing import Any, Dict, List

__version__: str

# ---------------------------------------------------------------------------
# Core ephemeris
# ---------------------------------------------------------------------------

def planet_longitude(
    jd: float, body: int, sidereal: bool = ..., ayanamsa: int = ...
) -> float:
    """Ecliptic longitude in degrees [0, 360).

    body: 0=Sun 1=Moon 2=Mercury 3=Venus 4=Mars 5=Jupiter 6=Saturn 7=Uranus
    8=Neptune 9=MeanNode(Rahu) 10=TrueNode 11=Pluto 12=Chiron 13=Ketu(Rahu+180).
    sidereal/ayanamsa: when sidereal=True, subtract the given ayanamsa (id 0=Lahiri).
    """
    ...

def planet_position(
    jd: float, body: int, sidereal: bool = ..., ayanamsa: int = ...
) -> Dict[str, Any]:
    """Full state: the pyswisseph calc_ut(..., FLG_SPEED) 6-tuple + retrograde.

    Returns {"longitude", "latitude", "distance", "lon_speed", "lat_speed",
    "dist_speed", "is_retrograde"}. Angles/speeds in degrees (speeds per day);
    distance/dist_speed in AU. longitude wrapped to [0, 360). is_retrograde is
    from the tropical longitude rate regardless of the sidereal flag.
    """
    ...

def all_planets(
    jd: float, sidereal: bool = ..., ayanamsa: int = ...
) -> Dict[str, float]:
    """{body_name: longitude_deg} for Sun..Pluto, Rahu, and Ketu."""
    ...

def julian_day(year: int, month: int, day: int, hour: float = ...) -> float:
    """Calendar date (UT) to Julian Day (UT1). Negative year = BCE."""
    ...

# ---------------------------------------------------------------------------
# Vedic
# ---------------------------------------------------------------------------

def nakshatra(moon_sidereal_deg: float) -> Dict[str, Any]:
    """{"name", "pada", "lord", "deity", "index"} for a sidereal longitude."""
    ...

def panchang(jd: float, ayanamsa: int = ...) -> Dict[str, Any]:
    """Five limbs: {"tithi": {"number","name","paksha"}, "nakshatra": str,
    "yoga": {"number","name"}, "karana": str, "vara": str}."""
    ...

def rashi(sidereal_deg: float) -> str:
    """Rashi name, e.g. "Mesha (Aries)"."""
    ...

def full_chart(
    jd: float, lat: float, lon: float, ayanamsa: int = ...
) -> Dict[str, Any]:
    """Vedic chart: {"planets": {name: {"longitude","nakshatra","pada","rashi",
    "lord"}}, "ascendant", "mc", "ayanamsa_deg", "cusps": [12]} (Whole Sign)."""
    ...

# ---------------------------------------------------------------------------
# Houses & ayanamsa
# ---------------------------------------------------------------------------

def houses(
    jd: float, lat: float, lon: float, system: int = ...
) -> Dict[str, Any]:
    """{"cusps": [12], "ascendant", "mc", "ic", "descendant", "vertex"}.

    system: 0=WholeSign 1=Equal 2=Placidus 3=Koch 4=Porphyry 5=Regiomontanus
    6=Campanus 7=Morinus 8=Alcabitius 9=Topocentric 10=Sripati 11=Vehlow
    12=Meridian 13=Krusinski.
    """
    ...

def ayanamsa(jd: float, system: int = ...) -> float:
    """Ayanamsa value in degrees. system id 0=Lahiri (17 systems, 0..16)."""
    ...

# ---------------------------------------------------------------------------
# Supplementary
# ---------------------------------------------------------------------------

def delta_t(jd: float) -> float:
    """ΔT (TT − UT1) in seconds (Stephenson-Morrison-Hohenkerk 2016)."""
    ...

def fixed_star_conjunctions(
    planet_lon: float, orb: float, year: float
) -> List[Dict[str, Any]]:
    """Fixed stars within `orb` degrees of `planet_lon` at the given year."""
    ...

def life_path(year: int, month: int, day: int) -> int:
    """Numerology life-path number."""
    ...

def expression_number(name: str, system: str) -> int:
    """Numerology expression number. system: "pythagorean" | "chaldean"."""
    ...

# ---------------------------------------------------------------------------
# String-named convenience variants
# ---------------------------------------------------------------------------

def planet_longitude_by_name(body: str, jd_ut1: float) -> float: ...
def sidereal_longitude(body: str, jd_ut1: float, ayanamsa: str) -> float: ...
def houses_by_name(
    jd: float, lat: float, lon: float, system: str
) -> Dict[str, Any]: ...
def ayanamsa_by_name(jd: float, system: str) -> float: ...

# ---------------------------------------------------------------------------
# DE440/JPL kernel control + aspects/synastry/transits/returns/progressions/
# vargas/dasha (crates/xalen-python/src/advanced.rs -- thin wrappers over
# existing Rust functions)
# ---------------------------------------------------------------------------

def load_de440_kernel(path: str) -> Dict[str, Any]:
    """Load a JPL DE440 .bsp kernel from a local path. Returns
    {"loaded", "tier", "label", "jd_start", "jd_end"}. Raises ValueError on a
    missing/unparseable file."""
    ...

def de440_status() -> Dict[str, Any]:
    """{"tier", "label", "has_kernel_loaded"} for the currently active engine."""
    ...

def unload_de440_kernel() -> None:
    """Revert to the VSOP87 analytical engine."""
    ...

def planet_longitude_with_engine(jd: float, body: int) -> Dict[str, Any]:
    """{"longitude", "engine"} using whichever engine is currently active."""
    ...

def aspects(
    positions: Dict[str, tuple[float, float]], major_only: bool = ...
) -> List[Dict[str, Any]]:
    """positions: {name: (longitude_deg, speed_deg_per_day)}. Returns a list of
    {"body1","body2","aspect_type","angle_deg","orb_deg","direction","exact_deg","is_major"}."""
    ...

def synastry(
    positions_a: Dict[str, tuple[float, float]],
    positions_b: Dict[str, tuple[float, float]],
) -> List[Dict[str, Any]]:
    """Aspects strictly between chart A and chart B (same shape as `aspects`)."""
    ...

def transits(
    transiting_positions: Dict[str, tuple[float, float]],
    natal_positions: Dict[str, tuple[float, float]],
) -> List[Dict[str, Any]]:
    """Aspects from transiting positions to natal positions (same shape as `aspects`)."""
    ...

def exact_return(body: str, natal_longitude_deg: float, search_start_jd: float) -> Dict[str, Any]:
    """body: sun/moon/mars/jupiter/saturn. Returns {"return_jd", "engine"}."""
    ...

def secondary_progression(
    birth_jd: float, target_jd: float, body: int, progression_type: str = ...
) -> Dict[str, Any]:
    """Returns {"progressed_jd", "longitude", "engine"}."""
    ...

def varga_sign(sidereal_lon_deg: float, division: int) -> Dict[str, Any]:
    """division: one of 1,2,3,4,7,9,10,12,16,20,24,27,30,40,45,60. Returns
    {"rashi", "division", "varga_name"}."""
    ...

def vimshottari_dasha(
    moon_sidereal_deg: float, birth_jd: float, depth: str = ...
) -> List[Dict[str, Any]]:
    """depth: "mahadasha" | "antardasha". Returns 9 Mahadasha period dicts."""
    ...
