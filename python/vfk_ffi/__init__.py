"""vfk-ffi: lớp ranh giới cho C ABI / UniFFI / WASM.

Mô phỏng API phẳng kiểu C: handle số nguyên + JSON. Đơn vị xoá được đổi sang UTF-16
(Windows, macOS, Android, iOS, JS đều đếm UTF-16), adapter không phải tự đổi.
"""
from __future__ import annotations

import json

from vfk_engine import Commit, ImeSession, KeyEvent, OutputMode, PassThrough, Preedit, Ports, Replace


def utf16_len(s: str) -> int:
    return len(s.encode("utf-16-le")) // 2


_sessions: dict[int, tuple[ImeSession, str]] = {}
_next = 1


def vfk_session_new(ports: Ports, mode: str) -> int:
    global _next
    h = _next
    _next += 1
    _sessions[h] = (ImeSession(ports, OutputMode(mode)), "")
    return h


def vfk_session_free(h: int) -> None:
    _sessions.pop(h, None)


def vfk_process_key(h: int, key: str, ctrl: bool = False, alt: bool = False, meta: bool = False) -> str:
    """Trả JSON. Với Replace, delete_before_utf16 tính trên đoạn chữ bị xoá."""
    s, _ = _sessions[h]
    before = s.display
    act = s.process_key(KeyEvent(key, ctrl, alt, meta))
    if isinstance(act, PassThrough):
        out = {"type": "pass"}
    elif isinstance(act, Preedit):
        out = {"type": "preedit", "text": act.text, "cursor_utf16": utf16_len(act.text[:act.cursor])}
    elif isinstance(act, Commit):
        out = {"type": "commit", "text": act.text, "pass_key": act.pass_key}
    else:
        assert isinstance(act, Replace)
        deleted = before[len(before) - act.delete_before:] if act.delete_before else ""
        out = {"type": "replace", "delete_before_utf16": utf16_len(deleted),
               "insert": act.insert, "pass_key": act.pass_key}
    return json.dumps(out, ensure_ascii=False)
