pub mod camera;
mod gpu;
pub mod pick;
mod renderer;

pub use camera::{Mat4, OrbitCamera};
pub use gpu::GpuMesh;
pub use pick::{PickHit, Picker};
pub use renderer::{
    RenderItem, Renderer, TargetSize, camera_uniform_bytes, camera_uniform_bytes_debug,
    clear_color_texture, create_depth_view,
};

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
    return vec4<f32>(normalize(in.world_normal) * 0.5 + vec3<f32>(0.5), 1.0); // DEBUG normals-as-colors
}
"#;

/// Selection highlight: solid tint over an extracted-face mesh.
pub const HIGHLIGHT_SHADER_WGSL: &str = r#"
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
fn fs_main(_in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.62, 0.10, 1.0);
}
"#;

/// Triangle-id picking: fragment writes global_triangle_id + 1 (0 = miss).
/// Face-id picking pass: normals carry the encoded face id; fragments write
/// it into an ordinary RGBA8 target (bypasses uint-attachment quirks).
/// Face-id picking pass: fragments write `global_triangle_index + 1` into an
/// R32Uint target (0 = background).
pub const PICK_SHADER_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
};
struct Base {
    value: vec4<u32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> base: Base;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) @interpolate(flat) payload: u32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) triangle_id: u32,
) -> VsOut {
    var out: VsOut;
    out.clip_pos = camera.view_proj * vec4<f32>(position, 1.0);
    out.payload = base.value.x + triangle_id;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<u32> {
    return vec4<u32>(in.payload + 1u, 0u, 0u, 0u);
}
"#;
