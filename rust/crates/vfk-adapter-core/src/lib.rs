//! Logic adapter dùng chung cho mọi port ViFastKey.
//!
//! Crate này **không** biết Fcitx5, IMK, IBus, Android hay Windows. Nó chỉ trả
//! lời ba câu hỏi:
//! 1. Field này có được xử lý tiếng Việt không?
//! 2. Surrounding snapshot có đủ an toàn để `Replace` không?
//! 3. Action của engine nên được thi hành như thế nào (preedit/commit/replace/
//!    pass/fail-safe), xoá bao nhiêu codepoint/UTF-16, chờ bao lâu?
//!
//! Port chỉ cần implement các primitive: set preedit, commit, delete surrounding,
//! forward key và event loop/timer.

pub mod field;
pub mod plan;
pub mod surrounding;
pub mod timing;

pub use field::{is_disabled, is_sensitive, is_terminal};
pub use plan::{plan_action, utf16_cursor_to_byte, AdapterAction, DeleteAmount, Plan, PlanContext, PlanKind};
pub use surrounding::{
    char_suffix, char_suffix_utf16_len, char_to_utf16_offset, check_replace, is_word_codepoint,
    next_codepoint, utf16_len, utf16_prefix, utf16_suffix, utf16_to_byte_offset, utf16_to_char_offset,
    ReplaceCheck, SurroundingSnapshot,
};
pub use timing::{replace_wait_ms, MAX_REPLACE_WAIT_MS, MIN_REPLACE_WAIT_MS, POST_COMMIT_DELAY_MS};
