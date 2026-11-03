"""Test lớp áp Action của IBus engine mà không cần IBus/GI thật.

`linux/ibus/ibus_engine.py` chỉ là cầu nối; test này giả lập `gi.repository.IBus`
và một `IbusCore` tối thiểu để kiểm tra preedit/commit/replace/fail-safe.
"""
from __future__ import annotations

import importlib.util
import sys
import types
from pathlib import Path
from types import SimpleNamespace

from vfk_adapter_core import SurroundingSnapshot
from vfk_engine import FieldKind, Replace

ROOT = Path(__file__).resolve().parents[2]


def _load_engine_module():
    gi = types.ModuleType("gi")
    gi.require_version = lambda *_args, **_kwargs: None
    repo = types.ModuleType("gi.repository")

    class Text:
        def __init__(self, text: str):
            self.text = text

        @classmethod
        def new_from_string(cls, text: str):
            return cls(text)

    class Engine:
        def __init__(self):
            self.committed = []
            self.preedit = []
            self.hidden = 0
            self.deleted = []

        def commit_text(self, text):
            self.committed.append(text.text)

        def hide_preedit_text(self):
            self.hidden += 1

        def show_preedit_text(self):
            pass

        def update_preedit_text(self, text, cursor, visible):
            self.preedit.append((text.text, cursor, visible))

        def delete_surrounding_text(self, offset, nchars):
            self.deleted.append((offset, nchars))

    ibus = types.ModuleType("IBus")
    ibus.Engine = Engine
    ibus.Text = Text
    ibus.ModifierType = SimpleNamespace(
        RELEASE_MASK=1 << 0,
        CONTROL_MASK=1 << 2,
        SHIFT_MASK=1 << 3,
        MOD1_MASK=1 << 4,
        SUPER_MASK=1 << 5,
        META_MASK=1 << 6,
    )
    for name in (
        "KEY_BackSpace",
        "KEY_Return",
        "KEY_KP_Enter",
        "KEY_Tab",
        "KEY_Escape",
        "KEY_Left",
        "KEY_Right",
        "KEY_Up",
        "KEY_Down",
        "KEY_Home",
        "KEY_End",
        "KEY_Delete",
        "KEY_Page_Up",
        "KEY_Page_Down",
        "KEY_comma",
    ):
        setattr(ibus, name, name)
    ibus.keyval_to_unicode = lambda keyval: None
    repo.IBus = ibus
    gi.repository = repo

    sys.modules["gi"] = gi
    sys.modules["gi.repository"] = repo

    path = ROOT / "linux" / "ibus" / "ibus_engine.py"
    spec = importlib.util.spec_from_file_location("vfk_ibus_engine_test", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def _engine(display: str, field: FieldKind = FieldKind.TEXT, surrounding=None):
    module = _load_engine_module()
    engine = module.ViFastKeyEngine()

    class Core:
        def __init__(self):
            self.display = display
            self.field_kind = field
            self.surrounding_snapshot = surrounding
            self.profile = SimpleNamespace(
                backspace_delay_ms=0, delay_per_char_ms=0, wait_ack=False
            )
            self.reset_called = 0
            self.cursor_moved_called = 0

        def reset(self):
            self.reset_called += 1
            self.display = ""

        def cursor_moved(self):
            self.cursor_moved_called += 1

    engine.core = Core()
    return engine


def test_fail_safe_commits_preedit():
    engine = _engine("a")
    consumed = engine._apply(Replace(delete_before=1, insert="á", pass_key=False), "a")

    # Chữ được chốt, nhưng phím gốc vẫn để app xử lý.
    assert consumed is False
    assert engine.committed == ["a"]
    assert engine.core.reset_called == 1
    assert engine.hidden == 1


def test_fail_safe_does_not_commit_empty():
    engine = _engine("")
    # Replace không có surrounding, display rỗng: không commit gì.
    engine._apply(Replace(delete_before=1, insert="á", pass_key=False), "")

    assert engine.committed == []
    assert engine.core.reset_called == 1


def test_replace_uses_surrounding():
    engine = _engine("Chào", surrounding=SurroundingSnapshot("Chào", 4, 4))
    consumed = engine._apply(Replace(delete_before=2, insert="áo", pass_key=False), "Chào")

    assert consumed is True
    assert engine.deleted == [(-2, 2)]
    assert engine.committed == ["áo"]


def test_replace_falls_back_when_surrounding_stale():
    # Sau khi xoá dấu cách, surrounding còn "cung " (app cập nhật chậm).
    engine = _engine("cung", surrounding=SurroundingSnapshot("cung ", 5, 5))
    consumed = engine._apply(Replace(delete_before=1, insert="ũ", pass_key=False), "cung")

    assert consumed is True
    assert engine.deleted == [(-1, 1)]
    assert engine.committed == ["ũ"]


def test_replace_delete_failure_keeps_composition():
    engine = _engine("Chào", surrounding=SurroundingSnapshot("Chào", 4, 4))

    def fail(*_args, **_kwargs):
        raise RuntimeError("app không nhận delete_surrounding_text")

    engine.delete_surrounding_text = fail
    engine._apply(Replace(delete_before=2, insert="áo", pass_key=False), "Chào")

    # Không xoá được thì chốt từ đang soạn thay vì reset làm mất.
    assert engine.committed == ["Chào"]
    assert engine.core.reset_called == 1


def test_password_passes_through():
    engine = _engine("a", field=FieldKind.PASSWORD)
    consumed = engine._apply(Replace(delete_before=1, insert="á", pass_key=False), "a")

    assert consumed is False
    assert engine.committed == []
    assert engine.hidden == 1


def test_cursor_location_does_not_flush_composition():
    engine = _engine("a")
    engine.do_set_cursor_location(0, 0, 0, 0)
    assert engine.core.cursor_moved_called == 0

    engine.core.display = ""
    engine.do_set_cursor_location(0, 0, 0, 0)
    assert engine.core.cursor_moved_called == 1
