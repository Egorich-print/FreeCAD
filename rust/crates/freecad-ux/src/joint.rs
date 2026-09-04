//! Assembly joints — Fusion-like joint helpers (Ponytail minimal).
//! Coincident, Revolute, Slider, Fixed — pure placement math.

use crate::measure::Point3;

/// Joint type (subset of Fusion; full Assembly solver has 13 — see `AssemblyJointKind`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointType {
    Rigid,    // Fixed
    Revolute, // Hinge
    Slider,   // Prismatic
    Coincident,
}

/// Full Assembly joint vocabulary (`JointObject.py:65-79`, `AssemblyUtils.h:49-64`).
/// Maps onto the [`JointType`] subset where a direct equivalent exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyJointKind {
    Fixed,
    Revolute,
    Cylindrical,
    Slider,
    Ball,
    Distance,
    Parallel,
    Perpendicular,
    Angle,
    RackPinion,
    Screw,
    Gears,
    Belt,
}

impl AssemblyJointKind {
    /// Parse a Python/C++ joint type name (case-sensitive, e.g. `"Revolute"`).
    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            "Fixed" => Some(Self::Fixed),
            "Revolute" => Some(Self::Revolute),
            "Cylindrical" => Some(Self::Cylindrical),
            "Slider" => Some(Self::Slider),
            "Ball" => Some(Self::Ball),
            "Distance" => Some(Self::Distance),
            "Parallel" => Some(Self::Parallel),
            "Perpendicular" => Some(Self::Perpendicular),
            "Angle" => Some(Self::Angle),
            "RackPinion" => Some(Self::RackPinion),
            "Screw" => Some(Self::Screw),
            "Gears" => Some(Self::Gears),
            "Belt" => Some(Self::Belt),
            _ => None,
        }
    }

    /// Map onto the UX subset. Compound/coupled/constraint joints have no direct
    /// equivalent yet (Cylindrical = Revolute+Slider, Ball = 3-DOF, Distance-0 ≈
    /// Coincident is distance-dependent, not type-dependent) → `None`.
    pub fn to_ux_type(self) -> Option<JointType> {
        match self {
            Self::Fixed => Some(JointType::Rigid),
            Self::Revolute => Some(JointType::Revolute),
            Self::Slider => Some(JointType::Slider),
            _ => None,
        }
    }

    /// Joints animatable in motion preview (`CommandCreateSimulation.py:736`).
    pub fn is_animatable(self) -> bool {
        matches!(self, Self::Revolute | Self::Slider | Self::Cylindrical)
    }
}

/// Joint definition between two placements (origin + Z axis)
#[derive(Debug, Clone, Copy)]
pub struct Joint {
    pub joint_type: JointType,
    /// Joint origins in world
    pub origin_a: Point3,
    pub origin_b: Point3,
    /// Joint Z axes (normalized)
    pub axis_a: Point3,
    pub axis_b: Point3,
}

impl Joint {
    /// Translation to align B to A (rigid)
    pub fn translation(&self) -> Point3 {
        [
            self.origin_a[0] - self.origin_b[0],
            self.origin_a[1] - self.origin_b[1],
            self.origin_a[2] - self.origin_b[2],
        ]
    }

    /// Angle between axes (0..pi)
    pub fn axis_angle(&self) -> f64 {
        let dot = self.axis_a[0] * self.axis_b[0]
            + self.axis_a[1] * self.axis_b[1]
            + self.axis_a[2] * self.axis_b[2];
        let la = (self.axis_a[0] * self.axis_a[0]
            + self.axis_a[1] * self.axis_a[1]
            + self.axis_a[2] * self.axis_a[2])
            .sqrt();
        let lb = (self.axis_b[0] * self.axis_b[0]
            + self.axis_b[1] * self.axis_b[1]
            + self.axis_b[2] * self.axis_b[2])
            .sqrt();
        if la < 1e-12 || lb < 1e-12 {
            return 0.0;
        }
        (dot / (la * lb)).clamp(-1.0, 1.0).acos()
    }

