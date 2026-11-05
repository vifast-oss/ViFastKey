"""vfk-charset: chuyển văn bản giữa Unicode và các bảng mã tiếng Việt.

Ví dụ:
    python -m apps.vfk_charset --list
    python -m apps.vfk_charset --to tcvn3 --text "Tiếng Việt" > vi.tcvn
    python -m apps.vfk_charset --from tcvn3 --file vi.tcvn --to unicode
    python -m apps.vfk_charset --detect --file vi.tcvn
    python -m apps.vfk_charset --to viqr --text "mưa."

Bảng một byte được ghi thẳng ra stdout/file dạng bytes; VIQR/NCR/C string ghi UTF-8.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

from vfk_core import (
    CHARSET_NAMES,
    change_case,
    charsets,
    decode_bytes,
    decode_charset,
    detect_charset,
    encode_bytes,
    encode_charset,
    normalize_charset,
    remove_diacritics,
)


def _read_input(args) -> tuple[str | bytes, str | None]:
    source = args.from_charset
    if args.file:
        raw = Path(args.file).read_bytes()
        single = {"tcvn3", "vni_windows", "viscii", "vps", "bkhcm1", "bkhcm2",
                  "vietware_x", "vietware_full"}
        if args.detect or (source and normalize_charset(source) in single):
            return raw, source
        return raw.decode("utf-8"), source
    text = "\n".join(args.text)
    return text, source


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="vfk-charset", description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--to", "-t", dest="to_charset", help="bảng mã đích")
    ap.add_argument("--from", "-f", dest="from_charset", help="bảng mã nguồn")
    ap.add_argument("--text", action="append", default=[], metavar="CHUỖI",
                    help="văn bản cần chuyển (lặp lại được)")
    ap.add_argument("--file", help="đọc từ file (bytes nếu bảng một byte)")
    ap.add_argument("--output", "-o", help="ghi ra file thay vì stdout")
    ap.add_argument("--detect", action="store_true", help="đoán bảng mã của input")
    ap.add_argument("--clipboard", action="store_true", help="đọc/ghi clipboard macOS (pbpaste/pbcopy)")
    ap.add_argument("--list", action="store_true", help="liệt kê bảng mã")
    ap.add_argument("--case", choices=["upper", "lower", "title"], help="đổi hoa/thường trước khi mã hoá")
    ap.add_argument("--remove-diacritics", action="store_true", help="bỏ dấu trước khi mã hoá")
    ap.add_argument("--version", action="version", version="ViFastKey charset 0.1.2")
    args = ap.parse_args(argv)

    if args.list:
        for key in charsets():
            print(f"{key:18} {CHARSET_NAMES[key]}")
        return 0

    if args.clipboard:
        if sys.platform != "darwin":
            print("--clipboard hiện chỉ hỗ trợ macOS", file=sys.stderr)
            return 1
        args.file = None
        data = subprocess.check_output(["pbpaste"])
        source = args.from_charset
    else:
        data, source = _read_input(args)
    if args.detect:
        for key in detect_charset(data):
            print(key)
        return 0

    if source:
        key = normalize_charset(source)
        if isinstance(data, bytes) and key in {"tcvn3", "vni_windows", "viscii", "vps",
                                               "bkhcm1", "bkhcm2", "vietware_x", "vietware_full"}:
            text = decode_bytes(data, key)
        else:
            text = decode_charset(data, key)
    else:
        text = data.decode("utf-8") if isinstance(data, bytes) else data

    if args.remove_diacritics:
        text = remove_diacritics(text)
    if args.case:
        text = change_case(text, args.case)

    target = args.to_charset or "unicode"
    key = normalize_charset(target)
    binary = key in {"tcvn3", "vni_windows", "viscii", "vps", "bkhcm1", "bkhcm2",
                     "vietware_x", "vietware_full"}
    payload: bytes = encode_bytes(text, key) if binary else encode_charset(text, key).encode("utf-8")

    if args.clipboard:
        subprocess.run(["pbcopy"], input=payload, check=True)
    elif args.output:
        Path(args.output).write_bytes(payload)
    else:
        sys.stdout.buffer.write(payload)
        sys.stdout.buffer.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())
