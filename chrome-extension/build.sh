#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
"$ROOT/web/build.sh"
rm -rf "$HERE/dist"
mkdir -p "$HERE/dist"
cp "$HERE"/src/manifest.json "$HERE"/src/background.js "$HERE"/src/popup.html \
   "$HERE"/src/popup.js "$HERE"/src/popup.css "$HERE/dist/"
cp -R "$ROOT/web/pkg" "$HERE/dist/pkg"
echo "Extension: $HERE/dist"
