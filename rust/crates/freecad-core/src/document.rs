//! Minimal document model: objects, placements, visibility.
//!
//! Sized for the viewer and future parametric expansion — not a port of
//! FreeCAD's App::Document.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ObjectId(pub u32);

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "obj#{}", self.0)
    }
}

/// Rigid transform: translation + unit quaternion rotation (x, y, z, w).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub pos: [f64; 3],
    /// Quaternion as (qx, qy, qz, qw).
    pub quat: [f64; 4],
}

impl Default for Placement {
    fn default() -> Self {
        Self::identity()
    }
}

impl Placement {
    pub const fn identity() -> Self {
        Self {
            pos: [0.0; 3],
            quat: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn from_translation(pos: [f64; 3]) -> Self {
        Self {
            pos,
            quat: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn is_identity(&self) -> bool {
        self.pos == [0.0; 3] && self.quat == [0.0, 0.0, 0.0, 1.0]
    }

    /// Returns a column-major 4×4 f32 matrix compatible with WGSL mat4x4.
    pub fn to_mat4(&self) -> [[f32; 4]; 4] {
        let [qx, qy, qz, qw] = self.quat;
        let xx = qx * qx;
        let xy = qx * qy;
        let xz = qx * qz;
        let xw = qx * qw;
        let yy = qy * qy;
        let yz = qy * qz;
        let yw = qy * qw;
        let zz = qz * qz;
        let zw = qz * qw;

        // Column-major rotation matrix
        let m00 = 1.0f32 - 2.0f32 * ((yy + zz) as f32);
        let m10 = 2.0f32 * ((xy + zw) as f32);
        let m20 = 2.0f32 * ((xz - yw) as f32);
        let m01 = 2.0f32 * ((xy - zw) as f32);
        let m11 = 1.0f32 - 2.0f32 * ((xx + zz) as f32);
        let m21 = 2.0f32 * ((yz + xw) as f32);
        let m02 = 2.0f32 * ((xz + yw) as f32);
        let m12 = 2.0f32 * ((yz - xw) as f32);
        let m22 = 1.0f32 - 2.0f32 * ((xx + yy) as f32);

        // Translation in column 3.
        [
            [m00, m10, m20, 0.0],
            [m01, m11, m21, 0.0],
            [m02, m12, m22, 0.0],
            [
                self.pos[0] as f32,
                self.pos[1] as f32,
                self.pos[2] as f32,
                1.0,
            ],
        ]
    }
}

/// A document-level scene object referencing a kernel shape by index.
#[derive(Debug, Clone)]
pub struct SceneObject {
    pub id: ObjectId,
    pub label: String,
    pub type_name: String,
    /// Index into the scene mesh list (`None` for non-geometric objects).
    pub shape_index: Option<usize>,
    pub placement: Placement,
    pub visible: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub objects: Vec<SceneObject>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, obj: SceneObject) -> ObjectId {
        let id = obj.id;
        self.objects.push(obj);
        id
    }

    pub fn find(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn visible_objects(&self) -> impl Iterator<Item = &SceneObject> {
        self.objects.iter().filter(|o| o.visible)
    }

    pub fn shape_objects(&self) -> impl Iterator<Item = &SceneObject> {
        self.objects
            .iter()
            .filter(|o| o.visible && o.shape_index.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_placement_is_noop() {
        let p = Placement::identity();
        let m = p.to_mat4();
        assert_eq!(m[0][0], 1.0);
        assert_eq!(m[3][0], 0.0);
    }

    #[test]
    fn translation_appears_in_column_3() {
        let p = Placement::from_translation([10.0, 20.0, 30.0]);
        let m = p.to_mat4();
        assert_eq!(m[3], [10.0, 20.0, 30.0, 1.0]);
    }

    #[test]
    fn document_add_and_visible() {
        let mut doc = Document::new();
        doc.add(SceneObject {
            id: ObjectId(1),
            label: "Plate".into(),
            type_name: "Part::Feature".into(),
            shape_index: Some(0),
            placement: Placement::identity(),
            visible: true,
        });
        doc.add(SceneObject {
            id: ObjectId(2),
            label: "Hidden".into(),
            type_name: "Part::Feature".into(),
            shape_index: None,
            placement: Placement::identity(),
            visible: false,
        });
        assert_eq!(doc.visible_objects().count(), 1);
        assert_eq!(doc.shape_objects().count(), 1);
    }
}
