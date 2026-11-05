"""vfk-ffi: lớp ranh giới cho C ABI / UniFFI / WASM (bản Python mô phỏng).

Đây là API phẳng kiểu C: handle số nguyên + JSON. Đơn vị xoá được đổi sang UTF-16
(Windows, macOS, Android, iOS, JS đều đếm UTF-16), adapter không phải tự đổi.

Bản Python dùng để thử nghiệm adapter và đối chiếu với C ABI thật trong `rust/`.
Mọi hàm đều nhận handle; handle sai/đã free được xử lý như session rỗng.
"""
from __future__ import annotations

import json
from dataclasses import asdict, dataclass

from vfk_core import CHARSET_NAMES, charsets, decode_charset, encode_bytes, encode_charset, normalize_charset
from vfk_engine import (
    AppProfile,
    Commit,
    Config,
    FieldKind,
    ImeSession,
    KeyEvent,
    OutputMode,
    PassThrough,
    Ports,
    Preedit,
    config_from_dict,
    config_to_dict,
    resolve_profile,
)

VERSION = "0.1.2"


def utf16_len(s: str) -> int:
    return len(s.encode("utf-16-le")) // 2


# ---------------------------------------------------------------- cấu hình
def merge_config_json(c: Config, patch_json: str) -> Config:
    """Trộn JSON một phần lên cấu hình hiện tại (giống Rust `Config::apply_json_patch`)."""
    patch = json.loads(patch_json)
    if not isinstance(patch, dict):
        raise ValueError("config JSON phải là object")
    base = config_to_dict(c)
    base.update(patch)
    return config_from_dict(base)


# ---------------------------------------------------------------- port proxy
class _ConfigStoreProxy:
    def __init__(self, inner):
        self.inner = inner
        self.version = 0

    def load(self) -> Config:
        return self.inner.load()

    def save(self, config: Config) -> None:
        self.inner.save(config)
        self.version += 1


class _AppContextProxy:
    def __init__(self, inner):
        self.inner = inner
        self.app = inner.current_app()

    def current_app(self) -> str | None:
        return self.app


class _SurroundingProxy:
    def __init__(self, inner):
        self.inner = inner
        self.text: str | None = None
        self.override = False

    def set_text(self, text: str | None) -> None:
        self.text = text
        self.override = True

    def before_cursor(self, n: int) -> str | None:
        if self.override:
            return None if self.text is None else self.text[-n:]
        return self.inner.before_cursor(n)


class _DictionaryProxy:
    def __init__(self, inner):
        self.inner = inner
        self.english: set[str] | None = None
        self.vietnamese: set[str] | None = None

    def is_english(self, word: str) -> bool:
        if self.english is not None:
            return word in self.english
        return self.inner.is_english(word)

    def is_vietnamese(self, word: str) -> bool:
        if self.vietnamese is not None:
            return word in self.vietnamese
        return self.inner.is_vietnamese(word)


@dataclass
class _Handle:
    session: ImeSession
    config: _ConfigStoreProxy
    app: _AppContextProxy
    surrounding: _SurroundingProxy
    dictionary: _DictionaryProxy
    mode: str


_sessions: dict[int, _Handle] = {}
_next = 1


def _handle(h: int) -> _Handle | None:
    return _sessions.get(h)


def _action_json(act, before: str = "") -> str:
    if isinstance(act, PassThrough):
        out = {"type": "pass"}
    elif isinstance(act, Preedit):
        out = {"type": "preedit", "text": act.text,
               "cursor_utf16": utf16_len(act.text[:act.cursor])}
    elif isinstance(act, Commit):
        out = {"type": "commit", "text": act.text, "pass_key": act.pass_key}
    else:
        out = {"type": "replace",
               "delete_before_utf16": _delete_before_utf16(before, act.delete_before),
               "insert": act.insert, "pass_key": act.pass_key}
    return json.dumps(out, ensure_ascii=False)


def vfk_session_new(ports: Ports, mode: str) -> int:
    """Tạo session. `ports` có thể dùng chung; FFI bọc lại để theo dõi cấu hình/surrounding."""
    global _next
    config = _ConfigStoreProxy(ports.config)
    app = _AppContextProxy(ports.app)
    surrounding = _SurroundingProxy(ports.surrounding)
    dictionary = _DictionaryProxy(ports.dictionary)
    wrapped = Ports(config, app, surrounding, dictionary)
    try:
        output_mode = OutputMode(mode)
    except ValueError:
        output_mode = OutputMode.PREEDIT      # giống C ABI: mode lạ -> preedit
    h = _next
    _next += 1
    _sessions[h] = _Handle(ImeSession(wrapped, output_mode), config, app,
                           surrounding, dictionary, mode)
    return h


def vfk_session_free(h: int) -> None:
    _sessions.pop(h, None)


def _delete_before_utf16(before: str, n: int) -> int:
    return utf16_len(before[len(before) - n:]) if n else 0


def vfk_process_key(h: int, key: str, ctrl: bool = False, alt: bool = False,
                    meta: bool = False) -> str:
    """Trả JSON. Với Replace, `delete_before_utf16` tính trên đoạn chữ bị xoá."""
    hd = _handle(h)
    if hd is None:
        return _action_json(PassThrough())
    before = hd.session.display
    act = hd.session.process_key(KeyEvent(key, ctrl, alt, meta))
    return _action_json(act, before)


