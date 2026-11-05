#!/usr/bin/env bash
# Deploy ViFastKey web lên LXD container trên host jp.
#   ./deploy/vifastkey_web/deploy-jp.sh
#   JP_HOST=root@jp CONTAINER=hoctuthien-vps VERSION=0.1.2 ./deploy/...
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/rust/Cargo.toml" | head -1)}"
JP_HOST="${JP_HOST:-root@jp}"
CONTAINER="${CONTAINER:-hoctuthien-vps}"
REMOTE_APP="${REMOTE_APP:-/opt/vifastkey}"
REMOTE_STAGE="${REMOTE_STAGE:-/tmp/vifastkey-stage}"
LOCAL_STAGE="${LOCAL_STAGE:-/private/tmp/vifastkey-stage}"
SITE="${SITE:-https://vifastkey.hoctuthien.com}"
STAMP="$(date +%Y%m%d-%H%M%S)"

echo "==> Build UEFI + stage web ($VERSION)"
"$ROOT/uefi/package.sh"
"$ROOT/deploy/vifastkey_web/sync-downloads.sh" "$VERSION"
"$ROOT/deploy/vifastkey_web/stage.sh" "$LOCAL_STAGE"

echo "==> Backup app files trong container (bỏ data/downloads)"
ssh "$JP_HOST" "lxc exec $CONTAINER -- bash -lc 'cd /opt && tar --exclude=vifastkey/data --exclude=vifastkey/downloads -czf /tmp/vifastkey-before-$STAMP.tgz vifastkey'"

echo "==> Rsync stage sang $JP_HOST:$REMOTE_STAGE"
rsync -a --delete "$LOCAL_STAGE/" "$JP_HOST:$REMOTE_STAGE/"

echo "==> Giải nén vào $REMOTE_APP (giữ data/.env)"
ssh "$JP_HOST" "lxc exec $CONTAINER -- mkdir -p $REMOTE_APP"
ssh "$JP_HOST" "tar -C $REMOTE_STAGE -czf - . | lxc exec $CONTAINER -- tar -xzf - -C $REMOTE_APP"
# Container chạy UID 10001; asset tải phải đọc được.
ssh "$JP_HOST" "lxc exec $CONTAINER -- chmod -R a+rX $REMOTE_APP/downloads"

echo "==> Build + restart container"
ssh "$JP_HOST" "lxc exec $CONTAINER -- bash -lc 'cd $REMOTE_APP && docker compose up -d --build'"

echo "==> Chờ healthcheck"
for _ in $(seq 1 30); do
    if curl -fsS "$SITE/healthz" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

echo "==> Kiểm tra nội dung + asset UEFI"
curl -fsS "$SITE/healthz"
echo
curl -fsS "$SITE/" | grep -F 'vifastkey-uefi-x86_64.zip' >/dev/null
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

echo "==> Deploy xong. Backup: /tmp/vifastkey-before-$STAMP.tgz (trong container)"
