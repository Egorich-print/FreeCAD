//! JNI viewer surface: STEP bytes in, OCCT geometry, wgpu render onto an
//! Android `Surface`. Lifecycle stays on the Java side; this module never
//! touches filesystem or UI toolkit internals.

#![allow(clippy::too_many_arguments)]

use std::ptr::NonNull;

use freecad_io::{Format, load_bytes};
use freecad_kernel::GeometryKernel;
use freecad_kernel_occt::OcctBackend;
use freecad_render::{GpuMesh, OrbitCamera, RenderItem, Renderer, TargetSize, create_depth_view};

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JObject};
use jni::sys::{jbyteArray, jint, jlong};
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use wgpu::SurfaceTargetUnsafe;

const STATUS_OK: i32 = 0;
const ERR_KERNEL: i32 = -1;
const ERR_MESH_EMPTY: i32 = -2;
const ERR_GPU: i32 = -3;

unsafe extern "C" {
    // libandroid.so — retained reference, released in Viewer::destroy.
    fn ANativeWindow_fromSurface(
        env: *mut jni::sys::JNIEnv,
        surface: jni::sys::jobject,
    ) -> *mut core::ffi::c_void;
    fn ANativeWindow_release(window: *mut core::ffi::c_void);
    fn ANativeWindow_getWidth(window: *mut core::ffi::c_void) -> i32;
    fn ANativeWindow_getHeight(window: *mut core::ffi::c_void) -> i32;
}

struct Viewer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    surface: Option<wgpu::Surface<'static>>,
    config: wgpu::SurfaceConfiguration,
    depth_size: TargetSize,
    depth: wgpu::TextureView,
    renderer: Renderer,
    meshes: Vec<GpuMesh>,
    camera: OrbitCamera,
    native_window: *mut core::ffi::c_void,
}

// Safety invariant (mission Rule 8): `native_window` is exclusively owned here
// (retained via ANativeWindow_fromSurface, released exactly once in destroy);
// all JNI entry points are called serially from the Android UI thread, so no
// aliasing of the Viewer or the window pointer occurs.
unsafe impl Send for Viewer {}

fn load_meshes(
    step_bytes: &[u8],
) -> Result<(Vec<freecad_core::mesh::MeshBuffer>, [f64; 3], [f64; 3]), i32> {
    let mut kernel = OcctBackend::new().map_err(|_| ERR_KERNEL)?;
    let shape = load_bytes(&mut kernel, step_bytes, Format::Step).map_err(|_| ERR_KERNEL)?;
    let bounds = kernel.bounds(&shape).map_err(|_| ERR_KERNEL)?;
    let mesh = kernel
        .tessellate(&shape, 0.5, 0.35)
        .map_err(|_| ERR_KERNEL)?;
    if mesh.is_empty() {
        return Err(ERR_MESH_EMPTY);
    }
    Ok((vec![mesh], bounds.min, bounds.max))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_freecad_viewer_MainActivity_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
    surface: JObject,
    step_bytes: jbyteArray,
) -> jlong {
    let bytes_array = unsafe { JByteArray::from_raw(step_bytes) };
    let bytes = match env.convert_byte_array(&bytes_array) {
        Ok(bytes) => bytes,
        Err(_) => return ERR_KERNEL as jlong,
    };

    // Safety invariant: `surface` is valid for this call; ANativeWindow keeps
    // its own retained reference which Viewer::destroy releases.
    let window = unsafe { ANativeWindow_fromSurface(env.get_raw(), surface.as_raw()) };
    if window.is_null() {
        return ERR_GPU as jlong;
    }

    let release_window = || unsafe { ANativeWindow_release(window) };

    let (mesh_buffers, bounds_min, bounds_max) = match load_meshes(&bytes) {
        Ok(ok) => ok,
        Err(code) => {
            release_window();
            return code as jlong;
        }
    };

    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    })) {
        Ok(adapter) => adapter,
        Err(_) => {
            release_window();
            return ERR_GPU as jlong;
        }
    };

    let Some(window_ptr) = NonNull::new(window) else {
        release_window();
        return ERR_GPU as jlong;
    };
    let handle = AndroidNdkWindowHandle::new(window_ptr);
    // Safety invariant: the retained ANativeWindow outlives the Surface
    // (released only in nativeDestroy after the surface is dropped).
    let target = SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: RawDisplayHandle::Android(AndroidDisplayHandle::new()),
        raw_window_handle: RawWindowHandle::AndroidNdk(handle),
    };
    let surface = match unsafe { instance.create_surface_unsafe(target) } {
        Ok(surface) => surface,
        Err(_) => {
            release_window();
            return ERR_GPU as jlong;
        }
    };

    let (device, queue) =
        match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("fc-android"),
            ..Default::default()
        })) {
            Ok(pair) => pair,
            Err(_) => {
                release_window();
                return ERR_GPU as jlong;
            }
        };

    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats[0];
    let size = TargetSize {
        width: unsafe { ANativeWindow_getWidth(window) }.max(1) as u32,
        height: unsafe { ANativeWindow_getHeight(window) }.max(1) as u32,
    };
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let renderer = Renderer::new(&device, format);
    let depth = create_depth_view(&device, size);

    let meshes: Vec<GpuMesh> = mesh_buffers
        .iter()
        .filter_map(|m| GpuMesh::from_mesh_buffer(&device, m).ok())
        .collect();

    let mut camera = OrbitCamera::default();
    camera.frame(bounds_min, bounds_max);

    let viewer = Box::new(Viewer {
        device,
        queue,
        instance,
        adapter,
        surface: Some(surface),
        config,
        depth_size: size,
        depth,
        renderer,
        meshes,
        camera,
        native_window: window,
    });
    Box::into_raw(viewer) as jlong
}