def vfk_process_char(h: int, codepoint: int, ctrl: bool = False, alt: bool = False,
                     meta: bool = False) -> str:
    """Như `vfk_process_key` nhưng nhận mã Unicode (tiện cho Android/JNI, Win32 WM_CHAR)."""
    if isinstance(codepoint, bool) or not isinstance(codepoint, int):
        return _action_json(PassThrough())
    if not 0 <= codepoint <= 0x10FFFF or 0xD800 <= codepoint <= 0xDFFF:
        return _action_json(PassThrough())
    return vfk_process_key(h, chr(codepoint), ctrl, alt, meta)


def vfk_focus_changed(h: int, app_id: str | None) -> str:
    hd = _handle(h)
    if hd is None:
        return _action_json(PassThrough())
    hd.app.app = app_id
    return _action_json(hd.session.focus_changed(app_id))


def vfk_set_surrounding_text(h: int, before_cursor: str | None) -> None:
    hd = _handle(h)
    if hd is not None:
        hd.surrounding.set_text(before_cursor)


def vfk_cursor_moved(h: int) -> str:
    hd = _handle(h)
    return _action_json(PassThrough()) if hd is None else _action_json(hd.session.cursor_moved())


def _field_kind(kind) -> FieldKind:
    if isinstance(kind, FieldKind):
        return kind
    if isinstance(kind, int):
        return {0: FieldKind.TEXT, 1: FieldKind.PASSWORD, 2: FieldKind.NUMBER,
                3: FieldKind.EMAIL, 4: FieldKind.URL, 5: FieldKind.TERMINAL}.get(kind, FieldKind.TEXT)
    try:
        return FieldKind(str(kind))
    except ValueError:
        return FieldKind.TEXT


def vfk_set_field_kind(h: int, kind) -> str:
    hd = _handle(h)
    if hd is None:
        return _action_json(PassThrough())
    return _action_json(hd.session.set_field_kind(_field_kind(kind)))


def vfk_toggle_vietnamese(h: int) -> str:
    hd = _handle(h)
    return _action_json(PassThrough()) if hd is None else _action_json(hd.session.toggle_vietnamese())


def vfk_is_enabled(h: int) -> bool:
    hd = _handle(h)
    return bool(hd and hd.session.enabled)


def vfk_reset(h: int) -> None:
    hd = _handle(h)
    if hd is not None:
        hd.session.reset()


def vfk_set_config_json(h: int, config_json: str) -> bool:
    """Thay/trộn cấu hình bằng JSON một phần. Trả False nếu JSON sai."""
    hd = _handle(h)
    if hd is None:
        return False
    try:
        c = merge_config_json(hd.session.config, config_json)
    except (TypeError, ValueError, KeyError):
        return False
    hd.config.save(c)
    hd.session.reload_config()
    return True


def vfk_config_json(h: int) -> str:
    hd = _handle(h)
    if hd is None:
        return "{}"
    return json.dumps(config_to_dict(hd.session.config), ensure_ascii=False)


def vfk_config_version(h: int) -> int:
    hd = _handle(h)
    return hd.config.version if hd else 0


def vfk_app_profile_json(h: int) -> str:
    hd = _handle(h)
    profile = resolve_profile(hd.session.config, hd.app.current_app()) if hd else AppProfile()
    d = asdict(profile)
    if profile.output_mode is not None:
        d["output_mode"] = profile.output_mode
    return json.dumps(d, ensure_ascii=False)


def vfk_load_dictionary(h: int, english: str | None = None,
                        vietnamese: str | None = None) -> None:
    hd = _handle(h)
    if hd is None:
        return
    if english is not None:
        hd.dictionary.english = {w.lower() for w in english.split()}
    if vietnamese is not None:
        hd.dictionary.vietnamese = {w.lower() for w in vietnamese.split()}


def vfk_charset_list() -> str:
    """Mỗi dòng `key<TAB>tên` (giống C ABI)."""
    return "\n".join(f"{key}\t{CHARSET_NAMES[key]}" for key in charsets())


def vfk_charset_encode(text: str, charset: str = "unicode") -> str:
    return encode_charset(text, charset)


def vfk_charset_decode(text: str, charset: str = "unicode") -> str:
    return decode_charset(text, charset)


def vfk_charset_encode_bytes(text: str, charset: str) -> bytes:
    return encode_bytes(text, normalize_charset(charset))


def vfk_charset_decode_bytes(data: bytes, charset: str) -> str:
    from vfk_core import decode_bytes
    return decode_bytes(data, normalize_charset(charset))


def vfk_version() -> str:
    return VERSION


__all__ = ["VERSION", "config_from_dict", "config_to_dict", "merge_config_json", "utf16_len", "vfk_app_profile_json",
           "vfk_config_json", "vfk_config_version", "vfk_cursor_moved", "vfk_focus_changed",
           "vfk_is_enabled", "vfk_load_dictionary", "vfk_process_char", "vfk_process_key",
           "vfk_reset", "vfk_session_free", "vfk_session_new", "vfk_set_config_json",
           "vfk_set_field_kind", "vfk_set_surrounding_text", "vfk_toggle_vietnamese", "vfk_version",
           "vfk_charset_list", "vfk_charset_encode", "vfk_charset_decode",
           "vfk_charset_encode_bytes", "vfk_charset_decode_bytes"]
