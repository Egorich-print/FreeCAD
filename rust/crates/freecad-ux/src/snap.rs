//! Snap logic mirrors SnapManager.cpp — line/arc middle snap.
//! Ponytail: pure functions, no viewer dependency.

/// Returns true if point should snap to line middle.
/// Threshold = 5% of length (SnapManager.cpp:343)
pub fn snap_to_line_middle(point: &mut [f64; 2], start: [f64; 2], end: [f64; 2]) -> bool {
    let len = ((end[0] - start[0]).powi(2) + (end[1] - start[1]).powi(2)).sqrt();
    if len < 1e-9 {
        return false;
    }
    let mid = [(start[0] + end[0]) * 0.5, (start[1] + end[1]) * 0.5];
    let dist = ((point[0] - mid[0]).powi(2) + (point[1] - mid[1]).powi(2)).sqrt();
    if dist < len * 0.05 {
        point[0] = mid[0];
        point[1] = mid[1];
        true
    } else {
        false
    }
}

/// Returns true if point should snap to arc middle (bisector).
/// Threshold = 10% of sweep angle (SnapManager.cpp:358)
pub fn snap_to_arc_middle(
    point: &mut [f64; 2],
    center: [f64; 2],
    radius: f64,
    start_angle: f64,
    end_angle: f64,
) -> bool {
    if radius < 1e-9 {
        return false;
    }
    let mut sweep = end_angle - start_angle;
    // normalize sweep to (-pi, pi] then take abs
    while sweep > std::f64::consts::PI {
        sweep -= 2.0 * std::f64::consts::PI;
    }
    while sweep <= -std::f64::consts::PI {
        sweep += 2.0 * std::f64::consts::PI;
    }
    let sweep_abs = sweep.abs();
    if sweep_abs < 1e-9 {
        return false;
    }
    let mid_angle = start_angle + sweep * 0.5;
    let mid = [
        center[0] + radius * mid_angle.cos(),
        center[1] + radius * mid_angle.sin(),
    ];
    // angular distance from point to mid
    let pt_angle = (point[1] - center[1]).atan2(point[0] - center[0]);
    let mut diff = (pt_angle - mid_angle).abs();
    while diff > std::f64::consts::PI {
        diff = (2.0 * std::f64::consts::PI - diff).abs();
    }
    if diff < sweep_abs * 0.10 {
        point[0] = mid[0];
        point[1] = mid[1];
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_middle_snaps() {
        let mut p = [5.1, 0.0];
        let snapped = snap_to_line_middle(&mut p, [0.0, 0.0], [10.0, 0.0]);
        assert!(snapped);
        assert_eq!(p, [5.0, 0.0]);
    }

    #[test]
    fn line_middle_no_snap_far() {
        let mut p = [0.0, 0.0];
        let snapped = snap_to_line_middle(&mut p, [0.0, 0.0], [10.0, 0.0]);
        assert!(!snapped);
    }

    #[test]
    fn arc_middle_snaps() {
        // center 0,0 radius 10 from 0 to 90deg, mid is 45deg => (7.07,7.07)
        let mut p = [7.0, 7.2];
        let snapped =
            snap_to_arc_middle(&mut p, [0.0, 0.0], 10.0, 0.0, std::f64::consts::FRAC_PI_2);
        assert!(snapped);
        assert!((p[0] - 7.07).abs() < 0.1);
    }

    #[test]
    fn arc_middle_no_snap() {
        let mut p = [10.0, 0.0];
        let snapped =
            snap_to_arc_middle(&mut p, [0.0, 0.0], 10.0, 0.0, std::f64::consts::FRAC_PI_2);
        assert!(!snapped);
    }
}
