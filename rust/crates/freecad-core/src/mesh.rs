use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshError {
    IndexCountNotMultipleOfThree,
    IndexOutOfBounds,
    NormalVertexCountMismatch,
    FaceRangeOutOfBounds,
    Empty,
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            MeshError::IndexCountNotMultipleOfThree => "index count is not a multiple of 3",
            MeshError::IndexOutOfBounds => "index refers to a vertex outside the position buffer",
            MeshError::NormalVertexCountMismatch => "normal count differs from vertex count",
            MeshError::FaceRangeOutOfBounds => "face range extends past the index buffer",
            MeshError::Empty => "mesh has no triangles",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for MeshError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FaceRange {
    pub face_id: u32,
    pub index_start: u32,
    pub index_count: u32,
}

impl FaceRange {
    pub fn triangle_count(&self) -> u32 {
        self.index_count / 3
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshBuffer {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub faces: Vec<FaceRange>,
}

impl MeshBuffer {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn face_ranges(&self) -> &[FaceRange] {
        &self.faces
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn validate(&self) -> Result<(), MeshError> {
        if !self.indices.len().is_multiple_of(3) {
            return Err(MeshError::IndexCountNotMultipleOfThree);
        }
        if self.normals.len() != self.positions.len() {
            return Err(MeshError::NormalVertexCountMismatch);
        }
        for &idx in &self.indices {
            if idx as usize >= self.positions.len() {
                return Err(MeshError::IndexOutOfBounds);
            }
        }
        let total = self.indices.len() as u32;
        for range in &self.faces {
            let end = range
                .index_start
                .checked_add(range.index_count)
                .ok_or(MeshError::FaceRangeOutOfBounds)?;
            if end > total {
                return Err(MeshError::FaceRangeOutOfBounds);
            }
        }
        if self.triangle_count() == 0 {
            return Err(MeshError::Empty);
        }
        Ok(())
    }

    pub fn bounds(&self) -> Option<BoundingBox> {
        let first = *self.positions.first()?;
        let mut bbox = BoundingBox {
            min: first,
            max: first,
        };
        for p in &self.positions {
            bbox.min = [
                bbox.min[0].min(p[0]),
                bbox.min[1].min(p[1]),
                bbox.min[2].min(p[2]),
            ];
            bbox.max = [
                bbox.max[0].max(p[0]),
                bbox.max[1].max(p[1]),
                bbox.max[2].max(p[2]),
            ];
        }
        Some(bbox)
    }

    pub fn triangle_at(&self, triangle: usize) -> Option<[[f32; 3]; 3]> {
        let base = triangle.checked_mul(3)?;
        let a = *self.positions.get(*self.indices.get(base)? as usize)?;
        let b = *self.positions.get(*self.indices.get(base + 1)? as usize)?;
        let c = *self.positions.get(*self.indices.get(base + 2)? as usize)?;
        Some([a, b, c])
    }

    pub fn face_id_for_triangle(&self, triangle: usize) -> Option<u32> {
        let index = (triangle * 3) as u32;
        self.faces
            .iter()
            .find(|r| index >= r.index_start && index < r.index_start + r.index_count)
            .map(|r| r.face_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl BoundingBox {
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    pub fn extents(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    pub fn diagonal(&self) -> f32 {
        let e = self.extents();
        (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt()
    }

    pub fn union(self, other: BoundingBox) -> BoundingBox {
        BoundingBox {
            min: [
                self.min[0].min(other.min[0]),
                self.min[1].min(other.min[1]),
                self.min[2].min(other.min[2]),
            ],
            max: [
                self.max[0].max(other.max[0]),
                self.max[1].max(other.max[1]),
                self.max[2].max(other.max[2]),
            ],
        }
    }
}
