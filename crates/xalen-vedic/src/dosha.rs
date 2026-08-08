use serde::{Deserialize, Serialize};

pub use xalen_coords::Planet;

/// Traditional (classical, pre-Uranian) ruler of a sidereal rashi.
///
/// `sign` is a 0-based sign index (0 = Aries .. 11 = Pisces). Dual-lordship
/// signs (Scorpio, Aquarius) return only the classical planetary ruler
/// (Mars, Saturn) — Rahu/Ketu are Jaimini co-lords, not used here since
/// Pitra Dosha's 9th-lord check is a classical (BPHS-style) reckoning.
pub fn traditional_sign_lord(sign: usize) -> Planet {
    match sign % 12 {
        0 => Planet::Mars,     // Aries
        1 => Planet::Venus,    // Taurus
        2 => Planet::Mercury,  // Gemini
        3 => Planet::Moon,     // Cancer
        4 => Planet::Sun,      // Leo
        5 => Planet::Mercury,  // Virgo
        6 => Planet::Venus,    // Libra
        7 => Planet::Mars,     // Scorpio
        8 => Planet::Jupiter,  // Sagittarius
        9 => Planet::Saturn,   // Capricorn
        10 => Planet::Saturn,  // Aquarius
        11 => Planet::Jupiter, // Pisces
        _ => unreachable!(),
    }
}

