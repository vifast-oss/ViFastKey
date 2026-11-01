#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
./tools/build.sh

# Homebrew QEMU on macOS ships the x86_64 code image. It has no separate
# x86_64 vars image, but the i386 vars template is compatible in practice.
OVMF_CODE="${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}"
OVMF_VARS_SRC="${OVMF_VARS_SRC:-/opt/homebrew/share/qemu/edk2-i386-vars.fd}"

if [[ ! -f "$OVMF_CODE" ]]; then
    for candidate in \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/edk2/x64/OVMF_CODE.fd \
        /usr/share/qemu/OVMF_CODE.fd
    do
        if [[ -f "$candidate" ]]; then
            OVMF_CODE="$candidate"
            break
        fi
    done
fi

if [[ ! -f "$OVMF_CODE" ]]; then
    echo "Không tìm thấy OVMF code image. Đặt OVMF_CODE=/đường/dẫn/OVMF_CODE.fd." >&2
    exit 1
fi

mkdir -p build
if [[ ! -f build/OVMF_VARS.fd ]]; then
    if [[ ! -f "$OVMF_VARS_SRC" ]]; then
        echo "Không tìm thấy OVMF vars template. Đặt OVMF_VARS_SRC=... ." >&2
        exit 1
    fi
    cp "$OVMF_VARS_SRC" build/OVMF_VARS.fd
fi

exec qemu-system-x86_64 \
    -machine q35 \
    -m 256M \
    -drive if=pflash,format=raw,readonly=on,unit=0,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,unit=1,file=build/OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:build/esp \
    -serial stdio \
    -no-reboot \
    "$@"
