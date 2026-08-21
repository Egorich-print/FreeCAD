#!/usr/bin/env bash
# Cross-builds the OCCT geometry toolkits for Android (headless: no
# Visualization / Draw / ApplicationFramework modules).
#
# STATUS: UNTESTED RECIPE — this environment has no OCCT source tree or NDK
# ninja setup wired for OCCT. It encodes the documented OCCT Android build
# (docs list NDK r19+, API 21+, ANDROID_STL=c++_shared) plus mandatory 16 KB
# page-size linker flags. Expect small adjustments (module names drift between
# OCCT 7.8 and 7.9).
#
# Usage:
#   ANDROID_NDK_ROOT=~/Library/Android/sdk/ndk/27.2.12479018 \
#     ./rust/android/build_occt_ndk.sh /path/to/OCCT-src
set -euo pipefail

occt_src="${1:?usage: build_occt_ndk.sh <OCCT source dir>}"
ndk="${ANDROID_NDK_ROOT:-$HOME/Library/Android/sdk/ndk/27.2.12479018}"
abi="${ANDROID_ABI:-arm64-v8a}"

toolchain="$ndk/build/cmake/android.toolchain.cmake"
[ -f "$toolchain" ] || { echo "NDK toolchain not found at $toolchain" >&2; exit 1; }
[ -d "$occt_src" ] || { echo "OCCT source not found at $occt_src" >&2; exit 1; }

build_dir="$occt_src/build-android-${abi}"
install_dir="$build_dir/install"

cmake -S "$occt_src" -B "$build_dir" -G Ninja \
  -DCMAKE_TOOLCHAIN_FILE="$toolchain" \
  -DANDROID_ABI="$abi" \
  -DANDROID_PLATFORM=android-24 \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX="$install_dir" \
  -DBUILD_LIBRARY_TYPE=Static \
  -DBUILD_MODULE_ApplicationFramework=OFF \
  -DBUILD_MODULE_Draw=OFF \
  -DBUILD_MODULE_Visualization=OFF \
  -DUSE_FREETYPE=OFF \
  -DUSE_FREEIMAGE=OFF \
  -DUSE_TBB=OFF \
  -DUSE_DRACO=OFF \
  -DUSE_FFMPEG=OFF \
  -DUSE_OPENVR=OFF \
  -DUSE_VTK=OFF \
  -DUSE_JEMALLOC=OFF \
  -DUSE_RAPIDJSON=OFF \
  -DBUILD_DOC_Overview=OFF \
  -DBUILD_TESTING=OFF \
  -DBUILD_SAMPLES=OFF \
  -DBUILD_TOOL_BRepToIGES=OFF \
  -DBUILD_TOOL_VRMLConverter=OFF \
  -DCMAKE_SHARED_LINKER_FLAGS="-Wl,-z,max-page-size=16384" \
  -DCMAKE_EXE_LINKER_FLAGS="-Wl,-z,max-page-size=16384"

cmake --build "$build_dir" --parallel "$(sysctl -n hw.ncpu)"
cmake --install "$build_dir"

echo
echo "installed to: $install_dir"
echo "next: OCCT_ROOT=$install_dir cargo ndk -t ${abi} build --release -p freecad-android"
