//! Face extraction for selection highlighting.

use crate::mesh::{FaceRange, MeshBuffer};

/// Returns a new mesh containing only the triangles of `face_id`.
///
/// The result keeps the original `face_id` in its single face range, so
/// highlight rendering and picking stay consistent. Vertex order and normals
/// are preserved verbatim.
pub fn extract_face(mesh: &MeshBuffer, face_id: u32) -> Option<MeshBuffer> {
    let range = mesh.faces.iter().find(|r| r.face_id == face_id)?;
    let start = range.index_start as usize;
    let end = start + range.index_count as usize;
    let indices = mesh.indices.get(start..end)?;

    let mut out = MeshBuffer {
        positions: Vec::with_capacity(indices.len()),
        normals: Vec::with_capacity(indices.len()),
        indices: (0..indices.len() as u32).collect(),
        faces: vec![FaceRange {
            face_id,
            index_start: 0,
            index_count: indices.len() as u32,
        }],
    };
    for &idx in indices {
        let idx = idx as usize;
        out.positions.push(*mesh.positions.get(idx)?);
        out.normals.push(*mesh.normals.get(idx)?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_face_keeps_identity_and_geometry() {
        let cube = crate::prim::cube(2.0);
        assert_eq!(cube.face_ranges().len(), 6);

        let face = extract_face(&cube, 3).expect("cube has face 3");
        face.validate().expect("extracted face validates");
        assert_eq!(face.triangle_count(), 2);
        assert_eq!(face.face_ranges().len(), 1);
        assert_eq!(face.face_ranges()[0].face_id, 3);

        // geometry preserved: every vertex lies on the -X plane of a 2-unit cube
        for p in &face.positions {
            assert!((p[0] - (-1.0)).abs() < 1e-6, "x={}", p[0]);
        }
    }

    #[test]
    fn missing_face_is_none() {
        let cube = crate::prim::cube(1.0);
        assert!(extract_face(&cube, 99).is_none());
    }
}

/// Packing limits for GPU picking: 12 bits of mesh index and 12 bits of face
/// id travel inside the normal xyz channels.
pub const PICK_MAX_MESH: u32 = (1 << 12) - 1;
pub const PICK_MAX_FACE: u32 = (1 << 12) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickEncodeError {
    TooLarge,
}

pub fn pack24(mesh_index: u32, face_id: u32) -> Result<u32, PickEncodeError> {
    if mesh_index > PICK_MAX_MESH || face_id > PICK_MAX_FACE {
        return Err(PickEncodeError::TooLarge);
    }
    Ok((mesh_index << 12) | face_id)
}

pub fn unpack24(v: u32) -> (u32, u32) {
    (v >> 12, v & 0x0FFF)
}

/// Normal whose RGB round-trip through `n*0.5+0.5` yields the packed value.
pub fn encode_pick_normal(mesh_index: u32, face_id: u32) -> Result<[f32; 3], PickEncodeError> {
    let v = pack24(mesh_index, face_id)? as f32;
    Ok([
        ((v % 256.0) - 127.5) / 127.5,
        ((v / 256.0).floor() % 256.0 - 127.5) / 127.5,
        ((v / 65536.0).floor() % 256.0 - 127.5) / 127.5,
    ])
}

/// Returns `(mesh_index, face_id)` for a hit pixel (alpha distinguishes
/// background: the clear color carries a = 0, geometry writes a = 255).
pub fn decode_pick_bytes(b: [u8; 4]) -> Option<(u32, u32)> {
    if b[3] == 0 {
        return None;
    }
    let v = u32::from(b[0]) | (u32::from(b[1]) << 8) | (u32::from(b[2]) << 16);
    Some(unpack24(v))
}

/// Mesh copy whose normals carry pick encoding (positions/indices preserved).
pub fn build_pick_mesh(mesh: &MeshBuffer, mesh_index: u32) -> Result<MeshBuffer, PickEncodeError> {
    let mut out = mesh.clone();
    for range in &out.faces {
        let start = range.index_start as usize;
        let end = start + range.index_count as usize;
        for &idx in &mesh.indices[start..end] {
            out.normals[idx as usize] = encode_pick_normal(mesh_index, range.face_id)?;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod pick_encoding_tests {
    use super::*;
    use crate::prim::cube;

    #[test]
    fn encode_decode_roundtrip() {
        for (m, f) in [(0u32, 1u32), (3u32, 4095u32), (17u32, 1234u32)] {
            let n = encode_pick_normal(m, f).unwrap();
            let enc = n.map(|c| ((c * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8);
            let px = [enc[0], enc[1], enc[2], 255u8];
            assert_eq!(decode_pick_bytes(px), Some((m, f)), "n={n:?} px={px:?}");
            assert_eq!(decode_pick_bytes([0, 0, 0, 0]), None, "background");
        }
        assert!(encode_pick_normal(PICK_MAX_MESH + 1, 0).is_err());
    }

    #[test]
    fn build_pick_mesh_replaces_normals_only() {
        let cube = cube(2.0);
        let pm = build_pick_mesh(&cube, 4).unwrap();
        assert_eq!(pm.positions.len(), cube.positions.len());
        let expected = encode_pick_normal(4, 0).unwrap();
        let r0 = &pm.face_ranges()[0];
        for k in 0..r0.index_count as usize {
            let i = pm.indices[r0.index_start as usize + k] as usize;
            assert_eq!(pm.normals[i], expected);
        }
    }
}
