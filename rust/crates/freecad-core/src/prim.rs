use crate::mesh::{FaceRange, MeshBuffer};

pub fn cube(size: f32) -> MeshBuffer {
    let h = size * 0.5;
    let mut mesh = MeshBuffer::default();

    let push_face = |mesh: &mut MeshBuffer, normal: [f32; 3], corners: [[f32; 3]; 4]| {
        let base = mesh.positions.len() as u32;
        for c in corners {
            mesh.positions.push(c);
            mesh.normals.push(normal);
        }
        let start = mesh.indices.len() as u32;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        mesh.faces.push(FaceRange {
            face_id: mesh.faces.len() as u32,
            index_start: start,
            index_count: 6,
        });
    };

    push_face(
        &mut mesh,
        [0.0, 0.0, 1.0],
        [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]],
    );
    push_face(
        &mut mesh,
        [0.0, 0.0, -1.0],
        [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]],
    );
    push_face(
        &mut mesh,
        [1.0, 0.0, 0.0],
        [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]],
    );
    push_face(
        &mut mesh,
        [-1.0, 0.0, 0.0],
        [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]],
    );
    push_face(
        &mut mesh,
        [0.0, 1.0, 0.0],
        [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]],
    );
    push_face(
        &mut mesh,
        [0.0, -1.0, 0.0],
        [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]],
    );

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_is_valid_and_bounded() {
        let mesh = cube(2.0);
        assert_eq!(mesh.triangle_count(), 12);
        mesh.validate().expect("cube mesh must validate");
        let bbox = mesh.bounds().expect("cube must have bounds");
        assert_eq!(bbox.min, [-1.0, -1.0, -1.0]);
        assert_eq!(bbox.max, [1.0, 1.0, 1.0]);
        for face in 0..6u32 {
            assert!(mesh.face_id_for_triangle((face * 2) as usize) == Some(face));
        }
    }
}
