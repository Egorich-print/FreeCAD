//! Orbit camera and hand-rolled column-major matrix math shared by every
//! freecad-render backend.

use std::f64::consts::FRAC_PI_2;

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
        let radius = 0.5
            * (diag[0] * diag[0] + diag[1] * diag[1] + diag[2] * diag[2])
                .max(1e-9)
                .sqrt();
        self.target = center;
        self.distance = (radius / self.fov_y_rad.sin()) * 1.2;
        self.near = (self.distance * 0.001).max(1e-3);
        self.far = self.distance * 100.0;
    }

    pub fn orbit(&mut self, delta_yaw: f64, delta_pitch: f64) {
        self.yaw_rad += delta_yaw;
        self.pitch_rad = (self.pitch_rad + delta_pitch).clamp(-FRAC_PI_2 + 1e-3, FRAC_PI_2 - 1e-3);
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

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, [0.0, 1.0, 0.0])
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_y_rad as f32,
            aspect,
            self.near as f32,
            self.far as f32,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    pub fn mul(&self, other: &Mat4) -> Mat4 {
        let mut out = [[0.0f32; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                let mut acc = 0.0;
                for k in 0..4 {
                    acc += self.0[k][row] * other.0[col][k];
                }
                out[col][row] = acc;
            }
        }
        Mat4(out)
    }

    pub fn look_at_rh(eye: [f64; 3], target: [f64; 3], up: [f64; 3]) -> Mat4 {
        let f = normalize3([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
        let s = normalize3(cross3(f, up));
        let u = cross3(s, f);

        Mat4([
            [s[0] as f32, u[0] as f32, (-f[0]) as f32, 0.0],
            [s[1] as f32, u[1] as f32, (-f[1]) as f32, 0.0],
            [s[2] as f32, u[2] as f32, (-f[2]) as f32, 0.0],
            [
                -(dot3(s, [eye[0], eye[1], eye[2]])) as f32,
                -(dot3(u, [eye[0], eye[1], eye[2]])) as f32,
                dot3(f, [eye[0], eye[1], eye[2]]) as f32,
                1.0,
            ],
        ])
    }

    pub fn perspective_rh(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let f = 1.0 / (fov_y_rad * 0.5).tan();
        let nf = 1.0 / (near - far);
        Mat4([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (far + near) * nf, -1.0],
            [0.0, 0.0, 2.0 * far * near * nf, 0.0],
        ])
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
        let m = Mat4::IDENTITY.mul(&Mat4::IDENTITY);
        assert_eq!(m, Mat4::IDENTITY);
    }

    #[test]
    fn perspective_maps_near_and_far_planes_correctly() {
        let (near, far) = (0.1f32, 100.0f32);
        let p = Mat4::perspective_rh(50f32.to_radians(), 1.0, near, far);
        let ndc_at = |z: f32| {
            let point = Mat4([[0.0; 4], [0.0; 4], [0.0; 4], [0.0, 0.0, z, 1.0]]);
            let out = p.mul(&point);
            let w = out.0[3][3];
            out.0[3][2] / w
        };
        assert!(
            (ndc_at(-near) - (-1.0)).abs() < 1e-4,
            "near plane maps to -1"
        );
        assert!((ndc_at(-far) - 1.0).abs() < 1e-3, "far plane maps to +1");
    }

    #[test]
    fn look_at_places_eye_distance_on_diagonal() {
        let cam = OrbitCamera::default();
        let eye = cam.eye();
        let d = ((eye[0] - cam.target[0]).powi(2)
            + (eye[1] - cam.target[1]).powi(2)
            + (eye[2] - cam.target[2]).powi(2))
        .sqrt();
        assert!((d - cam.distance).abs() < 1e-9);
    }
}
