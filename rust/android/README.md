# Android target — first vertical slice (M2.0)

## Verified status (2026-08, this machine)

| Step | Status | Evidence |
|---|---|---|
| OCCT 7.9.3 built for Android arm64-v8a | **works** | `build_occt_ndk.sh` → 47 static archives (NDK r27.2, API 24, PIC, no TBB/FreeImage/Qt/Draw) |
| `freecad-kernel-occt` links against Android OCCT | **works** | `OCCT_ROOT=... ./build_rust.sh arm64-v8a` → `libfreecad_android.so` 27.5 MB |
| 16 KB page alignment | **verified** | `llvm-readelf -l` → every LOAD align `0x4000`; `zipalign -c -P 16 4 <apk>` → OK |
| STEP fixture through real OCCT | **works** | `tests/fixtures/demo_part.step` imported byte-wise, 1 solid / 9 faces, mesh validated |
| wgpu cross-compile for Android | **works** | `cargo check --target aarch64-linux-android -p freecad-render` |
| APK assembly (no Gradle) | **works** | `build_apk.sh` → `app/build/freecad-viewer-debug.apk` (26 MB, signed) |
| APK contents | **verified** | `classes.dex`, `assets/demo_part.step`, `lib/arm64-v8a/*.so`, launchable `com.freecad.viewer.MainActivity` |
| **Runtime on device/emulator** | **BLOCKED here** | `adb devices` → none; no emulator/system-images in this SDK. Install command ready (below) |

## Reproduce

```bash
# 1. OCCT sources (pinned)
curl -L -o occt.tar.gz https://github.com/Open-Cascade-SAS/OCCT/archive/refs/tags/V7_9_3.tar.gz
tar xzf occt.tar.gz

# 2. OCCT for Android (static, minimal modules)
OCCT_SRC=$PWD/OCCT-7_9_3 ./rust/android/build_occt_ndk.sh arm64-v8a

# 3. Rust viewer .so (OCCT + wgpu + JNI)
OCCT_ROOT=$PWD/OCCT-7_9_3/build-android-arm64-v8a/install \
  ./rust/android/build_rust.sh arm64-v8a

# 4. APK (javac + d8 + aapt2 + zipalign + apksigner; no Gradle, offline)
./rust/android/build_apk.sh

# 5. On a real device / running emulator:
adb install -r rust/android/app/build/freecad-viewer-debug.apk
adb logcat -s FreeCAD
```

## Runtime architecture

```
MainActivity (Java, thin: Surface lifecycle + touch)
  ├─ Choreographer loop ──► nativeRender(handle)
  ├─ drag ────────────────► nativeOrbit(handle,dx,dy)
  └─ pinch ───────────────► nativeZoom(handle,factor)
        │ JNI
        ▼
freecad-android::view (feature "viewer")
  ├─ ANativeWindow_fromSurface (libandroid)
  ├─ STEP bytes → OcctBackend → tessellate → MeshBuffer
  ├─ wgpu Surface from AndroidNdk raw handles (Vulkan/GLES)
  └─ freecad-render::Renderer (same code as desktop)
```

Notes:
- Java (not Kotlin) because no `kotlinc` exists in the reference environment;
  the activity is intentionally a dumb shell, swapping in Kotlin later is a
  mechanical change.
- The debug keystore `app/debug.keystore` is generated on first sign and is
  git-ignored; it is not a production credential.
- `app/build/` is git-ignored; the APK is a build artifact, not a source file.

## Device matrix

| Field | Value |
|---|---|
| Android runtime proof | **pending hardware** (see blocker) |
| ABI | arm64-v8a (x86_64 supported by both scripts) |
| minSdk / targetSdk | 24 / 34 |
| NDK | r27.2.12479018 |
| OCCT | V7_9_3 (pinned tag) |
| wgpu backends expected on device | Vulkan primary, GLES fallback |

## Phase H reproduction (when hardware is available)

```bash
adb install -r rust/android/app/build/freecad-viewer-debug.apk
adb shell am start -n com.freecad.viewer/.MainActivity
adb logcat -s FreeCAD          # expect: "nativeInit(OCCT+mesh+wgpu init) took N ms"
adb shell dumpsys gfxinfo com.freecad.viewer   # frame stats
```

`nativeInit` already measures OCCT import + tessellation + GPU init wall time
and logs it; FPS can be derived from `dumpsys gfxinfo` frame times.
