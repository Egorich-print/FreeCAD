//! Scan + low-level diagnostics for the GPU id-buffer picker.

use freecad_render::pick::PickInput;
use freecad_render::{OrbitCamera, Picker, TargetSize};

fn setup() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    device.on_uncaptured_error(Box::new(|e| panic!("wgpu: {e}")));
    (device, queue)
}

#[test]
fn scan_and_diagnose() {
    let _ = setup();
    let picker = Picker;
    let size = TargetSize {
        width: 64,
        height: 64,
    };

    let cube = freecad_core::prim::cube(2.0);
    let front = freecad_core::selection::extract_face(&cube, 0).unwrap();
    let back = freecad_core::selection::extract_face(&cube, 1).unwrap();

    let items_front = [PickInput {
        mesh_index: 0,
        mesh: &front,
    }];
    let items_back = [PickInput {
        mesh_index: 0,
        mesh: &back,
    }];
    let items_both = [
        PickInput {
            mesh_index: 0,
            mesh: &front,
        },
        PickInput {
            mesh_index: 1,
            mesh: &back,
        },
    ];

    let mut cam = OrbitCamera {
        yaw_rad: 0.0,
        pitch_rad: 0.0,
        ..Default::default()
    };
    cam.frame([-1.0; 3], [1.0; 3]);

    for (label, inputs) in [
        ("front-only", &items_front[..]),
        ("back-only", &items_back[..]),
        ("both", &items_both[..]),
    ] {
        match picker.pick(&cam, size, inputs, 32, 32) {
            Some(h) => println!(
                "{label}: face={} tri={} dist={:.3}",
                h.face_id, h.triangle_id, h.distance
            ),
            None => println!("{label}: MISS"),
        }
    }

    // Depth ordering: the nearer (+Z) face must win even though the far one
    // is drawn last.
    let hit = picker
        .pick(&cam, size, &items_both, 32, 32)
        .expect("center hit");
    assert_eq!(hit.face_id, 0, "+Z nearer must win over -Z drawn later");
}
