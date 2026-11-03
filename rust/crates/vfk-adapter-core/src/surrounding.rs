//! Kiểm tra surrounding text trước khi `Replace`.
//!
//! Đây là phần dễ gây mất chữ nhất của mọi adapter:
//! * app trả snapshot cũ/không đầy đủ;
//! * cursor đang ở giữa từ;
//! * selection đang mở;
//! * emoji/astral làm lệch UTF-16;
//! * NFD làm số codepoint khác số code unit.
//!
//! Toàn bộ logic ở đây là pure Rust, không phụ thuộc Fcitx5/IMK/Android.

/// Snapshot app cung cấp, offset tính bằng UTF-16 code unit (đơn vị native
/// của macOS/iOS/Android/Windows).
#[derive(Debug, Clone, Copy)]
pub struct SurroundingSnapshot<'a> {
    pub text: &'a str,
    pub cursor_utf16: u32,
    pub anchor_utf16: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceCheck {
    /// Phần suffix thực sự cần xoá (UTF-8).
    pub deleted: String,
    /// Số UTF-16 code unit cần xoá (cho NSTextInputClient/Windows/Android).
    pub delete_utf16: u32,
    /// Số Unicode code point cần xoá (cho Fcitx5/IBus `deleteSurroundingText`).
    pub delete_chars: u32,
}

