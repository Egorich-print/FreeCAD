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
