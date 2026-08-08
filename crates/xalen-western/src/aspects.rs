// Portions of this file (the Kepler aspect angles/orbs on `AspectType`, the
// `AspectConfig`/`AspectResult` orb-and-strength model, out-of-sign detection,
// and the applying/separating algorithm in `detect_aspect`) are adapted from
// Anonyfox/celestine (MIT License), commit 954d63315ec00d29ba4becaef3f6a101497946b7:
//   - src/aspects/constants.ts (angles, default orbs, sign-separation tables)
//   - src/aspects/angular-separation.ts (normalize/separation/sign-index helpers)
//   - src/aspects/orbs.ts (orb resolution, linear strength model)
//   - src/aspects/aspect-detection.ts (out-of-sign check, applying/separating)
// See docs/THIRD_PARTY_SOURCES.md for full provenance. Ported to idiomatic
// Rust and adapted to XALEN's existing `AspectType`/`Aspect`/`find_all_aspects`
// conventions rather than transliterated; XALEN's pre-existing 11-aspect-type
// catalogue, default orbs, and `find_aspect`/`find_all_aspects` signatures are
// unchanged for backward compatibility — this is additive attribution, not a
// replacement of XALEN's own Apache-2.0 licensing on this file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Type of angular relationship between two celestial bodies.
pub enum AspectType {
    Conjunction,
    Opposition,
    Trine,
    Square,
    Sextile,
    SemiSextile,
    Quincunx,
    SemiSquare,
    Sesquiquadrate,
    Quintile,
    BiQuintile,
    /// Kepler aspect: 360°/7 ≈ 51.43° ("Harmonices Mundi", 1619).
    Septile,
    /// Kepler aspect: 360°/9 = 40°.
    Novile,
    /// Kepler aspect: 360°/10 = 36°.
    Decile,
}

impl AspectType {
    /// Return the exact angle in degrees for this aspect type.
    pub fn angle_deg(&self) -> f64 {
        match self {
            AspectType::Conjunction => 0.0,
            AspectType::Opposition => 180.0,
            AspectType::Trine => 120.0,
            AspectType::Square => 90.0,
            AspectType::Sextile => 60.0,
            AspectType::SemiSextile => 30.0,
            AspectType::Quincunx => 150.0,
            AspectType::SemiSquare => 45.0,
            AspectType::Sesquiquadrate => 135.0,
            AspectType::Quintile => 72.0,
            AspectType::BiQuintile => 144.0,
            AspectType::Septile => 360.0 / 7.0,
            AspectType::Novile => 40.0,
            AspectType::Decile => 36.0,
        }
    }

    /// Return the default orb (tolerance) in degrees for this aspect.
    ///
    /// The 11 pre-existing variants keep XALEN's original values unchanged
    /// (backward compatibility for existing callers). The 3 Kepler variants
    /// added in this package use Celestine's documented default (1°), since
    /// XALEN had no prior value for them.
    pub fn default_orb_deg(&self) -> f64 {
        match self {
            AspectType::Conjunction | AspectType::Opposition => 8.0,
            AspectType::Trine | AspectType::Square => 7.0,
            AspectType::Sextile => 5.0,
            AspectType::SemiSextile | AspectType::Quincunx => 2.0,
            AspectType::SemiSquare | AspectType::Sesquiquadrate => 2.0,
            AspectType::Quintile | AspectType::BiQuintile => 1.5,
            AspectType::Septile | AspectType::Novile | AspectType::Decile => 1.0,
        }
    }

    /// Returns `true` for Ptolemaic (major) aspects.
    pub fn is_major(&self) -> bool {
        matches!(
            self,
            AspectType::Conjunction
                | AspectType::Opposition
                | AspectType::Trine
                | AspectType::Square
                | AspectType::Sextile
        )
    }

    /// Returns `true` for the three Kepler (harmonic) aspects.
    pub fn is_kepler(&self) -> bool {
        matches!(
            self,
            AspectType::Septile | AspectType::Novile | AspectType::Decile
        )
    }

    /// The five major (Ptolemaic) aspect types.
    pub const MAJOR: &[AspectType] = &[
        AspectType::Conjunction,
        AspectType::Sextile,
        AspectType::Square,
        AspectType::Trine,
        AspectType::Opposition,
    ];

    /// The three Kepler (harmonic) aspects: septile, novile, decile.
    pub const KEPLER: &[AspectType] =
        &[AspectType::Septile, AspectType::Novile, AspectType::Decile];

    /// All 11 pre-existing aspect types (major + minor), UNCHANGED from
    /// before this package — existing callers of `find_all_aspects(_, ALL)`
    /// keep exactly the same behavior. Use [`AspectType::ALL_14`] to include
    /// the 3 Kepler aspects added in this package.
    pub const ALL: &[AspectType] = &[
        AspectType::Conjunction,
        AspectType::SemiSextile,
        AspectType::SemiSquare,
        AspectType::Sextile,
        AspectType::Quintile,
        AspectType::Square,
        AspectType::Trine,
        AspectType::Sesquiquadrate,
        AspectType::BiQuintile,
        AspectType::Quincunx,
        AspectType::Opposition,
    ];

