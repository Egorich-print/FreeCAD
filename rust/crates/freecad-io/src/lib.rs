pub mod fcstd;

use freecad_kernel::GeometryKernel;
use freecad_kernel::error::KernelError;

use crate::fcstd::FcStdError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Step,
    Brep,
    /// FreeCAD document (S0: shapes only).
    FcStd,
}

impl Format {
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.to_ascii_lowercase().as_str() {
            "step" | "stp" => Some(Format::Step),
            "brep" | "brp" | "brep.gz" => Some(Format::Brep),
            "fcstd" => Some(Format::FcStd),
            _ => None,
        }
    }
}

pub fn load_bytes<K>(kernel: &mut K, data: &[u8], format: Format) -> Result<K::Shape, K::Error>
where
    K: GeometryKernel,
    K::Error: From<KernelError>,
{
    match format {
        Format::Step => kernel.read_step(data),
        Format::Brep => kernel.read_brep(data),
        Format::FcStd => {
            let archive = fcstd::open_archive(data).map_err(KernelError::from)?;
            for obj in archive.document.shape_objects() {
                if let Some(bytes) = archive.shape_of(obj) {
                    return kernel.read_brep(bytes);
                }
            }
            Err(KernelError::from(FcStdError::NoShapePayload).into())
        }
    }
}

pub fn load_path<K>(
    kernel: &mut K,
    path: impl AsRef<std::path::Path>,
) -> Result<K::Shape, LoadError<K::Error>>
where
    K: GeometryKernel,
    K::Error: From<KernelError>,
{
    let path = path.as_ref();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let format = Format::from_extension(&ext).ok_or_else(|| LoadError::UnknownExtension(ext))?;
    let data = std::fs::read(path).map_err(LoadError::Io)?;
    load_bytes(kernel, &data, format).map_err(LoadError::Kernel)
}

pub fn store_bytes<K: GeometryKernel>(
    kernel: &mut K,
    shape: &K::Shape,
    format: Format,
) -> Result<Vec<u8>, K::Error> {
    match format {
        Format::Step => kernel.write_step(shape),
        Format::Brep => kernel.write_brep(shape),
        _ => unreachable!("FCStd store rejected by caller"),
    }
}

#[derive(Debug)]
pub enum LoadError<E> {
    Io(std::io::Error),
    UnknownExtension(String),
    Kernel(E),
}

impl From<crate::fcstd::FcStdError> for KernelError {
    fn from(e: crate::fcstd::FcStdError) -> Self {
        use freecad_kernel::error::KernelErrorKind;
        let kind = KernelErrorKind::Parse;
        let _ = kind;
        // All FCStd S0 failures are parse-class problems at this layer.
        Self::new(KernelErrorKind::Parse, e.to_string())
    }
}

/// Binary STL export from a tessellated mesh buffer.
pub fn export_stl(mesh: &freecad_core::MeshBuffer) -> Vec<u8> {
    let tris = mesh.triangle_count() as u32;
    let mut out = Vec::with_capacity(84 + tris as usize * 50);
    out.extend_from_slice(b"FreeCAD Rust STL export"); // 80-byte header (padded)
    out.resize(80, b' ');
    out.extend_from_slice(&tris.to_ne_bytes());
    for t in 0..mesh.triangle_count() {
        if let Some([a, b, c]) = mesh.triangle_at(t) {
            // normal = cross(b-a, c-a) normalized
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let nx = uy * vz - uz * vy;
            let ny = uz * vx - ux * vz;
            let nz = ux * vy - uy * vx;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            let (nx, ny, nz) = if len > 1e-12 {
                (nx / len, ny / len, nz / len)
            } else {
                (0.0, 0.0, 1.0)
            };
            for f in [nx, ny, nz] {
                out.extend_from_slice(&f.to_le_bytes());
            }
            for p in [a, b, c] {
                for v in p {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
            out.extend_from_slice(&0u16.to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
mod stl_tests {
    use super::*;

    #[test]
    fn stl_export_produces_valid_binary() {
        let cube = freecad_core::prim::cube(2.0);
        let stl = export_stl(&cube);
        assert_eq!(stl.len(), 84 + 50 * 12);
        let tri_count = u32::from_le_bytes(stl[80..84].try_into().unwrap());
        assert_eq!(tri_count, 12);
        // first triangle: normal should be unit-ish
        let nx = f32::from_le_bytes(stl[84..88].try_into().unwrap());
        assert!(nx.abs() <= 1.01);
    }
}
