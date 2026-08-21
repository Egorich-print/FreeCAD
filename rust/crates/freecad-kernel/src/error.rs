use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelErrorKind {
    InvalidInput,
    Parse,
    Geometry,
    Unsupported,
}

impl fmt::Display for KernelErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            KernelErrorKind::InvalidInput => "invalid input",
            KernelErrorKind::Parse => "parse failure",
            KernelErrorKind::Geometry => "geometry operation failure",
            KernelErrorKind::Unsupported => "operation unsupported by this kernel",
        };
        f.write_str(msg)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelError {
    pub kind: KernelErrorKind,
    pub message: String,
}

impl KernelError {
    pub fn new(kind: KernelErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn geometry(message: impl Into<String>) -> Self {
        Self::new(KernelErrorKind::Geometry, message)
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(KernelErrorKind::Parse, message)
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(KernelErrorKind::InvalidInput, message)
    }
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for KernelError {}
