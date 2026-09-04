//! freecad-ux — Fusion-parity UX logic in Rust 2024.
//!
//! Ponytail principle: smallest code that actually works.
//! C++ shims call into here for chamfer drag math and sketch snapping.

pub mod chamfer;
pub mod joint;
pub mod measure;
pub mod sketch;
pub mod snap;

pub use chamfer::{ChamferParams, ChamferType, drag_to_value, snap_value, validate_chamfer};
pub use joint::{AssemblyJointKind, Joint, JointType};
pub use measure::{Point3, angle_at, bbox_diagonal, distance, point_to_segment_dist};
pub use sketch::{
    ArcData, EdgeProj, FaceProj, HybridPolicy, SnapCandidate, SnapKind, face_edges_to_external,
    ghost_edge_for_snap, hybrid_snap_policy, mid_snap_threshold, nearest_snap, should_auto_import,
};
pub use snap::{snap_to_arc_middle, snap_to_line_middle};
