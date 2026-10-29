#!/usr/bin/env bash
# Cài input method vào ~/Library/Input Methods và mở System Settings.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
APP="$HERE/build/ViFastKey.app"
DEST="$HOME/Library/Input Methods/ViFastKey.app"

if [ ! -d "$APP" ]; then
  "$HERE/build.sh"
fi

mkdir -p "$HOME/Library/Input Methods"
rm -rf "$DEST"
cp -R "$APP" "$DEST"
xattr -dr com.apple.quarantine "$DEST" 2>/dev/null || true
codesign --force --deep --sign - "$DEST" >/dev/null 2>&1 || true

echo "Đã cài: $DEST"
open "$DEST" 2>/dev/null || true
sleep 1
"$DEST/Contents/MacOS/ViFastKeyIM" --register-input-source >/dev/null 2>&1 || true
"$DEST/Contents/MacOS/ViFastKeyIM" --enable-input-source >/dev/null 2>&1 || true
echo
echo "Bước tiếp theo:"
echo "  1. Mở System Settings → Keyboard → Input Sources → Edit… → +"
echo "  2. Chọn Vietnamese → ViFastKey (hoặc tìm 'ViFastKey')"
echo "  3. Đăng xuất/đăng nhập nếu chưa thấy; sau đó chọn input source ViFastKey"
echo "  4. Cấu hình: $HOME/Library/Application Support/ViFastKey/config.json"
echo
open "x-apple.systempreferences:com.apple.Keyboard-Settings.extension" 2>/dev/null || true
