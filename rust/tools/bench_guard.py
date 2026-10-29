#!/usr/bin/env python3
"""Chạy bench Rust và fail nếu vượt ngưỡng (rộng, tránh flaky CI)."""
import re
import subprocess
import sys

out = subprocess.check_output(["cargo", "run", "-q", "--release", "-p", "vfk-core", "--example", "bench"],
                              text=True)
print(out.strip())
m = re.search(r"transform:\s*([\d.]+)\s*µs/phím\s+Composer:\s*([\d.]+)", out)
if not m:
    print("Không đọc được kết quả bench", file=sys.stderr)
    sys.exit(1)
transform, composer = map(float, m.groups())
# Ngưỡng rộng cho runner CI; mục tiêu thực tế < 5/< 2.
if transform > 30 or composer > 15:
    print(f"Bench quá chậm: transform={transform} composer={composer}", file=sys.stderr)
    sys.exit(1)