pub fn utf16_len(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

pub fn char_to_utf16_offset(text: &str, chars: u32) -> Option<u32> {
    let mut count = 0u32;
    for (offset, ch) in text.chars().enumerate() {
        if offset as u32 == chars {
            return Some(count);
        }
        count += ch.len_utf16() as u32;
    }
    (chars as usize == text.chars().count()).then_some(count)
}

pub fn utf16_to_char_offset(text: &str, units: u32) -> Option<u32> {
    let mut used = 0u32;
    let mut chars = 0u32;
    for ch in text.chars() {
        if used == units {
            return Some(chars);
        }
        let width = ch.len_utf16() as u32;
        if used + width > units {
            return None; // cắt giữa surrogate pair
        }
        used += width;
        chars += 1;
    }
    (used == units).then_some(chars)
}

/// Đổi offset theo codepoint (char) sang byte UTF-8.
pub fn char_to_byte_offset(text: &str, chars: u32) -> Option<usize> {
    let mut count = 0u32;
    for (offset, _) in text.char_indices() {
        if count == chars {
            return Some(offset);
        }
        count += 1;
    }
    (count == chars).then_some(text.len())
}

pub fn utf16_to_byte_offset(text: &str, units: u32) -> Option<usize> {
    let mut used = 0u32;
    for (offset, ch) in text.char_indices() {
        if used == units {
            return Some(offset);
        }
        let width = ch.len_utf16() as u32;
        if used + width > units {
            return None;
        }
        used += width;
    }
    (used == units).then_some(text.len())
}

pub fn utf16_prefix(text: &str, units: u32) -> Option<&str> {
    let offset = utf16_to_byte_offset(text, units)?;
    Some(&text[..offset])
}

pub fn utf16_suffix(text: &str, units: u32) -> Option<&str> {
    if units == 0 {
        return Some("");
    }
    let total = utf16_len(text);
    if units > total {
        return None;
    }
    let offset = utf16_to_byte_offset(text, total - units)?;
    // Bảo đảm điểm cắt không rơi vào giữa surrogate pair.
    utf16_to_char_offset(text, total - units)?;
    Some(&text[offset..])
}

/// Lấy suffix theo số **char** (dùng cho `Action::Replace.delete_before`).
pub fn char_suffix(text: &str, chars: u32) -> Option<&str> {
    let total = text.chars().count();
    if chars as usize > total {
        return None;
    }
    let start =
        text.char_indices().nth(total - chars as usize).map(|(offset, _)| offset).unwrap_or(text.len());
    Some(&text[start..])
}

pub fn char_suffix_utf16_len(text: &str, chars: u32) -> Option<u32> {
    char_suffix(text, chars).map(utf16_len)
}

pub fn next_codepoint(text: &str, cursor_utf16: u32) -> Option<char> {
    let offset = utf16_to_byte_offset(text, cursor_utf16)?;
    text[offset..].chars().next()
}

pub fn is_word_codepoint(codepoint: char) -> bool {
    let cp = codepoint as u32;
    if cp < 0x80 {
        return codepoint.is_ascii_alphanumeric() || codepoint == '_';
    }
    // Latin mở rộng + khối tiếng Việt + combining marks.
    (0x00C0..=0x024F).contains(&cp) || (0x0300..=0x036F).contains(&cp) || (0x1E00..=0x1EFF).contains(&cp)
}

/// Xác minh `Replace` có an toàn không.
///
/// `shown_before` là text engine tin là đang hiển thị trước phím hiện tại.
/// Nếu rỗng (trường hợp adopt previous word), dùng phần trước cursor của app.
pub fn check_replace(
    shown_before: &str,
    delete_utf16: u32,
    surrounding: SurroundingSnapshot<'_>,
) -> Option<ReplaceCheck> {
    if delete_utf16 == 0 || surrounding.cursor_utf16 != surrounding.anchor_utf16 {
        return None;
    }

    let before_cursor = utf16_prefix(surrounding.text, surrounding.cursor_utf16)?;
    let shown = if shown_before.is_empty() { before_cursor } else { shown_before };
    let deleted = utf16_suffix(shown, delete_utf16)?;
    if deleted.is_empty() || !before_cursor.ends_with(deleted) {
        return None;
    }

    // Caret ở giữa từ: adopt một prefix cụt sẽ sửa sai phần còn lại.
    if let Some(next) = next_codepoint(surrounding.text, surrounding.cursor_utf16) {
        if is_word_codepoint(next) {
            return None;
        }
    }

    Some(ReplaceCheck {
        deleted: deleted.to_owned(),
        delete_utf16,
        delete_chars: deleted.chars().count() as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &str, cursor: u32, anchor: u32) -> SurroundingSnapshot<'_> {
        SurroundingSnapshot { text, cursor_utf16: cursor, anchor_utf16: anchor }
    }

    #[test]
    fn safe_suffix() {
        let check = check_replace("tiếng", 3, snapshot("tiếng", 5, 5)).unwrap();
        assert_eq!(check.deleted, "ếng");
        assert_eq!(check.delete_utf16, 3);
        assert_eq!(check.delete_chars, 3);
    }

    #[test]
    fn adopt_previous_word_uses_before_cursor() {
        let check = check_replace("", 3, snapshot("tiếng", 5, 5)).unwrap();
        assert_eq!(check.deleted, "ếng");
    }

    #[test]
    fn rejects_stale() {
        assert!(check_replace("tiếng", 3, snapshot("hello", 5, 5)).is_none());
    }

    #[test]
    fn rejects_selection() {
        assert!(check_replace("tiếng", 3, snapshot("tiếng", 5, 2)).is_none());
    }

    #[test]
    fn rejects_mid_word() {
        assert!(check_replace("", 1, snapshot("tiếng", 2, 2)).is_none());
    }

    #[test]
    fn handles_astral() {
        let check = check_replace("🙂a", 3, snapshot("🙂a", 3, 3)).unwrap();
        assert_eq!(check.deleted, "🙂a");
        assert_eq!(check.delete_utf16, 3);
        assert_eq!(check.delete_chars, 2);
        assert_eq!(utf16_suffix("🙂a", 3), Some("🙂a"));
        assert!(utf16_suffix("🙂a", 2).is_none()); // cắt giữa surrogate pair
    }

    #[test]
    fn handles_nfd() {
        let nfd = "vie\u{0323}\u{0302}t"; // việt ở dạng NFD
        let check = check_replace(nfd, 5, snapshot(nfd, 6, 6)).unwrap();
        assert_eq!(check.delete_chars, 5);
        assert_eq!(check.delete_utf16, 5);
    }

    #[test]
    fn char_suffix_and_offsets() {
        assert_eq!(char_suffix("tiếng", 3), Some("ếng"));
        assert_eq!(char_suffix_utf16_len("🙂a", 2), Some(3));
        assert_eq!(char_to_utf16_offset("🙂a", 1), Some(2));
        assert_eq!(char_to_byte_offset("🙂a", 1), Some(4));
        assert_eq!(char_to_byte_offset("🙂a", 2), Some(5));
        assert_eq!(char_to_byte_offset("🙂a", 3), None);
        assert_eq!(utf16_to_char_offset("🙂a", 2), Some(1));
        assert_eq!(utf16_to_char_offset("🙂a", 1), None);
        assert!(is_word_codepoint('ế'));
        assert!(!is_word_codepoint(' '));
    }
}
