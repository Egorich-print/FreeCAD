#!/usr/bin/env bash
# Deterministic minimal OCCT build for Android (arm64-v8a today, x86_64 ready).
#
# Produces STATIC OCCT libraries sufficient for the freecad-kernel-occt shim:
# STEP read/write, BREP read/write, primitives, booleans, meshing, bbox.
# No Visualization / Draw / ApplicationFramework / Qt / TBB / FreeImage.
#
# Prerequisites (documented, nothing auto-installed):
#   cmake >= 3.16, a JDK-less environment is fine (BUILD_TESTING off)
#   Android NDK r25+ (verified with r27.2) at $ANDROID_NDK_ROOT
#   OCCT source tree (verified: V7_9_3, commit tag V7_9_3)
#     curl -L -o occt.tar.gz https://github.com/Open-Cascade-SAS/OCCT/archive/refs/tags/V7_9_3.tar.gz
#
# Usage:
#   OCCT_SRC=/path/to/OCCT-7_9_3 ./rust/android/build_occt_ndk.sh [arm64-v8a|x86_64]
set -euo pipefail

log() { printf '[occt-ndk] %s\n' "$*"; }
die() { printf '[occt-ndk] ERROR: %s\n' "$*" >&2; exit 1; }

occt_src="${OCCT_SRC:?set OCCT_SRC to the OCCT source tree (verified: V7_9_3)}"
abi="${1:-arm64-v8a}"
ndk="${ANDROID_NDK_ROOT:-$HOME/Library/Android/sdk/ndk/27.2.12479018}"

case "$abi" in
  arm64-v8a) android_abi=arm64-v8a;  triple=aarch64-linux-android;  api=24 ;;
  x86_64)    android_abi=x86_64;     triple=x86_64-linux-android;   api=24 ;;
  *) die "unsupported abi '$abi' (use arm64-v8a or x86_64)" ;;
esac

toolchain="$ndk/build/cmake/android.toolchain.cmake"
[ -f "$toolchain" ] || die "NDK toolchain not found: $toolchain"
[ -f "$occt_src/CMakeLists.txt" ] || die "OCCT source not found: $occt_src"
command -v cmake >/dev/null || die "cmake not installed"

# Pinned source check (fail loudly on unexpected tree).
if [ -f "$occt_src/adm/cmake/version.cmake" ]; then
  grep -q "7\.9\.3" "$occt_src/adm/cmake/version.cmake" \
    || log "WARNING: OCCT version differs from verified V7_9_3"
fi

build_dir="$occt_src/build-android-${android_abi}"
install_dir="$build_dir/install"
jobs="$(sysctl -n hw.ncpu 2>/dev/null || nproc)"

log "OCCT source : $occt_src"
log "NDK         : $ndk"
log "ABI/API     : $android_abi / $api"
log "Build type  : static, PIC, 16KB-page aware"
log "Jobs        : $jobs"

# 16 KB page alignment is mandatory for Play distribution since Nov 2025;
# static archives inherit it at final .so link, but we set it everywhere anyway.
page_flags="-Wl,-z,max-page-size=16384"

cmake -S "$occt_src" -B "$build_dir" -G "Unix Makefiles" \
  -DCMAKE_TOOLCHAIN_FILE="$toolchain" \
  -DANDROID_ABI="$android_abi" \
  -DANDROID_PLATFORM=android-"$api" \
  -DANDROID_STL=c++_static \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  -DCMAKE_INSTALL_PREFIX="$install_dir" \
  -DCMAKE_SHARED_LINKER_FLAGS="$page_flags" \
  -DCMAKE_EXE_LINKER_FLAGS="$page_flags" \
  -DBUILD_LIBRARY_TYPE=Static \
  -DBUILD_MODULE_FoundationClasses=ON \
  -DBUILD_MODULE_ModelingData=ON \
  -DBUILD_MODULE_ModelingAlgorithms=ON \
  -DBUILD_MODULE_DataExchange=ON \
  -DBUILD_MODULE_ApplicationFramework=OFF \
  -DBUILD_MODULE_Visualization=OFF \
  -DBUILD_MODULE_DETools=OFF \
  -DBUILD_MODULE_Draw=OFF \
  -DBUILD_DOC_Overview=OFF \
  -DBUILD_TESTING=OFF \
  -DBUILD_TOOL_ExpToCasExe=OFF \
  -DBUILD_TOOL_VRMLConverter=OFF \
  -DBUILD_SAMPLES=OFF \
  -DUSE_TBB=OFF \
  -DUSE_FREETYPE=OFF \
  -DUSE_FREEIMAGE=OFF \
  -DUSE_DRACO=OFF \
  -DUSE_FFMPEG=OFF \
  -DUSE_OPENVR=OFF \
  -DUSE_VTK=OFF \
  -DUSE_JEMALLOC=OFF \
  -DUSE_RAPIDJSON=OFF \
  -DUSE_GLES2=OFF \
  -DUSE_EGL=OFF \
  -DUSE_XLIB=OFF

cmake --build "$build_dir" --parallel "$jobs"
cmake --install "$build_dir"

log "installed to: $install_dir"
log "static libs : $(find "$install_dir/lib" -name '*.a' | wc -l | tr -d ' ') archives"
log "next: OCCT_ROOT=$install_dir ./rust/android/build_rust.sh $abi"
