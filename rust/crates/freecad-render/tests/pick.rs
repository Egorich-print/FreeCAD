//! GPU-assisted picking: deterministic face identification on the cube
//! fixture (six distinct face-ranges) plus an adjacent-faces fixture that
//! would fail if picking only proved "hit somewhere in the object".

use freecad_core::mesh::{FaceRange, MeshBuffer};
use freecad_core::selection::extract_face;
use freecad_render::pick::PickInput;
use freecad_render::{
    GpuMesh, OrbitCamera, PickHit, Picker, RenderItem, Renderer, TargetSize, clear_color_texture,
    create_depth_view,
};

fn camera_looking_down_minus_z() -> OrbitCamera {
    let mut camera = OrbitCamera {
        yaw_rad: 0.0,
        pitch_rad: 0.0,
        ..Default::default()
    };
    camera.frame([-1.0; 3], [1.0; 3]);
    camera
}

#[test]
fn picks_center_face_misses_background_and_survives_orbit() {
    let size = TargetSize {
        width: 320,
        height: 240,
    };
    let picker = Picker;

    let cube_buffer = freecad_core::prim::cube(2.0);
    let inputs = [PickInput {
        mesh_index: 0,
        mesh: &cube_buffer,
    }];

    // Camera on +Z axis: screen center lands exactly on face 0 (+Z).
    let camera = camera_looking_down_minus_z();
    let hit: PickHit = picker
        .pick(&camera, size, &inputs, 160, 120)
        .expect("center must hit");
    assert_eq!(hit.mesh_index, 0);
    assert_eq!(hit.face_id, 0, "+Z face identified");
    assert!(
        hit.distance > 1.0 && hit.distance < 2.5,
        "distance {}",
        hit.distance
    );

    // Sky pixel: pull the camera back so the top rows are empty space.
    let mut far_camera = camera;
    far_camera.zoom(3.0);
    assert!(
        picker.pick(&far_camera, size, &inputs, 160, 4).is_none(),
        "background is a miss"
    );

    // Orbit to the opposite side: center now hits face 1 (-Z).
    let mut back = camera_looking_down_minus_z();
    back.yaw_rad = std::f64::consts::PI;
    let hit_back = picker
        .pick(&back, size, &inputs, 160, 120)
        .expect("opposite center must hit");
    assert_eq!(hit_back.face_id, 1, "-Z face after orbit");

    // Orbit yaw by 90°: eye moves to +X side → face 2 (+X).
    let mut side = camera_looking_down_minus_z();
    side.yaw_rad = std::f64::consts::FRAC_PI_2;
    let hit_side = picker
        .pick(&side, size, &inputs, 160, 120)
        .expect("side center must hit");
    assert_eq!(hit_side.face_id, 2, "+X face after orbit");
}

#[test]
fn adjacent_faces_with_separate_ranges_are_distinguished() {
    let size = TargetSize {
        width: 400,
        height: 200,
    };
    let picker = Picker;

    // Two coplanar quads sharing an edge at x=0, each its own face-range.
    let mut mesh = MeshBuffer::default();
    let quad = |mesh: &mut MeshBuffer, x0: f32, x1: f32, face_id: u32| {
        let base = mesh.positions.len() as u32;
        for corner in [
            [x0, -1.0, 0.0],
            [x1, -1.0, 0.0],
            [x1, 1.0, 0.0],
            [x0, 1.0, 0.0],
        ] {
            mesh.positions.push(corner);
            mesh.normals.push([0.0, 0.0, 1.0]);
        }
        let start = mesh.indices.len() as u32;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        mesh.faces.push(FaceRange {
            face_id,
            index_start: start,
            index_count: 6,
        });
    };
    quad(&mut mesh, -2.0, 0.0, 10);
    quad(&mut mesh, 0.0, 2.0, 11);
    mesh.validate().unwrap();

    let inputs = [PickInput {
        mesh_index: 0,
        mesh: &mesh,
    }];
    let camera = camera_looking_down_minus_z();
    let left = picker
        .pick(&camera, size, &inputs, 100, 100)
        .expect("left quad hit");
    let right = picker
        .pick(&camera, size, &inputs, 300, 100)
        .expect("right quad hit");

    assert_eq!(left.face_id, 10, "adjacent left face");
    assert_eq!(right.face_id, 11, "adjacent right face");
}

#[test]
fn highlight_overlay_draws_and_pick_stays_deterministic() {
    let (device, queue) = setup_gpu();
    let size = TargetSize {
        width: 160,
        height: 120,
    };
    let renderer = Renderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let picker = Picker;

    let cube_buffer = freecad_core::prim::cube(2.0);
    let gpu_mesh = GpuMesh::from_mesh_buffer(&device, &cube_buffer).unwrap();

    let camera = camera_looking_down_minus_z();
    renderer.update_camera(&queue, &camera, size.width as f32 / size.height as f32);

    // Highlight the +Z face and render.
    let highlighted = extract_face(&cube_buffer, 0).expect("face extraction");
    let hl_gpu = GpuMesh::from_mesh_buffer(&device, &highlighted).unwrap();
    let items = [RenderItem {
        mesh: &gpu_mesh,
        highlight: Some(&hl_gpu),
    }];

    let (_tex, color_view) = clear_color_texture(&device, size);
    let depth_view = create_depth_view(&device, size);
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.render(&mut encoder, &color_view, &depth_view, size, &items);
    queue.submit(Some(encoder.finish()));

    // Picking against the plain buffer stays deterministic.
    let inputs = [PickInput {
        mesh_index: 0,
        mesh: &cube_buffer,
    }];
    let hit = picker
        .pick(&camera, size, &inputs, 80, 60)
        .expect("hit through overlay flow");
    assert_eq!(hit.face_id, 0);
}

fn setup_gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("adapter for pick tests");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("fc-pick-test"),
        ..Default::default()
    }))
    .expect("device")
}
