#!/usr/bin/env bash
# Deterministic APK assembly for the FreeCAD Rust Android viewer.
#
# No Gradle, no network: javac + d8 + aapt2 + zip + zipalign + apksigner from
# the installed SDK. Java (not Kotlin) is used because no kotlinc exists in
# the reference environment; the activity is a thin shell by design.
#
# Prerequisites:
#   ANDROID_SDK_ROOT (default ~/Library/Android/sdk) with:
#     platforms/android-34, build-tools/35.0.0, platform-tools
#   JDK 17+ (javac/keytool)
#   The Rust .so built beforehand:
#     OCCT_ROOT=... ./rust/android/build_rust.sh arm64-v8a
#
# Output: rust/android/app/build/freecad-viewer-debug.apk
set -euo pipefail

log() { printf '[apk] %s\n' "$*"; }
die() { printf '[apk] ERROR: %s\n' "$*" >&2; exit 1; }

sdk="${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}"
ndk="${ANDROID_NDK_ROOT:-$HOME/Library/Android/sdk/ndk/27.2.12479018}"
bt="$sdk/build-tools/35.0.0"
platform="$sdk/platforms/android-34/android.jar"
abi="${ABI:-arm64-v8a}"
target_triplet=aarch64-linux-android

[ -x "$bt/aapt2" ] || die "aapt2 not found: $bt/aapt2"
[ -f "$platform" ] || die "android.jar not found: $platform"
[ -x "$sdk/build-tools/35.0.0/zipalign" ] || die "zipalign not found"

app_dir="$(cd "$(dirname "$0")/app" && pwd)"
rust_dir="$(cd "$app_dir/../.." && pwd)"
so_src="$rust_dir/target/${target_triplet}/release/libfreecad_android.so"
[ -f "$so_src" ] || die "missing $so_src — run: OCCT_ROOT=<install> ./rust/android/build_rust.sh $abi"

build="$app_dir/build"
rm -rf "$build"
mkdir -p "$build/classes" "$build/dex" "$build/stage/lib/$abi" "$build/stage/assets"

log "1/6 compile Java"
javac --release 11 -cp "$platform" -d "$build/classes" \
  "$app_dir/java/com/freecad/viewer/MainActivity.java"

log "2/6 dex"
"$bt/d8" --release --lib "$platform" --min-api 24 \
  --output "$build/dex" $(find "$build/classes" -name '*.class')

log "3/6 stage payload"
cp "$build/dex/classes.dex" "$build/stage/"
cp "$app_dir/assets/demo_part.step" "$build/stage/assets/"
cp "$so_src" "$build/stage/lib/$abi/"

readelf="$ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-readelf"
if "$readelf" -d "$so_src" | grep -q "libc++_shared"; then
  stl_src="$ndk/toolchains/llvm/prebuilt/darwin-x86_64/sysroot/usr/lib/aarch64-v8a/libc++_shared.so"
  [ -f "$stl_src" ] || stl_src="$ndk/toolchains/llvm/prebuilt/darwin-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so"
  [ -f "$stl_src" ] || die "libc++_shared.so needed but not found in NDK sysroot"
  cp "$stl_src" "$build/stage/lib/$abi/"
  log "bundled libc++_shared.so (dynamic STL detected)"
else
  log ".so is self-contained (static libc++)"
fi

log "4/6 aapt2 link (manifest + assets, no res)"
(cd "$build/stage" && zip -q -rX unsigned.apk classes.dex assets)
(cd "$build/stage" && zip -q -rX -0 unsigned.apk lib)
"$bt/aapt2" link \
  -I "$platform" \
  --manifest "$app_dir/AndroidManifest.xml" \
  -A "$build/stage/assets" \
  --min-sdk-version 24 --target-sdk-version 34 \
  -o "$build/manifest.apk"

# Merge: manifest.apk provides the compiled manifest; payload comes from stage.
merged="$build/stage/unsigned.apk"
python3 - "$build/manifest.apk" "$merged" <<'PY'
import sys, zipfile
manifest_apk, payload_apk = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(manifest_apk) as src:
    names = src.namelist()
    data = {n: src.read(n) for n in names}
with zipfile.ZipFile(payload_apk, 'a', zipfile.ZIP_DEFLATED) as dst:
    for n, blob in data.items():
        if n in ('AndroidManifest.xml',):
            dst.writestr(n, blob)
PY

log "5/6 zipalign (4-byte entries, 16KB page alignment for native libs)"
"$bt/zipalign" -f -P 16 4 "$merged" "$build/aligned.apk"

log "6/6 sign (local debug keystore, generated if absent)"
keystore="$app_dir/debug.keystore"
if [ ! -f "$keystore" ]; then
  keytool -genkeypair -keystore "$keystore" -storepass freecadrust \
    -keypass freecadrust -alias freecad -keyalg RSA -keysize 2048 \
    -validity 10000 -dname "CN=FreeCAD Rust Debug,O=FreeCAD,C=RU" >/dev/null 2>&1
fi
"$bt/apksigner" sign --ks "$keystore" --ks-pass pass:freecadrust \
  --out "$build/freecad-viewer-debug.apk" "$build/aligned.apk"
"$bt/apksigner" verify "$build/freecad-viewer-debug.apk"

size=$(du -h "$build/freecad-viewer-debug.apk" | cut -f1)
log "OK: $build/freecad-viewer-debug.apk ($size)"
log "install: $sdk/platform-tools/adb install -r $build/freecad-viewer-debug.apk"
