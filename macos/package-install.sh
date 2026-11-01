#!/usr/bin/env bash
# Installer nằm trong ZIP phát hành: copy ViFastKey.app vào ~/Library/Input Methods.
# Không ký lại app — ký ad-hoc lần nữa sẽ đổi CDHash và macOS sẽ hỏi lại quyền.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
APP="$HERE/ViFastKey.app"
DEST="$HOME/Library/Input Methods/ViFastKey.app"

if [ ! -d "$APP" ]; then
  echo "Không thấy ViFastKey.app cạnh install.sh." >&2
  exit 1
fi

# Process cũ vẫn giữ bundle đã bị xoá; IMK sẽ khởi động lại bản mới khi cần.
pkill -x ViFastKeyIM 2>/dev/null || true

mkdir -p "$HOME/Library/Input Methods"
rm -rf "$DEST"
cp -R "$APP" "$DEST"
xattr -dr com.apple.quarantine "$DEST" 2>/dev/null || true

if ! codesign --verify --deep --strict "$DEST" >/dev/null 2>&1; then
  echo "Cảnh báo: chữ ký không hợp lệ; ký ad-hoc (macOS sẽ phải cấp lại quyền)."
  codesign --force --deep --sign - "$DEST" >/dev/null 2>&1 || true
fi

# Chỉ đăng ký; việc bật input source do người dùng làm trong System Settings.
"$DEST/Contents/MacOS/ViFastKeyIM" --register-input-source >/dev/null 2>&1 || true

echo "Đã cài: $DEST"
echo
echo "Bước tiếp theo:"
echo "  1. Đăng xuất rồi đăng nhập (bắt buộc một lần)."
echo "  2. System Settings → Keyboard → Text Input → Edit → + → Vietnamese → ViFastKey."
echo "  3. macOS hỏi \"Allow ViFastKey to enable ViFastKey?\" → bấm Allow (một lần)."
echo "  4. Gõ thử: Tieengs Vieejt → Tiếng Việt."
echo
echo "Không cài lại nhiều lần: mỗi lần cài lại macOS coi là app mới và hỏi lại quyền."
echo
open "x-apple.systempreferences:com.apple.Keyboard-Settings.extension" 2>/dev/null || true
