//! Assembly joints — Fusion-like joint helpers (Ponytail minimal).
//! Coincident, Revolute, Slider, Fixed — pure placement math.

use crate::measure::Point3;

/// Joint type (subset of Fusion)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointType {
    Rigid,    // Fixed
    Revolute, // Hinge
    Slider,   // Prismatic
    Coincident,
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
    pub fn is_coincident(&self, tol: f64) -> bool {
        let t = self.translation();
        (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt() <= tol
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
    }
}
