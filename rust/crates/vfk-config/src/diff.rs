//! So sánh hai document JSON để hiện preview trước khi import.
//!
//! Object được so sánh đệ quy theo từng key; array và scalar được coi là một
//! giá trị nguyên khối. Kết quả giữ thứ tự key ổn định để test/diff dễ đọc.

use std::collections::BTreeSet;

use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    /// Đường dẫn kiểu `macros.ko`, `app_profiles.code.input_method`.
    pub path: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

impl Change {
    pub fn is_added(&self) -> bool {
        self.before.is_none() && self.after.is_some()
    }

    pub fn is_removed(&self) -> bool {
        self.before.is_some() && self.after.is_none()
    }

    pub fn is_changed(&self) -> bool {
        self.before.is_some() && self.after.is_some()
    }
}

/// Trả về danh sách thay đổi từ `before` sang `after`. Hai giá trị bằng nhau
/// trả về vector rỗng.
pub fn diff_json(before: &Value, after: &Value) -> Vec<Change> {
    let mut changes = Vec::new();
    diff_at("", before, after, &mut changes);
    changes
}

fn diff_at(path: &str, before: &Value, after: &Value, changes: &mut Vec<Change>) {
    if before == after {
        return;
    }

    match (before, after) {
        (Value::Object(before), Value::Object(after)) => {
            let keys: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
            for key in keys {
                let child = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
                match (before.get(key), after.get(key)) {
                    (Some(before), Some(after)) => diff_at(&child, before, after, changes),
                    (Some(before), None) => {
                        changes.push(Change { path: child, before: Some(before.clone()), after: None })
                    }
                    (None, Some(after)) => {
                        changes.push(Change { path: child, before: None, after: Some(after.clone()) })
                    }
                    (None, None) => {}
                }
            }
        }
        _ => changes.push(Change {
            path: if path.is_empty() { "(root)".into() } else { path.to_string() },
            before: Some(before.clone()),
            after: Some(after.clone()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn equal_values_have_no_changes() {
        assert!(diff_json(&json!({"a": 1}), &json!({"a": 1})).is_empty());
    }

    #[test]
    fn scalar_change_is_reported_at_path() {
        let changes = diff_json(&json!({"input_method": "telex"}), &json!({"input_method": "vni"}));
        assert_eq!(
            changes,
            vec![Change {
                path: "input_method".into(),
                before: Some(json!("telex")),
                after: Some(json!("vni")),
            }]
        );
        assert!(changes[0].is_changed());
    }

    #[test]
    fn nested_maps_are_compared_key_by_key() {
        let changes = diff_json(
            &json!({"macros": {"ko": "không", "vn": "Việt Nam"}}),
            &json!({"macros": {"ko": "không", "vn": "Vietnam", "dc": "được"}}),
        );
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].path, "macros.dc");
        assert!(changes[0].is_added());
        assert_eq!(changes[1].path, "macros.vn");
        assert!(changes[1].is_changed());
    }

    #[test]
    fn removed_key_is_reported() {
        let changes = diff_json(&json!({"macros": {"ko": "không"}}), &json!({}));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "macros");
        assert!(changes[0].is_removed());
    }

    #[test]
    fn arrays_are_compared_as_a_whole() {
        let changes = diff_json(&json!({"excluded_apps": ["a", "b"]}), &json!({"excluded_apps": ["a"]}));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "excluded_apps");
        assert_eq!(changes[0].after, Some(json!(["a"])));
    }
}
