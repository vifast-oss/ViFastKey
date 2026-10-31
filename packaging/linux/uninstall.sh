#!/usr/bin/env bash
# Gỡ ViFastKey Settings khỏi user hiện tại; giữ config/macro.
set -euo pipefail

BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_DIR/applications"
ICON_ROOT="$DATA_DIR/icons/hicolor"

rm -f "$BIN_DIR/vifastkey-settings"
rm -f "$APPS_DIR/vifastkey-settings.desktop"
rm -f "$ICON_ROOT/192x192/apps/vifastkey.png"
rm -f "$ICON_ROOT/scalable/apps/vifastkey.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$ICON_ROOT" >/dev/null 2>&1 || true
fi

echo "Đã gỡ UI ViFastKey Settings. Config người dùng vẫn được giữ."
