#!/usr/bin/env bash
# Dựng WASM + đồng bộ asset thương hiệu cho web demo.
# Chạy từ repo root: ./web/build.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/rust"

cargo build --release -p vfk-wasm --target wasm32-unknown-unknown
mkdir -p "$ROOT/web/pkg"
wasm-bindgen --target web \
  --out-dir "$ROOT/web/pkg" \
  --out-name vfk_wasm \
  target/wasm32-unknown-unknown/release/vfk_wasm.wasm

cd "$ROOT"
mkdir -p web/brand web/fonts
cp resources/brand/vifastkey-icon-192.png \
   resources/brand/vifastkey-icon-512.png \
   resources/brand/vifastkey-lockup.png \
   resources/brand/vifastkey-lockup-cream.png \
   resources/brand/favicon.ico \
   resources/brand/favicon-16.png \
   resources/brand/favicon-32.png \
   resources/brand/favicon-48.png \
   resources/brand/favicon-128.png \
   resources/brand/apple-touch-icon.png \
   resources/brand/og-image.png \
   resources/brand/site.webmanifest \
   web/brand/
cp resources/fonts/BeVietnamPro-Regular.ttf resources/fonts/BeVietnamPro-OFL.txt web/fonts/

echo "Xong. Chạy: python3 -m http.server 8080 -d web"
echo "Mở:   http://127.0.0.1:8080/"
