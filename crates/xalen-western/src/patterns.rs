use crate::aspects::angular_distance;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspectPattern {
    GrandTrine,
    TSquare,
    GrandCross,
    Yod,
    Kite,
    Stellium,
    MysticRectangle,
}

/// Canonical alias for [`AspectPattern`] (the pattern-type enum) used by the
/// newer named-body API below ([`find_named_patterns`]). Same type, not a
/// duplicate model — matches this package's naming convention of adapting to
/// an existing XALEN-equivalent concept rather than introducing a second enum
/// (see [`crate::aspects::AspectPhase`] for the same treatment of direction).
pub type AspectPatternType = AspectPattern;

pub fn detect_patterns(positions_deg: &[f64], orb: f64) -> Vec<(AspectPattern, Vec<usize>)> {
    let mut patterns = Vec::new();
    let n = positions_deg.len();

    // Grand Trine: 3 planets each ~120° apart
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                let d_ij = angular_distance(positions_deg[i], positions_deg[j]);
                let d_jk = angular_distance(positions_deg[j], positions_deg[k]);
                let d_ik = angular_distance(positions_deg[i], positions_deg[k]);
                if (d_ij - 120.0).abs() < orb
                    && (d_jk - 120.0).abs() < orb
                    && (d_ik - 120.0).abs() < orb
                {
                    patterns.push((AspectPattern::GrandTrine, vec![i, j, k]));
                }
            }
        }
    }

    // T-Square: 2 in opposition, both square a 3rd
    for i in 0..n {
        for j in (i + 1)..n {
            let d_ij = angular_distance(positions_deg[i], positions_deg[j]);
            if (d_ij - 180.0).abs() < orb {
                for k in 0..n {
                    if k == i || k == j {
                        continue;
                    }
                    let d_ik = angular_distance(positions_deg[i], positions_deg[k]);
                    let d_jk = angular_distance(positions_deg[j], positions_deg[k]);
                    if (d_ik - 90.0).abs() < orb && (d_jk - 90.0).abs() < orb {
                        patterns.push((AspectPattern::TSquare, vec![i, j, k]));
                    }
                }
            }
        }
    }

    // Yod: 2 in sextile, both quincunx to apex
    for i in 0..n {
        for j in (i + 1)..n {
            let d_ij = angular_distance(positions_deg[i], positions_deg[j]);
            if (d_ij - 60.0).abs() < orb {
                for k in 0..n {
                    if k == i || k == j {
                        continue;
                    }
                    let d_ik = angular_distance(positions_deg[i], positions_deg[k]);
                    let d_jk = angular_distance(positions_deg[j], positions_deg[k]);
                    if (d_ik - 150.0).abs() < orb && (d_jk - 150.0).abs() < orb {
                        patterns.push((AspectPattern::Yod, vec![i, j, k]));
                    }
                }
            }
        }
    }

    // Stellium: 3+ planets within 30° arc
    for i in 0..n {
        let mut cluster = vec![i];
        for j in 0..n {
            if j == i {
                continue;
            }
            if angular_distance(positions_deg[i], positions_deg[j]) < 30.0 {
                cluster.push(j);
            }
        }
        if cluster.len() >= 3 {
            cluster.sort();
            cluster.dedup();
            let key = cluster.clone();
            if !patterns
                .iter()
                .any(|(p, v)| *p == AspectPattern::Stellium && *v == key)
            {
                patterns.push((AspectPattern::Stellium, key));
            }
        }
    }

    // Grand Cross: four planets forming two oppositions, mutually squared.
    // A–C and B–D are oppositions (180°); the four "sides" A–B, B–C, C–D, D–A
    // are all squares (90°).  Iterate all 4-combinations and check the two
    // diagonals + four sides.  Members are returned in opposition-pair order
    // [A, C, B, D] so the two opposition axes are (members[0],members[1]) and
    // (members[2],members[3]).
    for a in 0..n {
        for b in (a + 1)..n {
            for c in (b + 1)..n {
                for d in (c + 1)..n {
                    let idx = [a, b, c, d];
                    if let Some(order) = grand_cross_order(positions_deg, idx, orb) {
                        patterns.push((AspectPattern::GrandCross, order));
                    }
                }
            }
        }
    }

    // Mystic Rectangle: two oppositions whose endpoints are joined by two
    // trines and two sextiles.  For bodies on opposition axes (p,q) and (r,s):
    // one of {p,q} trines one of {r,s} and sextiles the other, and the
    // remaining endpoint mirrors it.  Members returned as [p, q, r, s] (the two
    // opposition axes are (members[0],members[1]) and (members[2],members[3])).
    for a in 0..n {
        for b in (a + 1)..n {
            for c in (b + 1)..n {
                for d in (c + 1)..n {
                    let idx = [a, b, c, d];
                    if let Some(order) = mystic_rectangle_order(positions_deg, idx, orb) {
                        patterns.push((AspectPattern::MysticRectangle, order));
                    }
                }
            }
        }
    }

    // Kite: a Grand Trine (three planets 120° apart) plus a fourth body that
    // is in opposition to one trine member ("tail") and sextile to the other
    // two.  Members returned as [tail, apex, wing1, wing2] where `apex` is the
    // trine member opposed by the tail and wing1/wing2 are the sextile pair.
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                // Must be a grand trine first.
                if !is_grand_trine(positions_deg, [i, j, k], orb) {
                    continue;
                }
                for tail in 0..n {
                    if tail == i || tail == j || tail == k {
                        continue;
                    }
                    if let Some(order) = kite_order(positions_deg, [i, j, k], tail, orb) {
                        patterns.push((AspectPattern::Kite, order));
                    }
                }
            }
        }
    }

    patterns
}

