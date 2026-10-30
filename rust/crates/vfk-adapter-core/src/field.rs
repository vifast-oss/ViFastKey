//! Field policy dùng chung.
//!
//! Mỗi port tự map capability/EditorInfo/secure-input của mình sang
//! [`vfk_engine::FieldKind`]; phần quyết định "có xử lý tiếng Việt hay không"
//! nằm ở đây để Fcitx5/macOS/Android/iOS/Windows không lệch nhau.

use vfk_engine::FieldKind;

/// Field không được đưa vào engine (password, số, email).
/// URL/omnibox vẫn cho gõ tiếng Việt (search); engine tự bật chế độ URL-safe.
pub fn is_disabled(kind: FieldKind) -> bool {
    matches!(kind, FieldKind::Password | FieldKind::Number | FieldKind::Email)
}

/// Terminal vẫn gõ tiếng Việt nhưng adapter nên ưu tiên preedit.
pub fn is_terminal(kind: FieldKind) -> bool {
    matches!(kind, FieldKind::Terminal)
}

/// Field nhạy cảm: composition cũ phải bị huỷ, không commit.
pub fn is_sensitive(kind: FieldKind) -> bool {
    matches!(kind, FieldKind::Password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_fields() {
        assert!(is_disabled(FieldKind::Password));
        assert!(is_disabled(FieldKind::Number));
        assert!(is_disabled(FieldKind::Email));
        assert!(!is_disabled(FieldKind::Url));
        assert!(!is_disabled(FieldKind::Text));
        assert!(!is_disabled(FieldKind::Terminal));
        assert!(is_sensitive(FieldKind::Password));
        assert!(!is_sensitive(FieldKind::Text));
    }
}
