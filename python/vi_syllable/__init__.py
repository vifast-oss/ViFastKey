"""vi-syllable: mô hình âm tiết tiếng Việt (thuần, không I/O).

Tương ứng crate Rust `vi-syllable`, có thể dùng chung với frontend ViFastTts.
Âm tiết = âm đầu (onset) + cụm vần nguyên âm (âm đệm + âm chính + bán âm cuối) + âm cuối (coda) + thanh.
"""
from __future__ import annotations

import unicodedata
from dataclasses import dataclass
from enum import Enum

# ---------------------------------------------------------------- bảng dữ liệu
VOWELS = set("aăâeêioôơuưy")
MARKED_VOWELS = set("ăâêôơư")

# Âm đầu, sắp xếp dài trước để so khớp dài nhất. "gi" và "qu" xử lý riêng.
ONSETS = sorted(
    ["ngh", "ng", "nh", "gh", "kh", "ph", "th", "tr", "ch",
     "b", "c", "d", "đ", "g", "h", "k", "l", "m", "n", "p", "r", "s", "t", "v", "x"],
    key=len, reverse=True,
)
ALL_ONSETS = set(ONSETS) | {"gi", "qu"}
ONSET_PREFIXES = {o[:i] for o in ALL_ONSETS for i in range(1, len(o) + 1)}

CODAS = {"c", "ch", "m", "n", "ng", "nh", "p", "t"}
STOP_CODAS = {"c", "ch", "p", "t"}          # chỉ đi với thanh sắc / nặng
# Âm cuối k chỉ dùng cho tên riêng/địa danh (Đắk Lắk). Bật bằng tuỳ chọn allow_coda_k.
CODA_K = "k"
CODAS_WITH_K = CODAS | {CODA_K}
STOP_CODAS_WITH_K = STOP_CODAS | {CODA_K}
CH_NH_CLUSTERS = {"a", "ê", "i", "oa", "uê", "uy"}

# Cụm nguyên âm đứng cuối âm tiết (không có phụ âm cuối)
OPEN_CLUSTERS = {
    "a", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y",
    "ai", "ao", "au", "ay", "âu", "ây", "eo", "êu", "ia", "iu", "oa", "oe",
    "oi", "ôi", "ơi", "ua", "uê", "ui", "uy", "ưa", "ưi", "ưu", "uơ",
    "iêu", "yêu", "oai", "oay", "oao", "oeo", "uây", "uôi", "ươi", "ươu", "uya", "uyu",
}
# Cụm nguyên âm đi được với phụ âm cuối
CLOSED_CLUSTERS = {
    "a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư",
    "oa", "oă", "oe", "uâ", "uê", "uy", "iê", "yê", "uô", "ươ", "uyê", "oo",
}
ALL_CLUSTERS = OPEN_CLUSTERS | CLOSED_CLUSTERS
CLUSTER_PREFIXES = {c[:i] for c in ALL_CLUSTERS for i in range(1, len(c) + 1)}

FRONT_VOWELS = set("eêiy")

# Thanh: 0 ngang, 1 sắc, 2 huyền, 3 hỏi, 4 ngã, 5 nặng
TONE_COMBINING = {1: "\u0301", 2: "\u0300", 3: "\u0309", 4: "\u0303", 5: "\u0323"}
COMBINING_TONE = {v: k for k, v in TONE_COMBINING.items()}
# Dấu phụ: ^ mũ, ( trăng, + móc, - gạch ngang (đ)
MARK_COMBINING = {"^": "\u0302", "(": "\u0306", "+": "\u031b"}
COMBINING_MARK = {v: k for k, v in MARK_COMBINING.items()}

LETTERS = set("abcdefghijklmnopqrstuvwxyzđ") | VOWELS


class Status(Enum):
    VALID = "valid"        # âm tiết hoàn chỉnh hợp lệ
    PARTIAL = "partial"    # đang gõ dở, còn có thể thành hợp lệ
    INVALID = "invalid"    # không thể là âm tiết tiếng Việt


class ToneStyle(Enum):
    OLD = "old"   # hòa, thúy
    NEW = "new"   # hoà, thuý


@dataclass(frozen=True)
class Syllable:
    onset: str
    cluster: str
    coda: str
    cluster_start: int


