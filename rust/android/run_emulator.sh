#!/usr/bin/env bash
# M2.1 — Android Runtime Proof on a headless ARM64 emulator (Apple Silicon HVF).
#
# Boots the bundled-model viewer APK without any window, then captures:
#   - nativeInit timing (OCCT import + tessellation + GPU init) from logcat
#   - frame stats from dumpsys gfxinfo
#   - a real screenshot pulled from the device
#
# Prerequisites:
#   sdkmanager "emulator" "system-images;android-34;google_apis;arm64-v8a"
#   APK built: ./rust/android/build_apk.sh
set -euo pipefail

log() { printf '[m21] %s\n' "$*"; }
die() { printf '[m21] ERROR: %s\n' "$*" >&2; exit 1; }

sdk="${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}"
emu_bin="$sdk/emulator/emulator"
adb="$sdk/platform-tools/adb"
avd_name="${AVD_NAME:-fc-arm64}"
apk="$sdk/../../ai-workstation/Projects/FreeCAD/rust/android/app/build/freecad-viewer-debug.apk"
[ -f "$apk" ] || apk="$(dirname "$0")/app/build/freecad-viewer-debug.apk"
[ -f "$apk" ] || die "APK not found (run build_apk.sh first)"
[ -x "$emu_bin" ] || die "emulator not installed"

log "creating AVD '$avd_name' (idempotent)"
echo no | "$sdk/cmdline-tools/latest/bin/avdmanager" create avd \
  -n "$avd_name" \
  -k "system-images;android-34;google_apis;arm64-v8a" \
  -d pixel_6 >/dev/null 2>&1 || true

log "booting headless emulator (swiftshader_indirect GPU)"
"$emu_bin" -avd "$avd_name" \
  -no-window -no-audio -no-boot-anim \
  -gpu swiftshader_indirect \
  -memory 4096 -cores 4 \
  > /tmp/emulator.log 2>&1 &
emu_pid=$!
trap 'kill $emu_pid 2>/dev/null || true' EXIT

log "waiting for device (up to 300 s)"
"$adb" wait-for-device
deadline=$((SECONDS + 300))
until [ "$("$adb" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; do
  [ $SECONDS -gt $deadline ] && die "emulator boot timeout"
  sleep 5
done
log "boot completed"

"$adb" uninstall com.freecad.viewer >/dev/null 2>&1 || true
log "installing $(basename "$apk")"
"$adb" install -r "$apk" | tail -1

log "launching MainActivity"
"$adb" logcat -c
"$adb" shell am start -W -n com.freecad.viewer/.MainActivity | grep -E "Status|TotalTime" || true
sleep 8

log "--- FreeCAD logcat (init timing) ---"
"$adb" logcat -d -s FreeCAD | tail -20

log "--- frame stats ---"
"$adb" shell dumpsys gfxinfo com.freecad.viewer | grep -E "Total frames|Janky frames|Frame time" | head -5 || true

shot="/tmp/freecad-android-first-frame.png"
"$adb" exec-out screencap -p > "$shot"
log "screenshot: $shot ($(du -h "$shot" | cut -f1))"

log "--- package / abi / gpu info ---"
"$adb" shell getprop ro.build.version.release
"$adb" shell getprop ro.product.cpu.abi
"$adb" shell getprop ro.hardware.vulkan 2>/dev/null || true
"$adb" shell dumpsys SurfaceFlinger | grep -iE "GLES:|Vulkan" | head -2 || true

log "DONE — runtime proof artifacts captured"
