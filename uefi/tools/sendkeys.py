#!/usr/bin/env python3
"""Send HMP `sendkey` commands to a QEMU monitor socket.

Example:
    python3 tools/sendkeys.py build/monitor.sock t i e e n g s spc
    python3 tools/sendkeys.py build/monitor.sock --shot build/shot.ppm t e x t
"""

import socket
import sys
import time


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2

    sock_path = sys.argv[1]
    args = sys.argv[2:]
    shot = None
    if args[:1] == ["--shot"]:
        if len(args) < 2:
            return 2
        shot = args[1]
        args = args[2:]

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(sock_path)
    time.sleep(0.2)

    for key in args:
        sock.sendall(f"sendkey {key}\n".encode())
        time.sleep(0.15)

    if shot:
        time.sleep(0.5)
        sock.sendall(f"screendump {shot}\n".encode())
        time.sleep(0.5)

    sock.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
