"""Adapter mẫu: cài đặt các port + ô nhập giả lập cho cả 2 mô hình Preedit / Replace.

Adapter thật (TSF, IMK, Fcitx5, Android...) có cùng vai trò: nhận phím từ OS,
gọi session.process_key, rồi áp Action lên ô nhập của OS.
"""
from __future__ import annotations

import copy
import json
from dataclasses import asdict
from pathlib import Path

from vfk_engine import (AppProfile, Commit, Config, ImeSession, KeyEvent, PassThrough, Preedit, Replace)
from vi_syllable import ToneStyle


# ---------------------------------------------------------------- port ra
class MemoryConfigStore:
    def __init__(self, config: Config | None = None):
        self._c = config or Config()

    def load(self) -> Config:
        return copy.deepcopy(self._c)

    def save(self, config: Config) -> None:
        self._c = copy.deepcopy(config)


class JsonFileConfigStore:
    def __init__(self, path: str | Path):
        self.path = Path(path)

    def load(self) -> Config:
        if not self.path.exists():
            return Config()
        d = json.loads(self.path.read_text(encoding="utf-8"))
        d["tone_style"] = ToneStyle(d.get("tone_style", "old"))
        if "app_profiles" in d:
            d["app_profiles"] = {k: AppProfile(**v) for k, v in d["app_profiles"].items()}
        return Config(**d)

    def save(self, config: Config) -> None:
        d = asdict(config)
        d["tone_style"] = config.tone_style.value
        self.path.write_text(json.dumps(d, ensure_ascii=False, indent=2), encoding="utf-8")


class StaticAppContext:
    def __init__(self, app: str | None = None):
        self.app = app

    def current_app(self) -> str | None:
        return self.app


DATA = Path(__file__).parent / "data"


def _load_words(name: str) -> set[str]:
    return set((DATA / name).read_text(encoding="utf-8").split())


class WordListDictionary:
    """Port Dictionary dựa trên 2 danh sách từ. Bản thật nên nạp danh sách lớn
    (vd ~7.000 âm tiết tiếng Việt phổ biến + ~20.000 từ tiếng Anh)."""

    def __init__(self, english=None, vietnamese=None):
        self.english = set(english) if english is not None else _load_words("english_common.txt")
        self.vietnamese = set(vietnamese) if vietnamese is not None else _load_words("vietnamese_common.txt")

    def is_english(self, word: str) -> bool:
        return word in self.english

    def is_vietnamese(self, word: str) -> bool:
        return word in self.vietnamese


SetDictionary = WordListDictionary   # tên cũ


# ---------------------------------------------------------------- ô nhập giả lập
class FakeTextField:
    """Mô phỏng ô nhập của OS. Đồng thời là port SurroundingText."""

    def __init__(self, text: str = ""):
        self.committed = text
        self.preedit = ""

    @property
    def text(self) -> str:
        return self.committed + self.preedit

    def before_cursor(self, n: int) -> str | None:
        return self.committed[-n:]

    def _os_default(self, ev: KeyEvent) -> None:
        if ev.ctrl or ev.alt or ev.meta:
            return
        if ev.key == "Backspace":
            self.committed = self.committed[:-1]
        elif ev.key == "Enter":
            self.committed += "\n"
        elif ev.is_char:
            self.committed += ev.key

    def apply(self, act, ev: KeyEvent | None = None) -> None:
        if isinstance(act, PassThrough):
            if ev:
                self._os_default(ev)
        elif isinstance(act, Preedit):
            self.preedit = act.text
        elif isinstance(act, Commit):
            self.committed += act.text
            self.preedit = ""
            if act.pass_key and ev:
                self._os_default(ev)
        elif isinstance(act, Replace):
            if act.delete_before:
                self.committed = self.committed[:-act.delete_before]
            self.committed += act.insert
            if act.pass_key and ev:
                self._os_default(ev)

    def type(self, session: ImeSession, keys) -> "FakeTextField":
        """keys: chuỗi ký tự, hoặc list chứa ký tự / tên phím / KeyEvent."""
        for k in keys:
            ev = k if isinstance(k, KeyEvent) else KeyEvent(k)
            self.apply(session.process_key(ev), ev)
        return self