    /// All 14 aspect types: the 11 in [`AspectType::ALL`] plus the 3 Kepler
    /// aspects (septile, novile, decile).
    pub const ALL_14: &[AspectType] = &[
        AspectType::Conjunction,
        AspectType::SemiSextile,
        AspectType::SemiSquare,
        AspectType::Sextile,
        AspectType::Quintile,
        AspectType::Square,
        AspectType::Trine,
        AspectType::Sesquiquadrate,
        AspectType::BiQuintile,
        AspectType::Quincunx,
        AspectType::Opposition,
        AspectType::Septile,
        AspectType::Novile,
        AspectType::Decile,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Whether an aspect is applying (tightening), separating, or exact.
pub enum AspectDirection {
    Applying,
    Separating,
    Exact,
}

/// Canonical name for [`AspectDirection`] used by the newer, richer
/// [`AspectResult`]/[`detect_aspect`] API added in this package. `AspectPhase`
/// is the same type as `AspectDirection` (not a duplicate model) — XALEN
/// already had this concept under the `AspectDirection` name, so rather than
/// introduce a second enum this is a plain alias, matching the task's
/// instruction to adapt names to an existing XALEN-equivalent concept.
pub type AspectPhase = AspectDirection;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A detected aspect between two bodies with orb and direction.
pub struct Aspect {
    pub aspect_type: AspectType,
    pub body1: String,
    pub body2: String,
    pub orb_deg: f64,
    pub direction: AspectDirection,
    pub exact_deg: f64,
}

/// Compute the shortest angular distance between two longitudes in degrees.
pub fn angular_distance(lon1_deg: f64, lon2_deg: f64) -> f64 {
    let diff = (lon2_deg - lon1_deg).rem_euclid(360.0);
    if diff > 180.0 { 360.0 - diff } else { diff }
}

// =============================================================================
// Configurable orbs, strength, out-of-sign detection, and the richer
// AspectResult API (adapted from celestine's aspects/{types,constants,orbs,
// aspect-detection}.ts — see file-header provenance comment and
// docs/THIRD_PARTY_SOURCES.md).
// =============================================================================

/// Configuration for the richer [`detect_aspect`]/[`find_all_aspects_ex`] API.
///
/// This is a growable config struct (not a flat function signature) so future
/// body-specific/luminary/angle orb modifiers can be added as new optional
/// fields without an API break — exactly the requirement this package was
/// built to satisfy. All fields have sensible defaults via [`Default`]/
/// [`AspectConfig::new`], so existing call sites can pass
/// `&AspectConfig::default()` and get the same 5 major aspects XALEN's
/// pre-existing `find_all_aspects(_, AspectType::MAJOR)` detects.
#[derive(Debug, Clone)]
pub struct AspectConfig {
    /// Which aspect types to detect. Defaults to the 5 Ptolemaic majors.
    pub aspect_types: Vec<AspectType>,
    /// Per-aspect-type orb overrides (degrees). Falls back to
    /// [`AspectType::default_orb_deg`] for any type not present here.
    pub orb_overrides: HashMap<AspectType, f64>,
    /// Whether to compute and report out-of-sign (dissociate) status.
    /// Does not filter results; see [`AspectConfig::exclude_out_of_sign`].
    pub include_out_of_sign: bool,
    /// If `true`, drop out-of-sign aspects from results entirely (rather than
    /// just flagging them). Default `false` (report, don't filter).
    pub exclude_out_of_sign: bool,
    /// Strength penalty (0.0-1.0) applied to out-of-sign aspects:
    /// `final_strength = strength * (1 - penalty)`. Default 0 (no penalty).
    pub out_of_sign_penalty: f64,
    /// Minimum strength (0-100) an aspect must have to be included. Default 0.
    pub minimum_strength: f64,
    /// Whether to compute applying/separating (requires speed data on both
    /// bodies; falls back to `None` phase when speeds are unavailable
    /// regardless of this flag). Default `true`.
    pub include_applying: bool,
}

impl Default for AspectConfig {
    fn default() -> Self {
        Self {
            aspect_types: AspectType::MAJOR.to_vec(),
            orb_overrides: HashMap::new(),
            include_out_of_sign: true,
            exclude_out_of_sign: false,
            out_of_sign_penalty: 0.0,
            minimum_strength: 0.0,
            include_applying: true,
        }
    }
}

impl AspectConfig {
    /// Default configuration: the 5 Ptolemaic major aspects, XALEN's default
    /// orbs, out-of-sign reported (not filtered), no strength floor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder: set which aspect types to detect.
    pub fn with_aspect_types(mut self, types: &[AspectType]) -> Self {
        self.aspect_types = types.to_vec();
        self
    }

    /// Builder: override the orb for a single aspect type.
    pub fn with_orb(mut self, aspect_type: AspectType, orb_deg: f64) -> Self {
        self.orb_overrides.insert(aspect_type, orb_deg);
        self
    }

    /// Effective orb for `aspect_type`: the override if present, else
    /// [`AspectType::default_orb_deg`].
    pub fn orb_for(&self, aspect_type: AspectType) -> f64 {
        self.orb_overrides
            .get(&aspect_type)
            .copied()
            .unwrap_or_else(|| aspect_type.default_orb_deg())
    }
}

/// Richer detected-aspect result: everything [`Aspect`] has, plus normalized
/// strength and out-of-sign metadata. Kept as a separate type (rather than
/// adding fields to [`Aspect`]) so the pre-existing `Aspect`/`find_all_aspects`
/// pair is completely unaffected — no breaking change to that struct's shape
/// or the functions that already return it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AspectResult {
    pub aspect_type: AspectType,
    pub body1: String,
    pub body2: String,
    /// Actual angular separation between the two bodies (0-180°).
    pub separation_deg: f64,
    /// Absolute deviation from the exact aspect angle (always >= 0).
    pub deviation_deg: f64,
    /// Orb that was used for this detection (after any config override).
    pub orb_deg: f64,
    /// Normalized strength: 100 at exact, decreasing linearly to 0 at the
    /// orb boundary. Never negative; an aspect strictly outside orb is not
    /// returned at all rather than reported at 0.
    pub strength: f64,
    /// Applying/separating/exact, or `None` if speed data was unavailable
    /// for one or both bodies (or `include_applying` was `false`).
    pub phase: Option<AspectPhase>,
    /// `true` if this is a geometrically valid aspect where the two bodies
    /// are NOT in the sign relationship traditionally expected for this
    /// aspect type (a "dissociate" aspect).
    pub is_out_of_sign: bool,
}

/// Sign (zodiac-index) separations traditionally expected for each aspect
/// type, used by [`is_out_of_sign`]. Adapted from celestine's
/// `ASPECT_SIGN_SEPARATIONS` (aspects/constants.ts).
fn expected_sign_separations(aspect_type: AspectType) -> &'static [u8] {
    match aspect_type {
        AspectType::Conjunction => &[0],
        AspectType::Sextile => &[2, 10],
        AspectType::Square => &[3, 9],
        AspectType::Trine => &[4, 8],
        AspectType::Opposition => &[6],
        AspectType::SemiSextile => &[1, 11],
        AspectType::SemiSquare => &[1, 2, 10, 11],
        AspectType::Quintile => &[2, 3, 9, 10],
        AspectType::Sesquiquadrate => &[4, 5, 7, 8],
        AspectType::BiQuintile => &[4, 5, 7, 8],
        AspectType::Quincunx => &[5, 7],
        AspectType::Septile => &[1, 2, 10, 11],
        AspectType::Novile => &[1, 2, 10, 11],
        AspectType::Decile => &[1, 2, 10, 11],
    }
}

