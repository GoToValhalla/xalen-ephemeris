//! Relationship-chart primitives used by composite and Davison charts.
//!
//! These are deterministic geometry/time primitives.  They intentionally do
//! not own chart rendering or interpretation; callers feed their outputs back
//! into the normal XALEN chart pipeline.

use serde::{Deserialize, Serialize};

/// Midpoint of two circular longitudes along the shortest arc.
pub fn circular_midpoint_deg(a_deg: f64, b_deg: f64) -> f64 {
    let a = a_deg.rem_euclid(360.0);
    let b = b_deg.rem_euclid(360.0);
    let delta = (b - a + 540.0).rem_euclid(360.0) - 180.0;
    (a + delta / 2.0).rem_euclid(360.0)
}

/// One point in a midpoint composite chart.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositePoint {
    pub name: String,
    pub longitude_deg: f64,
}

/// Compute midpoint-composite longitudes for corresponding named points.
///
/// The two input slices must have the same length and matching names in the
/// same order.  This strict contract prevents accidentally combining unlike
/// chart points.
pub fn composite_midpoints(
    chart_a: &[(String, f64)],
    chart_b: &[(String, f64)],
) -> Result<Vec<CompositePoint>, String> {
    if chart_a.len() != chart_b.len() {
        return Err("composite charts must contain the same number of points".to_string());
    }
    let mut out = Vec::with_capacity(chart_a.len());
    for ((name_a, lon_a), (name_b, lon_b)) in chart_a.iter().zip(chart_b.iter()) {
        if name_a != name_b {
            return Err(format!("composite point mismatch: {name_a} != {name_b}"));
        }
        if !lon_a.is_finite() || !lon_b.is_finite() {
            return Err(format!("non-finite longitude for composite point {name_a}"));
        }
        out.push(CompositePoint {
            name: name_a.clone(),
            longitude_deg: circular_midpoint_deg(*lon_a, *lon_b),
        });
    }
    Ok(out)
}

/// Geographical/time midpoint used to construct a Davison relationship chart.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct DavisonMidpoint {
    pub jd: f64,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
}

/// Compute the Davison midpoint in time and geographic position.
///
/// Longitude is averaged on the sphere's circular axis so locations straddling
/// ±180° do not incorrectly average to Greenwich.
pub fn davison_midpoint(
    jd_a: f64,
    lat_a: f64,
    lon_a: f64,
    jd_b: f64,
    lat_b: f64,
    lon_b: f64,
) -> Result<DavisonMidpoint, String> {
    for (name, value) in [
        ("jd_a", jd_a),
        ("lat_a", lat_a),
        ("lon_a", lon_a),
        ("jd_b", jd_b),
        ("lat_b", lat_b),
        ("lon_b", lon_b),
    ] {
        if !value.is_finite() {
            return Err(format!("{name} must be finite"));
        }
    }
    if !(-90.0..=90.0).contains(&lat_a) || !(-90.0..=90.0).contains(&lat_b) {
        return Err("latitude must be in -90..90".to_string());
    }
    if !(-180.0..=180.0).contains(&lon_a) || !(-180.0..=180.0).contains(&lon_b) {
        return Err("longitude must be in -180..180".to_string());
    }

    let lon_mid_360 = circular_midpoint_deg(lon_a.rem_euclid(360.0), lon_b.rem_euclid(360.0));
    let lon_signed = if lon_mid_360 > 180.0 {
        lon_mid_360 - 360.0
    } else {
        lon_mid_360
    };
    Ok(DavisonMidpoint {
        jd: (jd_a + jd_b) / 2.0,
        latitude_deg: (lat_a + lat_b) / 2.0,
        longitude_deg: lon_signed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circular_midpoint_handles_zero_wrap() {
        let mid = circular_midpoint_deg(350.0, 10.0);
        assert!(mid < 1e-12 || (360.0 - mid).abs() < 1e-12);
    }

    #[test]
    fn composite_requires_matching_names() {
        let a = vec![("Sun".to_string(), 350.0)];
        let b = vec![("Moon".to_string(), 10.0)];
        assert!(composite_midpoints(&a, &b).is_err());
    }

    #[test]
    fn composite_uses_shortest_arc() {
        let a = vec![("Sun".to_string(), 350.0)];
        let b = vec![("Sun".to_string(), 10.0)];
        let result = composite_midpoints(&a, &b).unwrap();
        assert!(result[0].longitude_deg < 1e-12 || (360.0 - result[0].longitude_deg).abs() < 1e-12);
    }

    #[test]
    fn davison_handles_dateline() {
        let m = davison_midpoint(100.0, 10.0, 170.0, 102.0, 20.0, -170.0).unwrap();
        assert_eq!(m.jd, 101.0);
        assert_eq!(m.latitude_deg, 15.0);
        assert!(m.longitude_deg.abs() == 180.0);
    }
}
