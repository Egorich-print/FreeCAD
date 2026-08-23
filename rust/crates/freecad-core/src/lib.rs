pub mod document;
pub mod ids;
pub mod mesh;
pub mod prim;
pub mod selection;

pub use document::{Document, ObjectId, Placement, SceneObject};
pub use ids::ShapeId;
pub use mesh::{BoundingBox, FaceRange, MeshBuffer, MeshError};
pub use selection::{
    PickEncodeError, build_pick_mesh, decode_pick_bytes, encode_pick_normal, extract_face, pack24,
    unpack24,
};
