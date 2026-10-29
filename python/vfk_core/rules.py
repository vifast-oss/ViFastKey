"""Bảng luật kiểu gõ dạng DỮ LIỆU (Strategy pattern).

Thêm kiểu gõ mới = thêm 1 InputMethodSpec, không sửa thuật toán.
Rust: struct + nạp từ TOML/JSON lúc build (include_str!).
"""
from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class MarkRule:
    mark: str            # "^" mũ, "(" trăng, "+" móc, "-" gạch (đ)
    targets: str         # các chữ cơ sở nhận dấu
    pair: bool = False   # True: áp cho cặp "uo" -> "ươ"


@dataclass(frozen=True)
class InputMethodSpec:
    name: str
    tones: dict[str, int]                                   # phím -> thanh
    remove_tone: str                                        # phím xoá thanh
    marks: dict[str, tuple[MarkRule, ...]]                  # phím -> luật dấu
    standalone: dict[str, tuple[str, str]] = field(default_factory=dict)   # chỉ khi không có đích: w -> ư
    insert_keys: dict[str, tuple[str, str]] = field(default_factory=dict)  # luôn chèn: [ -> ơ, ] -> ư
    digit_keys: bool = False                                # chữ số là phím chức năng (VNI)

    def is_function_key(self, k: str) -> bool:
        k = k.lower()
        return (k in self.tones or k == self.remove_tone or k in self.marks
                or k in self.standalone or k in self.insert_keys)

    def mark_key(self, base: str, mark: str) -> str | None:
        for key, rules in self.marks.items():
            for r in rules:
                if r.mark == mark and base in r.targets and not r.pair:
                    return key
        return None

    def tone_key(self, tone: int) -> str | None:
        return next((k for k, t in self.tones.items() if t == tone), None)


TELEX = InputMethodSpec(
    name="telex",
    tones={"s": 1, "f": 2, "r": 3, "x": 4, "j": 5},
    remove_tone="z",
    marks={
        "a": (MarkRule("^", "a"),),
        "e": (MarkRule("^", "e"),),
        "o": (MarkRule("^", "o"),),
        "w": (MarkRule("+", "uo", pair=True), MarkRule("+", "uo"), MarkRule("(", "a")),
        "d": (MarkRule("-", "d"),),
    },
    standalone={"w": ("u", "+")},
    insert_keys={"[": ("o", "+"), "]": ("u", "+")},
)

# Simple Telex: không có "w" đứng riêng, không có [ ]
SIMPLE_TELEX = InputMethodSpec(
    name="simple_telex",
    tones=TELEX.tones, remove_tone="z", marks=TELEX.marks,
)

VNI = InputMethodSpec(
    name="vni",
    tones={"1": 1, "2": 2, "3": 3, "4": 4, "5": 5},
    remove_tone="0",
    marks={
        "6": (MarkRule("^", "aeo"),),
        "7": (MarkRule("+", "uo", pair=True), MarkRule("+", "uo")),
        "8": (MarkRule("(", "a"),),
        "9": (MarkRule("-", "d"),),
    },
    digit_keys=True,
)

INPUT_METHODS = {"telex": TELEX, "simple_telex": SIMPLE_TELEX, "vni": VNI}

# Gõ nhanh phụ âm đầu (Quick Telex, kiểu OpenKey): cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th
QUICK_TELEX = {"c": "ch", "g": "gi", "k": "kh", "n": "ng", "q": "qu", "p": "ph", "t": "th"}
