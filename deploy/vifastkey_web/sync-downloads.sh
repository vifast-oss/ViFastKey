#!/usr/bin/env bash
# Copy release assets from dist/ into the downloads mirror used by Docker.
#   ./deploy/vifastkey_web/sync-downloads.sh 0.1.1
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${1:-0.1.1}"
SRC="${DIST_DIR:-$ROOT/dist}"
DEST="$ROOT/deploy/vifastkey_web/downloads/$VERSION"

if [[ ! -d "$SRC" ]]; then
    echo "Không thấy $SRC; chạy ./uefi/package.sh và các script đóng gói trước." >&2
    exit 1
fi

mkdir -p "$DEST"
find "$SRC" -maxdepth 1 -type f ! -name 'sha256sums.txt' -print0 \
    | while IFS= read -r -d '' file; do
        cp -f "$file" "$DEST/"
    done

# ZIP Windows có thể được tạo với mode 600; container chạy UID 10001
# nên phải cho phép đọc trước khi mount/download.
find "$DEST" -maxdepth 1 -type f -exec chmod 644 {} +

rm -f "$DEST/sha256sums.txt"
(
    cd "$DEST"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -- * > sha256sums.txt
    else
        shasum -a 256 -- * > sha256sums.txt
    fi
)
chmod 644 "$DEST/sha256sums.txt"

echo "Đã đồng bộ: $DEST"
ls -lh "$DEST" | tail -20
