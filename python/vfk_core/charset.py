"""Bảng mã tiếng Việt: Unicode, NFD, TCVN3, VNI-Windows, VISCII, VPS, BKHCM, Vietware,
CP1258, VIQR, NCR và C string.

Dữ liệu lấy từ `bamboo-core` (MIT) — xem `data/charsets.json` và phần ghi công trong
`THIRD_PARTY_NOTICES.md`. Bản này chỉ chép bảng ký tự, không chép engine GPL.

Quy ước:
- `encode()` trả về chuỗi Unicode của các codepoint/byte legacy (giống bamboo-core).
  Với bảng một byte, `encode_bytes()` đổi thẳng sang `bytes`.
- `decode()` nhận chuỗi đó (hoặc bytes) và trả về Unicode NFC.
- Các bảng TCVN3/Vietware Full mất phân biệt hoa/thường ở một số nguyên âm có dấu;
  decode theo quy tắc của bamboo-core: nếu hai ký tự chỉ khác hoa/thường thì chọn
  chữ thường, nếu khác hẳn thì để nguyên.
"""
from __future__ import annotations

import json
import string
import unicodedata
from functools import lru_cache
from pathlib import Path

_DATA = json.loads((Path(__file__).parent / "data" / "charsets.json").read_text(encoding="utf-8"))
_TABLES: dict[str, dict[str, str]] = {
    key: value["map"] for key, value in _DATA["charsets"].items()
}

DISPLAY_NAMES: dict[str, str] = {
    "unicode": "Unicode (NFC)",
    "unicode_nfd": "Unicode tổ hợp (NFD)",
    **{key: value["name"] for key, value in _DATA["charsets"].items()},
}

ALIASES = {
    "unicode": "unicode",
    "nfc": "unicode",
    "utf-8": "unicode",
    "utf8": "unicode",
    "unicode_nfd": "unicode_nfd",
    "unicode tổ hợp": "unicode_nfd",
    "nfd": "unicode_nfd",
    "decomposed": "unicode_nfd",
    "tcvn3": "tcvn3",
    "tcvn3 (abc)": "tcvn3",
    "abc": "tcvn3",
    "vni": "vni_windows",
    "vni windows": "vni_windows",
    "vni_windows": "vni_windows",
    "cp1258": "cp1258",
    "windows 1258 codepage": "cp1258",
    "windows-1258": "cp1258",
    "viqr": "viqr",
    "viscii": "viscii",
    "vps": "vps",
    "bkhcm1": "bkhcm1",
    "bkhcm 1": "bkhcm1",
    "bkhcm2": "bkhcm2",
    "bkhcm 2": "bkhcm2",
    "vietware_x": "vietware_x",
    "vietware x": "vietware_x",
    "vietware_full": "vietware_full",
    "vietware full": "vietware_full",
    "ncr_decimal": "ncr_decimal",
    "ncr decimal": "ncr_decimal",
    "ncr": "ncr_decimal",
    "ncr_hex": "ncr_hex",
    "ncr hex": "ncr_hex",
    "cstring_hex": "cstring_hex",
    "unicode c string hex": "cstring_hex",
    "cstring_decimal": "cstring_decimal",
    "unicode c string decimal": "cstring_decimal",
}

# Các bảng một byte: encode() trả về codepoint 0..255, có thể đổi thẳng sang bytes.
SINGLE_BYTE = {
    "tcvn3", "vni_windows", "viscii", "vps", "bkhcm1", "bkhcm2",
    "vietware_x", "vietware_full",
}

# VISCII dùng một số mã điều khiển cho chữ hoa; bảng encode của bamboo-core chỉ có
# chữ thường nên bổ sung khi decode.
_VISCII_EXTRA = {
    "\u0002": "Ẳ", "\u0005": "Ẵ", "\u0006": "Ẫ", "\u0014": "Ỷ",
    "\u0019": "Ỹ", "\u001e": "Ỵ", "\u00a0": "Õ",
}


def normalize_charset(name: str) -> str:
    key = name.strip().lower()
    if key in ALIASES:
        return ALIASES[key]
    if key in DISPLAY_NAMES:
        return key
    raise ValueError(f"Bảng mã không hỗ trợ: {name!r}")


def charsets() -> list[str]:
    return list(DISPLAY_NAMES)


@lru_cache(maxsize=None)
def _decode_table(key: str) -> tuple[dict[str, str], set[str], int]:
    table = _TABLES.get(key, {})
    groups: dict[str, list[str]] = {}
    for ch, code in table.items():
        groups.setdefault(code, []).append(ch)
    reverse: dict[str, str] = {}
    for code, chars in groups.items():
        if len(chars) == 1:
            reverse[code] = chars[0]
        elif len(chars) == 2 and chars[0].lower() == chars[1].lower():
            reverse[code] = next(ch for ch in chars if ch.islower())
        # Mã dùng chung cho hai chữ khác hẳn: không đoán.
    if key == "viscii":
        reverse.update(_VISCII_EXTRA)
    starts = {code[0] for code in reverse if code}
    max_len = max((len(code) for code in reverse), default=0)
    return reverse, starts, max_len


def _is_ascii_punct(ch: str) -> bool:
    return len(ch) == 1 and ch in string.punctuation


