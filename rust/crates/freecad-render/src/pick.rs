//! Face picking.
//!
//! v1 implementation: deterministic CPU Moeller-Trumbore ray cast against the
//! submitted [`MeshBuffer`]s. The same neutral [`PickHit`] contract a future
//! GPU id-buffer would fulfil; see docs/architecture/RUST_ANDROID_MIGRATION.md
//! for the documented Metal R32Uint rasterisation quirk that motivated the
//! CPU path for now.

use freecad_core::mesh::MeshBuffer;

use crate::camera::OrbitCamera;
use crate::renderer::TargetSize;

#[derive(Debug, Clone, Copy)]
pub struct PickInput<'a> {
    pub mesh_index: usize,
    pub mesh: &'a MeshBuffer,
}

#[derive(Debug, Clone, Copy)]
pub struct PickHit {
    pub mesh_index: usize,
    pub face_id: u32,
    /// Triangle local to the hit mesh.
    pub triangle_id: u32,
    /// Distance from the camera eye to the hit point.
    pub distance: f32,
}

/// Stateless marker type keeping the picker API shape.
#[derive(Default)]
pub struct Picker;

impl Picker {
    pub fn new(_device: &wgpu::Device) -> Self {
        Self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn pick(
        &self,
        camera: &OrbitCamera,
        size: TargetSize,
        inputs: &[PickInput<'_>],
        x: u32,
        y: u32,
    ) -> Option<PickHit> {
        if inputs.is_empty() || x >= size.width || y >= size.height {
            return None;
        }
        let (origin, dir) =
            camera.ray_through_pixel(x as f32, y as f32, size.width as f32, size.height as f32);

        let mut best: Option<(usize, u32, f64)> = None;
        let mut best_tri: Option<u32> = None;
        for input in inputs {
            for range in input.mesh.face_ranges() {
                for t in 0..range.triangle_count() {
                    let abs_tri = range.index_start as usize / 3 + t as usize;
                    if let Some(dist) = ray_cast_triangle(input.mesh, abs_tri, origin, dir) {
                        let better = best.as_ref().is_none_or(|(_, _, d)| dist < *d);
                        if better {
                            best = Some((input.mesh_index, range.face_id, dist));
                            best_tri = Some(abs_tri as u32);
                        }
                    }
                }
            }
        }
        best.map(|(mesh_index, face_id, distance)| PickHit {
            mesh_index,
            face_id,
            triangle_id: best_tri.unwrap_or_default(),
            distance: distance as f32,
        })
    }
}

fn ray_cast_triangle(
    mesh: &MeshBuffer,
    triangle: usize,
    origin: [f64; 3],
    dir: [f64; 3],
) -> Option<f64> {
    let [a, b, c] = mesh.triangle_at(triangle)?;
    let ax = f64::from(a[0]);
    let ay = f64::from(a[1]);
    let az = f64::from(a[2]);
    let e1 = [
        f64::from(b[0]) - ax,
        f64::from(b[1]) - ay,
        f64::from(b[2]) - az,
    ];
    let e2 = [
        f64::from(c[0]) - ax,
        f64::from(c[1]) - ay,
        f64::from(c[2]) - az,
    ];
    let p = cross(dir, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let t_vec = [origin[0] - ax, origin[1] - ay, origin[2] - az];
    let u = dot(t_vec, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(t_vec, e1);
    let v = dot(dir, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, q) * inv;
    (t > 1e-9).then_some(t)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
