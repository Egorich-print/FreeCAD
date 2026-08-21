use cxx::UniquePtr;
use freecad_core::ShapeId;
use freecad_core::mesh::{FaceRange, MeshBuffer};
use freecad_kernel::error::{KernelError, KernelErrorKind};
use freecad_kernel::kernel::{Bounds, GeometryKernel, ShapeStats};

mod bridge {
    #[cxx::bridge]
    pub mod ffi {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct ShapeStatsOut {
            pub vertices: u64,
            pub edges: u64,
            pub faces: u64,
            pub solids: u64,
        }

        #[derive(Debug, Clone, Copy, Default)]
        pub struct BoundsOut {
            pub min_x: f64,
            pub min_y: f64,
            pub min_z: f64,
            pub max_x: f64,
            pub max_y: f64,
            pub max_z: f64,
        }

        #[derive(Debug, Clone, Copy, Default)]
        pub struct FaceRangeOut {
            pub face_id: u32,
            pub index_start: u32,
            pub index_count: u32,
        }

        unsafe extern "C++" {
            include!("occt_shim.h");

            type OcctKernel;

            fn occt_kernel_new() -> UniquePtr<OcctKernel>;

            fn make_box(self: Pin<&mut OcctKernel>, dx: f64, dy: f64, dz: f64) -> u64;
            fn make_sphere(self: Pin<&mut OcctKernel>, radius: f64) -> u64;
            fn make_cylinder(self: Pin<&mut OcctKernel>, radius: f64, height: f64) -> u64;

            fn read_step(self: Pin<&mut OcctKernel>, data: &[u8]) -> u64;
            fn read_brep(self: Pin<&mut OcctKernel>, data: &[u8]) -> u64;
            fn write_step(self: Pin<&mut OcctKernel>, id: u64, out: &mut Vec<u8>) -> bool;
            fn write_brep(self: Pin<&mut OcctKernel>, id: u64, out: &mut Vec<u8>) -> bool;

            fn fuse(self: Pin<&mut OcctKernel>, a: u64, b: u64) -> u64;
            fn cut(self: Pin<&mut OcctKernel>, a: u64, b: u64) -> u64;
            fn common(self: Pin<&mut OcctKernel>, a: u64, b: u64) -> u64;

            fn tessellate(
                self: Pin<&mut OcctKernel>,
                id: u64,
                linear_deflection: f64,
                angular_deflection_rad: f64,
                positions: &mut Vec<f32>,
                normals: &mut Vec<f32>,
                indices: &mut Vec<u32>,
                faces: &mut Vec<FaceRangeOut>,
            ) -> bool;

            fn shape_stats(self: &OcctKernel, id: u64, out: &mut ShapeStatsOut) -> bool;
            fn bounds(self: &OcctKernel, id: u64, out: &mut BoundsOut) -> bool;

            fn destroy_shape(self: Pin<&mut OcctKernel>, id: u64);
            fn live_shape_count(self: &OcctKernel) -> usize;
            fn take_error(self: &OcctKernel) -> String;
        }
    }
}

/// OCCT backend implementing [`GeometryKernel`].
///
/// Shapes live in a C++-side registry and are referenced by [`ShapeId`];
/// no OCCT type ever crosses into Rust. Every fallible FFI call returns a
/// sentinel (`0` / `false`) and stores a message retrievable via `take_error`.
pub struct OcctBackend {
    inner: UniquePtr<bridge::ffi::OcctKernel>,
}

impl OcctBackend {
    pub fn new() -> Result<Self, KernelError> {
        let inner = bridge::ffi::occt_kernel_new();
        if inner.is_null() {
            return Err(KernelError::new(
                KernelErrorKind::Unsupported,
                "failed to allocate OCCT kernel",
            ));
        }
        Ok(Self { inner })
    }

    pub fn live_shape_count(&self) -> usize {
        self.inner.live_shape_count()
    }

    /// Safety invariant (mission Rule 8): `inner` is non-null for the whole
    /// lifetime of `OctBackend` (enforced in `new`); `as_mut` therefore never
    /// fails and no aliasing occurs — all callers hold `&mut self`.
    fn pin(&mut self) -> std::pin::Pin<&mut bridge::ffi::OcctKernel> {
        self.inner
            .as_mut()
            .expect("OCCT kernel pointer must stay valid")
    }

    fn error(&self, fallback: &str) -> KernelError {
        let message = self.inner.take_error();
        if message.is_empty() {
            KernelError::geometry(fallback)
        } else {
            KernelError::geometry(message)
        }
    }
}

fn convert_face_ranges(faces: &[bridge::ffi::FaceRangeOut]) -> Vec<FaceRange> {
    faces
        .iter()
        .map(|f| FaceRange {
            face_id: f.face_id,
            index_start: f.index_start,
            index_count: f.index_count,
        })
        .collect()
}

