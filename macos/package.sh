#!/usr/bin/env bash
# Đóng gói ViFastKey.app universal2 + ký/notarize tùy chọn.
#
#   ./macos/package.sh
#
# Biến môi trường cho bản phát hành:
#   SIGN_ID="Developer ID Application: ..."   # bỏ trống -> ad-hoc
#   NOTARY_PROFILE="vifastkey-notary"        # profile đã tạo bằng notarytool store-credentials
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
BUILD="$HERE/build"
APP="$BUILD/ViFastKey.app"
SIGN_ID="${SIGN_ID:-}"

echo "==> Build Rust C ABI (arm64 + x86_64)"
(cd "$ROOT/rust" && cargo build --release -p vfk-ffi --target aarch64-apple-darwin)
(cd "$ROOT/rust" && cargo build --release -p vfk-ffi --target x86_64-apple-darwin)
mkdir -p "$BUILD"
lipo -create \
  "$ROOT/rust/target/aarch64-apple-darwin/release/libvifastkey.a" \
  "$ROOT/rust/target/x86_64-apple-darwin/release/libvifastkey.a" \
  -output "$BUILD/libvifastkey-universal.a"

echo "==> Compile Swift universal2"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$HERE/Info.plist" "$APP/Contents/Info.plist"
sips -s format tiff "$ROOT/resources/brand/vifastkey-icon-192.png" --out "$APP/Contents/Resources/ViFastKey.tiff" >/dev/null
cp "$ROOT/resources/brand/vifastkey-icon-192.png" "$APP/Contents/Resources/ViFastKey.png"
printf 'APPL????' > "$APP/Contents/PkgInfo"
for arch in arm64 x86_64; do
  swiftc -O -target "$arch-apple-macos12.0" \
    -import-objc-header "$HERE/Sources/Bridging.h" \
    -framework Cocoa -framework InputMethodKit \
    "$BUILD/libvifastkey-universal.a" \
    "$HERE"/Sources/*.swift \
    -o "$BUILD/ViFastKeyIM-$arch"
done
lipo -create "$BUILD/ViFastKeyIM-arm64" "$BUILD/ViFastKeyIM-x86_64" \
  -output "$APP/Contents/MacOS/ViFastKeyIM"
rm -f "$BUILD/ViFastKeyIM-arm64" "$BUILD/ViFastKeyIM-x86_64"

if [ -n "$SIGN_ID" ]; then
  echo "==> Ký Developer ID: $SIGN_ID"
  codesign --force --deep --options runtime --timestamp --sign "$SIGN_ID" "$APP"
else
  echo "==> Ký ad-hoc (chỉ test local)"
  codesign --force --deep --sign - "$APP"
fi

ZIP="$BUILD/ViFastKey-macos-universal.zip"
ditto -c -k --keepParent "$APP" "$ZIP"
echo "==> $ZIP"

if [ -n "${NOTARY_PROFILE:-}" ] && [ -n "$SIGN_ID" ]; then
  echo "==> Notarize"
  xcrun notarytool submit "$ZIP" --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$APP"
  ditto -c -k --keepParent "$APP" "$ZIP"
  echo "==> Notarized: $ZIP"
fi