# ---------------------------------------------------------------- ký tự
def compose_char(base: str, mark: str | None = None, tone: int = 0, upper: bool = False) -> str:
    """Ghép chữ cơ sở + dấu phụ + thanh thành 1 ký tự NFC."""
    if mark == "-":
        ch = "đ" if base == "d" else base
    else:
        ch = base + (MARK_COMBINING.get(mark, "") if mark else "")
    if tone:
        ch += TONE_COMBINING[tone]
    ch = unicodedata.normalize("NFC", ch)
    return ch.upper() if upper else ch


def decompose_char(ch: str) -> tuple[str, str | None, int, bool]:
    """Tách 1 ký tự thành (chữ cơ sở thường, dấu phụ, thanh, viết hoa?)."""
    upper = ch != ch.lower()
    ch = ch.lower()
    if ch == "đ":
        return "d", "-", 0, upper
    parts = unicodedata.normalize("NFD", ch)
    base, mark, tone = parts[0], None, 0
    for c in parts[1:]:
        if c in COMBINING_MARK:
            mark = COMBINING_MARK[c]
        elif c in COMBINING_TONE:
            tone = COMBINING_TONE[c]
    return base, mark, tone, upper


def strip_tone(text: str) -> tuple[str, int]:
    """Bỏ thanh khỏi chuỗi, trả về (chuỗi không thanh, thanh)."""
    out, tone = [], 0
    for ch in text:
        b, m, t, up = decompose_char(ch)
        tone = t or tone
        out.append(compose_char(b, m, 0, up))
    return "".join(out), tone


# ---------------------------------------------------------------- phân tích
def parse(s: str) -> Syllable:
    """Tách chuỗi thường, không thanh thành âm đầu / cụm nguyên âm / phần còn lại."""
    if s.startswith("gi") and len(s) > 2 and s[2] in VOWELS:
        onset = "gi"              # gia, giữ, giếng
    elif s.startswith("qu"):
        onset = "qu"              # qua, quốc, quý
    else:
        onset = next((o for o in ONSETS if s.startswith(o)), "")
    i = j = len(onset)
    while j < len(s) and s[j] in VOWELS:
        j += 1
    return Syllable(onset, s[i:j], s[j:], i)


_DEMARK = {"ă": "a", "â": "a", "ê": "e", "ô": "o", "ơ": "o", "ư": "u"}


def _relax(c: str) -> set[str]:
    """Mọi biến thể bỏ bớt dấu phụ: 'ươ' -> {ươ, uơ, ưo, uo}."""
    out = {""}
    for ch in c:
        opts = {ch, _DEMARK.get(ch, ch)}
        out = {p + o for p in out for o in opts}
    return out


def _prefixes(c: str) -> set[str]:
    return {c[:i] for i in range(1, len(c) + 1)}


# Cụm nguyên âm (kể cả gõ dở, chưa đủ dấu) có thể hoàn thiện thành âm tiết mở hoặc đóng
_RELAXED_ANY = {r for c in ALL_CLUSTERS for p in _prefixes(c) for r in _relax(p)}
# coda (đang gõ, có thể dở) -> các cụm nguyên âm (đã bỏ bớt dấu) đi được với nó
_RELAXED_FOR_CODA: dict[str, set[str]] = {}
for _coda in CODAS:
    for _cl in CLOSED_CLUSTERS:
        if _coda in ("ch", "nh") and _cl not in CH_NH_CLUSTERS:
            continue
        for _p in _prefixes(_coda):
            _RELAXED_FOR_CODA.setdefault(_p, set()).update(_relax(_cl))

# Bảng thêm cho âm cuối k (tuỳ chọn tên riêng). k đi được với mọi cụm đóng.
_RELAXED_FOR_CODA_K = {k: set(v) for k, v in _RELAXED_FOR_CODA.items()}
for _cl in CLOSED_CLUSTERS:
    _RELAXED_FOR_CODA_K.setdefault(CODA_K, set()).update(_relax(_cl))


def _onset_ok(onset: str, first: str) -> bool:
    if onset in ("k", "gh", "ngh") and first not in FRONT_VOWELS:
        return False
    if onset in ("c", "ng", "q") and first in FRONT_VOWELS:
        return False
    if onset == "g" and first in "eê":
        return False
    if onset == "qu" and first in "uư":
        return False
    return True


