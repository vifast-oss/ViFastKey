"""Gợi ý từ local: prefix completion từ âm tiết Việt + từ tiếng Anh, không mạng.

Không phải AI/ML; là lexicon nhỏ, chạy O(log n + k) bằng bisect.
"""
from __future__ import annotations

from bisect import bisect_left
from pathlib import Path

from vfk_core.charset import remove_diacritics

DATA = Path(__file__).parent.parent / "vfk_core" / "data"
DEFAULT_FILES = ("vietnamese_syllables.txt", "english_suggest.txt")


def _osa_distance(a: str, b: str, limit: int) -> int | None:
    """Khoảng cách sửa lỗi (thay/thêm/xoá/hoán vị cạnh nhau), cắt nếu vượt limit."""
    if abs(len(a) - len(b)) > limit:
        return None
    previous_previous = [0] * (len(b) + 1)
    previous = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        current = [i] + [0] * len(b)
        for j, cb in enumerate(b, 1):
            value = min(previous[j] + 1, current[j - 1] + 1, previous[j - 1] + (ca != cb))
            if i > 1 and j > 1 and ca == b[j - 2] and a[i - 2] == cb:
                value = min(value, previous_previous[j - 2] + 1)
            current[j] = value
        previous_previous, previous = previous, current
    distance = previous[-1]
    return distance if distance <= limit else None


class SuggestionEngine:
    def __init__(self, words=(), *, limit: int = 8):
        self._words: set[str] = set()
        self._sorted: list[str] = []
        self._folded: dict[str, list[str]] = {}
        self._keys: list[str] = []
        self._priority: set[str] = set()
        self._priority_folded: list[tuple[str, str]] = []
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
            self._keys = sorted(folded)
            self._rebuild_priority()

    def _rebuild_priority(self) -> None:
        self._priority_folded = sorted(
            (remove_diacritics(word).lower(), word) for word in self._priority
        )

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
        self._rebuild_priority()

    def suggest(self, prefix: str, limit: int | None = None) -> list[str]:
        prefix = prefix.strip().lower()
        if not prefix:
            return []
        limit = limit or self.limit
        # Khớp cả có dấu và không dấu: "tie" gợi ý "tiếng".
        folded = remove_diacritics(prefix).lower()
        out: list[str] = []
        seen: set[str] = set()
        start = bisect_left(self._keys, folded)
        for key in self._keys[start:]:
            if not key.startswith(folded):
                break
            for word in self._folded[key]:
                if word not in seen:
                    seen.add(word)
                    out.append(word)
        if not out:
            return self._fuzzy_suggest(folded, limit)
        out.sort(key=lambda w: (w not in self._priority, len(w), w))
        return out[:limit]

    def _fuzzy_candidates(self, folded: str, max_distance: int) -> list[tuple[int, int, int, str]]:
        candidates: list[tuple[int, int, int, str]] = []
        for key, word in self._priority_folded:
            if abs(len(key) - len(folded)) > max_distance:
                continue
            distance = _osa_distance(folded, key, max_distance)
            if distance is None or distance == 0:
                continue
            first_mismatch = int(folded[:1] != key[:1])
            candidates.append((distance, first_mismatch, len(word), word))
        candidates.sort()
        deduped: list[tuple[int, int, int, str]] = []
        for candidate in candidates:
            if not deduped or deduped[-1][3] != candidate[3]:
                deduped.append(candidate)
        if deduped:
            best = deduped[0][0]
            deduped = [c for c in deduped if c[0] == best]
        return deduped

    def _fuzzy_suggest(self, folded: str, limit: int) -> list[str]:
        if len(folded) < 4:
            return []
        max_distance = 2 if len(folded) >= 7 else 1
        return [c[3] for c in self._fuzzy_candidates(folded, max_distance)[:limit]]

    def autocorrect(self, word: str) -> str | None:
        """Tự sửa khi từ không hợp lệ và có đúng một ứng viên tiếng Việt thông dụng."""
        original = word.strip()
        folded = remove_diacritics(original).lower()
        if len(folded) < 4 or not folded.isalpha():
            return None
        if folded in self._folded:
            return None
        max_distance = 2 if len(folded) >= 7 else 1
        common = _vietnamese_common_folded()
        corrected: str | None = None
        keys: set[str] = set()
        for _, first_mismatch, _, candidate in self._fuzzy_candidates(folded, max_distance):
            if first_mismatch:
                continue
            key = remove_diacritics(candidate).lower()
            if key not in common:
                continue
            prefix = 0
            for a, b in zip(folded, key):
                if a != b:
                    break
                prefix += 1
            if prefix < 2:
                continue
            keys.add(key)
            corrected = candidate
        if len(keys) != 1 or corrected is None:
            return None
        if original and original[0].isupper():
            if len(original) > 1 and original.isupper():
                return corrected.upper()
            return corrected[0].upper() + corrected[1:]
        return corrected

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


def autocorrect(word: str) -> str | None:
    return default_engine().autocorrect(word)


_COMMON_FOLDED: set[str] | None = None


def _vietnamese_common_folded() -> set[str]:
    global _COMMON_FOLDED
    if _COMMON_FOLDED is None:
        try:
            words = (DATA / "vietnamese_common.txt").read_text(encoding="utf-8").split()
        except OSError:
            words = []
        _COMMON_FOLDED = {remove_diacritics(word).lower() for word in words}
    return _COMMON_FOLDED


__all__ = ["SuggestionEngine", "default_engine", "suggest", "autocorrect", "DEFAULT_FILES"]