/// True when indices `[a,b,c]` form a Grand Trine (each pair ~120° apart).
fn is_grand_trine(pos: &[f64], idx: [usize; 3], orb: f64) -> bool {
    let [a, b, c] = idx;
    (angular_distance(pos[a], pos[b]) - 120.0).abs() < orb
        && (angular_distance(pos[b], pos[c]) - 120.0).abs() < orb
        && (angular_distance(pos[a], pos[c]) - 120.0).abs() < orb
}

/// Return true if the angular distance between two positions matches `target`
/// within `orb`.
fn is_aspect(pos: &[f64], i: usize, j: usize, target: f64, orb: f64) -> bool {
    (angular_distance(pos[i], pos[j]) - target).abs() < orb
}

/// If `[a,b,c,d]` form a Grand Cross, return the four indices ordered as two
/// opposition axes: `[A, C, B, D]` (axis 1 = A–C, axis 2 = B–D).
fn grand_cross_order(pos: &[f64], idx: [usize; 4], orb: f64) -> Option<Vec<usize>> {
    let [a, b, c, d] = idx;
    // Find a pairing of the 4 indices into two oppositions.
    // The three distinct pairings of 4 items into 2 pairs:
    //   (a,b)+(c,d), (a,c)+(b,d), (a,d)+(b,c)
    let pairings = [[(a, b), (c, d)], [(a, c), (b, d)], [(a, d), (b, c)]];
    for [(p1a, p1b), (p2a, p2b)] in pairings {
        let opp1 = is_aspect(pos, p1a, p1b, 180.0, orb);
        let opp2 = is_aspect(pos, p2a, p2b, 180.0, orb);
        if !(opp1 && opp2) {
            continue;
        }
        // All four cross-sides must be squares (90°): p1a–p2a, p1a–p2b,
        // p1b–p2a, p1b–p2b.
        let sides = is_aspect(pos, p1a, p2a, 90.0, orb)
            && is_aspect(pos, p1a, p2b, 90.0, orb)
            && is_aspect(pos, p1b, p2a, 90.0, orb)
            && is_aspect(pos, p1b, p2b, 90.0, orb);
        if sides {
            // [A, C, B, D] — opposition axes (0,1) and (2,3).
            return Some(vec![p1a, p1b, p2a, p2b]);
        }
    }
    None
}

/// If `[a,b,c,d]` form a Mystic Rectangle, return the four indices ordered as
/// two opposition axes: `[p, q, r, s]` (axis 1 = p–q, axis 2 = r–s).  The
/// connecting sides are two trines and two sextiles.
fn mystic_rectangle_order(pos: &[f64], idx: [usize; 4], orb: f64) -> Option<Vec<usize>> {
    let [a, b, c, d] = idx;
    let pairings = [[(a, b), (c, d)], [(a, c), (b, d)], [(a, d), (b, c)]];
    for [(p, q), (r, s)] in pairings {
        // Both diagonals must be oppositions.
        if !(is_aspect(pos, p, q, 180.0, orb) && is_aspect(pos, r, s, 180.0, orb)) {
            continue;
        }
        // The four sides connecting the two axes must be exactly two trines
        // and two sextiles, arranged so each endpoint has one of each.
        // Side set: p–r, p–s, q–r, q–s.  Valid rectangle:
        //   (p–r trine, p–s sextile, q–r sextile, q–s trine)  OR
        //   (p–r sextile, p–s trine, q–r trine, q–s sextile)
        let pr_t = is_aspect(pos, p, r, 120.0, orb);
        let ps_s = is_aspect(pos, p, s, 60.0, orb);
        let qr_s = is_aspect(pos, q, r, 60.0, orb);
        let qs_t = is_aspect(pos, q, s, 120.0, orb);

        let pr_s = is_aspect(pos, p, r, 60.0, orb);
        let ps_t = is_aspect(pos, p, s, 120.0, orb);
        let qr_t = is_aspect(pos, q, r, 120.0, orb);
        let qs_s = is_aspect(pos, q, s, 60.0, orb);

        if (pr_t && ps_s && qr_s && qs_t) || (pr_s && ps_t && qr_t && qs_s) {
            return Some(vec![p, q, r, s]);
        }
    }
    None
}

/// Given a Grand Trine `[i,j,k]` and a candidate `tail`, return the Kite
/// ordering `[tail, apex, wing1, wing2]` if `tail` is in opposition to exactly
/// one trine member (the apex) and sextile to the other two (the wings).
fn kite_order(pos: &[f64], trine: [usize; 3], tail: usize, orb: f64) -> Option<Vec<usize>> {
    let members = trine;
    // The apex is the trine member opposed by the tail.
    let apex = members
        .iter()
        .copied()
        .find(|&m| is_aspect(pos, tail, m, 180.0, orb))?;
    let wings: Vec<usize> = members.iter().copied().filter(|&m| m != apex).collect();
    if wings.len() != 2 {
        return None;
    }
    // The tail must sextile both wings.
    if is_aspect(pos, tail, wings[0], 60.0, orb) && is_aspect(pos, tail, wings[1], 60.0, orb) {
        Some(vec![tail, apex, wings[0], wings[1]])
    } else {
        None
    }
}

