use std::sync::Arc;

use freecad_core::mesh::{MeshBuffer, MeshError};
use wgpu::util::DeviceExt;

/// GPU-resident mesh uploaded from [`MeshBuffer`].
///
/// Keeps CPU-side face ranges so selection/picking can map a triangle back to
/// the OCCT face it came from without touching any OCCT type.
#[derive(Debug)]
pub struct GpuMesh {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    pub index_count: u32,
    face_ranges: Vec<(u32, u32, u32)>,
    /// Expanded non-indexed copy used by the pick pipeline (pos+normal+tid).
    pick_buf: wgpu::Buffer,
    /// Retained CPU-side source so applications can extract highlighted
    /// faces without keeping their own copy.
    pub source: Arc<MeshBuffer>,
}

pub const VERTEX_STRIDE_BYTES: u64 = 24;
pub const PICK_STRIDE_BYTES: u64 = 28;

impl GpuMesh {
    pub fn from_mesh_buffer(device: &wgpu::Device, mesh: &MeshBuffer) -> Result<Self, MeshError> {
        mesh.validate()?;
        assert_eq!(mesh.positions.len(), mesh.normals.len(), "validated above");

        let mut packed = Vec::with_capacity(mesh.positions.len() * 6);
        for (p, n) in mesh.positions.iter().zip(mesh.normals.iter()) {
            packed.extend_from_slice(&p[..]);
            packed.extend_from_slice(&n[..]);
        }

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fc-mesh-vertices"),
            contents: bytemuck_bytes(&packed),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fc-mesh-indices"),
            contents: u32_bytes(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let pick_data = build_pick_data(mesh);
        let pick_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fc-mesh-pick"),
            contents: &pick_data,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
        });

        let face_ranges = mesh
            .faces
            .iter()
            .map(|r| (r.face_id, r.index_start, r.index_count))
            .collect();

        Ok(Self {
            vertex_buf,
            index_buf,
            index_count: mesh.indices.len() as u32,
            face_ranges,
            pick_buf,
            source: Arc::new(mesh.clone()),
        })
    }

    pub fn face_id_for_triangle(&self, triangle: usize) -> Option<u32> {
        let index = (triangle * 3) as u32;
        self.face_ranges
            .iter()
            .find(|(_, start, count)| index >= *start && index < *start + *count)
            .map(|(face, _, _)| *face)
    }

    pub(crate) fn attach<'r>(&'r self, render_pass: &mut wgpu::RenderPass<'r>) {
        render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        render_pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    /// Debug/test access to the raw uploaded vertex bytes.
    pub fn debug_vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buf
    }

    #[allow(dead_code)]
    pub fn debug_pick_buffer(&self) -> &wgpu::Buffer {
        &self.pick_buf
    }

    #[allow(dead_code)] // reserved for the GPU id-buffer picking path
    pub(crate) fn attach_pick(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        verts: std::ops::Range<u32>,
    ) {
        render_pass.set_vertex_buffer(0, self.pick_buf.slice(..));
        render_pass.draw(verts, 0..1);
    }

    #[allow(dead_code)] // reserved for GPU id-buffer refinement
    pub(crate) fn triangle_vertex_positions(&self, triangle: usize) -> Option<[[f32; 3]; 3]> {
        self.source.triangle_at(triangle)
    }
}

/// `packed` is `Vec<f32>`; transmute-free byte view without adding bytemuck.
fn bytemuck_bytes(data: &[f32]) -> &[u8] {
    let len = std::mem::size_of_val(data);
    let ptr = data.as_ptr().cast::<u8>();
    // Safety invariant (mission Rule 8): &[f32] is fully initialised, aligned
    // (align(f32)==align(u8)), and u8 has no validity constraints; the slice
    // is borrowed for the duration of this call only.
    unsafe { core::slice::from_raw_parts(ptr, len) }
}

/// Expands an indexed mesh into per-corner pick vertices:
/// `[position: f32x3][normal: f32x3][triangle_id: u32]` per corner.
pub(crate) fn build_pick_data(mesh: &MeshBuffer) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::with_capacity(mesh.indices.len() * PICK_STRIDE_BYTES as usize);
    for (tri, chunk) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        let tid = (tri as u32).to_ne_bytes();
        for &idx in chunk {
            let idx = idx as usize;
            for v in mesh.positions[idx] {
                data.extend_from_slice(&v.to_ne_bytes());
            }
            for v in mesh.normals[idx] {
                data.extend_from_slice(&v.to_ne_bytes());
            }
            data.extend_from_slice(&tid);
        }
    }
    data
}

/// Indices variant: u32 buffer contents.
pub(crate) fn u32_bytes(data: &[u32]) -> &[u8] {
    let len = std::mem::size_of_val(data);
    // Safety invariant: same reasoning as `bytemuck_bytes`.
    unsafe { core::slice::from_raw_parts(data.as_ptr().cast::<u8>(), len) }
}

/// Vertex layout shared by the lit and picking pipelines.
#[allow(dead_code)] // reserved for the GPU id-buffer picking path
pub fn main_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: VERTEX_STRIDE_BYTES,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
        ],
    }
}

/// Vertex layout of the picking pipeline: position + encoded-id normal.
#[allow(dead_code)] // reserved for the GPU id-buffer picking path
pub fn pick_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: PICK_STRIDE_BYTES,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 24,
                shader_location: 2,
            },
        ],
    }
}

#[cfg(test)]
mod pick_data_tests {
    use super::*;

    #[test]
    fn pick_data_layout_matches_stride() {
        let cube = freecad_core::prim::cube(2.0);
        let data = build_pick_data(&cube);
        assert_eq!(data.len(), cube.indices.len() * PICK_STRIDE_BYTES as usize);

        // First corner: position of indices[0], normal of indices[0], tid 0.
        let i0 = cube.indices[0] as usize;
        let expected: Vec<u8> = cube.positions[i0]
            .iter()
            .chain(cube.normals[i0].iter())
            .flat_map(|v| v.to_ne_bytes())
            .chain(0u32.to_ne_bytes())
            .collect();
        assert_eq!(&data[..28], &expected[..]);

        // Second triangle's corners carry tid 1.
        let t1_start = 3 * PICK_STRIDE_BYTES as usize;
        assert_eq!(&data[t1_start + 24..t1_start + 28], &1u32.to_ne_bytes());
    }
}
