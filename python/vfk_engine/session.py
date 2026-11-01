"""Port vào (driving port): ImeSession. Mọi adapter nền tảng chỉ gọi vào đây.

Bất biến quan trọng (đã có property test):
  * Chữ trên màn hình của từ đang gõ == self.display (sau khi áp mọi Action).
  * Bên trong luôn dùng NFC; chỉ chuẩn hoá NFC/NFD tại đầu ra.
"""
from __future__ import annotations

import unicodedata
from dataclasses import dataclass
from enum import Enum

from vfk_core import INPUT_METHODS, MAX_RAW, TELEX, Composer, TransformOptions, diff, rebuild
from vi_syllable import Status

from .ports import Config, Ports, normalize_app_id, resolve_profile


# ---------------------------------------------------------------- input / output
@dataclass(frozen=True)
class KeyEvent:
    key: str                  # 1 ký tự, hoặc: Backspace Enter Tab Escape Left Right Up Down Home End Delete
    ctrl: bool = False
    alt: bool = False
    meta: bool = False

    @property
    def is_char(self) -> bool:
        return len(self.key) == 1


class FieldKind(Enum):
    TEXT = "text"
    PASSWORD = "password"   # luôn tắt bộ gõ (tránh lộ mật khẩu trong preedit, sai mật khẩu)
    NUMBER = "number"
    EMAIL = "email"
    URL = "url"
    TERMINAL = "terminal"   # adapter nên dùng PREEDIT (commit nguyên tử) cho terminal


DISABLED_FIELDS = {FieldKind.PASSWORD, FieldKind.NUMBER, FieldKind.EMAIL}


class OutputMode(Enum):
    PREEDIT = "preedit"   # TSF, IMK, Fcitx5, IBus, Android, iOS (chữ gạch chân)
    REPLACE = "replace"   # hook + gửi backspace (kiểu UniKey/OpenKey)


@dataclass(frozen=True)
class PassThrough:
    """Engine không xử lý, trả phím cho hệ điều hành."""


@dataclass(frozen=True)
class Preedit:
    text: str
    cursor: int


@dataclass(frozen=True)
class Replace:
    delete_before: int        # đơn vị: code point của chuỗi đầu ra (FFI đổi sang UTF-16)
    insert: str
    pass_key: bool = False    # sau khi sửa, để phím gốc đi tiếp (dấu cách, Enter...)


@dataclass(frozen=True)
class Commit:
    text: str
    pass_key: bool = False


Action = PassThrough | Preedit | Replace | Commit

MODIFIER_KEYS = {"Shift", "Control", "Alt", "Meta", "Super", "CapsLock", "Option", "Command", "Fn"}
NAV_KEYS = {"Tab", "Left", "Right", "Up", "Down", "Home", "End", "Delete", "PageUp", "PageDown"}
SENTENCE_END = set(".!?…")
SENTENCE_CLOSERS = set("\"')]»”’›")


