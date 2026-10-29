"""Gợi ý từ local: prefix completion từ âm tiết Việt + từ tiếng Anh, không mạng.

Không phải AI/ML; là lexicon nhỏ, chạy O(log n + k) bằng bisect.
"""
from __future__ import annotations

from pathlib import Path

from vfk_core.charset import remove_diacritics

DATA = Path(__file__).parent.parent / "vfk_core" / "data"
DEFAULT_FILES = ("vietnamese_syllables.txt", "english_suggest.txt")


class SuggestionEngine:
    def __init__(self, words=(), *, limit: int = 8):
        self._words: set[str] = set()
        self._sorted: list[str] = []
        self._folded: dict[str, list[str]] = {}
        self._priority: set[str] = set()
        self.limit = limit
        self.add_words(words)

    def add_words(self, words) -> None:
        added = False
        for word in words:
            word = word.strip()
            if word and word not in self._words:
                self._words.add(word)
                added = True
        if added:
            self._sorted = sorted(self._words)
            folded: dict[str, list[str]] = {}
            for word in self._sorted:
                key = remove_diacritics(word).lower()
                folded.setdefault(key, []).append(word)
            self._folded = folded

    def load_file(self, path: str | Path) -> None:
        try:
            self.add_words(Path(path).read_text(encoding="utf-8").split())
        except OSError:
            pass

    def add_priority(self, words) -> None:
        self.add_words(words)
        for word in words:
            word = word.strip()
            if word:
                self._priority.add(word)

    def suggest(self, prefix: str, limit: int | None = None) -> list[str]:
        prefix = prefix.strip().lower()
        if not prefix:
            return []
        # Khớp cả có dấu và không dấu: "tie" gợi ý "tiếng".
        folded = remove_diacritics(prefix).lower()
        out: list[str] = []
        seen: set[str] = set()
        for key in sorted(self._folded):
            if not key.startswith(folded):
                continue
            for word in self._folded[key]:
                if word not in seen:
                    seen.add(word)
                    out.append(word)
        out.sort(key=lambda w: (w not in self._priority, len(w), w))
        return out[: (limit or self.limit)]

    def __len__(self) -> int:
        return len(self._words)


_engine: SuggestionEngine | None = None


def default_engine() -> SuggestionEngine:
    global _engine
    if _engine is None:
        _engine = SuggestionEngine()
        for name in ("vietnamese_common.txt", "english_common.txt"):
            _engine.load_file(DATA / name)
            _engine.add_priority((DATA / name).read_text(encoding="utf-8").split())
        for name in DEFAULT_FILES:
            _engine.load_file(DATA / name)
    return _engine


def suggest(prefix: str, limit: int = 8) -> list[str]:
    return default_engine().suggest(prefix, limit)


__all__ = ["SuggestionEngine", "default_engine", "suggest", "DEFAULT_FILES"]
