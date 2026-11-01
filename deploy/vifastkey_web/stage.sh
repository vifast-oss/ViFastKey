#!/usr/bin/env bash
# Tạo build context phẳng cho Docker (Dockerfile dùng COPY python/ + static/).
# Chạy từ repo root: ./deploy/vifastkey_web/stage.sh [/duong/dan/dich]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:-/private/tmp/vifastkey-stage}"

rm -rf "$OUT"
mkdir -p "$OUT"
cp "$ROOT/deploy/vifastkey_web/app.py" "$OUT/app.py"
cp "$ROOT/deploy/vifastkey_web/requirements.txt" "$OUT/requirements.txt"
cp "$ROOT/deploy/vifastkey_web/Dockerfile" "$OUT/Dockerfile"
cp "$ROOT/deploy/vifastkey_web/docker-compose.yml" "$OUT/docker-compose.yml"
cp "$ROOT/deploy/vifastkey_web/.dockerignore" "$OUT/.dockerignore"

rsync -a --exclude '__pycache__' --exclude '.pytest_cache' \
  --exclude '.ruff_cache' --exclude '*.egg-info' --exclude '._*' \
  "$ROOT/python/" "$OUT/python/"
rsync -a --exclude '.DS_Store' --exclude '._*' \
  "$ROOT/web/" "$OUT/static/"

if [ -d "$ROOT/deploy/vifastkey_web/downloads" ]; then
  rsync -a --exclude '.gitignore' --exclude '.DS_Store' --exclude '._*' \
    "$ROOT/deploy/vifastkey_web/downloads/" "$OUT/downloads/"
fi

echo "Staging: $OUT"
du -sh "$OUT"