// =============================================================================
// Named-body pattern API (adapted from Anonyfox/celestine, MIT License,
// commit 954d63315ec00d29ba4becaef3f6a101497946b7: src/aspects/patterns.ts).
// See docs/THIRD_PARTY_SOURCES.md for full provenance.
//
// The index/flat-orb API above (`detect_patterns`) pre-dates this package and
// is unchanged. This section adds a richer, named-body counterpart that
// operates on already-computed `AspectResult`s (from
// `crate::aspects::find_all_aspects_ex`) — matching how celestine's own
// pattern detectors consume a pre-computed aspect list rather than raw
// positions, so per-aspect-type orbs, strength, and applying/separating are
// already baked into what "counts" as e.g. a square or a trine. Ported to
// idiomatic Rust, not transliterated line-for-line.
// =============================================================================

use crate::aspects::{AspectResult, AspectType};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A detected pattern with full body names and the contributing
/// [`AspectResult`]s — the named-body counterpart to the `(AspectPattern,
/// Vec<usize>)` pairs [`detect_patterns`] returns.
pub struct AspectPatternMatch {
    pub pattern_type: AspectPatternType,
    pub bodies: Vec<String>,
    /// The [`AspectResult`]s that make up this pattern.
    pub aspects: Vec<AspectResult>,
    pub description: String,
}

fn find_between<'a>(
    aspects: &'a [AspectResult],
    aspect_type: AspectType,
    body1: &str,
    body2: &str,
) -> Option<&'a AspectResult> {
    aspects.iter().find(|a| {
        a.aspect_type == aspect_type
            && ((a.body1 == body1 && a.body2 == body2) || (a.body1 == body2 && a.body2 == body1))
    })
}

fn of_type(aspects: &[AspectResult], aspect_type: AspectType) -> Vec<&AspectResult> {
    aspects
        .iter()
        .filter(|a| a.aspect_type == aspect_type)
        .collect()
}

fn other_body<'a>(aspect: &'a AspectResult, known: &str) -> &'a str {
    if aspect.body1 == known {
        &aspect.body2
    } else {
        &aspect.body1
    }
}

fn sorted_bodies(bodies: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = bodies.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

/// Detect T-Square patterns: an opposition with a common square-apex from
/// both ends. Named-body counterpart to the index-based detection embedded
/// in [`detect_patterns`].
pub fn detect_t_square_named(aspects: &[AspectResult]) -> Vec<AspectPatternMatch> {
    let mut patterns = Vec::new();
    let oppositions = of_type(aspects, AspectType::Opposition);
    let squares = of_type(aspects, AspectType::Square);
    let mut seen: Vec<Vec<String>> = Vec::new();

    for opp in &oppositions {
        let squares_from_1: Vec<&&AspectResult> = squares
            .iter()
            .filter(|sq| sq.body1 == opp.body1 || sq.body2 == opp.body1)
            .collect();
        let squares_from_2: Vec<&&AspectResult> = squares
            .iter()
            .filter(|sq| sq.body1 == opp.body2 || sq.body2 == opp.body2)
            .collect();

        for sq1 in &squares_from_1 {
            let apex1 = other_body(sq1, &opp.body1);
            for sq2 in &squares_from_2 {
                let apex2 = other_body(sq2, &opp.body2);
                if apex1 == apex2 && apex1 != opp.body1 && apex1 != opp.body2 {
                    let bodies = sorted_bodies(&[&opp.body1, &opp.body2, apex1]);
                    if seen.contains(&bodies) {
                        continue;
                    }
                    seen.push(bodies.clone());
                    patterns.push(AspectPatternMatch {
                        pattern_type: AspectPatternType::TSquare,
                        bodies: vec![opp.body1.clone(), opp.body2.clone(), apex1.to_string()],
                        aspects: vec![(*opp).clone(), (**sq1).clone(), (**sq2).clone()],
                        description: format!(
                            "T-Square with {apex1} as apex, {}-{} opposition",
                            opp.body1, opp.body2
                        ),
                    });
                }
            }
        }
    }
    patterns
}

/// Detect Grand Trine patterns: 3 bodies mutually trine.
pub fn detect_grand_trine_named(aspects: &[AspectResult]) -> Vec<AspectPatternMatch> {
    let mut patterns = Vec::new();
    let trines = of_type(aspects, AspectType::Trine);
    if trines.len() < 3 {
        return patterns;
    }

    let mut bodies: Vec<String> = Vec::new();
    for t in &trines {
        if !bodies.contains(&t.body1) {
            bodies.push(t.body1.clone());
        }
        if !bodies.contains(&t.body2) {
            bodies.push(t.body2.clone());
        }
    }

    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            for k in (j + 1)..bodies.len() {
                let (a, b, c) = (&bodies[i], &bodies[j], &bodies[k]);
                let ab = find_between(aspects, AspectType::Trine, a, b);
                let bc = find_between(aspects, AspectType::Trine, b, c);
                let ac = find_between(aspects, AspectType::Trine, a, c);
                if let (Some(ab), Some(bc), Some(ac)) = (ab, bc, ac) {
                    patterns.push(AspectPatternMatch {
                        pattern_type: AspectPatternType::GrandTrine,
                        bodies: vec![a.clone(), b.clone(), c.clone()],
                        aspects: vec![ab.clone(), bc.clone(), ac.clone()],
                        description: format!("Grand Trine: {a}, {b}, {c}"),
                    });
                }
            }
        }
    }
    patterns
}