/// Zodiac sign index (0 = Aries .. 11 = Pisces) for an ecliptic longitude.
fn sign_index(lon_deg: f64) -> u8 {
    (lon_deg.rem_euclid(360.0) / 30.0) as u8 % 12
}

/// Forward sign separation from `lon1` to `lon2` (0-11), matching celestine's
/// `signSeparation`: `(sign2 - sign1) mod 12`, always non-negative.
fn sign_separation(lon1_deg: f64, lon2_deg: f64) -> u8 {
    let s1 = sign_index(lon1_deg) as i32;
    let s2 = sign_index(lon2_deg) as i32;
    (((s2 - s1) % 12) + 12) as u8 % 12
}

/// Determine whether an aspect between two longitudes is out-of-sign
/// (dissociate): geometrically within orb of `aspect_type`, but the two
/// bodies are not in the sign relationship traditionally expected for that
/// aspect (e.g. a trine-by-degree between Aries and Virgo rather than two
/// fire signs). Ported from celestine's `isOutOfSign`
/// (aspects/aspect-detection.ts).
pub fn is_out_of_sign(lon1_deg: f64, lon2_deg: f64, aspect_type: AspectType) -> bool {
    let sep = sign_separation(lon1_deg, lon2_deg);
    !expected_sign_separations(aspect_type).contains(&sep)
}

/// Normalized aspect strength: 100 at exact, decreasing linearly to 0 at the
/// orb boundary, floored at 0 beyond it. Ported from celestine's
/// `calculateStrength` (aspects/orbs.ts) — same linear decay model.
pub fn aspect_strength(deviation_deg: f64, orb_deg: f64) -> f64 {
    let deviation = deviation_deg.abs();
    if orb_deg <= 0.0 {
        return if deviation == 0.0 { 100.0 } else { 0.0 };
    }
    if deviation >= orb_deg {
        return 0.0;
    }
    100.0 * (1.0 - deviation / orb_deg)
}

/// Determine applying (tightening) vs. separating (widening) by comparing
/// today's deviation from exact to tomorrow's (a 1-day linear extrapolation
/// using the given speeds). Ported from celestine's `calculateIsApplying`
/// (aspects/aspect-detection.ts): applying iff tomorrow's deviation is
/// smaller than today's. Handles the 0°/360° wrap correctly because it goes
/// through [`angular_distance`], which always returns the shortest arc.
fn is_applying(
    lon1_deg: f64,
    lon2_deg: f64,
    speed1_deg_per_day: f64,
    speed2_deg_per_day: f64,
    aspect_angle_deg: f64,
) -> bool {
    let current_sep = angular_distance(lon1_deg, lon2_deg);
    let current_dev = (current_sep - aspect_angle_deg).abs();

    let future_lon1 = lon1_deg + speed1_deg_per_day;
    let future_lon2 = lon2_deg + speed2_deg_per_day;
    let future_sep = angular_distance(future_lon1, future_lon2);
    let future_dev = (future_sep - aspect_angle_deg).abs();

    future_dev < current_dev
}

/// Detect the aspect (if any) between two named, positioned bodies under the
/// given configuration. Returns the closest-matching aspect type when
/// multiple configured types could match the same separation (rare with
/// sane orbs), mirroring celestine's `findMatchingAspect` best-match rule.
///
/// This is the configurable counterpart to the pre-existing [`find_aspect`]:
/// [`find_aspect`] is unchanged and keeps working for existing callers:
/// this function is additive, not a replacement.
pub fn detect_aspect(
    body1: (&str, f64, Option<f64>),
    body2: (&str, f64, Option<f64>),
    config: &AspectConfig,
) -> Option<AspectResult> {
    let (name1, lon1, speed1) = body1;
    let (name2, lon2, speed2) = body2;

    let separation = angular_distance(lon1, lon2);

    // Best match = smallest deviation among configured aspect types within orb.
    let mut best: Option<(AspectType, f64, f64)> = None; // (type, deviation, orb)
    for &aspect_type in &config.aspect_types {
        let orb = config.orb_for(aspect_type);
        let deviation = (separation - aspect_type.angle_deg()).abs();
        if deviation <= orb && best.is_none_or(|(_, best_dev, _)| deviation < best_dev) {
            best = Some((aspect_type, deviation, orb));
        }
    }
    let (aspect_type, deviation, orb) = best?;

    let out_of_sign = is_out_of_sign(lon1, lon2, aspect_type);
    if out_of_sign && config.exclude_out_of_sign {
        return None;
    }

    let mut strength = aspect_strength(deviation, orb);
    if out_of_sign && config.out_of_sign_penalty > 0.0 {
        strength *= 1.0 - config.out_of_sign_penalty.clamp(0.0, 1.0);
    }
    if strength < config.minimum_strength {
        return None;
    }

    let phase = if config.include_applying {
        match (speed1, speed2) {
            (Some(s1), Some(s2)) => {
                if deviation < 0.01 {
                    Some(AspectPhase::Exact)
                } else if is_applying(lon1, lon2, s1, s2, aspect_type.angle_deg()) {
                    Some(AspectPhase::Applying)
                } else {
                    Some(AspectPhase::Separating)
                }
            }
            _ => None,
        }
    } else {
        None
    };

    Some(AspectResult {
        aspect_type,
        body1: name1.to_string(),
        body2: name2.to_string(),
        separation_deg: separation,
        deviation_deg: deviation,
        orb_deg: orb,
        strength,
        phase,
        is_out_of_sign: out_of_sign,
    })
}

