import json
import random
import unicodedata

import pytest

from adapters import FakeTextField, MemoryConfigStore, StaticAppContext, WordListDictionary
from vfk_core import TELEX, transform
from vfk_engine import (Commit, Config, FieldKind, ImeSession, KeyEvent, OutputMode, PassThrough, Ports,
                        Preedit, Replace)
from vfk_ffi import utf16_len, vfk_process_key, vfk_session_new
from vi_syllable import ToneStyle

MODES = [OutputMode.REPLACE, OutputMode.PREEDIT]
DICT = WordListDictionary()


def make(mode, config=None, app="notepad", text=""):
    field = FakeTextField(text)
    store = MemoryConfigStore(config or Config(macros={"ko": "không", "vn": "Việt Nam"}))
    ports = Ports(store, StaticAppContext(app), field, DICT)
    return ImeSession(ports, mode), field, store


BS = "Backspace"


# ---------------------------------------------------------------- gõ cơ bản
@pytest.mark.parametrize("mode", MODES)
def test_sentence(mode):
    s, f, _ = make(mode)
    f.type(s, "Tieengs Vieejt raast hay, nguwowif Vieejt thif thuowng nhau. ")
    assert f.text == "Tiếng Việt rất hay, người Việt thì thương nhau. "


@pytest.mark.parametrize("mode", MODES)
def test_vni_sentence(mode):
    s, f, _ = make(mode, Config(input_method="vni"))
    f.type(s, "Vie65t Nam, nguo7i2 d9i ho5c 2 na8m. ")
    assert f.text == "Việt Nam, người đi học 2 năm. "


@pytest.mark.parametrize("mode", MODES)
def test_new_tone_style(mode):
    s, f, _ = make(mode, Config(tone_style=ToneStyle.NEW))
    f.type(s, "hoaf bifnh thuyr ")
    assert f.text == "hoà bình thuỷ "


# ---------------------------------------------------------------- backspace
@pytest.mark.parametrize("mode", MODES)
def test_backspace_deletes_visible_char_keeps_marks(mode):
    s, f, _ = make(mode)
    f.type(s, list("tieengs") + [BS])
    assert f.text == "tiến"
    f.type(s, list("g "))
    assert f.text == "tiếng "


@pytest.mark.parametrize("mode", MODES)
def test_backspace_then_change_tone(mode):
    s, f, _ = make(mode)
    f.type(s, list("nguwowif") + [BS])
    assert f.text == "ngườ"
    f.type(s, list("ix "))
    assert f.text == "ngưỡi "        # thanh mới thay thanh cũ


@pytest.mark.parametrize("mode", MODES)
def test_backspace_into_literal_word(mode):
    s, f, _ = make(mode)
    f.type(s, list("text") + [BS])
    assert f.text == "tex"          # không được biến thành 'tẽ'
    f.type(s, list("t "))
    assert f.text == "text "


@pytest.mark.parametrize("mode", MODES)
def test_backspace_whole_word_then_more(mode):
    s, f, _ = make(mode)
    f.type(s, list("abc ") + list("vieejt") + [BS] * 3)
    assert f.text == "abc v"
    f.type(s, [BS, BS])
    assert f.text == "abc"


@pytest.mark.parametrize("mode", MODES)
def test_backspace_nfd(mode):
    s, f, _ = make(mode, Config(unicode_form="NFD"))
    f.type(s, list("vieejt") + [BS])
    assert f.text == unicodedata.normalize("NFD", "việ")


# ---------------------------------------------------------------- kết thúc từ
@pytest.mark.parametrize("mode", MODES)
def test_macro_and_case(mode):
    s, f, _ = make(mode)
    f.type(s, "ko Ko KO vn ")
    assert f.text == "không Không KHÔNG Việt Nam "


@pytest.mark.parametrize("mode", MODES)
def test_dictionary_restore(mode):
    s, f, _ = make(mode)
    f.type(s, "data her mix this did ")
    # data->dât, her->hẻ, mix->mĩ không phải từ Việt thông dụng -> khôi phục
    # this->thí, did->đi là từ Việt -> giữ tiếng Việt
    assert f.text == "data her mix thí đi "


@pytest.mark.parametrize("mode", MODES)
def test_spell_restore_without_dictionary(mode):
    s, f, _ = make(mode, Config(auto_restore_english=False))
    f.type(s, "text law windows boss ")
    assert f.text == "text law windows bos "     # 'ss' = hoàn tác; không có từ điển thì giữ 'bos'


@pytest.mark.parametrize("mode", MODES)
def test_double_key_english_restored_by_dictionary(mode):
    s, f, _ = make(mode)
    f.type(s, "server boss pass miss ass ")
    assert f.text == "server boss pass miss as "


@pytest.mark.parametrize("mode", MODES)
def test_escape_restores_raw(mode):
    s, f, _ = make(mode)
    f.type(s, list("vieejt") + ["Escape"] + list(" "))
    assert f.text == "vieejt "


@pytest.mark.parametrize("mode", MODES)
def test_enter_and_punctuation(mode):
    s, f, _ = make(mode)
    f.type(s, list("xin chaof") + ["Enter"] + list("ddi ddaau?"))
    assert f.text == "xin chào\nđi đâu?"


