//! Biến `Action` của engine thành việc adapter cần làm.
//!
//! Adapter không tự quyết định xoá hay không; nó chỉ thi hành [`Plan`].

use vfk_engine::{Action, AppProfile, FieldKind};

use crate::field;
use crate::surrounding::{
    char_suffix_utf16_len, check_replace, utf16_suffix, utf16_to_byte_offset, SurroundingSnapshot,
};
use crate::timing::replace_wait_ms;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteAmount {
    /// Số char của text hiển thị (Action::Replace.delete_before).
    Chars(u32),
    /// Số UTF-16 code unit (C ABI / macOS / Android / Windows).
    Utf16(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterAction {
    Pass,
    Preedit { text: String, cursor_utf16: u32 },
    Commit { text: String, pass_key: bool },
    Replace { insert: String, delete: DeleteAmount, pass_key: bool },
}

impl From<Action> for AdapterAction {
    fn from(action: Action) -> Self {
        match action {
            Action::PassThrough => AdapterAction::Pass,
            Action::Preedit { text, cursor } => {
                let cursor_utf16 =
                    text.char_indices().nth(cursor).map(|(offset, _)| offset).unwrap_or(text.len());
                let cursor_utf16 = crate::surrounding::utf16_len(&text[..cursor_utf16]);
                AdapterAction::Preedit { text, cursor_utf16 }
            }
            Action::Commit { text, pass_key } => AdapterAction::Commit { text, pass_key },
            Action::Replace { delete_before, insert, pass_key } => {
                AdapterAction::Replace { insert, delete: DeleteAmount::Chars(delete_before as u32), pass_key }
            }
        }
    }
}

pub struct PlanContext<'a> {
    pub field_kind: FieldKind,
    /// Text engine tin là đang hiển thị trước phím hiện tại.
    pub before_display: &'a str,
    /// Snapshot đầy đủ của app; `None` nếu app không hỗ trợ.
    pub surrounding: Option<SurroundingSnapshot<'a>>,
    /// Cho phép port tự quản lý vùng text (Windows hook không có surrounding).
    /// Chỉ dùng khi adapter thực sự sở hữu toàn bộ đường nhập.
    pub allow_unverified_replace: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanKind {
    Pass,
    Preedit,
    Commit,
    Replace,
    FailSafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub kind: PlanKind,
    pub text: String,
    pub cursor_utf16: u32,
    pub delete_utf16: u32,
    pub delete_chars: u32,
    pub wait_ms: u32,
    pub pass_key: bool,
    pub reason: &'static str,
}

impl Plan {
    fn pass(reason: &'static str) -> Self {
        Plan {
            kind: PlanKind::Pass,
            text: String::new(),
            cursor_utf16: 0,
            delete_utf16: 0,
            delete_chars: 0,
            wait_ms: 0,
            pass_key: false,
            reason,
        }
    }

    fn fail(reason: &'static str) -> Self {
        Plan { kind: PlanKind::FailSafe, reason, ..Plan::pass(reason) }
    }
}

pub fn plan_action(action: &AdapterAction, context: &PlanContext<'_>, profile: &AppProfile) -> Plan {
    if field::is_disabled(context.field_kind) {
        return Plan::pass("field-disabled");
    }

    match action {
        AdapterAction::Pass => Plan::pass("engine-pass"),
        AdapterAction::Preedit { text, cursor_utf16 } => Plan {
            kind: PlanKind::Preedit,
            text: text.clone(),
            cursor_utf16: *cursor_utf16,
            delete_utf16: 0,
            delete_chars: 0,
            wait_ms: 0,
            pass_key: false,
            reason: "preedit",
        },
        AdapterAction::Commit { text, pass_key } => Plan {
            kind: PlanKind::Commit,
            text: text.clone(),
            cursor_utf16: 0,
            delete_utf16: 0,
            delete_chars: 0,
            wait_ms: 0,
            pass_key: *pass_key,
            reason: "commit",
        },
        AdapterAction::Replace { insert, delete, pass_key } => {
            let delete_utf16 = match delete {
                DeleteAmount::Utf16(units) => *units,
                DeleteAmount::Chars(chars) => {
                    // Adopt previous word: display rỗng, suffix nằm trong
                    // surrounding trước cursor.
                    let source = if context.before_display.is_empty() {
                        context
                            .surrounding
                            .and_then(|s| crate::surrounding::utf16_prefix(s.text, s.cursor_utf16))
                            .unwrap_or("")
                    } else {
                        context.before_display
                    };
                    let Some(units) = char_suffix_utf16_len(source, *chars) else {
                        return Plan::fail("replace-suffix-out-of-range");
                    };
                    units
                }
            };

            if delete_utf16 == 0 {
                return Plan {
                    kind: PlanKind::Commit,
                    text: insert.clone(),
                    cursor_utf16: 0,
                    delete_utf16: 0,
                    delete_chars: 0,
                    wait_ms: 0,
                    pass_key: *pass_key,
                    reason: "replace-no-delete",
                };
            }

            if context.allow_unverified_replace {
                let shown = if context.before_display.is_empty() {
                    context
                        .surrounding
                        .and_then(|s| crate::surrounding::utf16_prefix(s.text, s.cursor_utf16))
                        .unwrap_or("")
                } else {
                    context.before_display
                };
                let Some(deleted) = utf16_suffix(shown, delete_utf16) else {
                    return Plan::fail("replace-unverified-suffix");
                };
                if deleted.is_empty() {
                    return Plan::fail("replace-unverified-empty");
                }
                let delete_chars = deleted.chars().count() as u32;
                return Plan {
                    kind: PlanKind::Replace,
                    text: insert.clone(),
                    cursor_utf16: 0,
                    delete_utf16,
                    delete_chars,
                    wait_ms: replace_wait_ms(profile, delete_chars),
                    pass_key: *pass_key,
                    reason: "replace-trusted-direct",
                };
            }

            let Some(surrounding) = context.surrounding else {
                return Plan::fail("replace-no-surrounding");
            };
            let Some(check) = check_replace(context.before_display, delete_utf16, surrounding) else {
                return Plan::fail("replace-surrounding-mismatch");
            };
            Plan {
                kind: PlanKind::Replace,
                text: insert.clone(),
                cursor_utf16: 0,
                delete_utf16: check.delete_utf16,
                delete_chars: check.delete_chars,
                wait_ms: replace_wait_ms(profile, check.delete_chars),
                pass_key: *pass_key,
                reason: "replace-verified",
            }
        }
    }
}

/// Tiện ích cho port cần đổi cursor UTF-16 sang byte offset.
pub fn utf16_cursor_to_byte(text: &str, cursor_utf16: u32) -> Option<usize> {
    utf16_to_byte_offset(text, cursor_utf16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surrounding::SurroundingSnapshot;

    fn ctx<'a>(
        field_kind: FieldKind,
        before: &'a str,
        snapshot: Option<SurroundingSnapshot<'a>>,
        allow_unverified: bool,
    ) -> PlanContext<'a> {
        PlanContext {
            field_kind,
            before_display: before,
            surrounding: snapshot,
            allow_unverified_replace: allow_unverified,
        }
    }

    fn replace(insert: &str, delete: u32) -> AdapterAction {
        AdapterAction::Replace {
            insert: insert.to_owned(),
            delete: DeleteAmount::Utf16(delete),
            pass_key: false,
        }
    }

    #[test]
    fn disabled_field_passes() {
        let plan = plan_action(
            &replace("x", 1),
            &ctx(FieldKind::Password, "a", None, false),
            &AppProfile::default(),
        );
        assert_eq!(plan.kind, PlanKind::Pass);
    }

    #[test]
    fn verified_replace() {
        let snapshot = SurroundingSnapshot { text: "tiếng", cursor_utf16: 5, anchor_utf16: 5 };
        let plan = plan_action(
            &replace("ềng", 3),
            &ctx(FieldKind::Text, "tiếng", Some(snapshot), false),
            &AppProfile::default(),
        );
        assert_eq!(plan.kind, PlanKind::Replace);
        assert_eq!(plan.delete_chars, 3);
        assert_eq!(plan.delete_utf16, 3);
    }

    #[test]
    fn stale_surrounding_fails_safe() {
        let snapshot = SurroundingSnapshot { text: "hello", cursor_utf16: 5, anchor_utf16: 5 };
        let plan = plan_action(
            &replace("ềng", 3),
            &ctx(FieldKind::Text, "tiếng", Some(snapshot), false),
            &AppProfile::default(),
        );
        assert_eq!(plan.kind, PlanKind::FailSafe);
    }

    #[test]
    fn selection_fails_safe() {
        let snapshot = SurroundingSnapshot { text: "tiếng", cursor_utf16: 5, anchor_utf16: 2 };
        let plan = plan_action(
            &replace("ềng", 3),
            &ctx(FieldKind::Text, "tiếng", Some(snapshot), false),
            &AppProfile::default(),
        );
        assert_eq!(plan.kind, PlanKind::FailSafe);
    }

    #[test]
    fn mid_word_fails_safe() {
        let snapshot = SurroundingSnapshot { text: "tiếng", cursor_utf16: 2, anchor_utf16: 2 };
        let plan = plan_action(
            &replace("f", 1),
            &ctx(FieldKind::Text, "", Some(snapshot), false),
            &AppProfile::default(),
        );
        assert_eq!(plan.kind, PlanKind::FailSafe);
    }

    #[test]
    fn astral_delete_chars() {
        let snapshot = SurroundingSnapshot { text: "🙂a", cursor_utf16: 3, anchor_utf16: 3 };
        let plan = plan_action(
            &replace("X", 3),
            &ctx(FieldKind::Text, "🙂a", Some(snapshot), false),
            &AppProfile::default(),
        );
        assert_eq!(plan.kind, PlanKind::Replace);
        assert_eq!(plan.delete_chars, 2);
        assert_eq!(plan.delete_utf16, 3);
    }

    #[test]
    fn chars_delete_uses_surrounding_when_display_empty() {
        let snapshot = SurroundingSnapshot { text: "tiếng", cursor_utf16: 5, anchor_utf16: 5 };
        let plan = plan_action(
            &AdapterAction::Replace {
                insert: "ềng".into(), delete: DeleteAmount::Chars(3), pass_key: false
            },
            &ctx(FieldKind::Text, "", Some(snapshot), false),
            &AppProfile::default(),
        );
        assert_eq!(plan.kind, PlanKind::Replace);
        assert_eq!(plan.delete_chars, 3);
        assert_eq!(plan.delete_utf16, 3);
    }

    #[test]
    fn trusted_direct_replace_without_surrounding() {
        let plan = plan_action(
            &AdapterAction::Replace { insert: "á".into(), delete: DeleteAmount::Chars(1), pass_key: false },
            &ctx(FieldKind::Text, "a", None, true),
            &AppProfile::default(),
        );
        assert_eq!(plan.kind, PlanKind::Replace);
        assert_eq!(plan.delete_chars, 1);
        assert_eq!(plan.delete_utf16, 1);
    }

    #[test]
    fn no_delete_becomes_commit() {
        let plan =
            plan_action(&replace("abc", 0), &ctx(FieldKind::Text, "", None, false), &AppProfile::default());
        assert_eq!(plan.kind, PlanKind::Commit);
        assert_eq!(plan.text, "abc");
    }
}
