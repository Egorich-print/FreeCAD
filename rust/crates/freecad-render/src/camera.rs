//! Orbit camera and view/projection matrices.
//!
//! Matrix math is delegated to glam (column-major, RH conventions) and stored
//! in the WGSL-compatible `[[f32; 4]; 4]` column-of-vectors layout.

use glam::{Mat4 as GMat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    pub target: [f64; 3],
    pub distance: f64,
    pub yaw_rad: f64,
    pub pitch_rad: f64,
    pub fov_y_rad: f64,
    pub near: f64,
    pub far: f64,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: [0.0; 3],
            distance: 5.0,
            yaw_rad: std::f64::consts::FRAC_PI_4,
            pitch_rad: 0.6,
            fov_y_rad: 50.0f64.to_radians(),
            near: 0.01,
            far: 10_000.0,
        }
    }
}

impl OrbitCamera {
    pub fn frame(&mut self, bounds_min: [f64; 3], bounds_max: [f64; 3]) {
        let center = [
            (bounds_min[0] + bounds_max[0]) * 0.5,
            (bounds_min[1] + bounds_max[1]) * 0.5,
            (bounds_min[2] + bounds_max[2]) * 0.5,
        ];
        let diag = [
            bounds_max[0] - bounds_min[0],
            bounds_max[1] - bounds_min[1],
            bounds_max[2] - bounds_min[2],
        ];
        let radius =
            0.5 * ((diag[0] * diag[0] + diag[1] * diag[1] + diag[2] * diag[2]).max(1e-9)).sqrt();
        self.target = center;
        self.distance = (radius / self.fov_y_rad.sin()) * 1.2;
        self.near = (self.distance * 0.001).max(1e-3);
        self.far = self.distance * 100.0;
    }

    pub fn orbit(&mut self, delta_yaw: f64, delta_pitch: f64) {
        self.yaw_rad += delta_yaw;
        self.pitch_rad = (self.pitch_rad + delta_pitch).clamp(
            -std::f64::consts::FRAC_PI_2 + 1e-3,
            std::f64::consts::FRAC_PI_2 - 1e-3,
        );
    }

    pub fn pan_screen(&mut self, delta_x: f64, delta_y: f64) {
        let scale = self.distance * 0.0016;
        let eye = self.eye();
        let forward = normalize3([
            self.target[0] - eye[0],
            self.target[1] - eye[1],
            self.target[2] - eye[2],
        ]);
        let right = normalize3(cross3(forward, [0.0, 1.0, 0.0]));
        let up = cross3(right, forward);
        for i in 0..3 {
            self.target[i] += (-right[i] * delta_x + up[i] * delta_y) * scale;
        }
    }

    pub fn zoom(&mut self, factor: f64) {
        self.distance = (self.distance * factor).clamp(self.near * 2.0, self.far * 0.5);
    }

    pub fn eye(&self) -> [f64; 3] {
        let cp = self.pitch_rad.cos();
        [
            self.target[0] + self.distance * cp * self.yaw_rad.sin(),
            self.target[1] + self.distance * self.pitch_rad.sin(),
            self.target[2] + self.distance * cp * self.yaw_rad.cos(),
        ]
    }

    /// World-space pick ray through the given pixel (origin = eye).
    pub fn ray_through_pixel(
        &self,
        px: f32,
        py: f32,
        width: f32,
        height: f32,
    ) -> ([f64; 3], [f64; 3]) {
        let ndc_x = px / width * 2.0 - 1.0;
        let ndc_y = 1.0 - py / height * 2.0;
        let eye = self.eye();
        let forward = normalize3([
            self.target[0] - eye[0],
            self.target[1] - eye[1],
            self.target[2] - eye[2],
        ]);
        let right = normalize3(cross3(forward, [0.0, 1.0, 0.0]));
        let up = cross3(right, forward);
        let tan_half = (self.fov_y_rad as f32 * 0.5).tan() as f64;
        let aspect = (width / height.max(1.0)) as f64;
        let dir = normalize3([
            forward[0]
                + right[0] * ndc_x as f64 * tan_half * aspect
                + up[0] * ndc_y as f64 * tan_half,
            forward[1]
                + right[1] * ndc_x as f64 * tan_half * aspect
                + up[1] * ndc_y as f64 * tan_half,
            forward[2]
                + right[2] * ndc_x as f64 * tan_half * aspect
                + up[2] * ndc_y as f64 * tan_half,
        ]);
        (eye, dir)
    }