/// Detect Grand Cross patterns: 2 non-overlapping oppositions with all 4
/// adjacent pairs squared.
pub fn detect_grand_cross_named(aspects: &[AspectResult]) -> Vec<AspectPatternMatch> {
    let mut patterns = Vec::new();
    let squares = of_type(aspects, AspectType::Square);
    let oppositions = of_type(aspects, AspectType::Opposition);
    if squares.len() < 4 || oppositions.len() < 2 {
        return patterns;
    }

    for i in 0..oppositions.len() {
        for j in (i + 1)..oppositions.len() {
            let (opp1, opp2) = (oppositions[i], oppositions[j]);
            let shares_body = opp1.body1 == opp2.body1
                || opp1.body1 == opp2.body2
                || opp1.body2 == opp2.body1
                || opp1.body2 == opp2.body2;
            if shares_body {
                continue;
            }

            let all_bodies = [
                opp1.body1.clone(),
                opp1.body2.clone(),
                opp2.body1.clone(),
                opp2.body2.clone(),
            ];

            let sq1 = find_between(aspects, AspectType::Square, &opp1.body1, &opp2.body1);
            let sq2 = find_between(aspects, AspectType::Square, &opp1.body1, &opp2.body2);
            let sq3 = find_between(aspects, AspectType::Square, &opp1.body2, &opp2.body1);
            let sq4 = find_between(aspects, AspectType::Square, &opp1.body2, &opp2.body2);

            if let (Some(sq1), Some(sq2), Some(sq3), Some(sq4)) = (sq1, sq2, sq3, sq4) {
                patterns.push(AspectPatternMatch {
                    pattern_type: AspectPatternType::GrandCross,
                    bodies: all_bodies.to_vec(),
                    aspects: vec![
                        opp1.clone(),
                        opp2.clone(),
                        sq1.clone(),
                        sq2.clone(),
                        sq3.clone(),
                        sq4.clone(),
                    ],
                    description: format!("Grand Cross: {}", all_bodies.join(", ")),
                });
            }
        }
    }
    patterns
}

/// Detect Yod ("Finger of God") patterns: a sextile base with a common
/// quincunx apex.
pub fn detect_yod_named(aspects: &[AspectResult]) -> Vec<AspectPatternMatch> {
    let mut patterns = Vec::new();
    let sextiles = of_type(aspects, AspectType::Sextile);
    let quincunxes = of_type(aspects, AspectType::Quincunx);
    if sextiles.is_empty() || quincunxes.len() < 2 {
        return patterns;
    }

    let mut seen: Vec<Vec<String>> = Vec::new();
    for sextile in &sextiles {
        let q_from_1: Vec<&&AspectResult> = quincunxes
            .iter()
            .filter(|q| q.body1 == sextile.body1 || q.body2 == sextile.body1)
            .collect();
        let q_from_2: Vec<&&AspectResult> = quincunxes
            .iter()
            .filter(|q| q.body1 == sextile.body2 || q.body2 == sextile.body2)
            .collect();

        for q1 in &q_from_1 {
            let apex1 = other_body(q1, &sextile.body1);
            for q2 in &q_from_2 {
                let apex2 = other_body(q2, &sextile.body2);
                if apex1 == apex2 && apex1 != sextile.body1 && apex1 != sextile.body2 {
                    let bodies = sorted_bodies(&[&sextile.body1, &sextile.body2, apex1]);
                    if seen.contains(&bodies) {
                        continue;
                    }
                    seen.push(bodies);
                    patterns.push(AspectPatternMatch {
                        pattern_type: AspectPatternType::Yod,
                        bodies: vec![
                            sextile.body1.clone(),
                            sextile.body2.clone(),
                            apex1.to_string(),
                        ],
                        aspects: vec![(*sextile).clone(), (**q1).clone(), (**q2).clone()],
                        description: format!("Yod with {apex1} as apex (Finger of God)"),
                    });
                }
            }
        }
    }
    patterns
}

/// Detect Kite patterns: a Grand Trine plus an opposition from one vertex to
/// a 4th body, with sextiles from that 4th body to the other two vertices.
pub fn detect_kite_named(aspects: &[AspectResult]) -> Vec<AspectPatternMatch> {
    let mut patterns = Vec::new();
    let grand_trines = detect_grand_trine_named(aspects);
    let oppositions = of_type(aspects, AspectType::Opposition);

    for gt in &grand_trines {
        for vertex in &gt.bodies {
            let opp = oppositions.iter().find(|o| {
                let touches = o.body1 == *vertex || o.body2 == *vertex;
                if !touches {
                    return false;
                }
                let far_end = other_body(o, vertex);
                !gt.bodies.contains(&far_end.to_string())
            });
            let Some(opp) = opp else { continue };

            let fourth = other_body(opp, vertex).to_string();
            let other_vertices: Vec<&String> = gt.bodies.iter().filter(|b| *b != vertex).collect();
            if other_vertices.len() != 2 {
                continue;
            }

            let sex1 = find_between(aspects, AspectType::Sextile, &fourth, other_vertices[0]);
            let sex2 = find_between(aspects, AspectType::Sextile, &fourth, other_vertices[1]);

            if let (Some(sex1), Some(sex2)) = (sex1, sex2) {
                let mut bodies = gt.bodies.clone();
                bodies.push(fourth.clone());
                let mut pattern_aspects = gt.aspects.clone();
                pattern_aspects.push((*opp).clone());
                pattern_aspects.push(sex1.clone());
                pattern_aspects.push(sex2.clone());
                patterns.push(AspectPatternMatch {
                    pattern_type: AspectPatternType::Kite,
                    bodies,
                    aspects: pattern_aspects,
                    description: format!("Kite with {fourth} as tail, {vertex} opposite"),
                });
            }
        }
    }
    patterns
}

