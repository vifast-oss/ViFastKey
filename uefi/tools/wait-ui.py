#!/usr/bin/env python3
"""Wait until the UEFI app (dark background) is visible in QEMU.

Usage: python3 tools/wait-ui.py <monitor.sock> <probe.ppm> [timeout_seconds]
"""

import socket
import sys
import time
from pathlib import Path


def center_rgb(path):
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if data[:2] != b"P6":
        return None
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
    center = ((height // 2) * width + width // 2) * 3
    return tuple(data[pos + center:pos + center + 3])


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    monitor, probe = sys.argv[1], Path(sys.argv[2])
    timeout = float(sys.argv[3]) if len(sys.argv) > 3 else 60.0

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(monitor)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        probe.unlink(missing_ok=True)
        sock.sendall(f"screendump {probe}\n".encode())
        time.sleep(0.8)
        rgb = center_rgb(probe)
        if rgb is not None and max(rgb) < 70:
            print(f"UI ready: {rgb}")
            sock.close()
            return 0
        time.sleep(0.4)
    sock.close()
    print("UI chưa sẵn sàng (center không tối)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