def check(s: str, tone: int = 0, allow_coda_k: bool = False) -> Status:
    """Kiểm tra chính tả 1 âm tiết (chuỗi thường, không thanh).

    VALID   : âm tiết hoàn chỉnh.
    PARTIAL : chưa hoàn chỉnh nhưng có thể thành hợp lệ bằng cách gõ thêm chữ và/hoặc thêm dấu phụ
              (cho phép gõ dấu tự do: 'chuyenj' + 'e' -> 'chuyện').
    INVALID : không thể thành âm tiết tiếng Việt.

    allow_coda_k: cho phép âm cuối k cho tên riêng/địa danh (Đắk Lắk). k là âm cuối tắc
    nên chỉ đi với thanh sắc/nặng, giống c/ch/p/t.
    """
    if not s:
        return Status.PARTIAL
    if any(c not in LETTERS for c in s):
        return Status.INVALID
    syl = parse(s)
    if not syl.cluster:
        if syl.coda:
            return Status.PARTIAL if s in ONSET_PREFIXES else Status.INVALID
        return Status.PARTIAL
    if any(c in VOWELS for c in syl.coda):
        return Status.INVALID
    cluster = syl.cluster
    if syl.onset == "qu" and cluster.startswith("y"):
        cluster = "u" + cluster               # quýt, quỳnh, quyết: kiểm tra như vần uy / uyê
    if not _onset_ok(syl.onset, syl.cluster[0]):
        return Status.INVALID
    stop_bad = tone in (2, 3, 4)
    codas = CODAS_WITH_K if allow_coda_k else CODAS
    stop_codas = STOP_CODAS_WITH_K if allow_coda_k else STOP_CODAS
    relaxed_for_coda = _RELAXED_FOR_CODA_K if allow_coda_k else _RELAXED_FOR_CODA
    coda = syl.coda
    # --- hợp lệ hoàn chỉnh
    if not coda:
        if cluster in OPEN_CLUSTERS:
            return Status.VALID
    elif (coda in codas and cluster in CLOSED_CLUSTERS
          and not (coda in ("ch", "nh") and cluster not in CH_NH_CLUSTERS)):
        if coda in stop_codas and stop_bad:
            return Status.INVALID
        return Status.VALID
    # --- còn có thể hoàn thiện
    if not coda:
        return Status.PARTIAL if cluster in _RELAXED_ANY else Status.INVALID
    if cluster in relaxed_for_coda.get(coda, ()):
        if stop_bad and coda in stop_codas:
            return Status.INVALID
        return Status.PARTIAL
    return Status.INVALID


def tone_allowed(s: str, tone: int, allow_coda_k: bool = False) -> bool:
    """Âm cuối tắc (c, ch, p, t, và k khi bật tên riêng) chỉ đi với thanh sắc / nặng."""
    syl = parse(s)
    stop_codas = STOP_CODAS_WITH_K if allow_coda_k else STOP_CODAS
    return not (syl.coda in stop_codas and tone in (2, 3, 4))


def tone_index(s: str, style: ToneStyle = ToneStyle.OLD) -> int | None:
    """Vị trí đặt thanh trong chuỗi thường, không thanh."""
    syl = parse(s)
    c, st = syl.cluster, syl.cluster_start
    if not c:
        return None
    if "ơ" in c:                                   # ươ -> ơ, uơ -> ơ
        return st + c.index("ơ")
    marked = [i for i, ch in enumerate(c) if ch in MARKED_VOWELS]
    if marked:                                     # ưa, uyê, uâ ...
        return st + marked[0]
    if syl.coda:                                   # toán, huỳnh
        return st + len(c) - 1
    if len(c) == 1:
        return st
    if len(c) == 3:                                # oái, khuỷu, ngoèo
        return st + 1
    if c in ("oa", "oe", "uy"):                    # hòa / hoà
        return st + (1 if style is ToneStyle.NEW else 0)
    return st                                      # mùa, kìa, hai


def place_tone(word: str, tone: int, style: ToneStyle = ToneStyle.OLD) -> str:
    """Đặt thanh lên một từ đã có dấu phụ (giữ hoa/thường)."""
    base, _ = strip_tone(word)
    if not tone:
        return base
    idx = tone_index(base.lower(), style)
    if idx is None:
        return base
    b, m, _, up = decompose_char(base[idx])
    return base[:idx] + compose_char(b, m, tone, up) + base[idx + 1:]
