#!/usr/bin/env bash
# Cài IBus engine cho user hiện tại (mặc định ~/.local).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
DEST="$PREFIX/share/vifastkey/ibus"
COMP="$PREFIX/share/ibus/component"
mkdir -p "$DEST" "$COMP"
install -m 0755 "$HERE/ibus_engine.py" "$DEST/ibus_engine.py"
sed "s|@PREFIX@|$PREFIX|g" "$HERE/vifastkey.xml.in" > "$COMP/vifastkey.xml"
python3 -m py_compile "$DEST/ibus_engine.py"
if command -v ibus >/dev/null 2>&1; then
  ibus write-cache || true
  ibus restart || true
fi
echo "Đã cài: $COMP/vifastkey.xml"
echo "Mở IBus Preferences → Input Method → Vietnamese → ViFastKey."
