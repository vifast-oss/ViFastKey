//! Port của tests/test_syllable.py.
use vi_syllable::*;

const VALID: &[&str] = &[
    "a", "á", "tiếng", "người", "quốc", "quý", "quýt", "quỳnh", "quyết", "giữ", "già", "gì", "gìn",
    "giếng", "khuỷu", "thuở", "huơ", "xoong", "boong", "ngoèo", "ngoẹo", "khuya", "khuyến",
    "nguyễn", "chuyện", "rượu", "hươu", "cứu", "hưu", "tuần", "xoắn", "hoà", "hòa", "thuý", "thúy",
    "khoẻ", "khỏe", "toàn", "toán", "ghế", "nghỉ", "kỹ", "kĩ", "huỳnh", "huých", "huỵch", "tuýp",
    "suýt", "khuếch", "huênh", "ách", "ếch", "ích", "anh", "ênh", "inh", "oanh", "uếch",
];
const INVALID: &[&str] = &["kan", "ka", "ce", "ci", "ge", "gê", "ngi", "ngia", "ghô", "bl", "tẽt", "bàt", "hảc", "quu", "aă"];
const PARTIAL: &[&str] = &[
    "ă", "â", "tiê", "ngh", "ng", "gh", "tr", "đ", "chuyen", "nguoi", "giuong", "uo", "thuo", "q", "qu",
];

fn status_of(w: &str) -> Status {
    let (s, t) = strip_tone(w);
    check(&s, t)
}

#[test]
fn valid() {
    for w in VALID {
        assert_eq!(status_of(w), Status::Valid, "{w}");
    }
}

#[test]
fn invalid() {
    for w in INVALID {
        assert_eq!(status_of(w), Status::Invalid, "{w}");
    }
}

#[test]
fn partial() {
    for w in PARTIAL {
        assert_eq!(status_of(w), Status::Partial, "{w}");
    }
}

const TONE_TABLE: &[(&str, u8, &str, &str)] = &[
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
];

#[test]
fn tone_placement() {
    for (base, tone, old, new) in TONE_TABLE {
        assert_eq!(place_tone(base, *tone, ToneStyle::Old), *old, "{base} old");
        assert_eq!(place_tone(base, *tone, ToneStyle::New), *new, "{base} new");
    }
}

#[test]
fn place_tone_keeps_case_and_replaces() {
    assert_eq!(place_tone("HOÀ", 1, ToneStyle::Old), "HÓA");
    assert_eq!(place_tone("Việt", 0, ToneStyle::Old), "Viêt");
}

#[test]
fn compose_decompose_roundtrip() {
    for ch in "aăâeêioôơuưyđAĂÂEÊIOÔƠUƯYĐ".chars() {
        for t in 0..6u8 {
            if lower(ch) == 'đ' && t != 0 {
                continue;
            }
            let d = decompose_char(ch);
            let c = compose_char(d.base, d.mark, t, ch.is_uppercase());
            assert_eq!(decompose_char(c).tone, t, "{ch} {t}");
        }
    }
}