/// Resolve the real 9th-house (whole-sign) lord's house position from Lagna.
///
/// `asc_sign` is the Ascendant's 0-based sign index. `planet_signs` gives the
/// 0-based sign index of each of the seven classical planets. The 9th house
/// sign is 8 signs forward of the Ascendant (whole-sign houses); its
/// traditional ruler's own whole-sign house from Lagna is returned.
///
/// Returns `None` if `planet_signs` does not contain the resolved ruler
/// (should not happen when all seven classical planets are supplied).
pub fn resolve_ninth_lord_house(
    asc_sign: usize,
    planet_signs: &[(Planet, usize)],
) -> Option<usize> {
    let ninth_sign = (asc_sign + 8) % 12;
    let lord = traditional_sign_lord(ninth_sign);
    let lord_sign = planet_signs
        .iter()
        .find(|(p, _)| *p == lord)
        .map(|(_, s)| *s)?;
    Some((lord_sign + 12 - asc_sign % 12) % 12 + 1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dosha {
    pub name: &'static str,
    pub present: bool,
    pub severity: DoshaSeverity,
    pub from_chart: &'static str,
    pub cancellations: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoshaSeverity {
    None,
    Mild,
    Moderate,
    Severe,
}

/// Detect Mangal (Manglik) Dosha and its classical cancellations.
///
/// The three `mars_house_from_*` arguments are 1-based house numbers (1..=12)
/// of Mars reckoned from the Lagna, the Moon and Venus respectively.
/// `mars_nakshatra_index` is the **0-based nakshatra index (0..=26)** of Mars —
/// used for the nakshatra-based cancellation, NOT a rashi/sign index. Passing a
/// rashi index here would mis-evaluate the cancellation.
pub fn detect_mangal_dosha(
    mars_house_from_lagna: usize,
    mars_house_from_moon: usize,
    mars_house_from_venus: usize,
    mars_nakshatra_index: usize,
    jupiter_aspects_mars: bool,
    mars_in_own_sign: bool,
    mars_in_exaltation: bool,
) -> Dosha {
    // Mars in houses 1, 2, 4, 7, 8, 12 from Lagna/Moon/Venus
    // Note: house 2 is included per South Indian tradition; some texts use only 1, 4, 7, 8, 12
    let dosha_houses = [1, 2, 4, 7, 8, 12];

    let from_lagna = dosha_houses.contains(&mars_house_from_lagna);
    let from_moon = dosha_houses.contains(&mars_house_from_moon);
    let from_venus = dosha_houses.contains(&mars_house_from_venus);

    let count = [from_lagna, from_moon, from_venus]
        .iter()
        .filter(|&&x| x)
        .count();

    if count == 0 {
        return Dosha {
            name: "Mangal Dosha",
            present: false,
            severity: DoshaSeverity::None,
            from_chart: "None",
            cancellations: vec![],
        };
    }

    let mut cancellations = Vec::new();

    if mars_in_own_sign {
        cancellations.push("Mars in own sign (Aries/Scorpio)");
    }
    if mars_in_exaltation {
        cancellations.push("Mars in exaltation (Capricorn)");
    }
    if jupiter_aspects_mars {
        cancellations.push("Jupiter aspects Mars");
    }
    // Mangal Dosha is cancelled when Mars occupies one of three nakshatras
    // (0-indexed): Mrigashira=4, Chitra=13, Dhanishta=22. This must key off the
    // exact nakshatra index, NOT a coarse rashi proxy.
    if matches!(mars_nakshatra_index, 4 | 13 | 22) {
        cancellations.push("Mars in Mrigashira/Chitra/Dhanishta nakshatra");
    }

    let severity = if !cancellations.is_empty() {
        DoshaSeverity::Mild
    } else {
        match count {
            3 => DoshaSeverity::Severe,
            2 => DoshaSeverity::Moderate,
            _ => DoshaSeverity::Moderate,
        }
    };

    let from_chart = match (from_lagna, from_moon, from_venus) {
        (true, true, true) => "Lagna + Moon + Venus",
        (true, true, false) => "Lagna + Moon",
        (true, false, true) => "Lagna + Venus",
        (false, true, true) => "Moon + Venus",
        (true, false, false) => "Lagna",
        (false, true, false) => "Moon",
        (false, false, true) => "Venus",
        _ => "None",
    };

    Dosha {
        name: "Mangal Dosha",
        present: true,
        severity,
        from_chart,
        cancellations,
    }
}

/// Detect Kaal Sarpa Dosha / Kaal Amrit Yoga.
///
/// Conditions:
/// - All 7 visible planets (Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn)
///   must be hemmed between Rahu and Ketu on one side of the axis.
/// - If even one planet lies outside the Rahu-Ketu axis, no dosha forms.
/// - Rahu-to-Ketu direction = Kaal Sarpa Dosha (ascending hemisphere).
/// - Ketu-to-Rahu direction = Kaal Amrit Yoga (descending hemisphere).
/// - 12 types named by Rahu's house: Anant, Kulik, Vasuki, Shankhpal,
///   Padma, Mahapadma, Takshak, Karkotak, Shankhachud, Ghatak, Vishdhar, Sheshnag.
/// - A planet conjunct Rahu/Ketu partially breaks the axis (reduced severity).
pub fn detect_kaal_sarpa(
    rahu_house: usize,
    ketu_house: usize,
    planet_houses: &[usize], // houses of Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn
) -> Dosha {
    // All 7 planets must be between Rahu and Ketu (within one hemisphere)
    let rahu = rahu_house;
    let ketu = ketu_house;

    let all_between = planet_houses.iter().all(|&h| {
        if rahu < ketu {
            h >= rahu && h <= ketu
        } else {
            h >= rahu || h <= ketu
        }
    });

    let all_between_reverse = planet_houses.iter().all(|&h| {
        if ketu < rahu {
            h >= ketu && h <= rahu
        } else {
            h >= ketu || h <= rahu
        }
    });

    let is_kaal_sarpa = all_between;
    let is_kaal_amrit = all_between_reverse && !is_kaal_sarpa;

    if is_kaal_sarpa || is_kaal_amrit {
        let ksd_type = match rahu_house {
            1 => "Anant",
            2 => "Kulik",
            3 => "Vasuki",
            4 => "Shankhpal",
            5 => "Padma",
            6 => "Mahapadma",
            7 => "Takshak",
            8 => "Karkotak",
            9 => "Shankhachud",
            10 => "Ghatak",
            11 => "Vishdhar",
            12 => "Sheshnag",
            _ => "Unknown",
        };

        let any_conjunct_node = planet_houses.iter().any(|&h| h == rahu || h == ketu);

        Dosha {
            name: if is_kaal_sarpa {
                "Kaal Sarpa Dosha"
            } else {
                "Kaal Amrit Yoga"
            },
            present: true,
            severity: if any_conjunct_node {
                DoshaSeverity::Moderate
            } else {
                DoshaSeverity::Severe
            },
            from_chart: ksd_type,
            cancellations: if any_conjunct_node {
                vec!["Planet conjunct Rahu/Ketu breaks the axis (partial KSD)"]
            } else {
                vec![]
            },
        }
    } else {
        Dosha {
            name: "Kaal Sarpa Dosha",
            present: false,
            severity: DoshaSeverity::None,
            from_chart: "None",
            cancellations: vec![],
        }
    }
}

pub fn detect_pitra_dosha(
    sun_house: usize,
    saturn_house: usize,
    rahu_house: usize,
    ninth_lord_house: usize,
    ninth_lord_debilitated: bool,
) -> Dosha {
    // Dusthana houses (6, 8, 12 from Lagna) are classically the houses of
    // affliction; the 9th lord (Pitri-karaka house lord) placed there is a
    // recognized Pitra Dosha trigger independent of debilitation.
    const DUSTHANA: [usize; 3] = [6, 8, 12];

    let mut triggers = Vec::new();

    if sun_house == 9 && saturn_house == 9 {
        triggers.push("Sun-Saturn conjunction in 9th house");
    }
    if rahu_house == 9 {
        triggers.push("Rahu in 9th house");
    }
    if ninth_lord_debilitated {
        triggers.push("9th lord debilitated");
    }
    if DUSTHANA.contains(&ninth_lord_house) {
        triggers.push("9th lord in a dusthana (6th/8th/12th house)");
    }
    if saturn_house == 9 {
        triggers.push("Saturn in 9th house");
    }

    Dosha {
        name: "Pitra Dosha",
        present: !triggers.is_empty(),
        severity: if triggers.len() >= 2 {
            DoshaSeverity::Severe
        } else if !triggers.is_empty() {
            DoshaSeverity::Moderate
        } else {
            DoshaSeverity::None
        },
        from_chart: if !triggers.is_empty() {
            "9th house affliction"
        } else {
            "None"
        },
        cancellations: triggers.into_iter().collect(),
    }
}

// ---------------------------------------------------------------------------
// Combustion (Asta / Moudhya) detection — Vedic orbs
// ---------------------------------------------------------------------------

/// Check whether a planet is combust (Asta/Moudhya) per Vedic orbs.
///
/// When a planet's ecliptic longitude is within the combustion orb of the
/// Sun, the planet is considered combust and loses strength.
///
/// Orbs follow the standard Vedic (Surya Siddhanta / BPHS) convention:
///
/// | Planet  | Direct orb | Retrograde orb |
/// |---------|-----------|---------------|
/// | Moon    | 12°       | 12°           |
/// | Mars    | 17°       | 17°           |
/// | Mercury | 14°       | 12°           |
/// | Jupiter | 11°       | 11°           |
/// | Venus   | 10°       |  8°           |
/// | Saturn  | 15°       | 15°           |
///
/// * `planet` — planet name (Sun/Rahu/Ketu always return `false`).
/// * `planet_lon` — planet's ecliptic longitude in degrees.
/// * `sun_lon` — Sun's ecliptic longitude in degrees.
///
/// For planets that have different retrograde orbs (Mercury, Venus), pass
/// `is_retrograde = true` to use the narrower orb.
pub fn is_combust(planet: &str, planet_lon: f64, sun_lon: f64) -> bool {
    is_combust_with_retrograde(planet, planet_lon, sun_lon, false)
}

/// Like [`is_combust`] but with explicit retrograde flag for Mercury/Venus.
pub fn is_combust_with_retrograde(
    planet: &str,
    planet_lon: f64,
    sun_lon: f64,
    is_retrograde: bool,
) -> bool {
    let orb = match (planet, is_retrograde) {
        ("Moon", _) => 12.0,
        ("Mars", _) => 17.0,
        ("Mercury", false) => 14.0,
        ("Mercury", true) => 12.0,
        ("Jupiter", _) => 11.0,
        ("Venus", false) => 10.0,
        ("Venus", true) => 8.0,
        ("Saturn", _) => 15.0,
        // Sun, Rahu, Ketu, and unknown planets cannot be combust
        _ => return false,
    };

    let diff = (planet_lon - sun_lon).rem_euclid(360.0);
    let separation = if diff > 180.0 { 360.0 - diff } else { diff };
    separation < orb
}

/// Detailed combustion status for a planet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombustionStatus {
    /// Planet is not combust — free from Sun's influence.
    Free,
    /// Planet is combust within the standard orb.
    Combust,
    /// Planet is deeply combust (within half the standard orb).
    DeeplyCombust,
}

/// Return detailed combustion status for a planet.
pub fn combustion_status(
    planet: &str,
    planet_lon: f64,
    sun_lon: f64,
    is_retrograde: bool,
) -> CombustionStatus {
    let orb = match (planet, is_retrograde) {
        ("Moon", _) => 12.0,
        ("Mars", _) => 17.0,
        ("Mercury", false) => 14.0,
        ("Mercury", true) => 12.0,
        ("Jupiter", _) => 11.0,
        ("Venus", false) => 10.0,
        ("Venus", true) => 8.0,
        ("Saturn", _) => 15.0,
        _ => return CombustionStatus::Free,
    };

    let diff = (planet_lon - sun_lon).rem_euclid(360.0);
    let separation = if diff > 180.0 { 360.0 - diff } else { diff };

    if separation < orb / 2.0 {
        CombustionStatus::DeeplyCombust
    } else if separation < orb {
        CombustionStatus::Combust
    } else {
        CombustionStatus::Free
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mangal_dosha_from_7th() {
        // 4th positional arg is now Mars' nakshatra index; 0 = Ashwini (no cancel).
        let d = detect_mangal_dosha(7, 3, 5, 0, false, false, false);
        assert!(d.present);
        assert_eq!(d.severity, DoshaSeverity::Moderate);
    }

    #[test]
    fn no_mangal_dosha() {
        let d = detect_mangal_dosha(5, 5, 5, 0, false, false, false);
        assert!(!d.present);
    }

    #[test]
    fn mangal_dosha_cancelled_by_own_sign() {
        let d = detect_mangal_dosha(7, 3, 5, 0, false, true, false);
        assert!(d.present);
        assert_eq!(d.severity, DoshaSeverity::Mild);
        assert!(!d.cancellations.is_empty());
    }

    #[test]
    fn severe_mangal_dosha() {
        // Nakshatra index 3 = Rohini — not a cancelling nakshatra → stays Severe.
        let d = detect_mangal_dosha(7, 8, 12, 3, false, false, false);
        assert!(d.present);
        assert_eq!(d.severity, DoshaSeverity::Severe);
    }

    #[test]
    fn mangal_cancellation_uses_exact_nakshatra_not_rashi() {
        const NAK_SPAN: f64 = 360.0 / 27.0; // 13.333...°

        // 35° → nakshatra index 2 (Krittika), which the OLD rashi-proxy code
        // (35° is in Taurus, rashi 1) would have wrongly treated as a cancelling
        // value. It is NOT one of {Mrigashira=4, Chitra=13, Dhanishta=22}.
        let krittika_idx = (35.0_f64 / NAK_SPAN) as usize;
        assert_eq!(krittika_idx, 2, "35° must resolve to Krittika (idx 2)");
        let d_no_cancel = detect_mangal_dosha(7, 3, 5, krittika_idx, false, false, false);
        assert!(d_no_cancel.present);
        assert!(
            d_no_cancel
                .cancellations
                .iter()
                .all(|c| !c.contains("nakshatra")),
            "Krittika must NOT trigger a nakshatra cancellation"
        );

        // 65° → nakshatra index 4 (Mrigashira), a genuine cancelling nakshatra.
        let mrigashira_idx = (65.0_f64 / NAK_SPAN) as usize;
        assert_eq!(mrigashira_idx, 4, "65° must resolve to Mrigashira (idx 4)");
        let d_cancel = detect_mangal_dosha(7, 3, 5, mrigashira_idx, false, false, false);
        assert!(d_cancel.present);
        assert_eq!(d_cancel.severity, DoshaSeverity::Mild);
        assert!(
            d_cancel
                .cancellations
                .iter()
                .any(|c| c.contains("Mrigashira")),
            "Mrigashira must trigger the nakshatra cancellation"
        );

        // Backward-compat HAZARD (pinned, not silently safe): a LEGACY caller
        // that wrongly passes a RASHI index (0..11) has it interpreted as a
        // nakshatra index. Rashi 4 (Simha/Leo) collides with nakshatra 4
        // (Mrigashira) and mis-fires a cancellation. This asserts the hazard so
        // the contract is explicit — callers MUST pass the 0-based NAKSHATRA
        // index (see `detect_mangal_dosha`'s doc), never a rashi index.
        let d_rashi_misuse = detect_mangal_dosha(7, 3, 5, /* rashi! */ 4, false, false, false);
        assert!(
            d_rashi_misuse
                .cancellations
                .iter()
                .any(|c| c.contains("Mrigashira")),
            "rashi index 4 is (wrongly) read as Mrigashira — confirms the param is a NAKSHATRA index"
        );
    }

    #[test]
    fn kaal_sarpa_present() {
        // Rahu in 1, Ketu in 7, all planets in houses 1-7
        let d = detect_kaal_sarpa(1, 7, &[2, 3, 4, 5, 6, 3, 4]);
        assert!(d.present);
        assert_eq!(d.name, "Kaal Sarpa Dosha");
    }

    #[test]
    fn kaal_sarpa_absent() {
        // Rahu in 1, Ketu in 7, one planet in house 10 (outside axis)
        let d = detect_kaal_sarpa(1, 7, &[2, 3, 10, 5, 6, 3, 4]);
        assert!(!d.present);
    }

    #[test]
    fn pitra_dosha_rahu_in_9th() {
        let d = detect_pitra_dosha(1, 4, 9, 5, false);
        assert!(d.present);
    }

    // -----------------------------------------------------------------------
    // 9th-lord resolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn traditional_sign_lord_exhaustive_table() {
        let expected = [
            (0, Planet::Mars),     // Aries
            (1, Planet::Venus),    // Taurus
            (2, Planet::Mercury),  // Gemini
            (3, Planet::Moon),     // Cancer
            (4, Planet::Sun),      // Leo
            (5, Planet::Mercury),  // Virgo
            (6, Planet::Venus),    // Libra
            (7, Planet::Mars),     // Scorpio
            (8, Planet::Jupiter),  // Sagittarius
            (9, Planet::Saturn),   // Capricorn
            (10, Planet::Saturn),  // Aquarius
            (11, Planet::Jupiter), // Pisces
        ];
        for (sign, planet) in expected {
            assert_eq!(
                traditional_sign_lord(sign),
                planet,
                "sign {sign} must be ruled by {planet:?}"
            );
        }
    }

    /// Standard 7-classical-planet sign layout used by the fixtures below.
    /// (Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn) sign indices.
    fn planet_signs(signs: [usize; 7]) -> Vec<(Planet, usize)> {
        let planets = [
            Planet::Sun,
            Planet::Moon,
            Planet::Mars,
            Planet::Mercury,
            Planet::Jupiter,
            Planet::Venus,
            Planet::Saturn,
        ];
        planets.into_iter().zip(signs).collect()
    }

    #[test]
    fn ninth_lord_house_resolves_correctly_for_aries_ascendant() {
        // Asc = Aries (0) → 9th sign = Sagittarius (8) → ruler Jupiter.
        // Jupiter placed in Cancer (3) → house from Lagna = 4.
        let signs = planet_signs([0, 0, 0, 0, 3, 0, 0]);
        let house = resolve_ninth_lord_house(0, &signs).unwrap();
        assert_eq!(house, 4);
    }

    #[test]
    fn ninth_lord_house_resolves_correctly_for_virgo_ascendant() {
        // Asc = Virgo (5) → 9th sign = Taurus (1) → ruler Venus.
        // Venus placed in Gemini (2) → house from Lagna = 10.
        let signs = planet_signs([0, 0, 0, 0, 0, 2, 0]);
        let house = resolve_ninth_lord_house(5, &signs).unwrap();
        assert_eq!(house, 10);
    }

    #[test]
    fn ninth_lord_house_differs_across_ascendants() {
        // Same planetary sign placements, different ascendants must generally
        // resolve to a different lord and a different house.
        let signs = planet_signs([1, 4, 7, 2, 9, 6, 11]);
        let house_aries = resolve_ninth_lord_house(0, &signs).unwrap(); // 9th=Sagittarius->Jupiter@sign9
        let house_cancer = resolve_ninth_lord_house(3, &signs).unwrap(); // 9th=Pisces->Jupiter@sign9
        let house_libra = resolve_ninth_lord_house(6, &signs).unwrap(); // 9th=Gemini->Mercury@sign2
        assert_ne!(house_aries, house_libra);
        assert_ne!(house_cancer, house_libra);
    }

    #[test]
    fn regression_saturn_is_not_implicitly_the_ninth_lord() {
        // Asc = Cancer (3) → 9th sign = Pisces (11) → real ruler is Jupiter,
        // NOT Saturn. The old placeholder used Saturn's own house as a stand-in
        // for the 9th-lord house; construct a chart where Saturn sits in a
        // dusthana (would have wrongly triggered "9th lord in dusthana") while
        // the real 9th lord (Jupiter) does NOT, proving the old shortcut would
        // have produced a false positive that the real resolution avoids.
        let signs_saturn_dusthana = planet_signs([0, 0, 0, 0, 3, 0, /* Saturn */ 8]);
        // Saturn sign 8, Asc sign 3 -> Saturn house = (8-3)%12+1 = 6 (dusthana).
        let saturn_house = (8 + 12 - 3) % 12 + 1;
        assert_eq!(saturn_house, 6, "sanity: Saturn's own house is a dusthana");

        // Real 9th lord (Jupiter, sign 3) house from Lagna = (3-3)%12+1 = 1 (not dusthana).
        let real_ninth_lord_house = resolve_ninth_lord_house(3, &signs_saturn_dusthana).unwrap();
        assert_eq!(
            real_ninth_lord_house, 1,
            "real 9th lord (Jupiter) house must be 1, not Saturn's house (6)"
        );
        assert_ne!(
            real_ninth_lord_house, saturn_house,
            "the real 9th-lord house must differ from Saturn's own house in this chart"
        );

        // Using the OLD placeholder (Saturn's own house) as ninth_lord_house
        // wrongly triggers the dusthana condition:
        let d_old_placeholder = detect_pitra_dosha(1, saturn_house, 2, saturn_house, false);
        assert!(
            d_old_placeholder.present,
            "old Saturn-substitution placeholder would wrongly flag Pitra Dosha here"
        );

        // Using the REAL resolved 9th-lord house correctly does NOT trigger it
        // (Saturn is not in the 9th house either, so no other trigger fires):
        let d_real = detect_pitra_dosha(1, saturn_house, 2, real_ninth_lord_house, false);
        assert!(
            !d_real.present,
            "real 9th-lord resolution must NOT flag Pitra Dosha in this chart"
        );
    }

    #[test]
    fn pitra_dosha_positive_ninth_lord_in_dusthana() {
        // 9th lord placed in the 8th house (dusthana) genuinely triggers.
        let d = detect_pitra_dosha(1, 2, 3, 8, false);
        assert!(d.present);
        assert!(
            d.cancellations.iter().any(|c| c.contains("dusthana")),
            "trigger reason must cite the dusthana placement"
        );
    }

    #[test]
    fn pitra_dosha_negative_no_affliction() {
        // No affliction: Sun/Saturn not in 9th, Rahu not in 9th, 9th lord
        // in a benign house (5th), not debilitated.
        let d = detect_pitra_dosha(2, 3, 4, 5, false);
        assert!(!d.present);
        assert_eq!(d.severity, DoshaSeverity::None);
    }

    #[test]
    fn pitra_dosha_deterministic_repeatability() {
        let d1 = detect_pitra_dosha(1, 4, 9, 6, true);
        let d2 = detect_pitra_dosha(1, 4, 9, 6, true);
        assert_eq!(d1.present, d2.present);
        assert_eq!(d1.severity, d2.severity);
        assert_eq!(d1.cancellations, d2.cancellations);
    }

    #[test]
    fn no_pitra_dosha() {
        let d = detect_pitra_dosha(1, 4, 3, 5, false);
        assert!(!d.present);
    }

    // -----------------------------------------------------------------------
    // Combustion tests
    // -----------------------------------------------------------------------

    #[test]
    fn moon_combust_within_12_deg() {
        assert!(is_combust("Moon", 100.0, 110.0)); // 10° < 12°
        assert!(!is_combust("Moon", 100.0, 115.0)); // 15° > 12°
    }

    #[test]
    fn mars_combust_within_17_deg() {
        assert!(is_combust("Mars", 50.0, 65.0)); // 15° < 17°
        assert!(!is_combust("Mars", 50.0, 70.0)); // 20° > 17°
    }

    #[test]
    fn mercury_combust_14_direct_12_retrograde() {
        // Direct: orb = 14°
        assert!(is_combust_with_retrograde("Mercury", 100.0, 113.0, false)); // 13° < 14°
        // Retrograde: orb = 12°
        assert!(!is_combust_with_retrograde("Mercury", 100.0, 113.0, true)); // 13° > 12°
        assert!(is_combust_with_retrograde("Mercury", 100.0, 111.0, true)); // 11° < 12°
    }

    #[test]
    fn venus_combust_10_direct_8_retrograde() {
        // Direct: orb = 10°
        assert!(is_combust_with_retrograde("Venus", 200.0, 209.0, false)); // 9° < 10°
        // Retrograde: orb = 8°
        assert!(!is_combust_with_retrograde("Venus", 200.0, 209.0, true)); // 9° > 8°
        assert!(is_combust_with_retrograde("Venus", 200.0, 207.0, true)); // 7° < 8°
    }

    #[test]
    fn jupiter_combust_within_11_deg() {
        assert!(is_combust("Jupiter", 300.0, 310.0)); // 10° < 11°
        assert!(!is_combust("Jupiter", 300.0, 315.0)); // 15° > 11°
    }

    #[test]
    fn saturn_combust_within_15_deg() {
        assert!(is_combust("Saturn", 0.0, 14.0)); // 14° < 15°
        assert!(!is_combust("Saturn", 0.0, 16.0)); // 16° > 15°
    }

    #[test]
    fn sun_never_combust() {
        assert!(!is_combust("Sun", 100.0, 100.0));
    }

    #[test]
    fn rahu_ketu_never_combust() {
        assert!(!is_combust("Rahu", 100.0, 100.0));
        assert!(!is_combust("Ketu", 100.0, 100.0));
    }

    #[test]
    fn combustion_wraps_around_360() {
        // Mars at 5°, Sun at 355° → separation = 10° < 17°
        assert!(is_combust("Mars", 5.0, 355.0));
    }

    #[test]
    fn combustion_status_levels() {
        // Saturn orb = 15°, deeply combust < 7.5°
        assert_eq!(
            combustion_status("Saturn", 100.0, 105.0, false),
            CombustionStatus::DeeplyCombust
        );
        assert_eq!(
            combustion_status("Saturn", 100.0, 112.0, false),
            CombustionStatus::Combust
        );
        assert_eq!(
            combustion_status("Saturn", 100.0, 120.0, false),
            CombustionStatus::Free
        );
    }

    #[test]
    fn is_combust_default_is_direct() {
        // is_combust uses direct orb (14° for Mercury)
        assert!(is_combust("Mercury", 100.0, 113.0)); // 13° < 14° (direct)
    }
}
