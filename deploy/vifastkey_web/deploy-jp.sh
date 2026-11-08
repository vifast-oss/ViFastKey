#!/usr/bin/env bash
# Deploy ViFastKey web lên host jp (chạy trực tiếp trên docker của jp,
# KHÔNG còn dùng LXD container như trước).
#
#   ./deploy/vifastkey_web/deploy-jp.sh
#   JP_HOST=root@jp VERSION=0.1.2 ./deploy/vifastkey_web/deploy-jp.sh
#
# Yêu cầu trên host jp:
#   - docker + docker compose plugin
#   - network ngoài tên `coolify` (do Coolify tạo) để Traefik nhận label
#   - /opt/vifastkey đã tồn tại với data/ + downloads/ (state giữ nguyên)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/rust/Cargo.toml" | head -1)}"
JP_HOST="${JP_HOST:-root@jp}"
REMOTE_APP="${REMOTE_APP:-/opt/vifastkey}"
LOCAL_STAGE="${LOCAL_STAGE:-/tmp/vifastkey-stage}"
SITE="${SITE:-https://key.vifast.org}"
STAMP="$(date +%Y%m%d-%H%M%S)"

echo "==> Build UEFI + stage web ($VERSION)"
"$ROOT/uefi/package.sh"
"$ROOT/deploy/vifastkey_web/sync-downloads.sh" "$VERSION"
"$ROOT/deploy/vifastkey_web/stage.sh" "$LOCAL_STAGE"

echo "==> Backup $REMOTE_APP trên $JP_HOST (bỏ data/downloads)"
ssh "$JP_HOST" "cd /opt && tar --exclude=vifastkey/data --exclude=vifastkey/downloads \
  -czf /tmp/vifastkey-before-$STAMP.tgz vifastkey"

echo "==> Rsync stage -> $JP_HOST:$REMOTE_APP"
rsync -a --delete --exclude data --exclude downloads --exclude .env \
  "$LOCAL_STAGE/" "$JP_HOST:$REMOTE_APP/"

echo "==> Rebuild + restart container"
ssh "$JP_HOST" "cd $REMOTE_APP && docker compose up -d --build"

echo "==> Verify $SITE"
curl -fsS "$SITE/" | grep -F 'panel-uefi' >/dev/null

TMP_ZIP="$(mktemp -t vifastkey-uefi-remote).zip"
trap 'rm -f "$TMP_ZIP"' EXIT
curl -fsSL "$SITE/downloads/$VERSION/vifastkey-uefi-x86_64.zip" -o "$TMP_ZIP"

LOCAL_ZIP="$ROOT/dist/vifastkey-uefi-x86_64.zip"
if command -v sha256sum >/dev/null 2>&1; then
  LOCAL_SHA="$(sha256sum "$LOCAL_ZIP" | awk '{print $1}')"
  REMOTE_SHA="$(sha256sum "$TMP_ZIP" | awk '{print $1}')"
else
  LOCAL_SHA="$(shasum -a 256 "$LOCAL_ZIP" | awk '{print $1}')"
  REMOTE_SHA="$(shasum -a 256 "$TMP_ZIP" | awk '{print $1}')"
fi
test "$LOCAL_SHA" = "$REMOTE_SHA"
echo "UEFI asset OK: $(wc -c < "$TMP_ZIP") bytes, sha256=$REMOTE_SHA"

echo "==> Deploy xong. Backup: /tmp/vifastkey-before-$STAMP.tgz (trên $JP_HOST)"
