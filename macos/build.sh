#!/usr/bin/env bash
# Build ViFastKey.app (macOS IMK input method). Chạy từ repo root hoặc macos/.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
APP="$HERE/build/ViFastKey.app"
ARCH="$(uname -m)"
TARGET="$ARCH-apple-macos12.0"

echo "==> Build Rust C ABI (release)"
(cd "$ROOT/rust" && cargo build --release -p vfk-ffi)

echo "==> Compile Swift IMK app ($ARCH)"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$HERE/Info.plist" "$APP/Contents/Info.plist"
sips -s format tiff "$ROOT/resources/brand/vifastkey-icon-192.png" --out "$APP/Contents/Resources/ViFastKey.tiff" >/dev/null
cp "$ROOT/resources/brand/vifastkey-icon-192.png" "$APP/Contents/Resources/ViFastKey.png"
"$HERE/make_icns.sh" "$APP/Contents/Resources/ViFastKey.icns"
printf 'APPL????' > "$APP/Contents/PkgInfo"

swiftc -O -target "$TARGET" \
  -import-objc-header "$HERE/Sources/Bridging.h" \
  -framework Cocoa -framework InputMethodKit \
  "$ROOT/rust/target/release/libvifastkey.a" \
  "$HERE"/Sources/*.swift \
  -o "$APP/Contents/MacOS/ViFastKeyIM"

# Ad-hoc signing đủ để chạy local. Muốn TCC ổn định khi phát hành:
#   codesign --force --deep --options runtime --sign "Developer ID Application: ..."
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true

echo "==> Self-test adapter (headless)"
swiftc -O -import-objc-header "$HERE/Sources/Bridging.h" -framework Foundation \
  "$ROOT/rust/target/release/libvifastkey.a" \
  "$HERE/Tests/main.swift" "$HERE/Sources/VfkBridge.swift" "$HERE/Sources/ActionApplier.swift" "$HERE/Sources/KeyMapper.swift" \
  -o "$HERE/build/vifastkey-selftest"
"$HERE/build/vifastkey-selftest"

echo "Xong: $APP"
echo "Cài:  $HERE/install.sh"
echo "Test: $HERE/doctor.sh"
