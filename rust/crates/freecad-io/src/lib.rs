use freecad_kernel::GeometryKernel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Step,
    Brep,
}

impl Format {
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.to_ascii_lowercase().as_str() {
            "step" | "stp" => Some(Format::Step),
            "brep" | "brp" | "brep.gz" => Some(Format::Brep),
            _ => None,
        }
    }
}

pub fn load_bytes<K: GeometryKernel>(
    kernel: &mut K,
    data: &[u8],
    format: Format,
) -> Result<K::Shape, K::Error> {
    match format {
        Format::Step => kernel.read_step(data),
        Format::Brep => kernel.read_brep(data),
    }
}

pub fn load_path<K: GeometryKernel>(
    kernel: &mut K,
    path: impl AsRef<std::path::Path>,
) -> Result<K::Shape, LoadError<K::Error>> {
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
    }
}

#[derive(Debug)]
pub enum LoadError<E> {
    Io(std::io::Error),
    UnknownExtension(String),
    Kernel(E),
}
