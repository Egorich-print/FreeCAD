use freecad_core::mesh::MeshBuffer;
use freecad_render::{
    GpuMesh, OrbitCamera, RenderItem, Renderer, TargetSize, clear_color_texture, create_depth_view,
};

/// Offscreen proof: GPU initialises, a mesh uploads, the render pass executes,
/// and produced pixels differ from the clear colour. Skips (does not fail) on
/// machines without an available adapter.
#[test]
#[cfg_attr(not(feature = "gpu-tests"), ignore)]
fn offscreen_render_produces_lit_pixels() {
    let instance = wgpu::Instance::default();
    let adapter =
        match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        {
            Ok(adapter) => adapter,
            Err(err) => {
                eprintln!("no wgpu adapter available ({err}); skipping offscreen proof");
                return;
            }
        };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("fc-render-test"),
        ..Default::default()
    }))
    .expect("device request");

    let size = TargetSize {
        width: 320,
        height: 240,
    };
    let renderer = Renderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mesh_buffer: MeshBuffer = freecad_core::prim::cube(2.0);
    let gpu_mesh = GpuMesh::from_mesh_buffer(&device, &mesh_buffer).expect("upload");

    let mut camera = OrbitCamera::default();
    camera.frame([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);
    renderer.update_camera(&queue, &camera, size.width as f32 / size.height as f32);

    let (color_tex, color_view) = clear_color_texture(&device, size);
    let depth_view = create_depth_view(&device, size);

    let bytes_per_row = (size.width * 4).next_multiple_of(256);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fc-readback"),
        size: (bytes_per_row * size.height.max(1)) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.render(
        &mut encoder,
        &color_view,
        &depth_view,
        size,
        &[RenderItem { mesh: &gpu_mesh }],
    );
    encoder.copy_texture_to_buffer(
        color_tex.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(size.height.max(1)),
            },
        },
        wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let (tx, rx) = std::sync::mpsc::channel();
    {
        let sender = tx.clone();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result.is_ok());
            });
    }
    device.poll(wgpu::PollType::Wait).expect("poll");
    rx.recv().expect("map completion signal");

    let lit = {
        let data = readback.slice(..).get_mapped_range();
        data.chunks_exact(4)
            .filter(|px| px[0] as u32 + px[1] as u32 + px[2] as u32 > CLEAR_SUM_THRESHOLD)
            .count()
    };
    readback.unmap();

    let total = (size.width * size.height) as usize;
    assert!(
        lit * 10 > total,
        "expected >10% lit pixels, got {lit}/{total}"
    );
}

const CLEAR_SUM_THRESHOLD: u32 = 120;
