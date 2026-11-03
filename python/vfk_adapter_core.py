"""Adapter-core bản Python, tương thích `rust/crates/vfk-adapter-core`.

IBus chạy bằng Python nên không thể link trực tiếp crate Rust. Module này giữ
đúng contract của `adapter-conformance/*.jsonl` và cùng logic với bản Rust:
field policy, surrounding guard, action plan, timing.

Port khác nên ưu tiên C ABI `vfk_plan_action_with_fallback`; module này chỉ dành cho Python.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from vfk_engine import (
    Commit,
    FieldKind,
    PassThrough,
    Preedit,
    Replace,
)

DISABLED_FIELDS = {FieldKind.PASSWORD, FieldKind.NUMBER, FieldKind.EMAIL}
MIN_REPLACE_WAIT_MS = 3
MAX_REPLACE_WAIT_MS = 60
POST_COMMIT_DELAY_MS = 10


class PlanKind(str, Enum):
    PASS = "pass"
    PREEDIT = "preedit"
    COMMIT = "commit"
    REPLACE = "replace"
    FAIL_SAFE = "fail_safe"


class ReplaceFallback(str, Enum):
    """Chính sách thử lại khi Replace fail-safe vì surrounding cũ."""
    NONE = "none"
    STALE_SURROUNDING = "stale_surrounding"
    ALWAYS = "always"


@dataclass(frozen=True)
class SurroundingSnapshot:
    text: str
    cursor_utf16: int
    anchor_utf16: int


@dataclass(frozen=True)
class ReplaceCheck:
    deleted: str
    delete_utf16: int
    delete_chars: int


@dataclass(frozen=True)
class ReplaceUtf16:
    """Replace action với số UTF-16 code unit (tương đương C ABI VfkAction)."""
    insert: str
    delete_utf16: int
    pass_key: bool = False


@dataclass(frozen=True)
class Plan:
    kind: PlanKind
    text: str = ""
    cursor_utf16: int = 0
    delete_utf16: int = 0
    delete_chars: int = 0
    wait_ms: int = 0
    pass_key: bool = False
    reason: str = ""


def utf16_len(text: str) -> int:
    return len(text.encode("utf-16-le")) // 2


def char_to_utf16_offset(text: str, chars: int) -> int | None:
    if chars < 0:
        return None
    used = 0
    for index, ch in enumerate(text):
        if index == chars:
            return used
        used += 2 if ord(ch) > 0xFFFF else 1
    return used if chars == len(text) else None


def utf16_to_byte_offset(text: str, units: int) -> int | None:
    if units < 0:
        return None
    used = 0
    for offset, ch in enumerate(text):
        if used == units:
            return offset
        width = 2 if ord(ch) > 0xFFFF else 1
        if used + width > units:
            return None
        used += width
    return len(text) if used == units else None


def utf16_prefix(text: str, units: int) -> str | None:
    offset = utf16_to_byte_offset(text, units)
    return None if offset is None else text[:offset]


def utf16_suffix(text: str, units: int) -> str | None:
    if units == 0:
        return ""
    total = utf16_len(text)
    if units < 0 or units > total:
        return None
    offset = utf16_to_byte_offset(text, total - units)
    return None if offset is None else text[offset:]


def char_suffix(text: str, chars: int) -> str | None:
    if chars < 0 or chars > len(text):
        return None
    return text[len(text) - chars:] if chars else ""


def char_suffix_utf16_len(text: str, chars: int) -> int | None:
    suffix = char_suffix(text, chars)
    return None if suffix is None else utf16_len(suffix)


def is_word_codepoint(codepoint: int) -> bool:
    if codepoint < 0x80:
        return chr(codepoint).isalnum() or codepoint == ord("_")
    return (0x00C0 <= codepoint <= 0x024F
            or 0x0300 <= codepoint <= 0x036F
            or 0x1E00 <= codepoint <= 0x1EFF)


def next_codepoint(text: str, cursor_utf16: int) -> int | None:
    offset = utf16_to_byte_offset(text, cursor_utf16)
    if offset is None or offset >= len(text):
        return None
    return ord(text[offset])


def check_replace(shown_before: str, delete_utf16: int,
                  surrounding: SurroundingSnapshot) -> ReplaceCheck | None:
    if delete_utf16 <= 0 or surrounding.cursor_utf16 != surrounding.anchor_utf16:
        return None
    before_cursor = utf16_prefix(surrounding.text, surrounding.cursor_utf16)
    if before_cursor is None:
        return None
    shown = before_cursor if shown_before == "" else shown_before
    deleted = utf16_suffix(shown, delete_utf16)
    if not deleted or not before_cursor.endswith(deleted):
        return None
    nxt = next_codepoint(surrounding.text, surrounding.cursor_utf16)
    if nxt is not None and is_word_codepoint(nxt):
        return None
    return ReplaceCheck(deleted, delete_utf16, len(deleted))


def replace_wait_ms(profile, delete_chars: int) -> int:
    if not getattr(profile, "wait_ack", False):
        return MIN_REPLACE_WAIT_MS
    wait = (getattr(profile, "backspace_delay_ms", 0)
            + getattr(profile, "delay_per_char_ms", 0) * delete_chars)
    return max(MIN_REPLACE_WAIT_MS, min(MAX_REPLACE_WAIT_MS, wait))


def _pass(reason: str) -> Plan:
    return Plan(PlanKind.PASS, reason=reason)


def _fail(reason: str) -> Plan:
    return Plan(PlanKind.FAIL_SAFE, reason=reason)


def plan_action(action, field_kind: FieldKind, before_display: str,
                surrounding: SurroundingSnapshot | None, profile,
                allow_unverified_replace: bool = False) -> Plan:
    """Biến Action Python thành Plan. Không mutate engine."""
    if field_kind in DISABLED_FIELDS:
        return _pass("field-disabled")

    if isinstance(action, PassThrough):
        return _pass("engine-pass")
    if isinstance(action, Preedit):
        cursor = char_to_utf16_offset(action.text, action.cursor)
        return Plan(PlanKind.PREEDIT, text=action.text,
                    cursor_utf16=cursor or 0, reason="preedit")
    if isinstance(action, Commit):
        return Plan(PlanKind.COMMIT, text=action.text,
                    pass_key=action.pass_key, reason="commit")
    if isinstance(action, ReplaceUtf16):
        insert = action.insert
        pass_key = action.pass_key
        source = before_display
        if not source and surrounding is not None:
            source = utf16_prefix(surrounding.text, surrounding.cursor_utf16) or ""
        delete_utf16 = action.delete_utf16
    elif isinstance(action, Replace):
        insert = action.insert
        pass_key = action.pass_key
        source = before_display
        if not source and surrounding is not None:
            source = utf16_prefix(surrounding.text, surrounding.cursor_utf16) or ""
        delete_utf16 = char_suffix_utf16_len(source, action.delete_before)
        if delete_utf16 is None:
            return _fail("replace-suffix-out-of-range")
    else:
        return _fail("unknown-action")

    if delete_utf16 == 0:
        return Plan(PlanKind.COMMIT, text=insert,
                    pass_key=pass_key, reason="replace-no-delete")

    if allow_unverified_replace:
        deleted = utf16_suffix(source, delete_utf16)
        if not deleted:
            return _fail("replace-unverified-suffix")
        delete_chars = len(deleted)
        return Plan(PlanKind.REPLACE, text=insert,
                    delete_utf16=delete_utf16, delete_chars=delete_chars,
                    wait_ms=replace_wait_ms(profile, delete_chars),
                    pass_key=pass_key, reason="replace-trusted-direct")

    if surrounding is None:
        return _fail("replace-no-surrounding")
    check = check_replace(before_display, delete_utf16, surrounding)
    if check is None:
        return _fail("replace-surrounding-mismatch")
    return Plan(PlanKind.REPLACE, text=insert,
                delete_utf16=check.delete_utf16, delete_chars=check.delete_chars,
                wait_ms=replace_wait_ms(profile, check.delete_chars),
                pass_key=pass_key, reason="replace-verified")


def plan_action_with_fallback(
    action, field_kind: FieldKind, before_display: str,
    surrounding: SurroundingSnapshot | None, profile,
    allow_unverified_replace: bool = False,
    fallback: ReplaceFallback = ReplaceFallback.NONE,
) -> Plan:
    """Planner có fallback khi surrounding cũ, dùng chung cho mọi adapter.

    Strict plan trước; chỉ khi ``Replace`` fail-safe mới tin ``before_display``
    và thử lại. Adapter chỉ chọn ``ReplaceFallback`` theo khả năng thực tế.
    """
    if not isinstance(fallback, ReplaceFallback):
        fallback = ReplaceFallback(fallback)
    plan = plan_action(action, field_kind, before_display, surrounding,
                       profile, allow_unverified_replace)
    if plan.kind != PlanKind.FAIL_SAFE or not isinstance(action, (Replace, ReplaceUtf16)):
        return plan
    # Selection hoặc caret giữa từ là ngữ cảnh thật sự không an toàn; không
    # được "tin before_display" để bỏ qua guard.
    if surrounding is not None:
        if surrounding.cursor_utf16 != surrounding.anchor_utf16:
            return plan
        nxt = next_codepoint(surrounding.text, surrounding.cursor_utf16)
        if nxt is not None and is_word_codepoint(nxt):
            return plan
    if fallback is ReplaceFallback.STALE_SURROUNDING and surrounding is None:
        return plan
    if fallback is ReplaceFallback.NONE:
        return plan
    return plan_action(action, field_kind, before_display, surrounding,
                       profile, allow_unverified_replace=True)


__all__ = [
    "MAX_REPLACE_WAIT_MS", "MIN_REPLACE_WAIT_MS", "POST_COMMIT_DELAY_MS",
    "Plan", "PlanKind", "ReplaceCheck", "ReplaceFallback", "ReplaceUtf16",
    "SurroundingSnapshot",
    "char_suffix", "char_suffix_utf16_len", "char_to_utf16_offset",
    "check_replace", "is_word_codepoint", "next_codepoint", "plan_action",
    "plan_action_with_fallback", "replace_wait_ms", "utf16_len", "utf16_prefix",
    "utf16_suffix", "utf16_to_byte_offset",
]