def _encode_viqr(text: str) -> str:
    table = _TABLES["viqr"]
    codes = set(table.values())
    out: list[str] = []
    prev = ""
    for ch in text:
        code = table.get(ch)
        escaped = ch == "\\" or (_is_ascii_punct(ch) and prev + ch in codes)
        if code is not None or escaped:
            if escaped:
                out.append("\\")
            out.append(code if code is not None else ch)
        else:
            out.append(ch)
        prev = "" if escaped else (code if code is not None else ch)
    return "".join(out)


def encode(text: str, charset: str = "unicode") -> str:
    """Chuyển Unicode -> bảng mã đích (dạng chuỗi codepoint legacy)."""
    key = normalize_charset(charset)
    if key == "unicode":
        return unicodedata.normalize("NFC", text)
    if key == "unicode_nfd":
        return unicodedata.normalize("NFD", text)
    if key == "viqr":
        return _encode_viqr(text)
    table = _TABLES[key]
    return "".join(table.get(ch, ch) for ch in text)


def decode(text: str | bytes, charset: str = "unicode") -> str:
    """Chuyển bảng mã -> Unicode NFC. `bytes` được hiểu là từng byte một."""
    key = normalize_charset(charset)
    if isinstance(text, bytes):
        text = text.decode("latin-1")
    if key == "unicode":
        return unicodedata.normalize("NFC", text)
    if key == "unicode_nfd":
        return unicodedata.normalize("NFC", text)
    reverse, starts, max_len = _decode_table(key)
    out: list[str] = []
    i = 0
    while i < len(text):
        if key == "viqr" and text[i] == "\\" and i + 1 < len(text) and _is_ascii_punct(text[i + 1]):
            out.append(text[i + 1])
            i += 2
            continue
        ch = text[i]
        found = None
        if ch in starts:
            for n in range(min(max_len, len(text) - i), 0, -1):
                code = text[i:i + n]
                if code in reverse:
                    found = (reverse[code], n)
                    break
        if found is not None and (found[0] != text[i] or len(found) > 1):
            out.append(found[0])
            i += found[1]
        else:
            out.append(ch)
            i += 1
    return unicodedata.normalize("NFC", "".join(out))


def encode_bytes(text: str, charset: str) -> bytes:
    """Như `encode()` nhưng trả bytes cho bảng một byte."""
    key = normalize_charset(charset)
    if key not in SINGLE_BYTE:
        raise ValueError(f"{charset!r} không phải bảng một byte; dùng encode()")
    encoded = encode(text, key)
    try:
        return encoded.encode("latin-1")
    except UnicodeEncodeError as exc:
        raise ValueError(f"Ký tự không có trong bảng {key}: {exc}") from exc


def decode_bytes(data: bytes, charset: str) -> str:
    key = normalize_charset(charset)
    if key not in SINGLE_BYTE:
        raise ValueError(f"{charset!r} không phải bảng một byte; dùng decode()")
    return decode(data, key)


def convert(text: str, from_charset: str, to_charset: str) -> str:
    return encode(decode(text, from_charset), to_charset)


_DETECT_PRIORITY = [
    "tcvn3", "vni_windows", "viscii", "vps", "bkhcm1", "bkhcm2", "vietware_x",
    "vietware_full", "cp1258", "viqr", "ncr_hex", "ncr_decimal", "cstring_hex",
    "cstring_decimal", "unicode_nfd", "unicode",
]


def detect(text: str | bytes) -> list[str]:
    """Đoán bảng mã: trả các ứng viên có nhiều chữ Việt nhất (không chắc chắn 100%)."""
    if isinstance(text, bytes):
        candidates = list(SINGLE_BYTE)
        raw = text.decode("latin-1")
    else:
        raw = text
        candidates = list(DISPLAY_NAMES)
    if "&#x" in raw.lower():
        candidates = ["ncr_hex"] + [c for c in candidates if c != "ncr_hex"]
    elif "&#" in raw:
        candidates = ["ncr_decimal"] + [c for c in candidates if c != "ncr_decimal"]
    elif "\\u" in raw or "\\x" in raw:
        candidates = ["cstring_hex", "cstring_decimal"] + [c for c in candidates if c not in {"cstring_hex", "cstring_decimal"}]
    scored = []
    for key in candidates:
        try:
            decoded = decode(raw, key)
        except ValueError:
            continue
        score = sum(1 for ch in decoded if ch not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ")
        if decoded != raw:
            scored.append((score, key))
    order = {key: i for i, key in enumerate(_DETECT_PRIORITY)}
    scored.sort(key=lambda item: (-item[0], order.get(item[1], 999)))
    return [key for _, key in scored]


def remove_diacritics(text: str) -> str:
    """Bỏ dấu phụ và thanh, giữ đ/Đ thành d/D (dùng cho tìm kiếm/chuyển bảng mã)."""
    out = []
    for ch in unicodedata.normalize("NFD", text):
        if unicodedata.combining(ch):
            continue
        out.append("d" if ch == "đ" else "D" if ch == "Đ" else ch)
    return unicodedata.normalize("NFC", "".join(out))


__all__ = ["ALIASES", "DISPLAY_NAMES", "SINGLE_BYTE", "charsets", "convert", "decode",
           "decode_bytes", "detect", "encode", "encode_bytes", "normalize_charset",
           "remove_diacritics"]
