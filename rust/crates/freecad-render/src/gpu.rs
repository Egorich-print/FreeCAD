use freecad_core::mesh::MeshBuffer;
use freecad_core::mesh::MeshError;
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
}

pub const VERTEX_STRIDE_BYTES: u64 = 24;

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
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fc-mesh-indices"),
            contents: u32_bytes(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
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
}

/// `packed` is `Vec<f32>`; transmute-free byte view without adding bytemuck.
fn bytemuck_bytes(data: &[f32]) -> &[u8] {
    let len = data.len() * core::mem::size_of::<f32>();
    let ptr = data.as_ptr().cast::<u8>();
    // Safety invariant (mission Rule 8): &[f32] is fully initialised, aligned
    // (align(f32)==align(u8)), and u8 has no validity constraints; the slice
    // is borrowed for the duration of this call only.
    unsafe { core::slice::from_raw_parts(ptr, len) }
}

/// Indices variant: u32 buffer contents.
pub(crate) fn u32_bytes(data: &[u32]) -> &[u8] {
    let len = data.len() * core::mem::size_of::<u32>();
    // Safety invariant: same reasoning as `bytemuck_bytes`.
    unsafe { core::slice::from_raw_parts(data.as_ptr().cast::<u8>(), len) }
}
