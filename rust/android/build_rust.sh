#!/usr/bin/env bash
# Builds the pure-Rust Android layer (freecad-android cdylib) for a given ABI.
#
# Usage:
#   ANDROID_NDK_ROOT=~/Library/Android/sdk/ndk/27.2.12479018 \
#       ./rust/android/build_rust.sh arm64-v8a
set -euo pipefail

abi="${1:-arm64-v8a}"
ndk="${ANDROID_NDK_ROOT:-$HOME/Library/Android/sdk/ndk/27.2.12479018}"

case "$abi" in
  arm64-v8a)     target=aarch64-linux-android;  api=24 ;;
  armeabi-v7a)   target=armv7a-linux-androideabi; api=24 ;;
  x86_64)        target=x86_64-linux-android;   api=24 ;;
  *) echo "unknown abi: $abi" >&2; exit 1 ;;
esac

triplet="${target}${api}"
linker="$ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin/${triplet}-clang"
[ -x "$linker" ] || { echo "NDK clang not found at $linker" >&2; exit 1; }

cd "$(dirname "$0")/.."

# 16 KB page alignment is mandatory for Play distribution (see
# docs/architecture/RUST_ANDROID_MIGRATION.md risk register).
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$linker"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$linker"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$linker"
export RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"

cargo build --release --target "$target" -p freecad-android

echo
echo "artifact: target/${target}/release/libfreecad_android.so"
