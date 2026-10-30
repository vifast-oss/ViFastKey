"""Lõi adapter IBus (Linux), tách khỏi PyGObject để test được trên macOS.

`ibus_engine.py` chỉ làm nhiệm vụ cầu nối IBus <-> Action; mọi hành vi tiếng Việt
nằm trong engine Python (hoặc có thể thay bằng C ABI/UniFFI).
"""
from __future__ import annotations

from pathlib import Path

from vfk_adapter_core import SurroundingSnapshot
from vfk_engine import Config, FieldKind, ImeSession, KeyEvent, OutputMode, Ports

from . import JsonFileConfigStore, MemoryConfigStore, WordListDictionary


class _Surrounding:
    def __init__(self) -> None:
        self.text: str | None = None
        self.cursor: int = 0
        self.anchor: int = 0

    def set(self, text: str | None, cursor: int | None = None,
            anchor: int | None = None) -> None:
        self.text = text
        self.cursor = len(text) if cursor is None and text is not None else (cursor or 0)
        self.anchor = self.cursor if anchor is None else anchor

    def before_cursor(self, n: int) -> str | None:
        if self.text is None:
            return None
        cursor = max(0, min(self.cursor, len(self.text)))
        return self.text[:cursor][-n:]

    def snapshot(self) -> SurroundingSnapshot | None:
        if self.text is None:
            return None
        return SurroundingSnapshot(self.text, self.cursor, self.anchor)


class IbusCore:
    """Chuyển phím/ngữ cảnh thành `Action`, không phụ thuộc GI/IBus."""

    def __init__(self, *, config_path: str | Path | None = None,
                 config_store=None, dictionary: WordListDictionary | None = None,
                 mode: OutputMode = OutputMode.PREEDIT) -> None:
        path = Path(config_path).expanduser() if config_path else None
        self._config_path = path
        self._config_mtime = self._mtime()
        self.store = config_store or (
            JsonFileConfigStore(path) if path else MemoryConfigStore(Config())
        )
        dictionary = dictionary or WordListDictionary()
        user = path.parent / "user_words.txt" if path else None
        if user:
            dictionary.load_user_file(user, vietnamese=True)
        self._surrounding = _Surrounding()
        self.session = ImeSession(
            Ports(self.store, _StaticApp(), self._surrounding, dictionary), mode
        )
        self.field_kind = FieldKind.TEXT

    def _mtime(self):
        if self._config_path is None:
            return None
        try:
            return self._config_path.stat().st_mtime_ns
        except OSError:
            return None

    def _reload_config_if_changed(self) -> None:
        """UI cấu hình chạy process khác; poll mtime, không reload giữa từ."""
        mtime = self._mtime()
        if mtime == self._config_mtime:
            return
        if self.session.display:
            return  # thử lại khi từ hiện tại kết thúc
        self._config_mtime = mtime
        self.session.reload_config()

    @property
    def display(self) -> str:
        return self.session.display

    @property
    def profile(self):
        return self.session.profile

    @property
    def surrounding_snapshot(self) -> SurroundingSnapshot | None:
        return self._surrounding.snapshot()

    def set_surrounding(self, text: str | None, cursor: int | None = None,
                        anchor: int | None = None) -> None:
        self._surrounding.set(text, cursor, anchor)

    def set_field_kind(self, kind: FieldKind):
        self.field_kind = kind
        return self.session.set_field_kind(kind)

    def process(self, key: str, ctrl: bool = False, alt: bool = False,
                meta: bool = False):
        self._reload_config_if_changed()
        return self.session.process_key(KeyEvent(key, ctrl=ctrl, alt=alt, meta=meta))

    def focus_in(self, app_id: str | None = None):
        self._reload_config_if_changed()
        self._surrounding.set(None)
        return self.session.focus_changed(app_id)

    def focus_out(self):
        return self.session.focus_changed(None)

    def reset(self) -> None:
        self.session.reset()

    def cursor_moved(self):
        return self.session.cursor_moved()

    def toggle(self):
        return self.session.toggle_vietnamese()


class _StaticApp:
    def current_app(self) -> str | None:
        return None


__all__ = ["IbusCore"]