/// Find all aspects among a set of named, positioned bodies under the given
/// configuration. Checks every unique pair once (no self-aspects, no
/// duplicate pairs), and returns results in a deterministic order: sorted by
/// strength descending, then by `(body1, body2)` ascending to break ties
/// stably (celestine's own sort is strength-only, which leaves same-strength
/// ties in insertion order; XALEN's additional tie-break gives fully
/// deterministic output regardless of input ordering — an intentional,
/// documented improvement, not a bug-for-bug port; see
/// docs/THIRD_PARTY_SOURCES.md for this and other intentional differences).
///
/// Additive alongside the pre-existing [`find_all_aspects`], which is
/// unchanged.
pub fn find_all_aspects_ex(
    bodies: &[(String, f64, Option<f64>)],
    config: &AspectConfig,
) -> Vec<AspectResult> {
    let mut results = Vec::new();
    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            let b1 = (bodies[i].0.as_str(), bodies[i].1, bodies[i].2);
            let b2 = (bodies[j].0.as_str(), bodies[j].1, bodies[j].2);
            if let Some(result) = detect_aspect(b1, b2, config) {
                results.push(result);
            }
        }
    }
    results.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.body1.cmp(&b.body1))
            .then_with(|| a.body2.cmp(&b.body2))
    });
    results
}

/// Check if two bodies form a specific aspect within the given orb.
pub fn find_aspect(
    lon1_deg: f64,
    lon2_deg: f64,
    speed1: f64,
    speed2: f64,
    aspects_to_check: &[AspectType],
    orb_multiplier: f64,
) -> Option<Aspect> {
    let dist = angular_distance(lon1_deg, lon2_deg);

    for &aspect_type in aspects_to_check {
        let target = aspect_type.angle_deg();
        let orb = aspect_type.default_orb_deg() * orb_multiplier;
        let diff = (dist - target).abs();

        if diff <= orb {
            let direction = if diff < 0.01 {
                AspectDirection::Exact
            } else {
                let _relative_speed = speed2 - speed1;
                let future_dist =
                    angular_distance(lon1_deg + speed1 * 0.1, lon2_deg + speed2 * 0.1);
                let future_diff = (future_dist - target).abs();
                if future_diff < diff {
                    AspectDirection::Applying
                } else {
                    AspectDirection::Separating
                }
            };

            return Some(Aspect {
                aspect_type,
                body1: String::new(),
                body2: String::new(),
                orb_deg: diff,
                direction,
                exact_deg: dist,
            });
        }
    }
    None
}

/// Find all aspects between two bodies across all aspect types.
pub fn find_all_aspects(
    positions: &[(String, f64, f64)], // (name, longitude_deg, speed_deg_day)
    aspects_to_check: &[AspectType],
) -> Vec<Aspect> {
    let mut results = Vec::new();
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            if let Some(mut asp) = find_aspect(
                positions[i].1,
                positions[j].1,
                positions[i].2,
                positions[j].2,
                aspects_to_check,
                1.0,
            ) {
                asp.body1 = positions[i].0.clone();
                asp.body2 = positions[j].0.clone();
                results.push(asp);
            }
        }
    }
    results
}

/// Find all Julian Dates when a transiting planet forms an exact aspect
/// to a fixed natal longitude.
///
/// This uses a coarse scan + bisection refinement strategy (same approach as
/// `xalen_ephem::event_search::find_crossing`).
///
/// * `natal_lon` — the natal planet's ecliptic longitude in degrees (fixed).
/// * `transit_fn` — a closure that returns the transiting planet's longitude
///   at a given JD: `transit_fn(jd) -> longitude_deg`.
/// * `aspect` — the aspect angle in degrees (0 = conjunction, 180 = opposition,
///   120 = trine, etc.).
/// * `jd_start` / `jd_end` — search window (Julian Dates).
///
/// Returns a `Vec<f64>` of JDs where the aspect is exact (to ~1e-8 day
/// precision, roughly 1 ms).
pub fn find_transit_aspect(
    natal_lon: f64,
    transit_fn: impl Fn(f64) -> f64,
    aspect: f64,
    jd_start: f64,
    jd_end: f64,
) -> Vec<f64> {
    if jd_start >= jd_end {
        return Vec::new();
    }

    // The aspect is exact when angular_distance(transit_lon, natal_lon) == aspect.
    // We define g(jd) = angular_distance(transit_fn(jd), natal_lon) - aspect.
    // Exact aspect happens at g(jd) == 0.
    //
    // However angular_distance is always [0, 180], so sign-change detection works
    // for most aspects.  For conjunction (aspect=0) and opposition (aspect=180)
    // there is an additional complication because g() can touch zero without
    // crossing it, but the bisection still converges because we check both
    // sign-changes and close approaches.
    //
    // Step size: 0.5 day catches even lunar transits (~13 deg/day).
    let step = 0.5_f64;
    let mut results = Vec::new();
    let mut jd = jd_start;

    let g = |t: f64| -> f64 { angular_distance(transit_fn(t), natal_lon) - aspect };

    let mut prev = g(jd);

    while jd < jd_end {
        jd += step;
        let curr = g(jd);

        // Sign change or very close approach
        if prev * curr < 0.0 || curr.abs() < 0.01 {
            // Bisect to refine
            if let Some(exact_jd) = bisect_aspect_crossing(&g, jd - step, jd, 60) {
                // De-duplicate: skip if we already found a crossing within 0.5 day
                if results
                    .last()
                    .is_none_or(|&last: &f64| (exact_jd - last).abs() > 0.5)
                {
                    results.push(exact_jd);
                }
            }
        }
        prev = curr;
    }
    results
}

