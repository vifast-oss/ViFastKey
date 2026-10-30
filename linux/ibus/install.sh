#!/usr/bin/env bash
# Cài IBus engine cho user hiện tại (mặc định ~/.local).
#
# IBus 1.5.29 trên Ubuntu chỉ quét /usr/share/ibus/component, nên component XML
# được cài system-wide; mã engine vẫn nằm trong PREFIX của user. Có thể override
# bằng IBUS_COMPONENT_DIR nếu bản IBus của bạn hỗ trợ đường dẫn khác.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
COMP="${IBUS_COMPONENT_DIR:-/usr/share/ibus/component}"
DEST="$PREFIX/share/vifastkey/ibus"

mkdir -p "$DEST"
install -m 0755 "$HERE/ibus_engine.py" "$DEST/ibus_engine.py"
python3 -m py_compile "$DEST/ibus_engine.py"

XML_TMP="$(mktemp)"
trap 'rm -f "$XML_TMP"' EXIT
sed "s|@PREFIX@|$PREFIX|g" "$HERE/vifastkey.xml.in" > "$XML_TMP"

if [ "$(id -u)" -eq 0 ] || [ -w "$COMP" ]; then
    mkdir -p "$COMP"
    install -m 0644 "$XML_TMP" "$COMP/vifastkey.xml"
else
    sudo mkdir -p "$COMP"
    sudo install -m 0644 "$XML_TMP" "$COMP/vifastkey.xml"
fi

# Icon for the IBus panel / input source picker.
ICON_DIR="$PREFIX/share/icons/hicolor/scalable/apps"
ICON_SRC="$HERE/../../resources/brand/vifastkey-icon.svg"
if [ -f "$ICON_SRC" ]; then
    mkdir -p "$ICON_DIR"
    install -m 0644 "$ICON_SRC" "$ICON_DIR/vifastkey.svg"
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
    fi
fi

if command -v ibus >/dev/null 2>&1; then
    if [ "$(id -u)" -eq 0 ] || [ -w "$COMP" ]; then
        ibus write-cache --system >/dev/null 2>&1 || true
    else
        sudo ibus write-cache --system >/dev/null 2>&1 || true
    fi
    ibus write-cache >/dev/null 2>&1 || true
    ibus restart >/dev/null 2>&1 || true
fi

echo "Đã cài engine: $DEST/ibus_engine.py"
echo "Đã cài component: $COMP/vifastkey.xml"
echo "Mở IBus Preferences → Input Method → Vietnamese → ViFastKey."
