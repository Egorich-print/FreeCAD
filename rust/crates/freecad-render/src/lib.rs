pub mod camera;
mod gpu;
mod renderer;

pub use camera::{Mat4, OrbitCamera};
pub use gpu::GpuMesh;
pub use renderer::{RenderItem, Renderer, TargetSize, clear_color_texture, create_depth_view};

pub const SHADER_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VsOut {
    var out: VsOut;
    out.clip_pos = camera.view_proj * vec4<f32>(position, 1.0);
    out.world_normal = normal;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let l = normalize(camera.light_dir.xyz);
    let diffuse = max(dot(n, l), 0.0);
    let base = vec3<f32>(0.62, 0.66, 0.71);
    let lit = base * (0.28 + 0.82 * diffuse);
    return vec4<f32>(lit, 1.0);
}
"#;