@pytest.mark.parametrize("mode", MODES)
def test_auto_capitalize(mode):
    s, f, _ = make(mode, Config(auto_capitalize=True))
    f.type(s, list("xin chaof. ddi ddaau? vaang") + ["Enter"] + list("oong ok"))
    assert f.text == "xin chào. Đi đâu? Vâng\nÔng ok"


@pytest.mark.parametrize("mode", MODES)
def test_ctrl_and_navigation_break_word(mode):
    s, f, _ = make(mode, Config(edit_previous_word=False))
    f.type(s, list("tieeng") + ["Left"] + list("s"))
    assert f.text == "tiêngs"


# ---------------------------------------------------------------- ngữ cảnh
def test_replace_mode_plain_letters_pass_through():
    s, _, _ = make(OutputMode.REPLACE)
    assert isinstance(s.process_key(KeyEvent("v")), PassThrough)
    assert isinstance(s.process_key(KeyEvent("i")), PassThrough)


def test_preedit_commit_on_focus_change():
    s, f, _ = make(OutputMode.PREEDIT)
    f.type(s, "vieejt")
    act = s.focus_changed("chrome")
    assert act == Commit("việt")


def test_excluded_app():
    s, f, _ = make(OutputMode.REPLACE, Config(excluded_apps=["vim"]), app="vim")
    f.type(s, "vieejt")
    assert f.text == "vieejt"


def test_toggle_remembered_per_app():
    s, f, store = make(OutputMode.REPLACE)
    s.toggle_vietnamese()
    assert store.load().app_modes["notepad"] is False
    s.focus_changed("chrome")
    assert s.enabled
    s.focus_changed("notepad")
    assert not s.enabled


def test_edit_previous_word():
    s, f, _ = make(OutputMode.REPLACE, text="tôi học tiêng")
    f.type(s, "s")
    assert f.text == "tôi học tiếng"


def test_edit_previous_word_invalid_key_keeps_word():
    s, f, _ = make(OutputMode.REPLACE, text="Việt")
    f.type(s, "f")
    assert f.text == "Việtf"


def test_edit_previous_word_ignores_english():
    s, f, _ = make(OutputMode.REPLACE, text="hello")
    f.type(s, "s")
    assert f.text == "hellos"


@pytest.mark.parametrize("mode", MODES)
def test_nfd_output(mode):
    s, f, _ = make(mode, Config(unicode_form="NFD"))
    f.type(s, "Vieejt Nam ")
    assert f.text == unicodedata.normalize("NFD", "Việt Nam ")


# ---------------------------------------------------------------- FFI
def test_ffi_utf16():
    _, f, store = make(OutputMode.REPLACE)
    h = vfk_session_new(Ports(store, StaticAppContext("x"), f, DICT), "replace")
    for k in "tieeng":
        vfk_process_key(h, k)
    out = json.loads(vfk_process_key(h, "s"))
    assert out == {"type": "replace", "delete_before_utf16": 3, "insert": "ếng", "pass_key": False}


def test_ffi_utf16_nfd():
    store = MemoryConfigStore(Config(unicode_form="NFD"))
    h = vfk_session_new(Ports(store, StaticAppContext("x"), FakeTextField(), DICT), "replace")
    for k in "vieej":
        vfk_process_key(h, k)
    out = json.loads(vfk_process_key(h, "t"))
    assert out["type"] == "pass"
    out = json.loads(vfk_process_key(h, "Backspace"))
    assert out == {"type": "replace", "delete_before_utf16": 1, "insert": "", "pass_key": False}
    out = json.loads(vfk_process_key(h, "Backspace"))     # việ -> vi: xoá 'ệ' = 3 code unit NFD
    assert out["delete_before_utf16"] == utf16_len(unicodedata.normalize("NFD", "ệ")) == 3


# ---------------------------------------------------------------- property / bất biến
KEYS = list("aeiouydwsfrxjzngthcAWS ,.") + [BS, BS, "Escape", "Enter", "Left"]


def test_property_invariants_random_typing():
    """Với chuỗi phím ngẫu nhiên: màn hình luôn khớp session; 2 mô hình output cho cùng kết quả."""
    rnd = random.Random(7)
    for _ in range(1500):
        keys = [rnd.choice(KEYS) for _ in range(rnd.randint(1, 25))]
        texts = []
        for mode in MODES:
            cfg = Config(macros={"ko": "không"}, edit_previous_word=False, restore_after_space=False)
            s, f, _ = make(mode, cfg)
            for k in keys:
                ev = KeyEvent(k)
                f.apply(s.process_key(ev), ev)
                shown = f.preedit if mode is OutputMode.PREEDIT else f.committed[len(f.committed) - len(s.display):]
                assert shown == s.display, (mode, keys)
            f.apply(s.process_key(KeyEvent("Enter")), KeyEvent("Enter"))
            texts.append(f.text)
        assert texts[0] == texts[1], keys


