#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

rustup target add x86_64-unknown-uefi >/dev/null
cargo build --release

mkdir -p build/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/vfk-uefi.efi \
   build/esp/EFI/BOOT/BOOTX64.EFI

echo "built: build/esp/EFI/BOOT/BOOTX64.EFI"
