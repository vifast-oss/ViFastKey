//! vfk-core: biến đổi phím -> chữ. Thuần, đồng bộ, không I/O.
//!
//! Port nguyên văn bản Python v4 (`vfk_core/`). Bản Python là oracle:
//! mọi khác biệt với `conformance/core.jsonl` là lỗi của bản Rust.

pub mod charset;
pub mod emoji;
pub mod rules;
pub mod transform;

pub use charset::{
    change_case, charset_names, convert as convert_charset, decode as decode_charset, decode_bytes,
    detect as detect_charset, encode as encode_charset, encode_bytes, normalize as normalize_charset,
    remove_diacritics, CHARSETS,
};
pub use emoji::{all_emoji, search as search_emoji};
pub use rules::{quick_telex, InputMethodSpec, MarkRule, SIMPLE_TELEX, TELEX, VIQR, VNI};
pub use transform::{
    rebuild, to_raw, transform, transform_str, Composer, Letter, TransformOptions, TransformResult,
    WordState, MAX_RAW,
};
pub use vi_syllable::{Mark, Status, ToneStyle};

/// Lệnh sửa tối thiểu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Số ký tự (Unicode scalar) cần xoá lùi.
    pub delete_before: usize,
    pub insert: String,
}

/// Giữ tiền tố chung, xoá phần còn lại của `old`, chèn phần còn lại của `new`.
pub fn diff(old: &str, new: &str) -> Edit {
    let o: Vec<char> = old.chars().collect();
    let n: Vec<char> = new.chars().collect();
    let mut p = 0;
    while p < o.len().min(n.len()) && o[p] == n[p] {
        p += 1;
    }
    Edit { delete_before: o.len() - p, insert: n[p..].iter().collect() }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn diff_basic() {
        assert_eq!(diff("tiêng", "tiếng"), Edit { delete_before: 3, insert: "ếng".into() });
        assert_eq!(diff("", "a"), Edit { delete_before: 0, insert: "a".into() });
        assert_eq!(diff("abc", ""), Edit { delete_before: 3, insert: String::new() });
    }

    #[test]
    fn smoke() {
        let o = TransformOptions::default();
        assert_eq!(transform_str("tieengs", &TELEX, &o, false).text, "tiếng");
        assert_eq!(transform_str("thuowng", &TELEX, &o, false).text, "thương");
        assert_eq!(transform_str("thuowr", &TELEX, &o, false).text, "thuở");
        assert_eq!(transform_str("vie65t", &VNI, &o, false).text, "việt");
        assert_eq!(transform_str("text", &TELEX, &o, false).text, "text");
    }
}
