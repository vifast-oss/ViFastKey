#!/usr/bin/env bash
# Đóng gói UEFI demo thành ZIP sẵn sàng giải nén vào USB FAT32.
#   ./uefi/package.sh
#   OUT_DIR=/tmp ./uefi/package.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/dist}"

cd "$ROOT/uefi"
rustup target add x86_64-unknown-uefi >/dev/null
cargo build --release

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/EFI/BOOT" "$OUT_DIR"

cp target/x86_64-unknown-uefi/release/vfk-uefi.efi \
   "$STAGE/EFI/BOOT/BOOTX64.EFI"
cp INSTALL.txt "$STAGE/INSTALL.txt"
cp README.md "$STAGE/README.md"
mkdir -p "$STAGE/docs"
cp docs/*.png "$STAGE/docs/"
cp "$ROOT/LICENSE" "$STAGE/LICENSE"
cp "$ROOT/THIRD_PARTY_NOTICES.md" "$STAGE/THIRD_PARTY_NOTICES.md"

ZIP="$OUT_DIR/vifastkey-uefi-x86_64.zip"
rm -f "$ZIP"
(
    cd "$STAGE"
    zip -9 -r "$ZIP" .
) >/dev/null

echo "Đã đóng gói: $ZIP"