    /// Check if coincident within tol
    /// Returns false for non-finite or negative tolerance (indistinguishable from "far").
    pub fn is_coincident(&self, tol: f64) -> bool {
        if !tol.is_finite() || tol < 0.0 {
            return false;
        }
        let t = self.translation();
        if !t.iter().all(|v| v.is_finite()) {
            return false;
        }
        (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt() <= tol
    }

    /// Clamp a joint translation/rotation drag value into enabled limits.
    /// Mirrors AssemblyObject.cpp limit logic (auto-swap inverted min/max).
    pub fn clamp_limit(
        value: f64,
        min: f64,
        max: f64,
        enabled_min: bool,
        enabled_max: bool,
    ) -> f64 {
        if !value.is_finite() {
            return 0.0;
        }
        let (mut lo, mut hi) = (min, max);
        if lo.is_finite() && hi.is_finite() && lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        let mut v = value;
        if enabled_min && lo.is_finite() && v < lo {
            v = lo;
        }
        if enabled_max && hi.is_finite() && v > hi {
            v = hi;
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn translation_align() {
        let j = Joint {
            joint_type: JointType::Rigid,
            origin_a: [0.0, 0.0, 0.0],
            origin_b: [1.0, 2.0, 3.0],
            axis_a: [0.0, 0.0, 1.0],
            axis_b: [0.0, 0.0, 1.0],
        };
        assert_eq!(j.translation(), [-1.0, -2.0, -3.0]);
        assert!(j.is_coincident(10.0));
        assert!(!j.is_coincident(0.1));
    }

    #[test]
    fn axis_angle() {
        let j = Joint {
            joint_type: JointType::Revolute,
            origin_a: [0.0; 3],
            origin_b: [0.0; 3],
            axis_a: [1.0, 0.0, 0.0],
            axis_b: [0.0, 1.0, 0.0],
        };
        assert!((j.axis_angle() - PI / 2.0).abs() < 1e-9);
    }

    #[test]
    fn coincident_tol() {
        let j = Joint {
            joint_type: JointType::Coincident,
            origin_a: [0.0, 0.0, 0.0],
            origin_b: [0.0, 0.0, 0.0005],
            axis_a: [0.0, 0.0, 1.0],
            axis_b: [0.0, 0.0, 1.0],
        };
        assert!(j.is_coincident(0.001));
        assert!(!j.is_coincident(0.0001));
        assert!(!j.is_coincident(f64::NAN));
        assert!(!j.is_coincident(-1.0));
    }

    #[test]
    fn assembly_kind_mapping() {
        assert_eq!(
            AssemblyJointKind::from_str("Revolute"),
            Some(AssemblyJointKind::Revolute)
        );
        assert_eq!(AssemblyJointKind::from_str("Nope"), None);
        assert_eq!(
            AssemblyJointKind::Fixed.to_ux_type(),
            Some(JointType::Rigid)
        );
        assert_eq!(
            AssemblyJointKind::Slider.to_ux_type(),
            Some(JointType::Slider)
        );
        assert_eq!(AssemblyJointKind::Cylindrical.to_ux_type(), None);
        assert_eq!(AssemblyJointKind::Ball.to_ux_type(), None);
        assert!(AssemblyJointKind::Revolute.is_animatable());
        assert!(AssemblyJointKind::Cylindrical.is_animatable());
        assert!(!AssemblyJointKind::Fixed.is_animatable());
        assert!(!AssemblyJointKind::Gears.is_animatable());
    }

    #[test]
    fn clamp_limit_swaps_and_guards() {
        assert_eq!(Joint::clamp_limit(5.0, 10.0, 0.0, true, true), 5.0);
        assert_eq!(Joint::clamp_limit(-5.0, 0.0, 10.0, true, true), 0.0);
        assert_eq!(Joint::clamp_limit(15.0, 0.0, 10.0, false, true), 10.0);
        assert_eq!(Joint::clamp_limit(15.0, 0.0, 10.0, false, false), 15.0);
        assert_eq!(Joint::clamp_limit(f64::NAN, 0.0, 10.0, true, true), 0.0);
    }
}
