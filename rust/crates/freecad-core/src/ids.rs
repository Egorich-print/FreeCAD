use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ShapeId(pub u64);

impl fmt::Display for ShapeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shape#{}", self.0)
    }
}

pub const INVALID_SHAPE_ID: ShapeId = ShapeId(u64::MAX);
