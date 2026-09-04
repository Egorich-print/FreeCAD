//! Measure / Inspect — Fusion-like distance/angle helpers.
//! Ponytail: pure math, no OCCT.

/// 3D point
pub type Point3 = [f64; 3];

/// Distance between two points
pub fn distance(a: Point3, b: Point3) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Angle between vectors ab and ac (0..pi) in radians
pub fn angle_at(a: Point3, b: Point3, c: Point3) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let dot = ab[0] * ac[0] + ab[1] * ac[1] + ab[2] * ac[2];
    let la = (ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2]).sqrt();
    let lc = (ac[0] * ac[0] + ac[1] * ac[1] + ac[2] * ac[2]).sqrt();
    if la < 1e-12 || lc < 1e-12 {
        return 0.0;
    }
    let cos = (dot / (la * lc)).clamp(-1.0, 1.0);
    cos.acos()
}

/// Bounding box diagonal (measure overall size)
pub fn bbox_diagonal(min: Point3, max: Point3) -> f64 {
    distance(min, max)
}

/// Closest distance point → segment ab
/// Returns NaN for non-finite inputs (does not mask bad geometry).
pub fn point_to_segment_dist(p: Point3, a: Point3, b: Point3) -> f64 {
    if ![p, a, b].iter().all(|pt| pt.iter().all(|v| v.is_finite())) {
        return f64::NAN;
    }
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let ab_len2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    if ab_len2 < 1e-18 {
        return distance(p, a);
    }
    let t = (ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / ab_len2;
    if !t.is_finite() {
        return f64::NAN;
    }
    let t = t.clamp(0.0, 1.0);
    let proj = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
    distance(p, proj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn dist_basic() {
        assert!((distance([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]) - 5.0).abs() < 1e-9);
        assert_eq!(distance([1.0, 1.0, 1.0], [1.0, 1.0, 1.0]), 0.0);
    }

    #[test]
    fn angle_right() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let c = [0.0, 1.0, 0.0];
        assert!((angle_at(a, b, c) - PI / 2.0).abs() < 1e-9);
    }

    #[test]
    fn angle_straight() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let c = [2.0, 0.0, 0.0];
        assert!((angle_at(a, b, c)).abs() < 1e-9);
    }

    #[test]
    fn bbox_diag() {
        assert!((bbox_diagonal([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]) - 3.0_f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn point_to_segment() {
        assert!(
            (point_to_segment_dist([0.5, 1.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]) - 1.0).abs()
                < 1e-9
        );
        assert!(
            (point_to_segment_dist([2.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]) - 1.0).abs()
                < 1e-9
        );
    }
}
