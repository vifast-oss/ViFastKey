"""vfk-suggest: gợi ý từ theo tiền tố (local, không mạng)."""
from __future__ import annotations

import argparse
import sys

from vfk_suggest import default_engine


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="vfk-suggest", description=__doc__)
    ap.add_argument("prefix", nargs="?", default="")
    ap.add_argument("--limit", type=int, default=8)
    ap.add_argument("--user", action="append", default=[], metavar="FILE",
                    help="nạp thêm danh sách từ (mỗi từ một dòng/khoảng trắng)")
    ap.add_argument("--list-size", action="store_true")
    args = ap.parse_args(argv)
    engine = default_engine()
    for path in args.user:
        engine.load_file(path)
    if args.list_size:
        print(len(engine))
        return 0
    if not args.prefix:
        ap.print_help()
        return 2
    for word in engine.suggest(args.prefix, args.limit):
        print(word)
    return 0


if __name__ == "__main__":
    sys.exit(main())
