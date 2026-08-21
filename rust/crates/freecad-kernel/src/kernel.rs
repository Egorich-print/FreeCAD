use core::fmt;

use freecad_core::mesh::MeshBuffer;

use crate::error::KernelError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShapeStats {
    pub vertices: u64,
    pub edges: u64,
    pub faces: u64,
    pub solids: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Bounds {
    pub fn center(&self) -> [f64; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    pub fn diagonal(&self) -> f64 {
        let e = [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ];
        (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt()
    }
}

/// The single geometry contract of the Rust stack.
///
/// Every backend (OCCT today, a native Rust kernel eventually) implements
/// exactly this surface; no OCCT type may appear above this trait.
pub trait GeometryKernel {
    /// Opaque handle to a shape owned by the kernel.
    type Shape: Copy + Eq + fmt::Debug;
    type Error: std::error::Error;

    fn make_box(&mut self, dx: f64, dy: f64, dz: f64) -> Result<Self::Shape, Self::Error>;
    fn make_sphere(&mut self, radius: f64) -> Result<Self::Shape, Self::Error>;
    fn make_cylinder(&mut self, radius: f64, height: f64) -> Result<Self::Shape, Self::Error>;

    /// Copy the shape with a translation applied; the source stays alive.
    fn move_by(
        &mut self,
        shape: &Self::Shape,
        dx: f64,
        dy: f64,
        dz: f64,
    ) -> Result<Self::Shape, Self::Error>;

    fn read_step(&mut self, data: &[u8]) -> Result<Self::Shape, Self::Error>;
    fn read_brep(&mut self, data: &[u8]) -> Result<Self::Shape, Self::Error>;
    fn write_step(&mut self, shape: &Self::Shape) -> Result<Vec<u8>, Self::Error>;
    fn write_brep(&mut self, shape: &Self::Shape) -> Result<Vec<u8>, Self::Error>;

    fn fuse(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error>;
    fn cut(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error>;
    fn common(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error>;

    /// Tessellate with absolute linear deflection and angular deflection in radians.
    fn tessellate(
        &mut self,
        shape: &Self::Shape,
        linear_deflection: f64,
        angular_deflection_rad: f64,
    ) -> Result<MeshBuffer, Self::Error>;

    fn destroy(&mut self, shape: Self::Shape);
    fn stats(&self, shape: &Self::Shape) -> Result<ShapeStats, Self::Error>;
    fn bounds(&self, shape: &Self::Shape) -> Result<Bounds, Self::Error>;
}

pub fn validate_deflections(linear: f64, angular_rad: f64) -> Result<(), KernelError> {
    if !(linear.is_finite() && linear > 0.0) || !(angular_rad.is_finite() && angular_rad > 0.0) {
        return Err(KernelError::invalid_input(
            "deflections must be finite and strictly positive",
        ));
    }
    Ok(())
}
