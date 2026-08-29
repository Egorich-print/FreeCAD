//! Chamfer drag logic — Fusion-style radius picker.
//!
//! Mirrors TaskChamferParameters gizmo math but in pure Rust for testability.

/// Chamfer type mirrors Part::ChamferType (TopoShape.h:220)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ChamferType {
    EqualDistance = 0,
    TwoDistances = 1,
    DistanceAngle = 2,
}

impl ChamferType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::TwoDistances,
            2 => Self::DistanceAngle,
            _ => Self::EqualDistance,
        }
    }
}

/// Parameters for a chamfer feature
#[derive(Debug, Clone, Copy)]
pub struct ChamferParams {
    pub chamfer_type: ChamferType,
    pub size: f64,
    pub size2: f64,
    pub angle_deg: f64,
    /// true = flipped direction (swap face normals)
    pub flip: bool,
}

impl Default for ChamferParams {
    fn default() -> Self {
        Self {
            chamfer_type: ChamferType::EqualDistance,
            size: 1.0,
            size2: 1.0,
            angle_deg: 45.0,
            flip: false,
        }
    }
}

/// Validate chamfer params — mirrors FeatureChamfer::execute checks
pub fn validate_chamfer(p: &ChamferParams) -> Result<(), &'static str> {
    if !p.size.is_finite() || !p.size2.is_finite() || !p.angle_deg.is_finite() {
        return Err("params must be finite");
    }
    if p.size < 0.0 || p.size2 < 0.0 {
        return Err("size must be >= 0");
    }
    // angle only relevant for DistanceAngle; other types ignore it (keep lenient)
    if p.chamfer_type == ChamferType::DistanceAngle && (p.angle_deg < 0.0 || p.angle_deg > 180.0) {
        return Err("angle must be in [0,180]");
    }
    // BRepFilletAPI_MakeChamfer fails if size is too large for edge; we clamp at 1e4
    if p.size > 1e4 || p.size2 > 1e4 {
        return Err("size too large");
    }
    Ok(())
}

/// Convert linear drag delta (mm along face normal) to new Size value.
/// Mirrors LinearGizmo::draggingContinued: value = initial + dragLength
/// where dragLength = (incCount*inc - addFactor)/multFactor
pub fn drag_to_value(initial: f64, drag_length: f64, min: f64, max: f64) -> f64 {
    if !initial.is_finite() || !drag_length.is_finite() || !min.is_finite() || !max.is_finite() {
        return min;
    }
    // defensive: ensure min <= max
    let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
    let mut v = initial + drag_length;
    if v < lo {
        v = lo;
    }
    if v > hi {
        v = hi;
    }
    v
}

/// Snap helper: coarse snap when Shift held, fine when Ctrl
pub fn snap_value(value: f64, step: f64, coarse: bool) -> f64 {
    if step <= 0.0 {
        return value;
    }
    let s = if coarse { step * 5.0 } else { step };
    (value / s).round() * s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_adds_correctly() {
        assert_eq!(drag_to_value(1.0, 0.5, 0.0, 10.0), 1.5);
        assert_eq!(drag_to_value(1.0, -2.0, 0.0, 10.0), 0.0); // clamp min
        assert_eq!(drag_to_value(9.0, 2.0, 0.0, 10.0), 10.0); // clamp max
    }

    #[test]
    fn validate_rejects_negative() {
        let mut p = ChamferParams::default();
        p.size = -1.0;
        assert!(validate_chamfer(&p).is_err());
    }

    #[test]
    fn validate_angle_range() {
        let mut p = ChamferParams::default();
        p.chamfer_type = ChamferType::DistanceAngle;
        p.angle_deg = 200.0;
        assert!(validate_chamfer(&p).is_err());
        p.angle_deg = 45.0;
        assert!(validate_chamfer(&p).is_ok());
    }

    #[test]
    fn snap_coarse_fine() {
        assert!((snap_value(1.23, 0.1, false) - 1.2).abs() < 1e-9);
        assert!((snap_value(1.23, 0.1, true) - 1.0).abs() < 1e-9); // coarse 0.5 step
    }

    #[test]
    fn chamfer_type_roundtrip() {
        assert_eq!(ChamferType::from_i32(0), ChamferType::EqualDistance);
        assert_eq!(ChamferType::from_i32(1), ChamferType::TwoDistances);
        assert_eq!(ChamferType::from_i32(2), ChamferType::DistanceAngle);
        assert_eq!(ChamferType::from_i32(99), ChamferType::EqualDistance);
    }
}
