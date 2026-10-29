"""vfk-emoji: tìm emoji theo tên/từ khoá tiếng Việt."""
from __future__ import annotations

import argparse
import sys

from vfk_core import search_emoji


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="vfk-emoji", description=__doc__)
    ap.add_argument("query", nargs="?", default="")
    ap.add_argument("--limit", type=int, default=8)
    args = ap.parse_args(argv)
    for char, name in search_emoji(args.query, args.limit):
        print(f"{char}\t{name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