/// Bisection helper for transit aspect search.
fn bisect_aspect_crossing(
    g: &impl Fn(f64) -> f64,
    mut lo: f64,
    mut hi: f64,
    max_iter: u32,
) -> Option<f64> {
    let g_lo = g(lo);
    let g_hi = g(hi);

    // If both endpoints have the same sign and neither is very close to zero,
    // this might be a touch-without-crossing near conjunction/opposition.
    // Still try: if one endpoint is close, bisect toward it.
    if g_lo * g_hi > 0.0 && g_lo.abs() > 0.5 && g_hi.abs() > 0.5 {
        return None;
    }

    for _ in 0..max_iter {
        let mid = (lo + hi) / 2.0;
        let g_mid = g(mid);

        if g_mid.abs() < 1e-8 || (hi - lo) < 1e-10 {
            return Some(mid);
        }

        // Prefer the interval containing the sign change
        if g(lo) * g_mid <= 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some((lo + hi) / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angular_distance_basic() {
        assert!((angular_distance(10.0, 130.0) - 120.0).abs() < 0.001);
        assert!((angular_distance(350.0, 10.0) - 20.0).abs() < 0.001);
        assert!((angular_distance(0.0, 180.0) - 180.0).abs() < 0.001);
    }

    #[test]
    fn conjunction_detected() {
        let asp = find_aspect(100.0, 103.0, 1.0, 0.5, AspectType::MAJOR, 1.0);
        assert!(asp.is_some());
        assert_eq!(asp.unwrap().aspect_type, AspectType::Conjunction);
    }

    #[test]
    fn opposition_detected() {
        let asp = find_aspect(10.0, 192.0, 1.0, 0.5, AspectType::MAJOR, 1.0);
        assert!(asp.is_some());
        assert_eq!(asp.unwrap().aspect_type, AspectType::Opposition);
    }

    #[test]
    fn trine_detected() {
        let asp = find_aspect(30.0, 151.0, 0.9, 0.5, AspectType::MAJOR, 1.0);
        assert!(asp.is_some());
        assert_eq!(asp.unwrap().aspect_type, AspectType::Trine);
    }

    #[test]
    fn no_aspect_when_out_of_orb() {
        let asp = find_aspect(10.0, 55.0, 1.0, 0.5, AspectType::MAJOR, 1.0);
        assert!(asp.is_none());
    }

    #[test]
    fn applying_vs_separating() {
        // Faster planet approaching exact conjunction
        let asp = find_aspect(100.0, 105.0, 1.5, 0.5, AspectType::MAJOR, 1.0).unwrap();
        assert_eq!(asp.direction, AspectDirection::Applying);

        // Faster planet moving away from conjunction
        let asp2 = find_aspect(100.0, 105.0, 0.5, 1.5, AspectType::MAJOR, 1.0).unwrap();
        assert_eq!(asp2.direction, AspectDirection::Separating);
    }

    #[test]
    fn find_all_aspects_multi() {
        let positions = vec![
            ("Sun".into(), 10.0, 1.0),
            ("Moon".into(), 130.0, 13.0),
            ("Mars".into(), 100.0, 0.5),
        ];
        let aspects = find_all_aspects(&positions, AspectType::MAJOR);
        assert!(!aspects.is_empty());
    }

    #[test]
    fn minor_aspects() {
        let asp = find_aspect(10.0, 41.0, 1.0, 0.5, AspectType::ALL, 1.0);
        assert!(asp.is_some());
        assert_eq!(asp.unwrap().aspect_type, AspectType::SemiSextile);
    }

    // -----------------------------------------------------------------------
    // find_transit_aspect tests
    // -----------------------------------------------------------------------

    #[test]
    fn transit_conjunction_linear() {
        // Transit planet moves 1 deg/day from 0°, natal planet at 30°.
        // Conjunction (aspect=0) at jd=30.
        let results = find_transit_aspect(30.0, |jd| jd * 1.0, 0.0, 0.0, 60.0);
        assert!(!results.is_empty(), "Should find at least one conjunction");
        assert!(
            (results[0] - 30.0).abs() < 0.01,
            "Conjunction should be near jd=30, got {}",
            results[0]
        );
    }

    #[test]
    fn transit_opposition_linear() {
        // Transit planet at (jd * 1.0) deg, natal at 30°.
        // Opposition (180°) when transit_lon = 210 → jd=210.
        let results = find_transit_aspect(30.0, |jd| jd * 1.0, 180.0, 200.0, 220.0);
        assert!(!results.is_empty(), "Should find opposition");
        assert!(
            (results[0] - 210.0).abs() < 0.01,
            "Opposition should be near jd=210, got {}",
            results[0]
        );
    }

    #[test]
    fn transit_trine_linear() {
        // Trine (120°) from natal at 0°: when transit = 120 or 240.
        let results = find_transit_aspect(0.0, |jd| jd * 1.0, 120.0, 100.0, 260.0);
        assert!(
            results.len() >= 2,
            "Should find at least 2 trines, got {}",
            results.len()
        );
    }

    #[test]
    fn transit_no_results_empty_window() {
        let results = find_transit_aspect(30.0, |jd| jd, 0.0, 50.0, 50.0);
        assert!(results.is_empty(), "Empty window should return no results");
    }

    #[test]
    fn transit_wrapping_conjunction() {
        // Transit crosses 360/0 boundary: natal at 5°, transit starts at 350°
        // and gains 1 deg/day.  Conjunction near jd=15 (350+15=365→5°).
        let results = find_transit_aspect(5.0, |jd| (350.0 + jd).rem_euclid(360.0), 0.0, 0.0, 30.0);
        assert!(
            !results.is_empty(),
            "Should find conjunction across 360/0 wrap"
        );
        assert!(
            (results[0] - 15.0).abs() < 1.0,
            "Conjunction should be near jd=15, got {}",
            results[0]
        );
    }

    // -------------------------------------------------------------------
    // Tests below this point are ported/adapted from Anonyfox/celestine
    // (MIT), commit 954d63315ec00d29ba4becaef3f6a101497946b7:
    //   - src/aspects/constants.test.ts
    //   - src/aspects/orbs.test.ts
    //   - src/aspects/aspect-detection.test.ts
    // Behavior (angles, orbs, strength formula, out-of-sign rule, applying/
    // separating rule) is celestine's; test values re-derived/re-checked for
    // the Rust API shape, not copy-pasted verbatim. See
    // docs/THIRD_PARTY_SOURCES.md.
    // -------------------------------------------------------------------

    // --- All 14 aspect types: angle + default orb sanity ---------------

    #[test]
    fn all_14_aspect_types_present_with_correct_angles() {
        let expected: &[(AspectType, f64)] = &[
            (AspectType::Conjunction, 0.0),
            (AspectType::SemiSextile, 30.0),
            (AspectType::SemiSquare, 45.0),
            (AspectType::Sextile, 60.0),
            (AspectType::Quintile, 72.0),
            (AspectType::Square, 90.0),
            (AspectType::Trine, 120.0),
            (AspectType::Sesquiquadrate, 135.0),
            (AspectType::BiQuintile, 144.0),
            (AspectType::Quincunx, 150.0),
            (AspectType::Opposition, 180.0),
            (AspectType::Septile, 360.0 / 7.0),
            (AspectType::Novile, 40.0),
            (AspectType::Decile, 36.0),
        ];
        assert_eq!(expected.len(), 14, "test itself must cover all 14 types");
        assert_eq!(AspectType::ALL_14.len(), 14);
        for &(t, angle) in expected {
            assert!(
                (t.angle_deg() - angle).abs() < 1e-9,
                "{t:?} angle mismatch: got {}, expected {angle}",
                t.angle_deg()
            );
            assert!(
                t.default_orb_deg() > 0.0,
                "{t:?} must have a positive default orb"
            );
        }
    }

    #[test]
    fn kepler_aspects_classified_correctly() {
        assert!(AspectType::Septile.is_kepler());
        assert!(AspectType::Novile.is_kepler());
        assert!(AspectType::Decile.is_kepler());
        assert!(!AspectType::Trine.is_kepler());
        assert_eq!(AspectType::KEPLER.len(), 3);
    }

    // --- Configurable orbs (AspectConfig) -------------------------------

    #[test]
    fn default_config_matches_major_aspects_only() {
        let config = AspectConfig::default();
        assert_eq!(config.aspect_types, AspectType::MAJOR.to_vec());
        assert_eq!(
            config.orb_for(AspectType::Trine),
            AspectType::Trine.default_orb_deg()
        );
    }

    #[test]
    fn orb_override_replaces_default_for_one_type_only() {
        let config = AspectConfig::new().with_orb(AspectType::Trine, 6.0);
        assert_eq!(config.orb_for(AspectType::Trine), 6.0);
        // Untouched types still use their own defaults.
        assert_eq!(
            config.orb_for(AspectType::Square),
            AspectType::Square.default_orb_deg()
        );
    }

    #[test]
    fn existing_callers_using_defaults_keep_working_unchanged() {
        // find_aspect/find_all_aspects (pre-existing API) must be byte-for-byte
        // unaffected by everything added in this package.
        let asp = find_aspect(100.0, 103.0, 1.0, 0.5, AspectType::MAJOR, 1.0);
        assert!(asp.is_some());
        assert_eq!(asp.unwrap().aspect_type, AspectType::Conjunction);
    }

    // --- Strength model --------------------------------------------------

    #[test]
    fn strength_at_exact_is_100() {
        assert_eq!(aspect_strength(0.0, 8.0), 100.0);
    }

    #[test]
    fn strength_halfway_to_orb_is_50() {
        assert_eq!(aspect_strength(4.0, 8.0), 50.0);
    }

    #[test]
    fn strength_at_orb_boundary_is_0() {
        assert_eq!(aspect_strength(8.0, 8.0), 0.0);
    }

    #[test]
    fn strength_beyond_orb_is_0_not_negative() {
        assert_eq!(aspect_strength(10.0, 8.0), 0.0);
    }

    #[test]
    fn strength_zero_orb_is_100_only_at_exact() {
        assert_eq!(aspect_strength(0.0, 0.0), 100.0);
        assert_eq!(aspect_strength(0.5, 0.0), 0.0);
    }

    // --- detect_aspect: basic cases -------------------------------------

    #[test]
    fn detect_exact_conjunction() {
        let config = AspectConfig::default();
        let r = detect_aspect(("Sun", 100.0, None), ("Moon", 100.0, None), &config).unwrap();
        assert_eq!(r.aspect_type, AspectType::Conjunction);
        assert_eq!(r.deviation_deg, 0.0);
        assert_eq!(r.strength, 100.0);
    }

    #[test]
    fn detect_just_inside_orb() {
        // Sun-Mars 3deg from exact square (default square orb = 7deg).
        let config = AspectConfig::default();
        let r = detect_aspect(("Sun", 0.0, None), ("Mars", 87.0, None), &config).unwrap();
        assert_eq!(r.aspect_type, AspectType::Square);
        assert!((r.deviation_deg - 3.0).abs() < 1e-9);
        assert!(r.strength < 100.0 && r.strength > 50.0);
    }

    #[test]
    fn detect_exactly_on_orb_boundary_is_included() {
        // Square orb default = 7deg; separation exactly 7deg from 90 = 97deg.
        let config = AspectConfig::default().with_aspect_types(&[AspectType::Square]);
        let r = detect_aspect(("Sun", 0.0, None), ("Mars", 97.0, None), &config);
        assert!(
            r.is_some(),
            "deviation == orb must still match (inclusive boundary)"
        );
        assert_eq!(r.unwrap().strength, 0.0);
    }

    #[test]
    fn detect_just_outside_orb_is_none() {
        let config = AspectConfig::default().with_aspect_types(&[AspectType::Square]);
        let r = detect_aspect(("Sun", 0.0, None), ("Mars", 97.5, None), &config);
        assert!(r.is_none(), "deviation just past orb must not match");
    }

    #[test]
    fn detect_no_aspect_within_orb_for_default_major_set() {
        // 45deg separation is a semi-square, not in the default major-only set.
        let config = AspectConfig::default();
        let r = detect_aspect(("Sun", 0.0, None), ("Mars", 45.0, None), &config);
        assert!(r.is_none());
    }

    #[test]
    fn custom_orb_override_widens_detection() {
        // Sun-Mars 8deg from exact square: default 7deg orb excludes it,
        // a widened 9deg orb includes it.
        let body2 = 82.0; // 8deg shy of 90
        let tight = AspectConfig::default().with_aspect_types(&[AspectType::Square]);
        assert!(detect_aspect(("Sun", 0.0, None), ("Mars", body2, None), &tight).is_none());

        let wide = AspectConfig::default()
            .with_aspect_types(&[AspectType::Square])
            .with_orb(AspectType::Square, 9.0);
        let r = detect_aspect(("Sun", 0.0, None), ("Mars", body2, None), &wide);
        assert!(r.is_some());
        assert!((r.unwrap().deviation_deg - 8.0).abs() < 1e-9);
    }

    // --- 0/360 wrap ------------------------------------------------------

    #[test]
    fn separation_across_0_360_boundary_359_to_1() {
        assert!((angular_distance(359.0, 1.0) - 2.0).abs() < 1e-9);
        assert!((angular_distance(1.0, 359.0) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn conjunction_detected_across_0_360_wrap() {
        // 359deg and 1deg are 2deg apart (conjunction), not 358deg apart.
        let config = AspectConfig::default();
        let r = detect_aspect(("A", 359.0, None), ("B", 1.0, None), &config).unwrap();
        assert_eq!(r.aspect_type, AspectType::Conjunction);
        assert!((r.deviation_deg - 2.0).abs() < 1e-9);
    }

    #[test]
    fn applying_detection_correct_across_0_360_wrap() {
        // Body at 359deg moving +1deg/day (crosses 0/360 next step), body at
        // 5deg moving +0.1deg/day: separation now = 6deg conjunction-ish,
        // tomorrow body1 at 0deg (=360), body2 at 5.1 -> separation 5.1deg,
        // deviation from conjunction (0) decreases 6 -> 5.1: applying.
        let config = AspectConfig::default();
        let r = detect_aspect(("A", 359.0, Some(1.0)), ("B", 5.0, Some(0.1)), &config).unwrap();
        assert_eq!(r.aspect_type, AspectType::Conjunction);
        assert_eq!(r.phase, Some(AspectPhase::Applying));
    }

    // --- Applying / separating / stationary -----------------------------

    #[test]
    fn applying_when_deviation_decreases_tomorrow() {
        // Sun at 0deg +1deg/day, Mars at 93deg +0.5deg/day: today 3deg past
        // square, tomorrow 2.5deg past square -> applying.
        let config = AspectConfig::default();
        let r = detect_aspect(("Sun", 0.0, Some(1.0)), ("Mars", 93.0, Some(0.5)), &config).unwrap();
        assert_eq!(r.aspect_type, AspectType::Square);
        assert_eq!(r.phase, Some(AspectPhase::Applying));
    }

    #[test]
    fn separating_when_deviation_increases_tomorrow() {
        // Sun at 0deg +1deg/day, Mars at 87deg +0.5deg/day: today 3deg shy of
        // square, tomorrow 3.5deg shy -> separating.
        let config = AspectConfig::default();
        let r = detect_aspect(("Sun", 0.0, Some(1.0)), ("Mars", 87.0, Some(0.5)), &config).unwrap();
        assert_eq!(r.aspect_type, AspectType::Square);
        assert_eq!(r.phase, Some(AspectPhase::Separating));
    }

    #[test]
    fn phase_is_none_without_speed_data() {
        let config = AspectConfig::default();
        let r = detect_aspect(("Sun", 0.0, None), ("Mars", 90.0, None), &config).unwrap();
        assert_eq!(r.phase, None);
    }

    #[test]
    fn stationary_near_zero_speed_still_classifies_without_panicking() {
        // Both bodies essentially motionless: deviation barely changes: must
        // not panic, and (since neither approaches nor recedes meaningfully)
        // resolves deterministically to one phase or the other, never a crash.
        let config = AspectConfig::default();
        let r = detect_aspect(
            ("Saturn", 0.0, Some(0.0001)),
            ("Pluto", 90.0, Some(0.0001)),
            &config,
        )
        .unwrap();
        assert!(r.phase.is_some());
    }

    // --- Out-of-sign -------------------------------------------------------

    #[test]
    fn in_sign_trine_between_fire_signs_is_not_out_of_sign() {
        // Aries (15deg) to Leo (135deg): both fire signs, canonical trine.
        assert!(!is_out_of_sign(15.0, 135.0, AspectType::Trine));
    }

    #[test]
    fn out_of_sign_trine_between_non_trine_signs() {
        // 28deg Aries to 2deg Virgo: 122deg deg-separation reads as
        // approximately trine, but Aries/Virgo are not trine signs.
        assert!(is_out_of_sign(28.0, 152.0, AspectType::Trine));
    }

    #[test]
    fn in_sign_conjunction_same_sign() {
        assert!(!is_out_of_sign(275.0, 280.0, AspectType::Conjunction));
    }

    #[test]
    fn out_of_sign_conjunction_across_sign_boundary() {
        // 29deg Pisces (359) to 1deg Aries (1): different signs despite tiny separation.
        assert!(is_out_of_sign(359.0, 1.0, AspectType::Conjunction));
    }

    #[test]
    fn out_of_sign_flag_surfaces_on_aspect_result() {
        let config = AspectConfig::default();
        let r = detect_aspect(("Mars", 29.0, None), ("Jupiter", 151.0, None), &config).unwrap();
        assert_eq!(r.aspect_type, AspectType::Trine);
        assert!(r.is_out_of_sign);
    }

    #[test]
    fn exclude_out_of_sign_filters_it_out() {
        let config = AspectConfig::default().with_aspect_types(&[AspectType::Trine]);
        let mut excluding = config.clone();
        excluding.exclude_out_of_sign = true;
        assert!(
            detect_aspect(("Mars", 29.0, None), ("Jupiter", 151.0, None), &excluding).is_none()
        );
        // Without exclusion, it's still reported (just flagged).
        assert!(detect_aspect(("Mars", 29.0, None), ("Jupiter", 151.0, None), &config).is_some());
    }

    // --- find_all_aspects_ex: no duplicates, deterministic ordering -----

    #[test]
    fn find_all_aspects_ex_no_duplicate_pairs() {
        let bodies = vec![
            ("Sun".to_string(), 0.0, Some(1.0)),
            ("Moon".to_string(), 90.0, Some(13.0)),
            ("Mars".to_string(), 180.0, Some(0.5)),
        ];
        let config = AspectConfig::default();
        let results = find_all_aspects_ex(&bodies, &config);
        let mut seen = std::collections::HashSet::new();
        for r in &results {
            let key = if r.body1 < r.body2 {
                (r.body1.clone(), r.body2.clone())
            } else {
                (r.body2.clone(), r.body1.clone())
            };
            assert!(seen.insert(key), "duplicate pair in results: {r:?}");
        }
        // 3 bodies at 0/90/180: Sun-Moon square, Moon-Mars square, Sun-Mars opposition.
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn find_all_aspects_ex_deterministic_ordering() {
        let bodies = vec![
            ("A".to_string(), 0.0, None),
            ("B".to_string(), 90.0, None),
            ("C".to_string(), 180.0, None),
        ];
        let config = AspectConfig::default();
        let r1 = find_all_aspects_ex(&bodies, &config);
        let r2 = find_all_aspects_ex(&bodies, &config);
        let key = |v: &[AspectResult]| -> Vec<(String, String, String)> {
            v.iter()
                .map(|a| {
                    (
                        a.body1.clone(),
                        a.body2.clone(),
                        format!("{:?}", a.aspect_type),
                    )
                })
                .collect()
        };
        assert_eq!(
            key(&r1),
            key(&r2),
            "repeated calls must produce identical ordering"
        );
    }

    // --- J2000.0 cross-reference fixtures (shared with Celestine's own
    // aspect-detection.test.ts, which cites the same JPL Horizons values) ---

    #[test]
    fn j2000_sun_moon_sextile_matches_celestine_fixture() {
        // Sun 280.3689092, Moon 223.323786 (celestine's own JPL J2000 fixture).
        let config = AspectConfig::default();
        let r = detect_aspect(
            ("Sun", 280.3689092, None),
            ("Moon", 223.323786, None),
            &config,
        )
        .unwrap();
        assert_eq!(r.aspect_type, AspectType::Sextile);
        assert!(r.deviation_deg < 4.0);
    }

    #[test]
    fn j2000_sun_mercury_no_aspect_at_default_orb_but_found_at_wider_orb() {
        // Sun 280.3689092, Mercury 271.8892699: ~8.48deg separation, just
        // outside the 8deg default conjunction orb.
        let default_cfg = AspectConfig::default();
        assert!(
            detect_aspect(
                ("Sun", 280.3689092, None),
                ("Mercury", 271.8892699, None),
                &default_cfg
            )
            .is_none()
        );

        let wide_cfg = AspectConfig::default().with_orb(AspectType::Conjunction, 9.0);
        let r = detect_aspect(
            ("Sun", 280.3689092, None),
            ("Mercury", 271.8892699, None),
            &wide_cfg,
        )
        .unwrap();
        assert_eq!(r.aspect_type, AspectType::Conjunction);
        assert!((r.deviation_deg - 8.48).abs() < 0.01);
    }
}