/// Detect Mystic Rectangle patterns: 2 non-overlapping oppositions whose
/// cross-pairs form 2 trines and 2 sextiles.
pub fn detect_mystic_rectangle_named(aspects: &[AspectResult]) -> Vec<AspectPatternMatch> {
    let mut patterns = Vec::new();
    let oppositions = of_type(aspects, AspectType::Opposition);
    let trines = of_type(aspects, AspectType::Trine);
    let sextiles = of_type(aspects, AspectType::Sextile);
    if oppositions.len() < 2 || trines.len() < 2 || sextiles.len() < 2 {
        return patterns;
    }

    let mut seen: Vec<Vec<String>> = Vec::new();
    for i in 0..oppositions.len() {
        for j in (i + 1)..oppositions.len() {
            let (opp1, opp2) = (oppositions[i], oppositions[j]);
            let shares_body = opp1.body1 == opp2.body1
                || opp1.body1 == opp2.body2
                || opp1.body2 == opp2.body1
                || opp1.body2 == opp2.body2;
            if shares_body {
                continue;
            }

            let all_bodies = [
                opp1.body1.clone(),
                opp1.body2.clone(),
                opp2.body1.clone(),
                opp2.body2.clone(),
            ];

            let trine_a1 = find_between(aspects, AspectType::Trine, &opp1.body1, &opp2.body1);
            let trine_a2 = find_between(aspects, AspectType::Trine, &opp1.body2, &opp2.body2);
            if let (Some(ta1), Some(ta2)) = (trine_a1, trine_a2) {
                let sex1 = find_between(aspects, AspectType::Sextile, &opp1.body1, &opp2.body2);
                let sex2 = find_between(aspects, AspectType::Sextile, &opp1.body2, &opp2.body1);
                if let (Some(s1), Some(s2)) = (sex1, sex2) {
                    let key =
                        sorted_bodies(&all_bodies.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                    if !seen.contains(&key) {
                        seen.push(key);
                        patterns.push(AspectPatternMatch {
                            pattern_type: AspectPatternType::MysticRectangle,
                            bodies: all_bodies.to_vec(),
                            aspects: vec![
                                opp1.clone(),
                                opp2.clone(),
                                ta1.clone(),
                                ta2.clone(),
                                s1.clone(),
                                s2.clone(),
                            ],
                            description: format!("Mystic Rectangle: {}", all_bodies.join(", ")),
                        });
                    }
                }
            }

            let trine_b1 = find_between(aspects, AspectType::Trine, &opp1.body1, &opp2.body2);
            let trine_b2 = find_between(aspects, AspectType::Trine, &opp1.body2, &opp2.body1);
            if let (Some(tb1), Some(tb2)) = (trine_b1, trine_b2) {
                let sex1 = find_between(aspects, AspectType::Sextile, &opp1.body1, &opp2.body1);
                let sex2 = find_between(aspects, AspectType::Sextile, &opp1.body2, &opp2.body2);
                if let (Some(s1), Some(s2)) = (sex1, sex2) {
                    let key =
                        sorted_bodies(&all_bodies.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                    if !seen.contains(&key) {
                        seen.push(key);
                        patterns.push(AspectPatternMatch {
                            pattern_type: AspectPatternType::MysticRectangle,
                            bodies: all_bodies.to_vec(),
                            aspects: vec![
                                opp1.clone(),
                                opp2.clone(),
                                tb1.clone(),
                                tb2.clone(),
                                s1.clone(),
                                s2.clone(),
                            ],
                            description: format!("Mystic Rectangle: {}", all_bodies.join(", ")),
                        });
                    }
                }
            }
        }
    }
    patterns
}

/// Detect Stellium patterns: connected components of 3+ bodies all in mutual
/// conjunction.
pub fn detect_stellium_named(aspects: &[AspectResult]) -> Vec<AspectPatternMatch> {
    let mut patterns = Vec::new();
    let conjunctions = of_type(aspects, AspectType::Conjunction);
    if conjunctions.len() < 2 {
        return patterns;
    }

    let mut adjacency: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for c in &conjunctions {
        adjacency
            .entry(c.body1.clone())
            .or_default()
            .push(c.body2.clone());
        adjacency
            .entry(c.body2.clone())
            .or_default()
            .push(c.body1.clone());
    }

    let mut visited = std::collections::HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();
    let mut node_order: Vec<&String> = adjacency.keys().collect();
    node_order.sort(); // deterministic traversal regardless of HashMap iteration order

    for start in node_order {
        if visited.contains(start) {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start.clone());
        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            component.push(current.clone());
            if let Some(neighbors) = adjacency.get(&current) {
                let mut sorted_neighbors = neighbors.clone();
                sorted_neighbors.sort();
                for n in sorted_neighbors {
                    if !visited.contains(&n) {
                        queue.push_back(n);
                    }
                }
            }
        }
        if component.len() >= 3 {
            components.push(component);
        }
    }

    for component in components {
        let relevant: Vec<AspectResult> = conjunctions
            .iter()
            .filter(|c| component.contains(&c.body1) && component.contains(&c.body2))
            .map(|c| (*c).clone())
            .collect();
        let count = component.len();
        patterns.push(AspectPatternMatch {
            pattern_type: AspectPatternType::Stellium,
            bodies: component.clone(),
            aspects: relevant,
            description: format!(
                "Stellium: {count} planets conjunct ({})",
                component.join(", ")
            ),
        });
    }
    patterns
}

/// Find all named-body aspect patterns among the given [`AspectResult`]s, in
/// order of rarity: Grand Cross, Kite, Mystic Rectangle, Grand Trine,
/// T-Square, Yod, Stellium — the same dispatch order as celestine's
/// `findPatterns`. This is the named-body counterpart to [`detect_patterns`];
/// both are available and neither is deprecated.
pub fn find_named_patterns(aspects: &[AspectResult]) -> Vec<AspectPatternMatch> {
    let mut patterns = Vec::new();
    patterns.extend(detect_grand_cross_named(aspects));
    patterns.extend(detect_kite_named(aspects));
    patterns.extend(detect_mystic_rectangle_named(aspects));
    patterns.extend(detect_grand_trine_named(aspects));
    patterns.extend(detect_t_square_named(aspects));
    patterns.extend(detect_yod_named(aspects));
    patterns.extend(detect_stellium_named(aspects));
    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grand_trine_detected() {
        let positions = vec![10.0, 130.0, 250.0]; // 120° apart
        let patterns = detect_patterns(&positions, 8.0);
        assert!(
            patterns
                .iter()
                .any(|(p, _)| *p == AspectPattern::GrandTrine)
        );
    }

    #[test]
    fn t_square_detected() {
        // Opposition at 10° and 190°, square focal at 100°
        let positions = vec![10.0, 190.0, 100.0];
        let patterns = detect_patterns(&positions, 8.0);
        assert!(patterns.iter().any(|(p, _)| *p == AspectPattern::TSquare));
    }

    #[test]
    fn yod_detected() {
        // Sextile at 10° and 70°, both quincunx to 220°
        let positions = vec![10.0, 70.0, 220.0];
        let patterns = detect_patterns(&positions, 5.0);
        assert!(
            patterns.iter().any(|(p, _)| *p == AspectPattern::Yod),
            "Yod should be detected: 10-70 (sextile 60°), 10-220 (quincunx ~150°), 70-220 (quincunx ~150°)"
        );
    }

    #[test]
    fn stellium_detected() {
        let positions = vec![100.0, 105.0, 110.0, 115.0, 250.0];
        let patterns = detect_patterns(&positions, 8.0);
        assert!(patterns.iter().any(|(p, _)| *p == AspectPattern::Stellium));
    }

    #[test]
    fn no_patterns_when_scattered() {
        let positions = vec![10.0, 80.0, 160.0, 230.0]; // no clean aspects
        let patterns = detect_patterns(&positions, 5.0);
        let major = patterns
            .iter()
            .filter(|(p, _)| *p != AspectPattern::Stellium)
            .count();
        assert_eq!(
            major, 0,
            "Scattered positions shouldn't form major patterns"
        );
    }

    // ── Grand Cross ──────────────────────────────────────────────────────

    #[test]
    fn grand_cross_detected() {
        // Two oppositions (0–180, 90–270), all four sides square.
        let positions = vec![0.0, 90.0, 180.0, 270.0];
        let patterns = detect_patterns(&positions, 6.0);
        let gc: Vec<_> = patterns
            .iter()
            .filter(|(p, _)| *p == AspectPattern::GrandCross)
            .collect();
        assert!(
            !gc.is_empty(),
            "Grand Cross must be detected, got {patterns:#?}"
        );
        // All four bodies participate.
        let mut members = gc[0].1.clone();
        members.sort();
        assert_eq!(members, vec![0, 1, 2, 3]);
        // Members are ordered as two opposition axes: (0,1) and (2,3).
        let m = &gc[0].1;
        assert!((angular_distance(positions[m[0]], positions[m[1]]) - 180.0).abs() < 6.0);
        assert!((angular_distance(positions[m[2]], positions[m[3]]) - 180.0).abs() < 6.0);
    }

    #[test]
    fn grand_cross_with_orb() {
        // Slightly off but within 6° orb.
        let positions = vec![1.0, 92.0, 179.0, 271.0];
        let patterns = detect_patterns(&positions, 6.0);
        assert!(
            patterns
                .iter()
                .any(|(p, _)| *p == AspectPattern::GrandCross)
        );
    }

    #[test]
    fn grand_cross_not_from_grand_trine() {
        // A grand trine alone (3 bodies) must NOT produce a Grand Cross.
        let positions = vec![10.0, 130.0, 250.0];
        let patterns = detect_patterns(&positions, 8.0);
        assert!(
            !patterns
                .iter()
                .any(|(p, _)| *p == AspectPattern::GrandCross)
        );
    }

    // ── Kite ─────────────────────────────────────────────────────────────

    #[test]
    fn kite_detected() {
        // Grand trine at 0,120,240; tail at 180 opposes apex 0 and sextiles
        // the two wings (120, 240).
        let positions = vec![0.0, 120.0, 240.0, 180.0];
        let patterns = detect_patterns(&positions, 6.0);
        let kites: Vec<_> = patterns
            .iter()
            .filter(|(p, _)| *p == AspectPattern::Kite)
            .collect();
        assert!(
            !kites.is_empty(),
            "Kite must be detected, got {patterns:#?}"
        );
        // members = [tail, apex, wing1, wing2]; tail opposes apex.
        let m = &kites[0].1;
        assert!(
            (angular_distance(positions[m[0]], positions[m[1]]) - 180.0).abs() < 6.0,
            "tail must oppose apex"
        );
        // tail sextiles both wings.
        assert!((angular_distance(positions[m[0]], positions[m[2]]) - 60.0).abs() < 6.0);
        assert!((angular_distance(positions[m[0]], positions[m[3]]) - 60.0).abs() < 6.0);
    }

    #[test]
    fn kite_requires_grand_trine() {
        // Two oppositions but NO 120° grand-trine spine → no kite. (0,30,180,210
        // has only 30/150/180° relations among its bodies — no trine triple.)
        let positions = vec![0.0, 30.0, 180.0, 210.0];
        let patterns = detect_patterns(&positions, 5.0);
        assert!(!patterns.iter().any(|(p, _)| *p == AspectPattern::Kite));
    }

    // ── Mystic Rectangle ──────────────────────────────────────────────────

    #[test]
    fn mystic_rectangle_detected() {
        // Two oppositions (0–180, 60–240) joined by two trines and two sextiles.
        let positions = vec![0.0, 60.0, 180.0, 240.0];
        let patterns = detect_patterns(&positions, 6.0);
        let mr: Vec<_> = patterns
            .iter()
            .filter(|(p, _)| *p == AspectPattern::MysticRectangle)
            .collect();
        assert!(
            !mr.is_empty(),
            "Mystic Rectangle must be detected, got {patterns:#?}"
        );
        // members ordered as two opposition axes (0,1) and (2,3).
        let m = &mr[0].1;
        assert!((angular_distance(positions[m[0]], positions[m[1]]) - 180.0).abs() < 6.0);
        assert!((angular_distance(positions[m[2]], positions[m[3]]) - 180.0).abs() < 6.0);
    }

    #[test]
    fn mystic_rectangle_not_grand_cross() {
        // The mystic rectangle (trine/sextile sides) must NOT register as a
        // Grand Cross (which needs square sides).
        let positions = vec![0.0, 60.0, 180.0, 240.0];
        let patterns = detect_patterns(&positions, 6.0);
        assert!(
            !patterns
                .iter()
                .any(|(p, _)| *p == AspectPattern::GrandCross),
            "Mystic Rectangle should not be a Grand Cross"
        );
    }

    #[test]
    fn grand_cross_not_mystic_rectangle() {
        // The Grand Cross (square sides) must NOT register as a Mystic
        // Rectangle (which needs trine/sextile sides).
        let positions = vec![0.0, 90.0, 180.0, 270.0];
        let patterns = detect_patterns(&positions, 6.0);
        assert!(
            !patterns
                .iter()
                .any(|(p, _)| *p == AspectPattern::MysticRectangle),
            "Grand Cross should not be a Mystic Rectangle"
        );
    }

    #[test]
    fn scattered_four_no_complex_patterns() {
        // No GrandCross / Kite / MysticRectangle from random spread.
        let positions = vec![5.0, 47.0, 158.0, 211.0];
        let patterns = detect_patterns(&positions, 5.0);
        assert!(!patterns.iter().any(|(p, _)| matches!(
            p,
            AspectPattern::GrandCross | AspectPattern::Kite | AspectPattern::MysticRectangle
        )));
    }

    // -------------------------------------------------------------------
    // Named-body API tests, ported/adapted from Anonyfox/celestine (MIT),
    // commit 954d63315ec00d29ba4becaef3f6a101497946b7:
    //   src/aspects/patterns.test.ts (PERFECT_* fixtures + JPL J2000.0 set)
    // See docs/THIRD_PARTY_SOURCES.md.
    // -------------------------------------------------------------------

    use crate::aspects::{AspectConfig, AspectType, find_all_aspects_ex};

    fn aspects_for(bodies: &[(&str, f64)]) -> Vec<AspectResult> {
        let positions: Vec<(String, f64, Option<f64>)> = bodies
            .iter()
            .map(|(n, lon)| (n.to_string(), *lon, None))
            .collect();
        let config = AspectConfig::default().with_aspect_types(AspectType::ALL_14);
        find_all_aspects_ex(&positions, &config)
    }

    #[test]
    fn named_t_square_positive() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 180.0), ("C", 90.0)]);
        let patterns = detect_t_square_named(&aspects);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, AspectPatternType::TSquare);
        let mut bodies = patterns[0].bodies.clone();
        bodies.sort();
        assert_eq!(bodies, vec!["A", "B", "C"]);
    }

    #[test]
    fn named_t_square_negative_without_opposition() {
        let aspects = aspects_for(&[("A", 0.0), ("C", 90.0)]);
        assert!(detect_t_square_named(&aspects).is_empty());
    }

    #[test]
    fn named_grand_trine_positive() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 120.0), ("C", 240.0)]);
        let patterns = detect_grand_trine_named(&aspects);
        assert_eq!(patterns.len(), 1);
        assert!(
            patterns[0]
                .aspects
                .iter()
                .all(|a| a.aspect_type == AspectType::Trine)
        );
    }

    #[test]
    fn named_grand_trine_negative_only_two_trines() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 120.0), ("C", 250.0)]);
        assert!(detect_grand_trine_named(&aspects).is_empty());
    }

    #[test]
    fn named_grand_cross_positive() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 90.0), ("C", 180.0), ("D", 270.0)]);
        let patterns = detect_grand_cross_named(&aspects);
        assert_eq!(patterns.len(), 1);
        let squares = patterns[0]
            .aspects
            .iter()
            .filter(|a| a.aspect_type == AspectType::Square)
            .count();
        let oppositions = patterns[0]
            .aspects
            .iter()
            .filter(|a| a.aspect_type == AspectType::Opposition)
            .count();
        assert_eq!((squares, oppositions), (4, 2));
    }

    #[test]
    fn named_grand_cross_negative_missing_one_square() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 90.0), ("C", 180.0), ("D", 250.0)]);
        assert!(detect_grand_cross_named(&aspects).is_empty());
    }

    #[test]
    fn named_yod_positive() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 60.0), ("C", 210.0)]);
        let patterns = detect_yod_named(&aspects);
        assert_eq!(patterns.len(), 1);
        let quincunxes = patterns[0]
            .aspects
            .iter()
            .filter(|a| a.aspect_type == AspectType::Quincunx)
            .count();
        assert_eq!(quincunxes, 2);
    }

    #[test]
    fn named_yod_near_miss() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 60.0), ("C", 215.0)]);
        assert!(detect_yod_named(&aspects).is_empty());
    }

    #[test]
    fn named_kite_positive() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 120.0), ("C", 240.0), ("D", 180.0)]);
        let patterns = detect_kite_named(&aspects);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].bodies.len(), 4);
    }

    #[test]
    fn named_kite_near_miss_no_opposition() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 120.0), ("C", 240.0), ("D", 150.0)]);
        assert!(detect_kite_named(&aspects).is_empty());
    }

    #[test]
    fn named_mystic_rectangle_positive() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 60.0), ("C", 180.0), ("D", 240.0)]);
        let patterns = detect_mystic_rectangle_named(&aspects);
        assert_eq!(patterns.len(), 1);
        let opps = patterns[0]
            .aspects
            .iter()
            .filter(|a| a.aspect_type == AspectType::Opposition)
            .count();
        let trines = patterns[0]
            .aspects
            .iter()
            .filter(|a| a.aspect_type == AspectType::Trine)
            .count();
        let sextiles = patterns[0]
            .aspects
            .iter()
            .filter(|a| a.aspect_type == AspectType::Sextile)
            .count();
        assert_eq!((opps, trines, sextiles), (2, 2, 2));
    }

    #[test]
    fn named_mystic_rectangle_negative_missing_sextile() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 60.0), ("C", 180.0), ("D", 220.0)]);
        assert!(detect_mystic_rectangle_named(&aspects).is_empty());
    }

    #[test]
    fn named_stellium_positive_four_bodies() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 5.0), ("C", 8.0), ("D", 3.0)]);
        let patterns = detect_stellium_named(&aspects);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].bodies.len(), 4);
    }

    #[test]
    fn named_stellium_negative_only_two_bodies() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 5.0)]);
        assert!(detect_stellium_named(&aspects).is_empty());
    }

    #[test]
    fn named_stellium_minimum_three_bodies() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 5.0), ("C", 8.0)]);
        let patterns = detect_stellium_named(&aspects);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].bodies.len(), 3);
    }

    #[test]
    fn named_find_patterns_empty_for_no_pattern() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 5.0)]);
        assert!(find_named_patterns(&aspects).is_empty());
    }

    #[test]
    fn named_find_patterns_detects_grand_trine_in_isolation() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 120.0), ("C", 240.0)]);
        let patterns = find_named_patterns(&aspects);
        assert!(
            patterns
                .iter()
                .any(|p| p.pattern_type == AspectPatternType::GrandTrine)
        );
    }

    #[test]
    fn named_grand_trine_across_0_360_boundary() {
        let aspects = aspects_for(&[("A", 350.0), ("B", 110.0), ("C", 230.0)]);
        let patterns = detect_grand_trine_named(&aspects);
        assert_eq!(
            patterns.len(),
            1,
            "Grand Trine must be detected even when a vertex is near 0/360"
        );
    }

    #[test]
    fn named_no_duplicate_patterns_for_same_body_set() {
        let aspects = aspects_for(&[("A", 0.0), ("B", 120.0), ("C", 240.0)]);
        assert_eq!(detect_grand_trine_named(&aspects).len(), 1);
    }

    #[test]
    fn named_j2000_real_data_does_not_panic() {
        let bodies: &[(&str, f64)] = &[
            ("Sun", 280.3689092),
            ("Moon", 223.323786),
            ("Mercury", 271.8892699),
            ("Venus", 241.5657794),
            ("Mars", 327.9632921),
            ("Jupiter", 25.2530685),
            ("Saturn", 40.3956366),
            ("Uranus", 314.809168),
            ("Neptune", 303.1930003),
            ("Pluto", 251.4547644),
        ];
        let aspects = aspects_for(bodies);
        let _patterns = find_named_patterns(&aspects);
    }
}
