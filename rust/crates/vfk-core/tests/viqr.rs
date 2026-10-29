//! VIQR input method: dấu câu là phím chức năng.

use vfk_core::*;

fn t(raw: &str) -> String {
    transform_str(raw, &VIQR, &TransformOptions::default(), true).text
}

#[test]
fn viqr_words() {
    for (raw, expected) in [
        ("vie^t.", "việt"),
        ("tie^ng'", "tiếng"),
        ("dda^y'", "đấy"),
        ("hoa?", "hỏa"),
        ("mua+", "mưa"),
        ("tu+o+", "tươ"),
    ] {
        assert_eq!(t(raw), expected, "{raw}");
    }
}

#[test]
fn viqr_roundtrip_rebuild() {
    for w in ["việt", "tiếng", "đấy", "hỏa"] {
        let raw = to_raw(w, &VIQR).unwrap_or_else(|| panic!("to_raw {w}"));
        assert_eq!(t(&raw), w, "{w}");
    }
}
