//! Chạy bộ adapter conformance dùng chung.
//!
//! Các port Swift/Java/Python/C++ có thể đọc cùng `adapter-conformance/*.jsonl`
//! và so sánh plan. Test này giữ hợp đồng hành vi cho bản Rust.

use serde_json::Value;
use vfk_adapter_core::{
    plan_action, AdapterAction, DeleteAmount, PlanContext, PlanKind, SurroundingSnapshot,
};
use vfk_engine::{AppProfile, FieldKind};

fn field_kind(value: &str) -> FieldKind {
    match value {
        "password" => FieldKind::Password,
        "number" => FieldKind::Number,
        "email" => FieldKind::Email,
        "url" => FieldKind::Url,
        "terminal" => FieldKind::Terminal,
        _ => FieldKind::Text,
    }
}

fn expected_kind(value: &str) -> PlanKind {
    match value {
        "pass" => PlanKind::Pass,
        "preedit" => PlanKind::Preedit,
        "commit" => PlanKind::Commit,
        "replace" => PlanKind::Replace,
        "fail_safe" => PlanKind::FailSafe,
        other => panic!("unknown expected kind: {other}"),
    }
}

#[test]
fn shared_adapter_scenarios() {
    let raw = include_str!("../../../../adapter-conformance/replace.jsonl");
    let mut count = 0;
    for (line_no, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        count += 1;
        let value: Value =
            serde_json::from_str(line).unwrap_or_else(|error| panic!("line {}: {error}", line_no + 1));
        let name = value["name"].as_str().unwrap_or("unnamed");
        let before = value["before"].as_str().unwrap_or("");
        let surrounding_value = &value["surrounding"];
        let surrounding = if surrounding_value.is_null() {
            None
        } else {
            Some(SurroundingSnapshot {
                text: surrounding_value["text"].as_str().unwrap_or(""),
                cursor_utf16: surrounding_value["cursor"].as_u64().unwrap_or(0) as u32,
                anchor_utf16: surrounding_value["anchor"].as_u64().unwrap_or(0) as u32,
            })
        };
        let action_value = &value["action"];
        let text = action_value["text"]
            .as_str()
            .or_else(|| action_value["insert"].as_str())
            .unwrap_or("")
            .to_owned();
        let pass_key = action_value["pass_key"].as_bool().unwrap_or(false);
        let action = match action_value["kind"].as_str().unwrap_or("pass") {
            "pass" => AdapterAction::Pass,
            "preedit" => AdapterAction::Preedit {
                text,
                cursor_utf16: action_value["cursor_utf16"].as_u64().unwrap_or(0) as u32,
            },
            "commit" => AdapterAction::Commit { text, pass_key },
            "replace" => AdapterAction::Replace {
                insert: text,
                delete: DeleteAmount::Utf16(action_value["delete_utf16"].as_u64().unwrap_or(0) as u32),
                pass_key,
            },
            other => panic!("{name}: unknown action kind {other}"),
        };
        let context = PlanContext {
            field_kind: field_kind(value["field"].as_str().unwrap_or("text")),
            before_display: before,
            surrounding,
            allow_unverified_replace: value["allow_unverified_replace"].as_bool().unwrap_or(false),
        };
        let plan = plan_action(&action, &context, &AppProfile::default());
        let expected = &value["expected"];
        assert_eq!(plan.kind, expected_kind(expected["kind"].as_str().unwrap_or("pass")), "{name}: kind");
        assert_eq!(
            plan.delete_utf16,
            expected["delete_utf16"].as_u64().unwrap_or(0) as u32,
            "{name}: delete_utf16"
        );
        assert_eq!(
            plan.delete_chars,
            expected["delete_chars"].as_u64().unwrap_or(0) as u32,
            "{name}: delete_chars"
        );
        assert_eq!(plan.pass_key, expected["pass_key"].as_bool().unwrap_or(false), "{name}: pass_key");
        if let Some(text) = expected["text"].as_str() {
            assert_eq!(plan.text, text, "{name}: text");
        }
    }
    assert!(count >= 10, "expected at least 10 scenarios, got {count}");
}
