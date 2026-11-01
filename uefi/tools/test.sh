#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

rustup target add x86_64-unknown-uefi >/dev/null
cargo build --release --features selftest

mkdir -p build/selftest-esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/vfk-uefi.efi \
   build/selftest-esp/EFI/BOOT/BOOTX64.EFI

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
    echo "Không tìm thấy OVMF code image (đặt OVMF_CODE=...)." >&2
    exit 1
fi
if [[ ! -f build/OVMF_VARS.fd ]]; then
    cp "$OVMF_VARS_SRC" build/OVMF_VARS.fd
fi

rm -f build/selftest.log build/selftest-qemu.out
qemu-system-x86_64 \
    -machine q35 \
    -m 256M \
    -drive if=pflash,format=raw,readonly=on,unit=0,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,unit=1,file=build/OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:build/selftest-esp \
    -display none \
    -serial none \
    -debugcon file:build/selftest.log \
    -global isa-debugcon.iobase=0xe9 \
    -no-reboot \
    > build/selftest-qemu.out 2>&1 &
qemu_pid=$!
trap 'kill "$qemu_pid" 2>/dev/null || true; wait "$qemu_pid" 2>/dev/null || true' EXIT

for _ in $(seq 1 60); do
    if grep -q "SELFTEST PASS\|SELFTEST FAIL" build/selftest.log 2>/dev/null; then
        break
    fi
    sleep 0.5
done

if ! grep -q "SELFTEST PASS" build/selftest.log 2>/dev/null; then
    echo "UEFI self-test FAILED" >&2
    cat build/selftest.log 2>/dev/null || true
    exit 1
fi

cat build/selftest.log
echo "UEFI self-test OK"