impl GeometryKernel for OcctBackend {
    type Shape = ShapeId;
    type Error = KernelError;

    fn make_box(&mut self, dx: f64, dy: f64, dz: f64) -> Result<Self::Shape, Self::Error> {
        let id = self.pin().make_box(dx, dy, dz);
        if id == 0 {
            return Err(self.error("make_box failed"));
        }
        Ok(ShapeId(id))
    }

    fn make_sphere(&mut self, radius: f64) -> Result<Self::Shape, Self::Error> {
        let id = self.pin().make_sphere(radius);
        if id == 0 {
            return Err(self.error("make_sphere failed"));
        }
        Ok(ShapeId(id))
    }

    fn make_cylinder(&mut self, radius: f64, height: f64) -> Result<Self::Shape, Self::Error> {
        let id = self.pin().make_cylinder(radius, height);
        if id == 0 {
            return Err(self.error("make_cylinder failed"));
        }
        Ok(ShapeId(id))
    }

    fn read_step(&mut self, data: &[u8]) -> Result<Self::Shape, Self::Error> {
        let id = self.pin().read_step(data);
        if id == 0 {
            return Err(self.error("STEP import failed"));
        }
        Ok(ShapeId(id))
    }

    fn read_brep(&mut self, data: &[u8]) -> Result<Self::Shape, Self::Error> {
        let id = self.pin().read_brep(data);
        if id == 0 {
            return Err(self.error("BREP import failed"));
        }
        Ok(ShapeId(id))
    }

    fn write_step(&mut self, shape: &Self::Shape) -> Result<Vec<u8>, Self::Error> {
        let mut out = Vec::new();
        if !self.pin().write_step(shape.0, &mut out) {
            return Err(self.error("STEP export failed"));
        }
        Ok(out)
    }

    fn write_brep(&mut self, shape: &Self::Shape) -> Result<Vec<u8>, Self::Error> {
        let mut out = Vec::new();
        if !self.pin().write_brep(shape.0, &mut out) {
            return Err(self.error("BREP export failed"));
        }
        Ok(out)
    }

    fn fuse(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error> {
        let id = self.pin().fuse(a.0, b.0);
        if id == 0 {
            return Err(self.error("fuse failed"));
        }
        Ok(ShapeId(id))
    }

    fn cut(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error> {
        let id = self.pin().cut(a.0, b.0);
        if id == 0 {
            return Err(self.error("cut failed"));
        }
        Ok(ShapeId(id))
    }

    fn common(&mut self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape, Self::Error> {
        let id = self.pin().common(a.0, b.0);
        if id == 0 {
            return Err(self.error("common failed"));
        }
        Ok(ShapeId(id))
    }

    fn tessellate(
        &mut self,
        shape: &Self::Shape,
        linear_deflection: f64,
        angular_deflection_rad: f64,
    ) -> Result<MeshBuffer, Self::Error> {
        freecad_kernel::validate_deflections(linear_deflection, angular_deflection_rad)
            .map_err(|e| KernelError::new(KernelErrorKind::InvalidInput, e.message))?;

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut ffi_faces = Vec::new();
        let ok = self.pin().tessellate(
            shape.0,
            linear_deflection,
            angular_deflection_rad,
            &mut positions,
            &mut normals,
            &mut indices,
            &mut ffi_faces,
        );
        if !ok {
            return Err(self.error("tessellation failed"));
        }
        let to_vec3 = |flat: Vec<f32>| -> Vec<[f32; 3]> {
            flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()
        };
        let mesh = MeshBuffer {
            positions: to_vec3(positions),
            normals: to_vec3(normals),
            indices,
            faces: convert_face_ranges(&ffi_faces),
        };
        mesh.validate()
            .map_err(|e| KernelError::invalid_input(e.to_string()))?;
        Ok(mesh)
    }

    fn destroy(&mut self, shape: Self::Shape) {
        self.pin().destroy_shape(shape.0);
    }

    fn stats(&self, shape: &Self::Shape) -> Result<ShapeStats, Self::Error> {
        let mut out = bridge::ffi::ShapeStatsOut::default();
        if !self.inner.shape_stats(shape.0, &mut out) {
            return Err(self.error("shape_stats failed"));
        }
        Ok(ShapeStats {
            vertices: out.vertices,
            edges: out.edges,
            faces: out.faces,
            solids: out.solids,
        })
    }

    fn bounds(&self, shape: &Self::Shape) -> Result<Bounds, Self::Error> {
        let mut out = bridge::ffi::BoundsOut::default();
        if !self.inner.bounds(shape.0, &mut out) {
            return Err(self.error("bounds failed"));
        }
        Ok(Bounds {
            min: [out.min_x, out.min_y, out.min_z],
            max: [out.max_x, out.max_y, out.max_z],
        })
    }
}
