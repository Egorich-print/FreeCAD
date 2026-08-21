use std::ffi::c_char;

pub const VERSION_MAJOR: u32 = 0;
pub const VERSION_MINOR: u32 = 1;
pub const VERSION_PATCH: u32 = 0;

/// Packed semver for cheap JNI probing.
#[unsafe(no_mangle)]
pub extern "C" fn freecad_android_version() -> u32 {
    (VERSION_MAJOR << 16) | (VERSION_MINOR << 8) | VERSION_PATCH
}

/// ABI sanity probe: reference cube tessellated by the pure-Rust core.
#[unsafe(no_mangle)]
pub extern "C" fn freecad_android_cube_triangles(size: f32) -> u32 {
    freecad_core::prim::cube(size).triangle_count() as u32
}

/// Static NUL-terminated string; C callers never free it.
#[unsafe(no_mangle)]
pub extern "C" fn freecad_android_kernel_status() -> *const c_char {
    static STATUS: &[u8] = b"occt backend not linked into this android build yet\0";
    STATUS.as_ptr().cast()
}
