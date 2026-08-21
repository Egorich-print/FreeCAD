#!/usr/bin/env bash
# Builds the Android layer of the FreeCAD Rust stack.
#
# Without OCCT_ROOT: pure-Rust probes only (freecad-android default features).
# With OCCT_ROOT:    full viewer (OCCT geometry + wgpu) via the `viewer` feature.
#
# Usage:
#   ./rust/android/build_rust.sh [arm64-v8a|x86_64]
#   OCCT_ROOT=/path/occt-install ./rust/android/build_rust.sh arm64-v8a
set -euo pipefail

log() { printf '[fc-android] %s\n' "$*"; }
die() { printf '[fc-android] ERROR: %s\n' "$*" >&2; exit 1; }

abi="${1:-arm64-v8a}"
ndk="${ANDROID_NDK_ROOT:-$HOME/Library/Android/sdk/ndk/27.2.12479018}"

case "$abi" in
  arm64-v8a)   target=aarch64-linux-android;    api=24 ;;
  armeabi-v7a) target=armv7a-linux-androideabi; api=24 ;;
  x86_64)      target=x86_64-linux-android;     api=24 ;;
  *) die "unknown abi: $abi (arm64-v8a | armeabi-v7a | x86_64)" ;;
esac

toolchain_bin="$ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin"
triplet="${target}${api}"
clang="$toolchain_bin/${triplet}-clang"
clangxx="$toolchain_bin/${triplet}-clang++"
llvm_ar="$toolchain_bin/llvm-ar"
[ -x "$clang" ] || die "NDK clang not found: $clang"
[ -x "$clangxx" ] || die "NDK clang++ not found: $clangxx"

cd "$(dirname "$0")/.."

# cc-rs picks the NDK compiler through target-scoped env vars; rustc linker is
# set the same way so no global machine state is touched.
export CC_${target//-/_}="$clang"
export CXX_${target//-/_}="$clangxx"
export AR_${target//-/_}="$llvm_ar"
# clang++ (not clang) so the NDK driver adds the static libc++ search path
# and links c++/c++abi itself — OCCT needs the C++ runtime.
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$clangxx"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$clangxx"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$clangxx"

# 16 KB page alignment is mandatory for Play distribution (Nov 2025+).
export RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"

features=""
packages="-p freecad-android"
if [ -n "${OCCT_ROOT:-}" ]; then
  [ -d "$OCCT_ROOT/lib" ] || die "OCCT_ROOT/lib not found: $OCCT_ROOT"
  log "viewer feature enabled (OCCT at $OCCT_ROOT)"
  features="--features viewer"
else
  log "OCCT_ROOT not set: building probes-only build (no geometry)"
fi

log "target=$target api=$api ndk=$(basename "$ndk")"
cargo build --release --target "$target" $features $packages

artifact="target/${target}/release/libfreecad_android.so"
[ -f "$artifact" ] || die "expected artifact missing: $artifact"
log "artifact: $artifact"
