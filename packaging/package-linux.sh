#!/usr/bin/env bash
# Đóng gói vifastkey-settings cho Linux x86_64.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${1:-$ROOT/dist}"
VERSION="${VERSION:-0.1.2}"
BIN="${VIFASTKEY_SETTINGS_BIN:-$ROOT/rust/target/release/vifastkey-settings}"

if [ ! -x "$BIN" ]; then
  echo "Không thấy binary: $BIN (chạy cargo build --release -p vifastkey-settings)" >&2
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
pkg="$tmp/vifastkey-settings-linux-x86_64-$VERSION"
mkdir -p "$pkg" "$OUT_DIR"

install -m 0755 "$BIN" "$pkg/vifastkey-settings"
install -m 0755 "$ROOT/packaging/linux/install.sh" "$pkg/install.sh"
install -m 0755 "$ROOT/packaging/linux/uninstall.sh" "$pkg/uninstall.sh"
install -m 0644 "$ROOT/packaging/linux/README.md" "$pkg/README.md"
install -m 0644 "$ROOT/rust/apps/vfk-settings/README.md" "$pkg/DEVELOPER.md"
install -m 0644 "$ROOT/linux/vifastkey-settings.desktop" "$pkg/vifastkey-settings.desktop"
install -m 0644 "$ROOT/resources/brand/vifastkey-icon-192.png" "$pkg/vifastkey.png"
install -m 0644 "$ROOT/resources/brand/vifastkey-icon.svg" "$pkg/vifastkey.svg"
install -m 0644 "$ROOT/LICENSE-MIT" "$pkg/LICENSE-MIT"
install -m 0644 "$ROOT/LICENSE-APACHE" "$pkg/LICENSE-APACHE"
install -m 0644 "$ROOT/THIRD_PARTY_NOTICES.md" "$pkg/THIRD_PARTY_NOTICES.md"

archive="$OUT_DIR/vifastkey-settings-linux-x86_64-$VERSION.tar.gz"
tar -czf "$archive" -C "$tmp" "vifastkey-settings-linux-x86_64-$VERSION"
echo "Đã tạo $archive"
ls -lh "$archive"
