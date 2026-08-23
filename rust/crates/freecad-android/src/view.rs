//! JNI viewer surface: STEP bytes in, OCCT geometry, wgpu render onto an
//! Android `Surface`. Lifecycle stays on the Java side; this module never
//! touches filesystem or UI toolkit internals.

#![allow(clippy::too_many_arguments)]

use std::ptr::NonNull;

use freecad_core::mesh::MeshBuffer;
use freecad_io::{Format, load_bytes};
use freecad_kernel::GeometryKernel;
use freecad_kernel_occt::OcctBackend;
use freecad_render::pick::{PickInput, Picker};
use freecad_render::{GpuMesh, OrbitCamera, RenderItem, Renderer, TargetSize, create_depth_view};

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JObject};
use jni::sys::{jbyteArray, jfloat, jint, jlong};
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use wgpu::SurfaceTargetUnsafe;

const STATUS_OK: i32 = 0;
// nativeInit failure codes (returned as 0 handle; logged by Java).
const ERR_KERNEL: i32 = -1;
const ERR_MESH_EMPTY: i32 = -2;
const ERR_WINDOW: i32 = -3;
const ERR_SURFACE_CREATE: i32 = -4;
const ERR_ADAPTER: i32 = -5;
const ERR_MESH_UPLOAD: i32 = -7;
// nativeRender failure codes.
const RENDER_NO_SURFACE: i32 = -31;
const RENDER_GET_TEXTURE: i32 = -33;

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
    surface: Option<wgpu::Surface<'static>>,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
    renderer: Renderer,
    picker: Picker,
    meshes: Vec<GpuMesh>,
    mesh_buffers: Vec<MeshBuffer>,
    selected: Option<(usize, u32)>,
    camera: OrbitCamera,
    native_window: *mut core::ffi::c_void,
}

// Safety invariant (mission Rule 8): `native_window` is exclusively owned here
// (retained via ANativeWindow_fromSurface, released exactly once in destroy);
// all JNI entry points are called serially from the Android UI thread, so no
// aliasing of the Viewer or the window pointer occurs.
unsafe impl Send for Viewer {}

type LoadedModel = (Vec<freecad_core::mesh::MeshBuffer>, [f64; 3], [f64; 3]);

fn load_meshes(step_bytes: &[u8]) -> Result<LoadedModel, i32> {
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
    env: JNIEnv,
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
        return ERR_WINDOW as jlong;
    }

    let release_window = || unsafe { ANativeWindow_release(window) };

    let (mesh_buffers, bounds_min, bounds_max) = match load_meshes(&bytes) {
        Ok((m, mn, mx)) => (m, mn, mx),
        Err(_) => {
            release_window();
            return 0;
        }
    };

    let instance = wgpu::Instance::default();

    let Some(window_ptr) = NonNull::new(window) else {
        release_window();
        return ERR_WINDOW as jlong;
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
            return ERR_SURFACE_CREATE as jlong;
        }
    };

    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    })) {
        Ok(adapter) => adapter,
        Err(_) => {
            release_window();
            return ERR_ADAPTER as jlong;
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
                return 0;
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
    let picker = Picker::new(&device);
    let depth = create_depth_view(&device, size);

    let mut meshes = Vec::new();
    for m in &mesh_buffers {
        match GpuMesh::from_mesh_buffer(&device, m) {
            Ok(gpu) => meshes.push(gpu),
            Err(_) => {
                release_window();
                return ERR_MESH_UPLOAD as jlong;
            }
        }
    }

    let mut camera = OrbitCamera::default();
    camera.frame(bounds_min, bounds_max);

    let viewer = Box::new(Viewer {
        device,
        queue,
        surface: Some(surface),
        config,
        depth,
        renderer,
        picker,
        meshes,
        mesh_buffers,
        selected: None,
        camera,
        native_window: window,
    });
    Box::into_raw(viewer) as jlong
}

/// Safety invariant: `handle` was created by Box::into_raw in nativeInit;
/// access is exclusive (single UI thread) and destruction happens exactly once
/// in nativeDestroy.
fn with_viewer<R>(handle: jlong, f: impl FnOnce(&mut Viewer) -> R) -> Option<R> {
    if handle == 0 {
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
    if handle != 0 {
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

/// Tap at screen pixel: resolves the face under the cursor and stores the
/// selection. Returns the picked face id or -1 on miss.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_freecad_viewer_MainActivity_nativeTap(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    x: jfloat,
    y: jfloat,
) -> jint {
    with_viewer(handle, |v| {
        let inputs: Vec<PickInput> = v
            .mesh_buffers
            .iter()
            .enumerate()
            .map(|(i, m)| PickInput {
                mesh_index: i,
                mesh: m,
            })
            .collect();
        match v.picker.pick(
            &v.camera,
            TargetSize {
                width: v.config.width,
                height: v.config.height,
            },
            &inputs,
            x as u32,
            y as u32,
        ) {
            Some(hit) => {
                v.selected = Some((hit.mesh_index, hit.face_id));
                hit.face_id as jint
            }
            None => {
                v.selected = None;
                -1
            }
        }
    })
    .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_freecad_viewer_MainActivity_nativeRender(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    let mut result = RENDER_NO_SURFACE;
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
            // Transient on emulators/software rasterizers: skip this frame.
            Err(wgpu::SurfaceError::Timeout) | Err(wgpu::SurfaceError::Outdated) => {
                result = STATUS_OK;
                return;
            }
            Err(err) => {
                result = match err {
                    wgpu::SurfaceError::Lost => -34,
                    wgpu::SurfaceError::OutOfMemory => -35,

                    _ => RENDER_GET_TEXTURE,
                };
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let size = TargetSize { width, height };

        viewer.renderer.update_camera(
            &viewer.queue,
            &viewer.camera,
            width as f32 / height.max(1) as f32,
        );
        let highlight = match viewer.selected {
            Some((mi, face)) => {
                freecad_core::selection::extract_face(&viewer.mesh_buffers[mi], face)
                    .and_then(|fm| GpuMesh::from_mesh_buffer(&viewer.device, &fm).ok())
            }
            None => None,
        };
        let selected_mesh = viewer.selected.map(|s| s.0);
        let highlight_gpu = highlight.as_ref();
        let items: Vec<RenderItem<'_>> = viewer
            .meshes
            .iter()
            .enumerate()
            .map(|(i, m)| RenderItem {
                mesh: m,
                highlight: if selected_mesh == Some(i) {
                    highlight_gpu
                } else {
                    None
                },
            })
            .collect();
        let mut encoder = viewer.device.create_command_encoder(&Default::default());
        viewer
            .renderer
            .render(&mut encoder, &view, &viewer.depth, size, &items);
        viewer.queue.submit(Some(encoder.finish()));
        frame.present();

        result = STATUS_OK;
    });
    result
}

/// Minimal wrapper over liblog so tap diagnostics reach logcat.
#[allow(dead_code)] // retained for on-device tap/init diagnostics
fn android_log(msg: &str) {
    unsafe extern "C" {
        fn __android_log_print(
            prio: i32,
            tag: *const core::ffi::c_char,
            fmt: *const core::ffi::c_char,
            ...
        ) -> i32;
    }
    let tag = c"FreeCAD";
    let cmsg = std::ffi::CString::new(msg).unwrap_or_default();

    // 3 = ANDROID_LOG_INFO
    unsafe { __android_log_print(3, tag.as_ptr(), c"%s".as_ptr(), cmsg.as_ptr()) };
}