# ---------------------------------------------------------------- session
class ImeSession:
    """Mỗi ô nhập / input context có 1 session. Đồng bộ, không thread, không async."""

    def __init__(self, ports: Ports, mode: OutputMode):
        self.ports = ports
        self.mode = mode
        self.config: Config = ports.config.load()
        self.raw: list[str] = []
        self.word = ""             # NFC, chữ đang hiển thị của từ hiện tại
        self.literal = False       # từ đang ở chế độ thô (sau Backspace vào từ không tái tạo được)
        self.escaped = False       # từng gõ escape đôi phím thanh (pharr, vissa)
        self.cap_next = False      # viết hoa đầu câu
        self._pending_end = False  # vừa gõ dấu kết câu, chờ dấu cách
        self.enabled = True
        self.field = FieldKind.TEXT
        self.last: tuple[list[str], str, bool, str] | None = None   # soft reset: (raw, word, literal, sep)
        self.last_macro: tuple[str, str] | None = None   # (text đã chèn, phím thô cần khôi phục)
        self.history: str = ""            # text đã commit gần đây, cho macro nhiều từ
        self.off_raw: list[str] = []      # phím đang gõ khi tắt tiếng Việt (macro off-mode)
        self.adopted: tuple[str, int] | None = None
        self.adopt_allowed = True
        self.focus_changed(ports.app.current_app())

    # ---------- tiện ích
    @property
    def im(self):
        # Kiểu gõ lạ trong config (file cũ/sửa tay) không được làm panic IME.
        name = self.profile.input_method or self.config.input_method
        return INPUT_METHODS.get(name, TELEX)

    @property
    def _url_safe(self) -> bool:
        return self.field is FieldKind.URL

    @property
    def _macro_on(self) -> bool:
        return self.config.macro_enabled and not self._url_safe and not self.literal

    def _starts_url_segment(self) -> bool:
        return bool(self.history) and self.history[-1] in ".@/:\\-_"

    @property
    def opt(self) -> TransformOptions:
        c = self.config
        tone_style = self.profile.tone_style if self.profile.tone_style is not None else c.tone_style
        spell_check = self.profile.spell_check if self.profile.spell_check is not None else c.spell_check
        url_safe = self._url_safe
        return TransformOptions(tone_style, spell_check, c.w_as_u and not url_safe,
                                c.bracket_keys and not url_safe,
                                c.quick_telex and not url_safe, c.allow_coda_k)

    @property
    def display(self) -> str:
        return self._out(self.word)

    def _out(self, s: str) -> str:
        return unicodedata.normalize(self.config.unicode_form, s)

    def _compute(self, final: bool = False) -> str:
        if self.literal or self._smart_caps_literal():
            return "".join(self.raw)
        comp = self._composer
        if comp is None or comp.raw != self.raw:            # raw bị thay thế (rebuild/soft reset)
            comp = self._composer = Composer(self.im, self.opt, self.raw)
        res = comp.result(final)
        if res.rendered:
            self.escaped = True
        if res.restored:
            self.literal = True                        # dính chế độ thô: text + s -> texts
        if res.restored and self.adopted:              # 'việt' + 'f' -> 'việtf', không phải 'vieetjf'
            word, n = self.adopted
            return word + "".join(self.raw[n:])
        return res.text

    def _smart_caps_literal(self) -> bool:
        r = self.raw
        if not (self.config.smart_caps and len(r) >= 2 and r[0].isupper() and r[1].isupper()):
            return False
        # Các cặp tạo chữ Việt bằng dấu phụ vẫn phải biến đổi, kể cả all-caps:
        # DD → Đ, AA → Â, AW → Ă, EE → Ê, OO → Ô, OW → Ơ, UW → Ư.
        return (r[0], r[1]) not in {
            ("D", "D"), ("A", "A"), ("A", "W"), ("E", "E"),
            ("O", "O"), ("O", "W"), ("U", "W"),
        }

    def reset(self) -> None:
        self.raw, self.word, self.literal, self.escaped = [], "", False, False
        self._composer: Composer | None = None
        self.adopted: tuple[str, int] | None = None   # (từ cũ đã nạp lại, số phím thô của nó)

    def _is_word_char(self, ch: str) -> bool:
        if "a" <= ch.lower() <= "z":
            return True
        if self.raw and ch in self.im.extra_word_keys:
            return True                        # VIQR: ^ ( + ' ` ? ~ . 0 thuộc từ
        if self.config.bracket_keys and ch in self.im.insert_keys:
            return True
        ascii_digit = "0" <= ch <= "9"
        if (self.config.macro_enabled and self.config.macro_digits
                and not self.im.digit_keys and ascii_digit and self.raw):
            return True                        # 'mp3', 'k8s': giữ số trong từ để macro hoạt động
        return self.im.digit_keys and ascii_digit and bool(self.raw)

    # ---------- cấu hình / ngữ cảnh
    def reload_config(self) -> Action:
        act = self._flush()
        self.config = self.ports.config.load()
        self._composer = None
        return act

    def focus_changed(self, app_id: str | None) -> Action:
        act = self._flush()
        self.cap_next = self._pending_end = False
        c = self.config
        key = normalize_app_id(app_id)
        self.profile = resolve_profile(c, app_id)
        if key and key in {normalize_app_id(a) for a in c.excluded_apps}:
            self.enabled = False
        elif self.profile.vietnamese is not None and key not in c.app_modes:
            self.enabled = self.profile.vietnamese
        else:
            self.enabled = c.app_modes.get(key, c.default_vietnamese)
        return act

    def set_field_kind(self, kind: FieldKind) -> Action:
        act = self._flush()
        self.field = kind
        return act

    def cursor_moved(self) -> Action:
        """Adapter gọi khi người dùng click chuột / con trỏ đổi chỗ (bài học fcitx5-unikey #53, ibus-bamboo #607)."""
        self.cap_next = self._pending_end = False
        self.adopt_allowed = False
        return self._flush()

    def toggle_vietnamese(self) -> Action:
        act = self._flush()
        self.enabled = not self.enabled
        app = normalize_app_id(self.ports.app.current_app())
        if app:
            self.config.app_modes[app] = self.enabled
            self.ports.config.save(self.config)
        return act

    def _flush(self) -> Action:
        """Chốt từ đang gõ dở trước khi đổi ngữ cảnh (không áp macro/khôi phục)."""
        text = self.display
        adopted = self.adopted is not None
        self.reset()
        self.last = None
        self.last_macro = None
        self.history = ""
        self.off_raw = []
        if self.mode is OutputMode.PREEDIT and text and not adopted:
            return Commit(text)
        return PassThrough()

    # ---------- xử lý phím
    def process_key(self, ev: KeyEvent) -> Action:
        if self.field in DISABLED_FIELDS:
            return PassThrough()
        if not self.enabled:
            return self._off_mode_key(ev)
        if ev.key in MODIFIER_KEYS:                 # Shift/Option nhấn riêng: không được làm mất từ đang gõ
            return PassThrough()
        if ev.key != "Backspace":
            self.last_macro = None                  # macro undo chỉ có hiệu lực ngay sau macro
        if ev.ctrl or ev.alt or ev.meta or ev.key in NAV_KEYS:
            self.cap_next = self._pending_end = False
            self.last = None
            return self._boundary_passthrough()
        if ev.key == "Backspace":
            return self._backspace()
        if ev.key == "Escape":
            return self._escape()
        if ev.key == "Enter":
            act = self._word_end(None)
            self.cap_next, self._pending_end = self.config.auto_capitalize, False
            return act
        if ev.is_char and self._is_word_char(ev.key):
            return self._letter(ev.key)
        if ev.is_char:
            return self._separator(ev.key)
        return self._boundary_passthrough()

    def _off_mode_key(self, ev: KeyEvent) -> Action:
        """Khi tắt tiếng Việt: vẫn gom phím để mở macro nếu người dùng bật."""
        if ev.key in MODIFIER_KEYS or ev.ctrl or ev.alt or ev.meta or ev.key in NAV_KEYS:
            self.off_raw = []
            self.last_macro = None
            return PassThrough()
        macro_on = self._macro_on and self.config.macro_in_off_mode
        if ev.is_char and macro_on and ("a" <= ev.key.lower() <= "z" or ev.key.isdigit()):
            if len(self.off_raw) < MAX_RAW:
                self.off_raw.append(ev.key)
            return PassThrough()
        if macro_on and self.off_raw and (ev.is_char or ev.key == "Enter"):
            raw = "".join(self.off_raw)
            macro = self._macro_text(raw, raw)
            self.off_raw = []
            if macro is not None and self.mode is OutputMode.REPLACE:
                sep = ev.key if ev.is_char else ""
                self.history = (self.history + macro + sep)[-256:]
                return Replace(len(raw), macro, pass_key=bool(sep) or ev.key == "Enter")
        self.off_raw = []
        return PassThrough()

    def _separator(self, ch: str) -> Action:
        act = self._word_end(ch)
        if ch in SENTENCE_END:
            self._pending_end = True
        elif ch in SENTENCE_CLOSERS:
            pass                                  # ngoặc kép/đóng không làm mất trạng thái câu
        elif ch.isspace():
            if self._pending_end or ch in "\n\r":
                self.cap_next = self.config.auto_capitalize
            self._pending_end = False
        else:
            self._pending_end = self.cap_next = False
        return act

    def _show(self, new: str, typed: str | None = None) -> Action:
        old_out, self.word = self.display, new
        new_out = self.display
        if self.mode is OutputMode.PREEDIT:
            if self.adopted is not None:              # sửa từ đã commit bằng Replace
                e = diff(old_out, new_out)
                return Replace(e.delete_before, e.insert)
            return Preedit(new_out, len(new_out))
        e = diff(old_out, new_out)
        if typed is not None and e.delete_before == 0 and e.insert == typed:
            return PassThrough()                    # chữ thường: để OS tự gõ, đỡ nháy
        return Replace(e.delete_before, e.insert)

    def _show_after_backspace(self, was_adopted: bool) -> Action:
        """Sau Backspace trên từ đã commit: vẫn phải dùng Replace, không preedit.

        Xoá adopted trước khi tính lại để `_compute` không ghép lại từ cũ, rồi
        đặt lại theo chữ mới. Nếu không, Preedit sẽ vẽ thêm ``ViFastKe`` vào
        committed text và thành ``ViFastKeyViFastKe``.
        """
        text = self._compute()
        if was_adopted:
            self.adopted = (text, len(self.raw))
        return self._show(text)

    def _letter(self, key: str) -> Action:
        ch = key
        self.last = None
        if not self.raw:
            if self.cap_next:
                ch = ch.upper()
            self.cap_next = self._pending_end = False
            self._adopt_previous_word()
            self.literal = not self.raw and self._starts_url_segment()
        self.raw.append(ch)
        if self._composer is not None and self._composer.raw == self.raw[:-1]:
            self._composer.push(ch)                         # đường nhanh: tăng dần
        if len(self.raw) > MAX_RAW:
            self.literal = True
        return self._show(self._compute(), typed=ch if ch == key else None)

    def _adopt_previous_word(self) -> None:
        """Con trỏ đứng ngay sau 1 từ đã gõ: nạp lại từ đó để sửa tiếp (vd thêm dấu)."""
        allowed = self.config.edit_previous_word
        if self.profile.edit_previous_word is not None:
            allowed = allowed and self.profile.edit_previous_word
        if not allowed or not self.adopt_allowed:
            return
        before = self.ports.surrounding.before_cursor(MAX_RAW + 1)
        if not before:
            return
        i = len(before)
        while i > 0 and (before[i - 1].isalpha() or unicodedata.combining(before[i - 1])):
            i -= 1
        on_screen = before[i:]
        if not on_screen or self._out(on_screen) != on_screen:
            return            # chữ trên màn hình khác dạng NFC/NFD đầu ra -> số ký tự xoá sẽ sai, bỏ qua
        word = unicodedata.normalize("NFC", on_screen)
        if len(word) > MAX_RAW // 2:
            return
        raw = rebuild(word, self.im, self.opt)
        if raw:
            self.raw, self.word, self.adopted = raw, word, (word, len(raw))

    def _backspace(self) -> Action:
        """Xoá 1 ký tự HIỂN THỊ (tiếng -> tiến), giữ dấu ở phần còn lại như UniKey/OpenKey."""
        if not self.raw:
            self.history = ""
            self.off_raw = []
            if self.config.macro_undo and self.mode is OutputMode.REPLACE and self.last_macro:
                inserted, restored = self.last_macro
                self.last_macro = None
                before = self.ports.surrounding.before_cursor(len(inserted))
                if before is not None and self._out(before) == inserted:
                    return Replace(len(inserted), restored)
            last, self.last = self.last, None
            if (last and self.config.restore_after_space and self.mode is OutputMode.REPLACE
                    and len(last[3]) == 1):
                self.raw, self.word, self.literal = list(last[0]), last[1], last[2]
                self.adopted = None
            return PassThrough()                    # OS tự xoá dấu cách
        target = self.word[:-1]
        if not target:
            act = self._show("")
            self.reset()
            return act
        # Bỏ phím lặp ở cuối từ literal và thử biến đổi phần còn lại.
        # Làm được nhiều lần liên tiếp: nhuwnggg -> BS -> nhuwngg -> BS -> nhưng;
        # textt -> BS -> text (vẫn literal), không biến thành tẽ.
        was_adopted = self.adopted is not None
        if (self.literal and len(self.raw) >= 2
                and self.raw[-1].lower() == self.raw[-2].lower()):
            candidate = list(self.raw[:-1])
            probe = Composer(self.im, self.opt, candidate)
            res = probe.result(False)
            self.adopted = None
            self._composer = None
            if not res.restored:
                self.raw = candidate
                self.literal = False
                self.escaped = False
                return self._show_after_backspace(was_adopted)
            self.raw = candidate
            self.literal = True
            self.escaped = False
            return self._show_after_backspace(was_adopted)
        # Từ đã escape đôi phím thanh phải quay lại được dấu khi xoá và gõ tiếp
        # (phar + rr, xoá 2 r, gõ r -> phả).
        if self.escaped or not self.literal:
            raw = rebuild(target, self.im, self.opt)
        elif ("".join(self.raw) == self.word
              and not self.ports.dictionary.is_english(self.word.lower())):
            # Từ literal thường giữ raw, nhưng nếu ký tự vừa xoá là thứ làm từ sai
            # chính tả thì phần còn lại có thể đã là tiếng Việt: nhưng + l ->
            # nhuwngl -> Backspace -> nhưng (không phải nhuwng).
            candidate = list(self.raw[:-1])
            res = Composer(self.im, self.opt, candidate).result(False)
            # Không quay lại nếu phần còn lại là âm tiết không hợp lệ, hoặc nếu
            # từ literal là một từ tiếng Anh đã biết: 'text' + Backspace phải
            # giữ 'tex', không được biến thành 'tẽ'. "thoảng" vẫn được khôi
            # phục dù không có trong từ điển common, vì không phải từ tiếng Anh.
            raw = candidate if (not res.restored and res.text != target
                                and res.status != Status.INVALID) else None
        else:
            raw = None
        self.adopted = None
        if raw is not None:
            self.raw, self.literal = raw, False
            self.escaped = False
        else:
            self.raw, self.literal = list(target), True
        return self._show_after_backspace(was_adopted)

    def _escape(self) -> Action:
        """Esc: huỷ biến đổi, trả về đúng phím đã gõ."""
        if not self.raw:
            return PassThrough()
        return self._finish("".join(self.raw), None, pass_key=False)

    def _word_end(self, sep: str | None) -> Action:
        self.last_macro = None
        if not self.raw:
            self.last = None
            if sep is not None:
                self.history = (self.history + sep)[-256:]
            return PassThrough()
        final = self._compute(final=True)
        raw = "".join(self.raw)
        if self._macro_on and self.mode is OutputMode.REPLACE:
            phrase = self.history + final
            hit = self._phrase_macro(phrase, raw)
            if hit is not None:
                key, expansion, prefix = hit
                sep_text = sep or ""
                inserted = expansion + sep_text
                self.last_macro = (inserted, key + sep_text) if sep is not None else None
                self.history = (prefix + inserted)[-256:]
                self.reset()
                self.last = None
                return Replace(len(key), expansion, pass_key=bool(sep))
        macro = self._macro(self.word) or self._macro(raw) if self._macro_on else None
        if macro:
            final = macro
            if sep is not None and self.config.macro_undo:
                self.last_macro = (self._out(macro) + sep, raw + sep)
        return self._finish(final, sep)

    def _finish(self, final: str, sep: str | None, pass_key: bool = True) -> Action:
        old_out = self.display
        final_out = self._out(final)
        if sep is not None and final == self.word:   # chỉ soft reset khi từ không bị macro/khôi phục đổi
            self.last = (list(self.raw), self.word, self.literal, sep)
        else:
            self.last = None
        if sep is not None:
            self.history = (self.history + final_out + sep)[-256:]
        adopted = self.adopted is not None
        self.reset()
        if self.mode is OutputMode.PREEDIT:
            if adopted:                               # từ cũ đã commit: thay bằng Replace
                out = final_out + (sep or "")
                e = diff(old_out, out)
                return Replace(e.delete_before, e.insert, pass_key=pass_key and sep is None)
            if sep is None:
                return Commit(final_out, pass_key=pass_key)
            return Commit(final_out + sep)
        if final_out == old_out:
            return PassThrough()
        e = diff(old_out, final_out)
        return Replace(e.delete_before, e.insert, pass_key=pass_key)

    def _boundary_passthrough(self) -> Action:
        text = self.display
        adopted = self.adopted is not None
        self.reset()
        if self.mode is OutputMode.PREEDIT and text and not adopted:
            return Commit(text, pass_key=True)
        return PassThrough()

    def _macro_text(self, matched: str, key: str) -> str | None:
        """Tra macro theo key; giữ kiểu hoa/thường của đoạn khớp."""
        m = self.config.macros.get(key.lower())
        if not m:
            return None
        if matched.isupper() and len(matched) > 1:
            return m.upper()
        if matched[:1].isupper():
            return m[:1].upper() + m[1:]
        return m

    def _phrase_macro(self, phrase: str, raw: str) -> tuple[str, str, str] | None:
        """Tìm macro dài nhất khớp đuôi câu, có kiểm tra biên từ."""
        low = phrase.lower()
        candidates = []
        for key in self.config.macros:
            k = key.lower()
            if not k or not low.endswith(k):
                continue
            prefix = phrase[: len(phrase) - len(k)]
            if prefix and (prefix[-1].isalnum() or prefix[-1] == "_"
                           or prefix[-1] in ".@/:\\-"):
                continue
            candidates.append((len(k), key, prefix))
        if not candidates:
            return None
        _, key, prefix = max(candidates, key=lambda item: item[0])
        matched = phrase[len(phrase) - len(key):]
        expansion = self._macro_text(matched, key)
        if expansion is None:
            return None
        # Chỉ dùng đường nhiều từ khi key vượt quá từ hiện tại; macro đúng từ
        # hiện tại đã do _word_end xử lý như cũ.
        if len(key) <= len(self.word):
            return None
        return key, expansion, prefix

    def _macro(self, word: str) -> str | None:
        m = self.config.macros.get(word.lower())
        if m is None:
            return None
        if word.isupper() and len(word) > 1:
            return m.upper()
        if word[:1].isupper():
            return m[:1].upper() + m[1:]
        return m