    pub fn view_matrix(&self) -> Mat4 {
        let e = self.eye();
        let t = self.target;
        Mat4(
            GMat4::look_at_rh(
                Vec3::new(e[0] as f32, e[1] as f32, e[2] as f32),
                Vec3::new(t[0] as f32, t[1] as f32, t[2] as f32),
                Vec3::Y,
            )
            .to_cols_array_2d(),
        )
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4(
            GMat4::perspective_rh(
                self.fov_y_rad as f32,
                aspect,
                self.near as f32,
                self.far as f32,
            )
            .to_cols_array_2d(),
        )
    }
}

/// Column-major matrix (`m[col][row]`), layout-compatible with the WGSL
/// `mat4x4<f32>` uniform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// Mathematical product self * rhs.
    pub fn mul(&self, rhs: &Mat4) -> Mat4 {
        let a = GMat4::from_cols_array_2d(&self.0);
        let b = GMat4::from_cols_array_2d(&rhs.0);
        Mat4(a.mul_mat4(&b).to_cols_array_2d())
    }
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize3(v: [f64; 3]) -> [f64; 3] {
    let len = dot3(v, v).sqrt();
    if len < 1e-12 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_mul_is_identity() {
        assert_eq!(Mat4::IDENTITY.mul(&Mat4::IDENTITY), Mat4::IDENTITY);
    }

    #[test]
    fn perspective_maps_near_and_far_planes_correctly() {
        let (near, far) = (0.1f32, 100.0f32);
        let p = Mat4(GMat4::perspective_rh(50f32.to_radians(), 1.0, near, far).to_cols_array_2d());
        let ndc_at = |z: f32| {
            let point = Mat4([[0.0; 4], [0.0; 4], [0.0; 4], [0.0, 0.0, z, 1.0]]);
            let out = p.mul(&point);
            out.0[3][2] / out.0[3][3]
        };
        assert!(
            (ndc_at(-near) - 0.0).abs() < 1e-4,
            "near maps to 0 (wgpu depth range)"
        );
        assert!((ndc_at(-far) - 1.0).abs() < 1e-3, "far maps to 1");
    }

    #[test]
    fn look_at_maps_eye_to_origin_and_target_in_front() {
        let cam = OrbitCamera::default();
        let v = cam.view_matrix();
        let eye_point = Mat4([
            [0.0; 4],
            [0.0; 4],
            [0.0; 4],
            [
                cam.eye()[0] as f32,
                cam.eye()[1] as f32,
                cam.eye()[2] as f32,
                1.0,
            ],
        ]);
        let out = v.mul(&eye_point);
        let xyz = [out.0[3][0], out.0[3][1], out.0[3][2]];
        assert!(
            xyz.iter().all(|c| c.abs() < 1e-4),
            "eye → origin, got {xyz:?}"
        );

        let target_point = Mat4([
            [0.0; 4],
            [0.0; 4],
            [0.0; 4],
            [
                cam.target[0] as f32,
                cam.target[1] as f32,
                cam.target[2] as f32,
                1.0,
            ],
        ]);
        let out_t = v.mul(&target_point);
        assert!(out_t.0[3][2] < 0.0, "target must sit at negative z");
    }

    #[test]
    fn vp_projects_front_face_center_to_screen_center() {
        let mut cam = OrbitCamera {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            ..Default::default()
        };
        cam.frame([-1.0; 3], [1.0; 3]);
        let vp = cam.projection_matrix(1.0).mul(&cam.view_matrix());
        // p = (0,0,1,1): w must be d − 1 > 0 and ndc center on x/y.
        let w = vp.0[3][3] + vp.0[3][2];
        assert!(w > 0.5, "w={w}");
        let ndc_x = vp.0[0][2] / w;
        let ndc_y = vp.0[1][2] / w;
        assert!(
            ndc_x.abs() < 1e-4 && ndc_y.abs() < 1e-4,
            "ndc=({ndc_x},{ndc_y})"
        );
    }
}
