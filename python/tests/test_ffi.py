"""Test lớp FFI phẳng (Python mô phỏng C ABI). Bản Rust có test tương ứng trong vfk-ffi."""
import json

from adapters import FakeTextField, MemoryConfigStore, StaticAppContext, WordListDictionary
from vfk_engine import Config, FieldKind, Ports


def _ports(config=None, text="", app="app", dictionary=None):
    field = FakeTextField(text)
    store = MemoryConfigStore(config or Config())
    return Ports(store, StaticAppContext(app), field, dictionary or WordListDictionary()), field, store


def _make(config=None, text="", app="app", dictionary=None):
    from vfk_ffi import vfk_session_new
    ports, field, store = _ports(config, text, app, dictionary)
    return vfk_session_new(ports, "replace"), field, store


def _act(raw):
    return json.loads(raw)


def test_ffi_process_char_and_utf16():
    from vfk_ffi import vfk_process_char, vfk_session_free
    h, _, _ = _make()
    for ch in "tieeng":
        vfk_process_char(h, ord(ch))
    out = _act(vfk_process_char(h, ord("s")))
    assert out == {"type": "replace", "delete_before_utf16": 3, "insert": "ếng",
                   "pass_key": False}
    vfk_session_free(h)


def test_ffi_invalid_codepoint_is_pass():
    from vfk_ffi import vfk_process_char, vfk_session_free
    h, _, _ = _make()
    for cp in (-1, 0xD800, 0x110000):
        assert _act(vfk_process_char(h, cp)) == {"type": "pass"}
    vfk_session_free(h)


def test_ffi_config_merge_keeps_other_fields():
    from vfk_ffi import vfk_config_json, vfk_process_key, vfk_session_free, vfk_set_config_json
    h, _, _ = _make(Config(macros={"ko": "không"}))
    assert vfk_set_config_json(h, '{"tone_style":"new"}')
    cfg = json.loads(vfk_config_json(h))
    assert cfg["tone_style"] == "new"
    assert cfg["macros"] == {"ko": "không"}
    # 'hoaf' -> hoà theo kiểu đặt dấu mới
    for ch in "hoa":
        vfk_process_key(h, ch)
    out = _act(vfk_process_key(h, "f"))
    assert out["type"] == "replace" and out["insert"] == "à"
    vfk_session_free(h)


def test_ffi_bad_config_and_unknown_keys():
    from vfk_ffi import vfk_config_json, vfk_session_free, vfk_set_config_json
    h, _, _ = _make()
    assert not vfk_set_config_json(h, "{nope")
    assert not vfk_set_config_json(h, '{"tone_style":"không tồn tại"}')
    assert vfk_set_config_json(h, '{"unknown_future_key": 1}')
    assert "unknown_future_key" not in json.loads(vfk_config_json(h))
    vfk_session_free(h)


def test_ffi_field_toggle_and_version():
    from vfk_ffi import (
        vfk_config_json,
        vfk_config_version,
        vfk_is_enabled,
        vfk_process_key,
        vfk_session_free,
        vfk_set_field_kind,
        vfk_toggle_vietnamese,
    )
    h, _, store = _make()
    assert vfk_is_enabled(h)
    vfk_set_field_kind(h, FieldKind.PASSWORD)
    assert _act(vfk_process_key(h, "a")) == {"type": "pass"}
    vfk_set_field_kind(h, 0)
    before = vfk_config_version(h)
    vfk_toggle_vietnamese(h)
    assert not vfk_is_enabled(h)
    assert vfk_config_version(h) == before + 1
    assert json.loads(vfk_config_json(h))["app_modes"]["app"] is False
    # toggle lại để không ảnh hưởng test khác
    vfk_toggle_vietnamese(h)
    vfk_session_free(h)


def test_ffi_focus_and_surrounding_override():
    from vfk_ffi import (
        vfk_cursor_moved,
        vfk_focus_changed,
        vfk_process_key,
        vfk_session_free,
        vfk_set_surrounding_text,
    )
    h, field, _ = _make(text="tôi học tiêng")
    # App kitty dùng preedit, nhưng session vẫn đang REPLACE -> sửa từ trước con trỏ.
    vfk_set_surrounding_text(h, "tôi học tiêng")
    vfk_focus_changed(h, "notepad")
    out = _act(vfk_process_key(h, "s"))
    assert out["type"] == "replace" and out["insert"] == "ếng"
    # Con trỏ đổi chỗ: chốt từ, từ tiếp theo gõ độc lập
    assert _act(vfk_cursor_moved(h)) == {"type": "pass"}
    vfk_session_free(h)


def test_ffi_app_profile_and_dictionary():
    from vfk_ffi import (
        vfk_app_profile_json,
        vfk_focus_changed,
        vfk_load_dictionary,
        vfk_process_key,
        vfk_session_free,
    )
    h, _, _ = _make()
    vfk_focus_changed(h, "/usr/bin/kitty")
    p = json.loads(vfk_app_profile_json(h))
    assert p["output_mode"] == "preedit" and p["edit_previous_word"] is False
    # Nạp từ điển vẫn hợp lệ (dùng cho backspace/gợi ý), nhưng không tự đổi từ Việt.
    vfk_load_dictionary(h, "mix", "")
    for ch in "mix":
        vfk_process_key(h, ch)
    out = _act(vfk_process_key(h, " "))
    assert out == {"type": "pass"}
    vfk_session_free(h)


def test_ffi_charset():
    from vfk_ffi import (
        vfk_charset_decode,
        vfk_charset_decode_bytes,
        vfk_charset_encode,
        vfk_charset_encode_bytes,
        vfk_charset_list,
    )
    assert "tcvn3" in vfk_charset_list()
    assert vfk_charset_encode("Tiếng Việt", "viqr") == "Tie^'ng Vie^.t"
    assert vfk_charset_decode("Tie^'ng Vie^.t", "viqr") == "Tiếng Việt"
    raw = vfk_charset_encode_bytes("Tiếng Việt", "tcvn3")
    assert raw.hex() == "5469d56e67205669d674"
    assert vfk_charset_decode_bytes(raw, "tcvn3") == "Tiếng Việt"


def test_ffi_null_handle_is_safe():
    from vfk_ffi import (
        vfk_config_json,
        vfk_config_version,
        vfk_cursor_moved,
        vfk_focus_changed,
        vfk_is_enabled,
        vfk_load_dictionary,
        vfk_process_char,
        vfk_process_key,
        vfk_reset,
        vfk_session_free,
        vfk_set_config_json,
        vfk_set_field_kind,
        vfk_set_surrounding_text,
        vfk_toggle_vietnamese,
    )
    assert _act(vfk_process_key(99999, "a")) == {"type": "pass"}
    assert _act(vfk_process_char(99999, ord("a"))) == {"type": "pass"}
    assert _act(vfk_focus_changed(99999, "x")) == {"type": "pass"}
    assert _act(vfk_cursor_moved(99999)) == {"type": "pass"}
    assert _act(vfk_set_field_kind(99999, 0)) == {"type": "pass"}
    assert _act(vfk_toggle_vietnamese(99999)) == {"type": "pass"}
    assert not vfk_is_enabled(99999)
    assert vfk_config_json(99999) == "{}"
    assert vfk_config_version(99999) == 0
    assert not vfk_set_config_json(99999, "{}")
    vfk_set_surrounding_text(99999, "x")
    vfk_load_dictionary(99999, "a", "b")
    vfk_reset(99999)
    vfk_session_free(99999)
