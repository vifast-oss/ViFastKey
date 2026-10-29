"""Lõi adapter IBus (Linux), tách khỏi PyGObject để test được trên macOS.

`ibus_engine.py` chỉ làm nhiệm vụ cầu nối IBus <-> Action; mọi hành vi tiếng Việt
nằm trong engine Python (hoặc có thể thay bằng C ABI/UniFFI).
"""
from __future__ import annotations

from pathlib import Path

from vfk_engine import Config, ImeSession, KeyEvent, OutputMode, Ports

from . import JsonFileConfigStore, MemoryConfigStore, WordListDictionary


class _Surrounding:
    def __init__(self) -> None:
        self.text: str | None = None

    def set(self, text: str | None) -> None:
        self.text = text

    def before_cursor(self, n: int) -> str | None:
        if self.text is None:
            return None
        return self.text[-n:]


class IbusCore:
    """Chuyển phím/ngữ cảnh thành `Action`, không phụ thuộc GI/IBus."""

    def __init__(self, *, config_path: str | Path | None = None,
                 config_store=None, dictionary: WordListDictionary | None = None,
                 mode: OutputMode = OutputMode.PREEDIT) -> None:
        path = Path(config_path).expanduser() if config_path else None
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

    def set_surrounding(self, text: str | None) -> None:
        self._surrounding.set(text)

    def process(self, key: str, ctrl: bool = False, alt: bool = False,
                meta: bool = False):
        return self.session.process_key(KeyEvent(key, ctrl=ctrl, alt=alt, meta=meta))

    def focus_in(self, app_id: str | None = None):
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
