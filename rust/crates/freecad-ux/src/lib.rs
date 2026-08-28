//! freecad-ux — Fusion-parity UX logic in Rust 2024.
//!
//! Ponytail principle: smallest code that actually works.
//! C++ shims call into here for chamfer drag math and sketch snapping.

pub mod chamfer;
pub mod sketch;
pub mod snap;

pub use chamfer::{ChamferParams, ChamferType, drag_to_value, validate_chamfer};
pub use sketch::{EdgeProj, FaceProj, SnapCandidate, face_edges_to_external, mid_snap_threshold};
pub use snap::{snap_to_arc_middle, snap_to_line_middle};
