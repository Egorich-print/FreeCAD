use freecad_core::ShapeId;
use freecad_core::mesh::MeshBuffer;

use crate::error::{KernelError, KernelErrorKind};
use crate::kernel::{Bounds, GeometryKernel, ShapeStats};

/// In-memory reference kernel used by tests and as the concrete first user of
/// the `GeometryKernel` contract before any C++ backend is linked.
#[derive(Debug, Default)]
pub struct MockKernel {
    next_id: u64,
    live_shapes: Vec<ShapeId>,
    op_log: Vec<&'static str>,
}

impl MockKernel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn live_shape_count(&self) -> usize {
        self.live_shapes.len()
    }

    pub fn op_log(&self) -> &[&'static str] {
        &self.op_log
    }

    fn allocate(&mut self, op: &'static str) -> ShapeId {
        let id = ShapeId(self.next_id);
        self.next_id += 1;
        self.live_shapes.push(id);
        self.op_log.push(op);
        id
    }

    fn check_live(&self, shape: ShapeId) -> Result<(), KernelError> {
        if self.live_shapes.contains(&shape) {
            Ok(())
        } else {
            Err(KernelError::new(
                KernelErrorKind::InvalidInput,
                format!("{shape} does not exist or was destroyed"),
            ))
        }
    }
}

fn unit_cube_mesh(scale: f64) -> MeshBuffer {
    let cube = freecad_core::prim::cube(scale as f32);
    MeshBuffer {
        positions: cube.positions,
        normals: cube.normals,
        indices: cube.indices,
        faces: cube.faces,
    }
}

impl GeometryKernel for MockKernel {
    type Shape = ShapeId;
    type Error = KernelError;

    fn make_box(&mut self, dx: f64, dy: f64, dz: f64) -> Result<Self::Shape, Self::Error> {
        if !(dx > 0.0 && dy > 0.0 && dz > 0.0) {
            return Err(KernelError::invalid_input(
                "box dimensions must be positive",
            ));
        }
        Ok(self.allocate("make_box"))
    }

    fn make_sphere(&mut self, radius: f64) -> Result<Self::Shape, Self::Error> {
        if radius <= 0.0 {
            return Err(KernelError::invalid_input("sphere radius must be positive"));
        }
        Ok(self.allocate("make_sphere"))
    }

    fn make_cylinder(&mut self, radius: f64, height: f64) -> Result<Self::Shape, Self::Error> {
        if radius <= 0.0 || height <= 0.0 {
            return Err(KernelError::invalid_input(
                "cylinder dimensions must be positive",
            ));
        }
        Ok(self.allocate("make_cylinder"))
    }

    fn move_by(
        &mut self,
        shape: &Self::Shape,
        _dx: f64,
        _dy: f64,
        _dz: f64,
    ) -> Result<Self::Shape, Self::Error> {
        self.check_live(*shape)?;
        Ok(self.allocate("move_by"))
    }

    fn read_step(&mut self, data: &[u8]) -> Result<Self::Shape, Self::Error> {
        let text = core::str::from_utf8(data)
            .map_err(|_| KernelError::parse("step payload is not utf-8"))?;
        if !text.contains("ISO-10303-21") {
            return Err(KernelError::parse("payload lacks ISO-10303-21 STEP header"));
        }
        Ok(self.allocate("read_step"))
    }

    fn read_brep(&mut self, data: &[u8]) -> Result<Self::Shape, Self::Error> {
        if !data.starts_with(b"DBRep") && !data.starts_with(b"CASCADE Topology V") {
            return Err(KernelError::parse("payload lacks OCCT BREP signature"));
        }
        Ok(self.allocate("read_brep"))
    }

    fn write_step(&mut self, shape: &Self::Shape) -> Result<Vec<u8>, Self::Error> {
        self.check_live(*shape)?;
        Ok(b"ISO-10303-21;\nDATA;\nEND-ISO-10303-21;\n".to_vec())
    }

    fn write_brep(&mut self, shape: &Self::Shape) -> Result<Vec<u8>, Self::Error> {
        self.check_live(*shape)?;
        Ok(b"DBRep Drawer\nCASCADE Topology V3\nEnd\n".to_vec())
    }

    fn fuse(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error> {
        self.check_live(*a)?;
        self.check_live(*b)?;
        Ok(self.allocate("fuse"))
    }

    fn cut(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error> {
        self.check_live(*a)?;
        self.check_live(*b)?;
        Ok(self.allocate("cut"))
    }

    fn common(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error> {
        self.check_live(*a)?;
        self.check_live(*b)?;
        Ok(self.allocate("common"))
    }

    fn tessellate(
        &mut self,
        shape: &Self::Shape,
        _linear_deflection: f64,
        _angular_deflection_rad: f64,
    ) -> Result<MeshBuffer, Self::Error> {
        self.check_live(*shape)?;
        let mut mesh = unit_cube_mesh(1.0);
        for range in &mut mesh.faces {
            range.face_id += 100;
        }
        Ok(mesh)
    }

    fn destroy(&mut self, shape: Self::Shape) {
        self.live_shapes.retain(|s| *s != shape);
    }

    fn stats(&self, shape: &Self::Shape) -> Result<ShapeStats, Self::Error> {
        self.check_live(*shape)?;
        Ok(ShapeStats {
            vertices: 8,
            edges: 12,
            faces: 6,
            solids: 1,
        })
    }

    fn bounds(&self, shape: &Self::Shape) -> Result<Bounds, Self::Error> {
        self.check_live(*shape)?;
        Ok(Bounds {
            min: [-0.5; 3],
            max: [0.5; 3],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_and_validation() {
        let mut k = MockKernel::new();
        let a = k.make_box(1.0, 2.0, 3.0).unwrap();
        let b = k.make_sphere(1.0).unwrap();
        assert_eq!(k.live_shape_count(), 2);

        let fused = k.fuse(&a, &b).unwrap();
        assert_eq!(k.live_shape_count(), 3);

        let mesh = k.tessellate(&fused, 0.1, 0.5).unwrap();
        mesh.validate().expect("mock mesh must validate");

        let bytes = k.write_brep(&fused).unwrap();
        assert!(bytes.starts_with(b"DBRep"));

        k.destroy(a);
        assert!(k.fuse(&a, &b).is_err(), "destroyed handle must be rejected");
        assert_eq!(k.live_shape_count(), 2);
    }

    #[test]
    fn parse_errors_are_typed() {
        let mut k = MockKernel::new();
        match k.read_step(b"garbage") {
            Err(e) => assert_eq!(e.kind, KernelErrorKind::Parse),
            Ok(_) => panic!("expected parse error"),
        }
    }
}
