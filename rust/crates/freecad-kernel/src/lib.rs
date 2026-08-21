pub mod error;
pub mod kernel;
pub mod mock;

pub use error::{KernelError, KernelErrorKind};
pub use kernel::{Bounds, GeometryKernel, ShapeStats, validate_deflections};
pub use mock::MockKernel;
