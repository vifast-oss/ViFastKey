use std::collections::{HashMap, HashSet};

use vfk_core::CHARSETS;
use vfk_engine::{normalize_app_id, AppProfile, CONFIG_SCHEMA_VERSION};

use crate::draft::Draft;
use crate::meta::FieldId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub field: Option<FieldId>,
    pub severity: Severity,
    pub message: String,
}

impl Issue {
    pub fn error(field: FieldId, message: impl Into<String>) -> Self {
        Issue { field: Some(field), severity: Severity::Error, message: message.into() }
    }

    pub fn warning(field: FieldId, message: impl Into<String>) -> Self {
        Issue { field: Some(field), severity: Severity::Warning, message: message.into() }
    }

    pub fn info(field: Option<FieldId>, message: impl Into<String>) -> Self {
        Issue { field, severity: Severity::Info, message: message.into() }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

pub const METHODS: &[&str] = &["telex", "simple_telex", "vni", "viqr"];

pub fn validate(draft: &Draft, presets: &HashMap<String, AppProfile>) -> Vec<Issue> {
    let config = draft.effective_config(presets);
    let mut issues = Vec::new();

    if !METHODS.contains(&config.input_method.as_str()) {
        issues.push(Issue::error(
            FieldId::InputMethod,
            format!(
                "Kiểu gõ “{}” không hợp lệ; dùng telex, simple_telex, vni hoặc viqr.",
                config.input_method
            ),
        ));
    }

    if config.schema_version > CONFIG_SCHEMA_VERSION {
        issues.push(Issue::warning(
            FieldId::SchemaVersion,
            format!(
                "File dùng schema {} (bản này hỗ trợ {}). Field lạ sẽ được giữ nguyên.",
                config.schema_version, CONFIG_SCHEMA_VERSION
            ),
        ));
    }

    let is_telex = matches!(config.input_method.as_str(), "telex" | "simple_telex");
    if !is_telex {
        if config.w_as_u {
            issues.push(Issue::warning(FieldId::WAsU, "w → ư chỉ áp dụng cho Telex/Simple Telex."));
        }
        if config.bracket_keys {
            issues
                .push(Issue::warning(FieldId::BracketKeys, "[ ] → ơ ư chỉ áp dụng cho Telex/Simple Telex."));
        }
        if config.quick_telex {
            issues
                .push(Issue::warning(FieldId::QuickTelex, "Quick Telex chỉ áp dụng cho Telex/Simple Telex."));
        }
    }

    if !config.macro_enabled {
        for (field, value, label) in [
            (FieldId::MacroDigits, config.macro_digits, "Macro có chữ số"),
            (FieldId::MacroUndo, config.macro_undo, "Backspace hoàn tác macro"),
            (FieldId::MacroInOffMode, config.macro_in_off_mode, "Macro khi tắt tiếng Việt"),
        ] {
            if value {
                issues.push(Issue::warning(
                    field,
                    format!("{label} đang bật nhưng Macro đang tắt; engine sẽ bỏ qua."),
                ));
            }
        }
    }

    for (key, value) in &config.macros {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            issues.push(Issue::error(FieldId::Macros, "Macro có key rỗng."));
        } else if trimmed != key {
            issues
                .push(Issue::warning(FieldId::Macros, format!("Macro “{key}” có khoảng trắng ở đầu/cuối.")));
        }
        if key.chars().any(char::is_uppercase) {
            issues.push(Issue::warning(
                FieldId::Macros,
                format!("Macro “{key}” nên viết thường; engine tra key lowercase."),
            ));
        }
        if key.len() > 80 {
            issues.push(Issue::warning(
                FieldId::Macros,
                format!("Macro key dài {} ký tự; nên giữ dưới 80.", key.len()),
            ));
        }
        if value.is_empty() {
            issues.push(Issue::warning(FieldId::Macros, format!("Macro “{key}” thay thế bằng chuỗi rỗng.")));
        }
    }

    let mut seen_excluded = HashSet::new();
    for app in &config.excluded_apps {
        let normalized = normalize_app_id(Some(app));
        if normalized.is_empty() {
            issues.push(Issue::error(FieldId::ExcludedApps, "App bị loại trừ có tên rỗng."));
        } else if normalized != *app {
            issues.push(Issue::warning(
                FieldId::ExcludedApps,
                format!("Nên dùng app id chuẩn hoá “{normalized}” thay cho “{app}”."),
            ));
        }
        if !seen_excluded.insert(normalized.clone()) {
            issues.push(Issue::warning(FieldId::ExcludedApps, format!("App “{app}” bị loại trừ trùng.")));
        }
        if let Some(profile) = draft.app_profiles.get(&normalized) {
            if profile.vietnamese == Some(true) {
                issues.push(Issue::warning(
                    FieldId::AppProfiles,
                    format!("“{app}” vừa bị loại trừ vừa ép bật tiếng Việt; loại trừ sẽ thắng."),
                ));
            }
        }
    }

    for (app, profile) in &draft.app_profiles {
        let normalized = normalize_app_id(Some(app));
        if normalized.is_empty() {
            issues.push(Issue::error(FieldId::AppProfiles, "App profile có tên rỗng."));
        } else if normalized != *app {
            issues.push(Issue::warning(
                FieldId::AppProfiles,
                format!("App profile “{app}” nên dùng id chuẩn hoá “{normalized}”."),
            ));
        }
        validate_profile(app, profile, &mut issues);
    }

    for app in config.app_modes.keys() {
        if !draft.app_profiles.contains_key(app) && !config.excluded_apps.contains(app) {
            issues.push(Issue::info(
                Some(FieldId::AppModes),
                format!("“{app}” đang có trạng thái Việt/Anh được adapter ghi nhớ."),
            ));
        }
    }

    issues
}

fn validate_profile(app: &str, profile: &AppProfile, issues: &mut Vec<Issue>) {
    if let Some(method) = &profile.input_method {
        if !METHODS.contains(&method.as_str()) {
            issues.push(Issue::error(
                FieldId::AppProfiles,
                format!("App “{app}” có kiểu gõ không hợp lệ: “{method}”."),
            ));
        }
    }
    if let Some(charset) = &profile.charset {
        if !CHARSETS.contains(&charset.as_str()) {
            issues.push(Issue::error(
                FieldId::AppProfiles,
                format!("App “{app}” có bảng mã không hợp lệ: “{charset}”."),
            ));
        }
    }
    if profile.backspace_delay_ms > 2_000 {
        issues.push(Issue::error(
            FieldId::AppProfiles,
            format!("App “{app}”: backspace delay tối đa 2000 ms."),
        ));
    }
    if profile.delay_per_char_ms > 500 {
        issues
            .push(Issue::error(FieldId::AppProfiles, format!("App “{app}”: delay mỗi ký tự tối đa 500 ms.")));
    }
}

/// Validate bảng macro dạng row để phát hiện trùng key sau lowercase.
pub fn validate_macro_rows(rows: &[(String, String)]) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut seen = HashSet::new();
    for (index, (key, value)) in rows.iter().enumerate() {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            issues.push(Issue::error(FieldId::Macros, format!("Dòng macro {} có key rỗng.", index + 1)));
            continue;
        }
        let normalized = trimmed.to_lowercase();
        if !seen.insert(normalized) {
            issues.push(Issue::error(
                FieldId::Macros,
                format!("Macro “{trimmed}” bị trùng (không phân biệt hoa/thường)."),
            ));
        }
        if value.is_empty() {
            issues.push(Issue::warning(
                FieldId::Macros,
                format!("Macro “{trimmed}” thay thế bằng chuỗi rỗng."),
            ));
        }
        if trimmed.len() > 80 {
            issues.push(Issue::warning(FieldId::Macros, format!("Macro key “{trimmed}” dài hơn 80 ký tự.")));
        }
    }
    issues
}
