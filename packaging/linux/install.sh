#!/usr/bin/env bash
# Cài ViFastKey Settings cho user hiện tại (không cần root).
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_DIR/applications"
ICON_ROOT="$DATA_DIR/icons/hicolor"

if [ ! -f "$HERE/vifastkey-settings" ]; then
  echo "Không thấy vifastkey-settings trong $HERE" >&2
  exit 1
fi

mkdir -p "$BIN_DIR" "$APPS_DIR" "$ICON_ROOT/192x192/apps" "$ICON_ROOT/scalable/apps"
install -m 0755 "$HERE/vifastkey-settings" "$BIN_DIR/vifastkey-settings"

# Dùng đường dẫn tuyệt đối để desktop entry không phụ thuộc PATH của session.
sed "s|^Exec=.*|Exec=$BIN_DIR/vifastkey-settings|" \
  "$HERE/vifastkey-settings.desktop" > "$APPS_DIR/vifastkey-settings.desktop"
chmod 0644 "$APPS_DIR/vifastkey-settings.desktop"

if [ -f "$HERE/vifastkey.png" ]; then
  install -m 0644 "$HERE/vifastkey.png" "$ICON_ROOT/192x192/apps/vifastkey.png"
fi
if [ -f "$HERE/vifastkey.svg" ]; then
  install -m 0644 "$HERE/vifastkey.svg" "$ICON_ROOT/scalable/apps/vifastkey.svg"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$ICON_ROOT" >/dev/null 2>&1 || true
fi

echo "Đã cài: $BIN_DIR/vifastkey-settings"
echo "Desktop entry: $APPS_DIR/vifastkey-settings.desktop"
echo "Mở bằng app menu hoặc: $BIN_DIR/vifastkey-settings"
