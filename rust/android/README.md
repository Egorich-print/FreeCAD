# Android target

## Verified status (2026-08, this machine)

| Step | Status | Evidence |
|---|---|---|
| Rust cross-compilation of `freecad-core`, `-kernel`, `-io` | **works** | `cargo check --target aarch64-linux-android` clean |
| `wgpu` renderer cross-compiles for Android | **works** | `cargo check --target aarch64-linux-android -p freecad-render` clean |
| Native ARM64 library link | **works** | `./android/build_rust.sh arm64-v8a` → `libfreecad_android.so`, ELF aarch64 |
| 16 KB page-size compliance | **verified** | `llvm-readelf -l` shows LOAD align `0x4000` on every segment |
| OCCT geometry backend on Android | **blocked here** | needs an OCCT NDK build; reproducible recipe in `build_occt_ndk.sh` (untested) |
| APK / Kotlin shell / device run | **Phase 2** | nothing exists yet by design |

Reproduce the library build:

```bash
./rust/android/build_rust.sh arm64-v8a
# artifact: rust/target/aarch64-linux-android/release/libfreecad_android.so
```

Inspect alignment yourself:

```bash
$NDK/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-readelf -l \
  rust/target/aarch64-linux-android/release/libfreecad_android.so | grep LOAD
```

## What blocks the OCCT backend on Android

`freecad-kernel-occt/build.rs` links desktop OCCT dylibs. For an Android APK the
same shim must link against OCCT static libs produced with the NDK. Nothing in
the shim is macOS-specific (pure STL + OCCT), so this is a build-environment
task, not a porting task:

1. Run `./rust/android/build_occt_ndk.sh <occt-source>` → produces
   `libTKernel.a … libTKDESTEP.a` under `build-android-arm64/install`.
2. Point the bridge at it: `OCCT_ROOT=<install> cargo ndk -t arm64-v8a build
   --release -p freecad-android`.
3. `build.rs` already prefers `$OCCT_ROOT`; add `--features` no changes required
   beyond teaching `header_dir()` about the install layout (already handled).

## Phase 2 plan (Android viewer)

```
app (Kotlin, single Activity)
 └─ SurfaceView + Choreographer tick
     │ JNI: freecad_android_open(bytes) -> handle
     │       freecad_android_mesh(handle) -> positions/normals/indices direct buffers
     ▼
 freecad-render (wgpu, Vulkan → GLES fallback)
```

Deliberately deferred until the desktop proof is validated further; SAF file
picking, autosave and crash recovery come after first on-device frame.
