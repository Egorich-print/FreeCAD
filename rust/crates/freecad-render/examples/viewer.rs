//! Minimal native CAD viewer: STEP/BREP → freecad-kernel-occt → MeshBuffer →
//! wgpu. The desktop proof of the full new-stack pipeline.
//!
//! Usage: `cargo run -p freecad-render --example viewer [model.step|model.brep]`
//! Without an argument it builds a parametric-style demo part with OCCT.

use freecad_core::mesh::MeshBuffer;
use freecad_io::{Format, load_bytes};
use freecad_kernel::GeometryKernel;
use freecad_kernel_occt::OcctBackend;
use freecad_render::{GpuMesh, OrbitCamera, RenderItem, Renderer};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

fn main() {
    let model_path = std::env::args().nth(1);
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = ViewerApp {
        model_path,
        state: None,
    };
    event_loop.run_app(&mut app).expect("run");
}

struct ViewerState {
    window: std::sync::Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
    renderer: Renderer,
    meshes: Vec<GpuMesh>,
    camera: OrbitCamera,
    drag: Option<(DragKind, f64, f64)>,
    modifiers: ModifiersState,
}

#[derive(Clone, Copy)]
enum DragKind {
    Orbit,
    Pan,
}

struct ViewerApp {
    model_path: Option<String>,
    state: Option<ViewerState>,
}

impl ApplicationHandler for ViewerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("FreeCAD-Rust viewer")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0f64, 800.0f64));
        let window = std::sync::Arc::new(event_loop.create_window(attrs).expect("window"));

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("fc-viewer"),
            ..Default::default()
        }))
        .expect("device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, format);
        let depth = freecad_render::create_depth_view(
            &device,
            freecad_render::TargetSize {
                width: config.width,
                height: config.height,
            },
        );

        let mesh_buffers = load_meshes(&self.model_path).expect("geometry pipeline");
        let mut camera = OrbitCamera::default();
        if let Some(bounds) = mesh_buffers
            .iter()
            .filter_map(|m| m.bounds())
            .reduce(freecad_core::BoundingBox::union)
        {
            camera.frame(bounds.min.map(f64::from), bounds.max.map(f64::from));
        }
        let meshes: Vec<GpuMesh> = mesh_buffers
            .iter()
            .map(|m| GpuMesh::from_mesh_buffer(&device, m).expect("upload"))
            .collect();

        self.state = Some(ViewerState {
            window,
            device,
            queue,
            surface,
            config,
            depth,
            renderer,
            meshes,
            camera,
            drag: None,
            modifiers: ModifiersState::default(),
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.config.width = size.width.max(1);
                state.config.height = size.height.max(1);
                state.surface.configure(&state.device, &state.config);
                state.depth = freecad_render::create_depth_view(
                    &state.device,
                    freecad_render::TargetSize {
                        width: state.config.width,
                        height: state.config.height,
                    },
                );
                state.window.request_redraw();
            }
            WindowEvent::ModifiersChanged(m) => state.modifiers = m.state(),
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => match (button_state, button) {
                (ElementState::Pressed, MouseButton::Left | MouseButton::Right) => {
                    let pan = button == MouseButton::Right || state.modifiers.shift_key();
                    state.drag =
                        Some((if pan { DragKind::Pan } else { DragKind::Orbit }, 0.0, 0.0));
                }
                (ElementState::Released, _) => state.drag = None,
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                if let Some((kind, last_x, last_y)) = state.drag.take() {
                    let dx = position.x - last_x;
                    let dy = position.y - last_y;
                    match kind {
                        DragKind::Orbit => {
                            state.camera.orbit(-dx * 0.006, -dy * 0.006);
                            state.drag = Some((kind, position.x, position.y));
                        }
                        DragKind::Pan => {
                            state.camera.pan_screen(dx, dy);
                            state.drag = Some((kind, position.x, position.y));
                        }
                    }
                    state.window.request_redraw();
                } else {
                    state.drag = None;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y / 40.0,
                };
                state.camera.zoom((-lines * 0.15).exp());
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => redraw(state),
            _ => {}
        }
    }
}

fn load_meshes(path: &Option<String>) -> Result<Vec<MeshBuffer>, Box<dyn std::error::Error>> {
    let mut kernel = OcctBackend::new()?;
    let shapes: Vec<freecad_core::ShapeId> = match path {
        Some(path) => {
            let data = std::fs::read(path)?;
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            let format =
                Format::from_extension(ext).ok_or_else(|| format!("unknown extension .{ext}"))?;
            vec![load_bytes(&mut kernel, &data, format)?]
        }
        None => demo_part(&mut kernel)?,
    };

    let mut out = Vec::new();
    for shape in &shapes {
        let mesh = kernel.tessellate(shape, 0.5, 0.35)?;
        out.push(mesh);
    }
    Ok(out)
}

fn demo_part(
    kernel: &mut OcctBackend,
) -> Result<Vec<freecad_core::ShapeId>, Box<dyn std::error::Error>> {
    let plate = kernel.make_box(120.0, 80.0, 12.0)?;
    let boss = kernel.make_cylinder(18.0, 60.0)?;
    let boss = kernel.move_by(&boss, 60.0, 40.0, 12.0)?;
    let fused = kernel.fuse(&plate, &boss)?;
    let hole = kernel.make_cylinder(8.0, 200.0)?;
    let hole = kernel.move_by(&hole, 60.0, 40.0, -50.0)?;
    let drilled = kernel.cut(&fused, &hole)?;
    Ok(vec![drilled])
}

fn redraw(state: &mut ViewerState) {
    let frame = match state.surface.get_current_texture() {
        Ok(frame) => frame,
        Err(_) => return,
    };
    let view = frame.texture.create_view(&Default::default());
    let size = freecad_render::TargetSize {
        width: state.config.width,
        height: state.config.height,
    };
    state.renderer.update_camera(
        &state.queue,
        &state.camera,
        state.config.width as f32 / state.config.height as f32,
    );
    let items: Vec<RenderItem<'_>> = state
        .meshes
        .iter()
        .map(|m| RenderItem {
            mesh: m,
            highlight: None,
        })
        .collect();
    let mut encoder = state.device.create_command_encoder(&Default::default());
    state
        .renderer
        .render(&mut encoder, &view, &state.depth, size, &items);
    state.queue.submit(Some(encoder.finish()));
    frame.present();
}
