import pytest

import vi_syllable as vs
from vi_syllable import Status, ToneStyle

VALID = ("a ă á tiếng người quốc quý quýt quỳnh quyết giữ già gì gìn giếng khuỷu thuở huơ xoong boong "
         "ngoèo ngoẹo khuya khuyến nguyễn chuyện rượu hươu cứu hưu tuần xoắn hoà hòa thuý thúy khoẻ khỏe "
         "toàn toán ghế nghỉ kỹ kĩ huỳnh huých huỵch tuýp suýt khuếch huênh ách ếch ích anh ênh inh oanh uếch "
         "đ").split()
VALID.remove("ă")
VALID.remove("đ")
INVALID = "kan ka ce ci ge gê ngi ngia ghô bl tẽt bàt hảc quu aă".split()
PARTIAL = "ă â tiê ngh ng gh tr đ chuyen nguoi giuong uo thuo q qu".split()


@pytest.mark.parametrize("w", VALID)
def test_valid(w):
    s, t = vs.strip_tone(w)
    assert vs.check(s, t) is Status.VALID


@pytest.mark.parametrize("w", INVALID)
def test_invalid(w):
    s, t = vs.strip_tone(w)
    assert vs.check(s, t) is Status.INVALID


@pytest.mark.parametrize("w", PARTIAL)
def test_partial(w):
    s, t = vs.strip_tone(w)
    assert vs.check(s, t) is Status.PARTIAL


# (từ không thanh, thanh, kiểu cũ, kiểu mới) theo quy tắc đặt dấu thanh của chữ Quốc ngữ
TONE_TABLE = [
    ("hoa", 2, "hòa", "hoà"), ("hoe", 2, "hòe", "hoè"), ("thuy", 3, "thủy", "thuỷ"),
    ("khoe", 3, "khỏe", "khoẻ"), ("toan", 2, "toàn", "toàn"), ("khuyu", 3, "khuỷu", "khuỷu"),
    ("thuơ", 3, "thuở", "thuở"), ("chuyên", 5, "chuyện", "chuyện"), ("gia", 2, "già", "già"),
    ("qua", 5, "quạ", "quạ"), ("giưa", 4, "giữa", "giữa"), ("mua", 2, "mùa", "mùa"),
    ("kia", 2, "kìa", "kìa"), ("ngươi", 2, "người", "người"), ("rươu", 5, "rượu", "rượu"),
    ("tuân", 2, "tuần", "tuần"), ("xoăn", 1, "xoắn", "xoắn"), ("nghia", 4, "nghĩa", "nghĩa"),
    ("ngoai", 2, "ngoài", "ngoài"), ("huynh", 2, "huỳnh", "huỳnh"), ("quy", 1, "quý", "quý"),
    ("quyêt", 1, "quyết", "quyết"), ("yêu", 1, "yếu", "yếu"), ("ngoeo", 5, "ngoẹo", "ngoẹo"),
    ("khuya", 1, "khuýa", "khuýa"), ("cưu", 1, "cứu", "cứu"), ("gin", 2, "gìn", "gìn"),
    ("gi", 2, "gì", "gì"), ("tuy", 1, "túy", "tuý"), ("oa", 1, "óa", "oá"),
]


@pytest.mark.parametrize("base,tone,old,new", TONE_TABLE)
def test_tone_placement(base, tone, old, new):
    assert vs.place_tone(base, tone, ToneStyle.OLD) == old
    assert vs.place_tone(base, tone, ToneStyle.NEW) == new


def test_place_tone_keeps_case_and_replaces():
    assert vs.place_tone("HOÀ", 1, ToneStyle.OLD) == "HÓA"
    assert vs.place_tone("Việt", 0) == "Viêt"


def test_compose_decompose_roundtrip():
    for ch in "aăâeêioôơuưyđAĂÂEÊIOÔƠUƯYĐ":
        for t in range(6):
            if ch.lower() == "đ" and t:
                continue
            c = vs.compose_char(*vs.decompose_char(ch)[:2], t, ch.isupper())
            assert vs.decompose_char(c)[2] == t