def test_property_type_then_backspace_all():
    rnd = random.Random(3)
    for _ in range(800):
        word = "".join(rnd.choice("aeiouydwsfrxjngthc") for _ in range(rnd.randint(1, 8)))
        for mode in MODES:
            s, f, _ = make(mode, Config(edit_previous_word=False))
            f.type(s, word)
            assert f.text == transform(word, TELEX).text
            n = len(s.display)
            f.type(s, [BS] * n)
            assert f.text == "", (mode, word)


def test_property_soft_reset_keeps_screen_in_sync():
    rnd = random.Random(11)
    for _ in range(1500):
        keys = [rnd.choice(KEYS + [BS, " "]) for _ in range(rnd.randint(1, 25))]
        s, f, _ = make(OutputMode.REPLACE, Config(edit_previous_word=False))
        for k in keys:
            ev = KeyEvent(k)
            f.apply(s.process_key(ev), ev)
            assert f.committed.endswith(s.display), keys


# ---------------------------------------------------------------- edge case từ các dự án khác
def test_soft_reset_backspace_after_space_then_retone():
    """vnkey soft_reset / fcitx5-unikey #50: 'Chào' + Space + Backspace + f -> hoàn tác thanh -> 'Chaof'."""
    s, f, _ = make(OutputMode.REPLACE, Config(edit_previous_word=False))
    f.type(s, list("Chaof ") + [BS] + list("f"))
    assert f.text == "Chaof"


def test_soft_reset_add_tone_after_space():
    s, f, _ = make(OutputMode.REPLACE, Config(edit_previous_word=False))
    f.type(s, list("tieeng ") + [BS] + list("s "))
    assert f.text == "tiếng "


def test_soft_reset_not_after_macro():
    s, f, _ = make(OutputMode.REPLACE, Config(macros={"ko": "không"}, edit_previous_word=False))
    f.type(s, list("ko ") + [BS] + list("s"))
    assert f.text == "khôngs"


@pytest.mark.parametrize("mode", MODES)
def test_modifier_alone_does_not_break_word(mode):
    """vnkey macOS: nhấn Option/Shift riêng không được reset từ."""
    s, f, _ = make(mode)
    f.type(s, list("tieeng") + ["Shift", "Option"] + list("s "))
    assert f.text == "tiếng "


@pytest.mark.parametrize("mode", MODES)
def test_password_field_disabled(mode):
    """fcitx5-lotus: ô mật khẩu không được biến đổi."""
    s, f, _ = make(mode)
    s.set_field_kind(FieldKind.PASSWORD)
    f.type(s, "aaas")
    assert f.text == "aaas"


def test_cursor_moved_commits_preedit():
    """fcitx5-unikey #53 / ibus-bamboo #607: click chuột phải chốt preedit, không để treo."""
    s, f, _ = make(OutputMode.PREEDIT)
    f.type(s, "vieejt")
    f.apply(s.cursor_moved())
    assert f.committed == "việt" and f.preedit == ""
    f.type(s, "s")
    assert f.text == "việts"


@pytest.mark.parametrize("mode", MODES)
def test_smart_caps(mode):
    s, f, _ = make(mode, Config(smart_caps=True))
    f.type(s, "HTTPS JSON DDI Vieejt ")
    assert f.text == "HTTPS JSON ĐI Việt "


@pytest.mark.parametrize("mode", MODES)
def test_uppercase_ww(mode):
    """vnkey #16: WW -> W (không ra w thường)."""
    s, f, _ = make(mode)
    f.type(s, "WW ")
    assert f.text == "W "


def test_edit_previous_word_skips_when_normal_form_differs():
    """Màn hình NFC nhưng đầu ra NFD: không được nạp lại (nếu không sẽ xoá sai số ký tự)."""
    s, f, _ = make(OutputMode.REPLACE, Config(unicode_form="NFD"), text="Việt")
    f.type(s, "s")
    assert f.text == "Việt" + "s"


def test_edit_previous_word_nfd_screen():
    nfd = unicodedata.normalize("NFD", "tiêng")
    s, f, _ = make(OutputMode.REPLACE, Config(unicode_form="NFD"), text=nfd)
    f.type(s, "s")
    assert f.text == unicodedata.normalize("NFD", "tiếng")


def test_app_profiles():
    from vfk_engine import resolve_profile
    c = Config()
    assert resolve_profile(c, "/usr/bin/kitty").output_mode == "preedit"
    assert resolve_profile(c, "C:\\Program Files\\Google\\Chrome.exe").wait_ack
    assert resolve_profile(c, "Firefox").edit_previous_word is False
    s, f, _ = make(OutputMode.REPLACE, app="minecraft")
    f.type(s, "vieejt")
    assert f.text == "vieejt"


def test_firefox_profile_disables_edit_previous_word():
    s, f, _ = make(OutputMode.REPLACE, app="firefox", text="tiêng")
    f.type(s, "s")
    assert f.text == "tiêngs"


def test_json_config_roundtrip(tmp_path):
    from adapters import JsonFileConfigStore
    st = JsonFileConfigStore(tmp_path / "c.json")
    c = Config(tone_style=ToneStyle.NEW, macros={"ko": "không"})
    st.save(c)
    assert st.load() == c
