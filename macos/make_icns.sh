#!/usr/bin/env bash
# Tạo ViFastKey.icns từ icon 1024px. Dùng: ./make_icns.sh <out.icns>
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
SRC="$ROOT/resources/brand/vifastkey-icon-1024.png"
OUT="${1:-$HERE/build/ViFastKey.icns}"

if [ ! -f "$SRC" ]; then
  echo "Không thấy $SRC; chạy resources/brand/build_assets.py trước." >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
ICONSET="$TMP/ViFastKey.iconset"
mkdir -p "$ICONSET"

for spec in "16 icon_16x16.png" "32 icon_16x16@2x.png" "32 icon_32x32.png" \
            "64 icon_32x32@2x.png" "128 icon_128x128.png" "256 icon_128x128@2x.png" \
            "256 icon_256x256.png" "512 icon_256x256@2x.png" "512 icon_512x512.png" \
            "1024 icon_512x512@2x.png"; do
  set -- $spec
  sips -z "$1" "$1" "$SRC" --out "$ICONSET/$2" >/dev/null
done

mkdir -p "$(dirname "$OUT")"
iconutil -c icns "$ICONSET" -o "$OUT"
echo "Đã tạo $OUT"
