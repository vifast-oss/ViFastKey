"""Test bảng mã legacy (bảng lấy từ bamboo-core MIT, xem THIRD_PARTY_NOTICES)."""
import pytest

from vfk_core import (
    CHARSET_NAMES,
    charsets,
    convert_charset,
    decode_bytes,
    decode_charset,
    detect_charset,
    encode_bytes,
    encode_charset,
    normalize_charset,
    remove_diacritics,
)

TEXT = "Tiếng Việt: Đắk Lắk, 2024!"


@pytest.mark.parametrize("charset", charsets())
def test_roundtrip(charset):
    encoded = encode_charset(TEXT, charset)
    assert decode_charset(encoded, charset) == TEXT


def test_known_viqr():
    assert encode_charset("tiếng Việt", "viqr") == "tie^'ng Vie^.t"
    assert decode_charset("tie^'ng Vie^.t", "viqr") == "tiếng Việt"


def test_viqr_escapes_punctuation():
    encoded = encode_charset("mưa.", "viqr")
    assert encoded == "mu+a\\."
    assert decode_charset(encoded, "viqr") == "mưa."


def test_known_tcvn3_bytes():
    assert encode_bytes("Tiếng Việt", "tcvn3").hex() == "5469d56e67205669d674"
    assert decode_bytes(bytes.fromhex("5469d56e67205669d674"), "tcvn3") == "Tiếng Việt"


def test_ncr_and_cstring():
    assert encode_charset("Việt", "ncr_decimal") == "Vi&#7879;t"
    assert encode_charset("Việt", "ncr_hex") == "Vi&#x1EC7;t"
    assert decode_charset("Vi&#7879;t", "ncr_decimal") == "Việt"
    assert decode_charset("Vi&#x1EC7;t", "ncr_hex") == "Việt"
    assert decode_charset(encode_charset("Việt", "cstring_hex"), "cstring_hex") == "Việt"


def test_cp1258_roundtrip():
    encoded = encode_charset("Việt", "cp1258")
    assert encoded != "Việt"
    assert decode_charset(encoded, "cp1258") == "Việt"


def test_aliases_and_errors():
    assert normalize_charset("VNI") == normalize_charset("vni windows")
    assert normalize_charset("ABC") == "tcvn3"
    with pytest.raises(ValueError):
        normalize_charset("khong-co")
    with pytest.raises(ValueError):
        encode_bytes("Việt", "viqr")


def test_detect():
    raw = encode_charset("tiếng Việt", "tcvn3")
    assert "tcvn3" in detect_charset(raw)
    assert detect_charset("Vi&#7879;t")[0] == "ncr_decimal"


def test_change_case():
    from vfk_core import change_case
    assert change_case("tiếng Việt", "upper") == "TIẾNG VIỆT"
    assert change_case("Tiếng Việt", "lower") == "tiếng việt"
    assert change_case("tiếng việt", "title") == "Tiếng Việt"
    with pytest.raises(ValueError):
        change_case("x", "bogus")


def test_convert_and_remove_diacritics():
    assert convert_charset("Việt", "unicode", "viqr") == "Vie^.t"
    assert convert_charset("Vie^.t", "viqr", "unicode") == "Việt"
    assert remove_diacritics("Tiếng Việt Đắk Lắk") == "Tieng Viet Dak Lak"


def test_all_names_are_unique():
    assert len(CHARSET_NAMES) == len(set(CHARSET_NAMES))
    assert "tcvn3" in CHARSET_NAMES and "viqr" in CHARSET_NAMES
