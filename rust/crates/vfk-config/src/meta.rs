//! ID field để UI đánh dấu dirty và hiển thị lỗi theo đúng chỗ.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldId {
    InputMethod,
    ToneStyle,
    SpellCheck,
    WAsU,
    BracketKeys,
    QuickTelex,
    AllowCodaK,
    AutoRestoreEnglish,
    MacroEnabled,
    MacroDigits,
    MacroUndo,
    MacroInOffMode,
    Macros,
    AutoCapitalize,
    ExcludedApps,
    AppModes,
    DefaultVietnamese,
    EditPreviousWord,
    AppProfiles,
    RestoreAfterSpace,
    SmartCaps,
    UnicodeForm,
    SchemaVersion,
}

impl FieldId {
    pub const ALL: &'static [FieldId] = &[
        FieldId::InputMethod,
        FieldId::ToneStyle,
        FieldId::SpellCheck,
        FieldId::WAsU,
        FieldId::BracketKeys,
        FieldId::QuickTelex,
        FieldId::AllowCodaK,
        FieldId::AutoRestoreEnglish,
        FieldId::MacroEnabled,
        FieldId::MacroDigits,
        FieldId::MacroUndo,
        FieldId::MacroInOffMode,
        FieldId::Macros,
        FieldId::AutoCapitalize,
        FieldId::ExcludedApps,
        FieldId::AppModes,
        FieldId::DefaultVietnamese,
        FieldId::EditPreviousWord,
        FieldId::AppProfiles,
        FieldId::RestoreAfterSpace,
        FieldId::SmartCaps,
        FieldId::UnicodeForm,
        FieldId::SchemaVersion,
    ];

    pub fn as_key(self) -> &'static str {
        match self {
            FieldId::InputMethod => "input_method",
            FieldId::ToneStyle => "tone_style",
            FieldId::SpellCheck => "spell_check",
            FieldId::WAsU => "w_as_u",
            FieldId::BracketKeys => "bracket_keys",
            FieldId::QuickTelex => "quick_telex",
            FieldId::AllowCodaK => "allow_coda_k",
            FieldId::AutoRestoreEnglish => "auto_restore_english",
            FieldId::MacroEnabled => "macro_enabled",
            FieldId::MacroDigits => "macro_digits",
            FieldId::MacroUndo => "macro_undo",
            FieldId::MacroInOffMode => "macro_in_off_mode",
            FieldId::Macros => "macros",
            FieldId::AutoCapitalize => "auto_capitalize",
            FieldId::ExcludedApps => "excluded_apps",
            FieldId::AppModes => "app_modes",
            FieldId::DefaultVietnamese => "default_vietnamese",
            FieldId::EditPreviousWord => "edit_previous_word",
            FieldId::AppProfiles => "app_profiles",
            FieldId::RestoreAfterSpace => "restore_after_space",
            FieldId::SmartCaps => "smart_caps",
            FieldId::UnicodeForm => "unicode_form",
            FieldId::SchemaVersion => "schema_version",
        }
    }

    pub fn is_map(self) -> bool {
        matches!(self, FieldId::Macros | FieldId::AppModes | FieldId::AppProfiles)
    }
}

/// Tập dirty field, giữ thứ tự ổn định để diff/test dễ đọc.
pub type DirtySet = BTreeSet<FieldId>;
