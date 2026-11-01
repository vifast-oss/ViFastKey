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

# Process cũ vẫn giữ bundle đã bị xoá; IMK sẽ khởi động lại bản mới khi cần.
pkill -x ViFastKeyIM 2>/dev/null || true

rm -rf "$DEST"
cp -R "$APP" "$DEST"
xattr -dr com.apple.quarantine "$DEST" 2>/dev/null || true

# KHÔNG ký lại sau khi copy: ký ad-hoc lần nữa sẽ đổi CDHash, macOS coi là app
# mới và hỏi lại "Allow ViFastKey to enable ViFastKey?". Chỉ ký khi bản build
# không có chữ ký hợp lệ.
if ! codesign --verify --deep --strict "$DEST" >/dev/null 2>&1; then
  echo "Cảnh báo: chữ ký không hợp lệ; ký ad-hoc (macOS sẽ phải cấp lại quyền)."
  codesign --force --deep --sign - "$DEST" >/dev/null 2>&1 || true
fi

echo "Đã cài: $DEST"
# Chỉ đăng ký một lần. Không gọi --enable-input-source: trên macOS mới, bật
# input method bên thứ ba phải do người dùng làm trong System Settings (hộp
# thoại Allow). Gọi enable 3 lần như trước đây làm hộp thoại hiện lặp lại.
"$DEST/Contents/MacOS/ViFastKeyIM" --register-input-source >/dev/null 2>&1 || true
# Mở app sau khi đăng ký để lần chạy đầu không tự đăng ký lại.
open "$DEST" 2>/dev/null || true
echo
echo "Bước tiếp theo:"
echo "  1. Đăng xuất/đăng nhập một lần để macOS làm mới danh sách input source"
echo "  2. System Settings → Keyboard → Input Sources → chọn ViFastKey"
echo "     Nếu macOS hỏi 'Allow ViFastKey to enable ViFastKey?' → bấm Allow (một lần)."
echo "  3. Alt+, bật/tắt tiếng Việt trong app"
echo "  4. Cấu hình: $HOME/Library/Application Support/ViFastKey/config.json"
echo
echo "Nếu hộp thoại Allow hiện lại nhiều lần: xem macos/README.md mục sự cố."
echo
open "x-apple.systempreferences:com.apple.Keyboard-Settings.extension" 2>/dev/null || true
