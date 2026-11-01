#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

rustup target add x86_64-unknown-uefi >/dev/null
cargo build --release
mkdir -p build/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/vfk-uefi.efi \
   build/esp/EFI/BOOT/BOOTX64.EFI

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

rm -f build/ui-monitor.sock build/ui.ppm build/ui-qemu.out
qemu-system-x86_64 \
    -machine q35 \
    -m 256M \
    -drive if=pflash,format=raw,readonly=on,unit=0,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,unit=1,file=build/OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:build/esp \
    -display none \
    -serial none \
    -monitor unix:build/ui-monitor.sock,server=on,wait=off \
    -no-reboot \
    > build/ui-qemu.out 2>&1 &
qemu_pid=$!
trap 'kill "$qemu_pid" 2>/dev/null || true; wait "$qemu_pid" 2>/dev/null || true' EXIT

# OVMF + TCG on Apple Silicon / CI can be slow; poll until the app paints.
for _ in $(seq 1 40); do
    if [[ -S build/ui-monitor.sock ]]; then
        break
    fi
    sleep 0.5
done
if ! python3 tools/wait-ui.py build/ui-monitor.sock build/ui-probe.ppm 60; then
    echo "UEFI UI chưa boot kịp" >&2
    tail -20 build/ui-qemu.out >&2 || true
    exit 1
fi

python3 tools/sendkeys.py build/ui-monitor.sock \
    --shot build/ui.ppm \
    shift-t i e e n g s spc shift-v i e e j t spc

python3 - <<'PY'
from pathlib import Path

path = Path("build/ui.ppm")
data = path.read_bytes()
assert data[:2] == b"P6", "không phải PPM P6"

pos = 2
values = []
while len(values) < 3:
    while data[pos:pos + 1] in b" \t\r\n":
        pos += 1
    if data[pos:pos + 1] == b"#":
        while data[pos:pos + 1] not in b"\r\n":
            pos += 1
        continue
    start = pos
    while data[pos:pos + 1] not in b" \t\r\n":
        pos += 1
    values.append(int(data[start:pos]))
pos += 1

width, height, _max = values
pixels = data[pos:]
assert len(pixels) >= width * height * 3, "PPM thiếu pixel"

center = ((height // 2) * width + width // 2) * 3
r, g, b = pixels[center:center + 3]
assert r < 70 and g < 70 and b < 70, f"màn hình không phải app (center={r},{g},{b})"

colors = set()
for i in range(0, len(pixels) - 2, 3 * 97):
    colors.add(pixels[i:i + 3])
assert len(colors) > 12, f"quá ít màu ({len(colors)}), có thể app chưa render"

print(f"UI OK: {width}x{height}, {len(colors)} màu mẫu")
PY

echo "UEFI UI smoke test OK"
