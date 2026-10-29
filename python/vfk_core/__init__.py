"""vfk-core: biến đổi phím -> chữ. Thuần, đồng bộ, không I/O (tương ứng crate Rust `vfk-core`)."""
from dataclasses import dataclass

from .rules import INPUT_METHODS, QUICK_TELEX, SIMPLE_TELEX, TELEX, VNI, InputMethodSpec, MarkRule
from .transform import MAX_RAW, Composer, TransformOptions, TransformResult, rebuild, to_raw, transform


@dataclass(frozen=True)
class Edit:
    delete_before: int    # số ký tự (code point) cần xoá lùi
    insert: str


def diff(old: str, new: str) -> Edit:
    """Lệnh sửa tối thiểu: giữ tiền tố chung, xoá phần còn lại của old, chèn phần còn lại của new."""
    p = 0
    while p < min(len(old), len(new)) and old[p] == new[p]:
        p += 1
    return Edit(len(old) - p, new[p:])


__all__ = ["INPUT_METHODS", "QUICK_TELEX", "SIMPLE_TELEX", "TELEX", "VNI", "MAX_RAW", "Composer", "rebuild", "InputMethodSpec", "MarkRule",
           "TransformOptions", "TransformResult", "transform", "to_raw", "Edit", "diff"]
