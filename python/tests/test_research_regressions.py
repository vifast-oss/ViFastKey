"""Regression cases lấy từ nghiên cứu SKey, UniKey, Telebit, VnKey, GõViệt, Funput.

Mỗi test ghi rõ hành vi mong đợi; không chép code/dữ liệu GPL.
"""
import pytest

from adapters import FakeTextField, MemoryConfigStore, StaticAppContext, WordListDictionary
from vfk_core import TELEX, VNI, TransformOptions, transform
from vfk_engine import Commit, Config, ImeSession, KeyEvent, OutputMode, PassThrough, Ports, Replace

DICT = WordListDictionary()


def _type(mode, keys, config=None):
    field = FakeTextField()
    session = ImeSession(
        Ports(MemoryConfigStore(config or Config()), StaticAppContext("app"), field, DICT), mode
    )
    for key in keys:
        ev = key if isinstance(key, KeyEvent) else KeyEvent(key)
        field.apply(session.process_key(ev), ev)
    return session, field


# ---------------------------------------------------------------- đ / viết tắt
@pytest.mark.parametrize("raw,expected", [
    ("ddc", "đc"),
    ("vcdd", "vcđ"),
    ("nddm", "nđm"),
    ("kdd", "kđ"),
    ("cmdd", "cmđ"),
    ("avcdd", "avcđ"),
    ("NDD", "NĐ"),
    ("NDd", "NĐ"),
    ("ddi", "đi"),
])
def test_d_stroke_shortcuts(raw, expected):
    assert transform(raw, TELEX, TransformOptions(), final=True).text == expected


@pytest.mark.parametrize("raw", ["add", "address", "daddy", "sudden", "riddle", "ladder"])
def test_d_shortcuts_do_not_corrupt_english(raw):
    assert transform(raw, TELEX, TransformOptions(), final=True).text == raw


def test_triple_d_escapes_to_literal_dd():
    assert transform("DDD", TELEX, TransformOptions(), final=True).text == "DD"


# ---------------------------------------------------------------- English / undo
@pytest.mark.parametrize("raw", [
    "banana", "Canada", "coffee", "cheese", "daddy", "airport", "mission",
    "database", "necessary", "paragraph", "acacia", "address", "toffee",
])
def test_english_words_keep_every_letter(raw):
    assert transform(raw, TELEX, TransformOptions(), final=True).text == raw


def test_single_key_undo_is_preserved():
    assert transform("ass", TELEX, TransformOptions(), final=True).text == "as"
    assert transform("aaa", TELEX, TransformOptions(), final=True).text == "aa"
    assert transform("aaan", TELEX, TransformOptions(), final=True).text == "aan"
    assert transform("chuss", TELEX, TransformOptions(), final=True).text == "chus"


# ---------------------------------------------------------------- VIQR
@pytest.mark.parametrize("raw,expected", [
    ("vie^t.", "việt"),
    ("tie^ng'", "tiếng"),
    ("dda^y'", "đấy"),
    ("hoa?", "hỏa"),
    ("mua+", "mưa"),
    ("tu+o+", "tươ"),
])
def test_viqr_words(raw, expected):
    from vfk_core import VIQR
    assert transform(raw, VIQR, TransformOptions(), final=True).text == expected


def test_viqr_engine_word_boundary():
    field = FakeTextField()
    session = ImeSession(
        Ports(MemoryConfigStore(Config(input_method="viqr")), StaticAppContext("app"), field, DICT),
        OutputMode.REPLACE,
    )
    field.type(session, "tie^ng' ")
    assert field.text == "tiếng "


# ---------------------------------------------------------------- VNI số
@pytest.mark.parametrize("raw,expected", [
    ("nam2024", "nam2024"),
    ("ba10", "ba10"),
    ("top10", "top10"),
    ("2024", "2024"),
    ("a10", "a10"),
    ("a11", "a1"),
    ("viet65", "việt"),
    ("nguye64n", "nguyễn"),
    ("toa10n", "toan"),
])
def test_vni_literal_numbers(raw, expected):
    assert transform(raw, VNI, TransformOptions(), final=True).text == expected


# ---------------------------------------------------------------- nguyên âm đơn / uơ
@pytest.mark.parametrize("raw,expected", [
    ("aw", "ă"), ("aws", "ắ"), ("aa", "â"), ("aas", "ấ"),
    ("ee", "ê"), ("ees", "ế"), ("oo", "ô"), ("oos", "ố"),
    ("uw", "ư"), ("uws", "ứ"), ("ow", "ơ"), ("ows", "ớ"),
])
def test_standalone_shaped_vowels_are_kept(raw, expected):
    assert transform(raw, TELEX, TransformOptions(), final=True).text == expected


@pytest.mark.parametrize("raw,expected", [
    ("duow", "dươ"), ("duowf", "dườ"), ("tuowf", "tườ"),
    ("luowj", "lượ"), ("nguow", "ngươ"), ("phuow", "phươ"),
    ("thuow", "thuơ"), ("huow", "huơ"), ("uow", "uơ"), ("quow", "quơ"),
])
def test_open_uo_depends_on_onset(raw, expected):
    assert transform(raw, TELEX, TransformOptions(), final=True).text == expected


# ---------------------------------------------------------------- engine
@pytest.mark.parametrize("mode", [OutputMode.REPLACE, OutputMode.PREEDIT])
def test_engine_is_sticky_after_restore(mode):
    _, field = _type(mode, "texts")
    assert field.text == "texts"


@pytest.mark.parametrize("mode", [OutputMode.REPLACE, OutputMode.PREEDIT])
def test_engine_restores_english_before_word_end(mode):
    _, field = _type(mode, "coffees ")
    assert field.text == "coffees "


@pytest.mark.parametrize("mode", [OutputMode.REPLACE, OutputMode.PREEDIT])
def test_escape_does_not_forward_the_key(mode):
    session, _ = _type(mode, "vieejt")
    action = session.process_key(KeyEvent("Escape"))
    if mode is OutputMode.REPLACE:
        assert isinstance(action, Replace) and action.pass_key is False
    else:
        assert isinstance(action, Commit) and action.pass_key is False


@pytest.mark.parametrize("mode", [OutputMode.REPLACE, OutputMode.PREEDIT])
def test_enter_still_forwards_the_key(mode):
    session, _ = _type(mode, "vieejt")
    action = session.process_key(KeyEvent("Enter"))
    if mode is OutputMode.REPLACE:
        assert isinstance(action, PassThrough) or (
            isinstance(action, Replace) and action.pass_key is True
        )
    else:
        assert isinstance(action, Commit) and action.pass_key is True
