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

# Ký app. Ad-hoc đủ để chạy local nhưng CDHash đổi sau mỗi lần build, nên macOS
# coi mỗi bản là app mới và hỏi lại "Allow ViFastKey to enable ViFastKey?".
# Nếu máy có Apple Development/Developer ID, tự dùng identity đó để quyền được
# giữ nguyên qua các lần build. Chỉ định thủ công:
#   VFK_CODESIGN_IDENTITY="Apple Development: ..." ./macos/build.sh
# Bản phát hành vẫn dùng package.sh (Developer ID + hardened runtime + notarize).
SIGN_ID="${VFK_CODESIGN_IDENTITY:-}"
if [ -z "$SIGN_ID" ]; then
  SIGN_ID="$(security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(.*\)"/\1/p' \
    | grep -E '^(Apple Development|Developer ID Application|Mac Developer)' \
    | head -1 || true)"
fi
if [ -n "$SIGN_ID" ]; then
  echo "==> Ký với identity: $SIGN_ID"
  codesign --force --deep --sign "$SIGN_ID" "$APP"
else
  echo "==> Ký ad-hoc (không thấy identity ký code; macOS sẽ hỏi lại quyền sau mỗi build)"
  codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
fi

echo "==> Self-test adapter (headless)"
swiftc -O -import-objc-header "$HERE/Sources/Bridging.h" -framework Foundation \
  "$ROOT/rust/target/release/libvifastkey.a" \
  "$HERE/Tests/main.swift" "$HERE/Sources/VfkBridge.swift" "$HERE/Sources/ActionApplier.swift" "$HERE/Sources/KeyMapper.swift" \
  -o "$HERE/build/vifastkey-selftest"
"$HERE/build/vifastkey-selftest"

echo "Xong: $APP"
echo "Cài:  $HERE/install.sh"
echo "Test: $HERE/doctor.sh"
