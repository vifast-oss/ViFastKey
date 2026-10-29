"""Đo thời gian mỗi phím: transform (tính lại từ đầu) và Composer (tăng dần).  python -m tools.bench"""
import time

from vfk_core import TELEX, Composer, transform

WORDS = ["nguwowif", "truwowngf", "khuyeens", "dduwowngf", "Vieejt", "thuowng", "quyeets"] * 300


def bench():
    n = sum(len(w) for w in WORDS)
    t = time.perf_counter()
    for w in WORDS:
        for i in range(1, len(w) + 1):
            transform(w[:i], TELEX)
    a = (time.perf_counter() - t) / n * 1e6
    t = time.perf_counter()
    for w in WORDS:
        c = Composer(TELEX)
        for k in w:
            c.push(k)
            c.result()
    b = (time.perf_counter() - t) / n * 1e6
    print(f"transform: {a:.0f} µs/phím   Composer: {b:.0f} µs/phím   (Python; Rust kỳ vọng < 5 µs)")


if __name__ == "__main__":
    bench()