/// Safety invariant: `handle` was created by Box::into_raw in nativeInit;
/// access is exclusive (single UI thread) and destruction happens exactly once
/// in nativeDestroy.
fn with_viewer<R>(handle: jlong, f: impl FnOnce(&mut Viewer) -> R) -> Option<R> {
    if handle <= 0 {
        return None;
    }
    // SAFETY: see above.
    Some(f(unsafe { &mut *(handle as *mut Viewer) }))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_freecad_viewer_MainActivity_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle > 0 {
        let mut viewer = unsafe { Box::from_raw(handle as *mut Viewer) };
        viewer.surface.take();
        unsafe { ANativeWindow_release(viewer.native_window) };
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_freecad_viewer_MainActivity_nativeOrbit(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    dx: f32,
    dy: f32,
) {
    with_viewer(handle, |v| v.camera.orbit(dx as f64, dy as f64));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_freecad_viewer_MainActivity_nativeZoom(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    factor: f32,
) {
    with_viewer(handle, |v| v.camera.zoom(factor as f64));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_freecad_viewer_MainActivity_nativeRender(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    let mut result = ERR_GPU;
    with_viewer(handle, |viewer| {
        let Some(surface) = viewer.surface.as_ref() else {
            return;
        };

        let width = unsafe { ANativeWindow_getWidth(viewer.native_window) }.max(1) as u32;
        let height = unsafe { ANativeWindow_getHeight(viewer.native_window) }.max(1) as u32;
        if width != viewer.config.width || height != viewer.config.height {
            viewer.config.width = width;
            viewer.config.height = height;
            surface.configure(&viewer.device, &viewer.config);
            viewer.depth = create_depth_view(&viewer.device, TargetSize { width, height });
        }

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let size = TargetSize { width, height };

        viewer.renderer.update_camera(
            &viewer.queue,
            &viewer.camera,
            width as f32 / height.max(1) as f32,
        );
        let items: Vec<RenderItem<'_>> = viewer
            .meshes
            .iter()
            .map(|m| RenderItem { mesh: m })
            .collect();
        let mut encoder = viewer.device.create_command_encoder(&Default::default());
        viewer
            .renderer
            .render(&mut encoder, &view, &viewer.depth, size, &items);
        viewer.queue.submit(Some(encoder.finish()));
        let _ = frame.present();

        result = STATUS_OK;
    });
    result
}
