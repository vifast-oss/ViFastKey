use std::collections::BTreeMap;
use std::fs;

use serde_json::{json, Value};
use vfk_config::{ConfigDocument, Error, FieldId, SettingsGroup};
use vfk_engine::AppProfile;

fn write(path: &std::path::Path, json: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, json).unwrap();
}

fn read(path: &std::path::Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn minimal_file_uses_defaults_and_saves_only_dirty_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"vni"}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    assert_eq!(doc.config().input_method, "vni");
    assert!(doc.config().spell_check);
    assert!(doc.profiles().is_empty());

    doc.draft_mut().config.spell_check = false;
    doc.mark(FieldId::SpellCheck);
    doc.save().unwrap().expect("phải ghi file");

    let saved = read(&path);
    assert_eq!(saved["input_method"], json!("vni"));
    assert_eq!(saved["spell_check"], json!(false));
    assert_eq!(saved["schema_version"], json!(1));
    assert!(saved.get("app_profiles").is_none());
    assert!(saved.get("macros").is_none());
}

#[test]
fn missing_file_can_be_created_without_edits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut doc = ConfigDocument::load(&path).unwrap();
    assert!(!doc.exists());
    doc.save().unwrap().expect("phải tạo file");
    assert_eq!(read(&path), json!({"schema_version": 1}));
}

#[test]
fn unknown_keys_are_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"telex","future_field":{"x":1}}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    doc.draft_mut().config.spell_check = false;
    doc.mark(FieldId::SpellCheck);
    doc.save().unwrap().unwrap();

    let saved = read(&path);
    assert_eq!(saved["future_field"], json!({"x": 1}));
}

#[test]
fn macros_merge_keeps_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"macros":{"ko":"không","vn":"Việt Nam"}}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    let mut macros = doc.config().macros.clone();
    macros.insert("dc".into(), "được".into());
    doc.set_macros(macros.into_iter().collect::<BTreeMap<_, _>>());
    doc.save().unwrap().unwrap();

    let saved = read(&path);
    assert_eq!(saved["macros"]["ko"], json!("không"));
    assert_eq!(saved["macros"]["vn"], json!("Việt Nam"));
    assert_eq!(saved["macros"]["dc"], json!("được"));
}

#[test]
fn app_profiles_are_not_materialized_from_presets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"telex"}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    assert!(doc.profiles().is_empty());
    doc.set_profile("kitty", AppProfile { output_mode: None, ..Default::default() });
    doc.save().unwrap().unwrap();

    let saved = read(&path);
    let profiles = saved["app_profiles"].as_object().unwrap();
    assert!(profiles.contains_key("kitty"));
    assert!(!profiles.contains_key("chrome"), "không được ghi preset chưa sửa");
}

#[test]
fn app_profile_edit_keeps_other_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"app_profiles":{"chrome":{"wait_ack":false},"kitty":{"wait_ack":true}}}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    let mut kitty = doc.profile("kitty").unwrap().clone();
    kitty.backspace_delay_ms = 7;
    doc.set_profile("kitty", kitty);
    doc.save().unwrap().unwrap();

    let saved = read(&path);
    assert_eq!(saved["app_profiles"]["chrome"]["wait_ack"], json!(false));
    assert_eq!(saved["app_profiles"]["kitty"]["backspace_delay_ms"], json!(7));
}

#[test]
fn import_replaces_but_keeps_imported_unknown_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"telex","old":true}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    doc.import_json(r#"{"input_method":"vni","future":{"x":1},"macros":{"ko":"không"}}"#).unwrap();
    doc.save().unwrap().unwrap();

    let saved = read(&path);
    assert_eq!(saved["input_method"], json!("vni"));
    assert_eq!(saved["future"], json!({"x": 1}));
    assert_eq!(saved["macros"]["ko"], json!("không"));
    assert!(saved.get("old").is_none(), "import phải thay thế document");
    assert!(saved.get("spell_check").is_none(), "import không được materialize default không có trong file");
}

#[test]
fn import_invalid_json_does_not_change_draft() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"telex"}"#);
    let mut doc = ConfigDocument::load(&path).unwrap();
    assert!(doc.import_json("{bad").is_err());
    assert_eq!(doc.config().input_method, "telex");
}

#[test]
fn corrupt_file_is_backed_up_and_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, "{not json");

    let err = ConfigDocument::load(&path).unwrap_err();
    match err {
        Error::Corrupt { backup, .. } => {
            let backup = backup.expect("phải có backup");
            assert!(backup.exists());
            assert_eq!(fs::read_to_string(&path).unwrap(), "{not json");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn explicit_default_can_replace_corrupt_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, "{bad");

    let mut doc = ConfigDocument::new_default(&path);
    doc.reset();
    doc.save().unwrap().unwrap();

    assert_eq!(read(&path), json!({"schema_version": 1}));
    assert!(fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("config.bad.")));
}

#[test]
fn reset_writes_minimal_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"vni","macros":{"ko":"không"}}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    doc.reset();
    doc.save().unwrap().unwrap();

    let saved = read(&path);
    assert_eq!(saved, json!({"schema_version": 1}));
}

#[test]
fn conflicting_field_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"telex"}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    doc.draft_mut().config.input_method = "vni".into();
    doc.mark(FieldId::InputMethod);

    // Người khác sửa cùng field trước khi UI lưu.
    write(&path, r#"{"input_method":"viqr"}"#);

    match doc.save() {
        Err(Error::Conflict { fields }) => assert_eq!(fields, vec!["input_method"]),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn external_unrelated_change_is_merged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"telex","spell_check":true}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    doc.draft_mut().config.spell_check = false;
    doc.mark(FieldId::SpellCheck);

    // Người khác đổi field khác.
    write(&path, r#"{"input_method":"vni","spell_check":true}"#);
    doc.save().unwrap().unwrap();

    let saved = read(&path);
    assert_eq!(saved["input_method"], json!("vni"));
    assert_eq!(saved["spell_check"], json!(false));
}

#[test]
fn validation_warns_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"vni","quick_telex":true,"macro_enabled":false,"macro_digits":true}"#);

    let doc = ConfigDocument::load(&path).unwrap();
    let issues = doc.validate();
    assert!(issues.iter().any(|i| i.message.contains("Quick Telex")));
    assert!(issues.iter().any(|i| i.message.contains("Macro có chữ số")));
}

#[test]
fn import_preview_reports_diff_without_touching_draft() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"telex","macros":{"ko":"không"}}"#);

    let doc = ConfigDocument::load(&path).unwrap();
    let preview = doc
        .preview_import(r#"{"input_method":"vni","macros":{"ko":"không","vn":"Việt Nam"},"future":{"x":1}}"#)
        .unwrap();

    assert_eq!(preview.change_count(), 2);
    assert!(preview.changes.iter().any(|c| c.path == "input_method"));
    assert!(preview.changes.iter().any(|c| c.path == "macros.vn"));
    assert!(!preview.has_errors());
    assert!(!doc.is_dirty(), "preview không được sửa bản nháp");
    assert_eq!(doc.config().input_method, "telex");

    let mut doc = doc;
    doc.apply_import(preview).unwrap();
    assert_eq!(doc.config().input_method, "vni");
    assert!(doc.is_dirty());
    doc.save().unwrap().unwrap();
    let saved = read(&path);
    assert_eq!(saved["input_method"], json!("vni"));
    assert_eq!(saved["future"], json!({"x": 1}));
}

#[test]
fn import_preview_reports_validation_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"input_method":"telex"}"#);

    let doc = ConfigDocument::load(&path).unwrap();
    let preview =
        doc.preview_import(r#"{"input_method":"nope","app_profiles":{"x":{"input_method":"bad"}}}"#).unwrap();
    assert!(preview.has_errors());
    assert!(preview.issues.iter().any(|i| i.message.contains("không hợp lệ")));
}

#[test]
fn reset_typing_group_writes_defaults_but_keeps_macros() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(
        &path,
        r#"{"input_method":"vni","tone_style":"old","spell_check":false,"quick_telex":true,"macros":{"ko":"không"}}"#,
    );

    let mut doc = ConfigDocument::load(&path).unwrap();
    let changes = doc.reset_group(SettingsGroup::Typing);
    assert!(changes >= 4);
    assert_eq!(doc.config().input_method, "telex");
    assert_eq!(doc.config().tone_style, vfk_config::ToneStyle::New);
    assert!(doc.config().spell_check);
    assert!(!doc.config().quick_telex);
    assert_eq!(doc.config().macros.get("ko").map(String::as_str), Some("không"));

    doc.save().unwrap().unwrap();
    let saved = read(&path);
    assert_eq!(saved["input_method"], json!("telex"));
    assert_eq!(saved["macros"]["ko"], json!("không"));
}

#[test]
fn reset_apps_group_removes_overrides_but_keeps_runtime_modes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(
        &path,
        r#"{"app_profiles":{"kitty":{"output_mode":"preedit"}},"excluded_apps":["minecraft"],"app_modes":{"chrome":false}}"#,
    );

    let mut doc = ConfigDocument::load(&path).unwrap();
    let changes = doc.reset_group(SettingsGroup::Apps);
    assert_eq!(changes, 2);
    assert!(doc.profiles().is_empty());
    assert!(doc.config().excluded_apps.is_empty());
    assert_eq!(doc.config().app_modes.get("chrome"), Some(&false));

    doc.save().unwrap().unwrap();
    let saved = read(&path);
    assert!(saved.get("app_profiles").is_none());
    assert_eq!(saved["excluded_apps"], json!([]));
    assert_eq!(saved["app_modes"]["chrome"], json!(false));
}

#[test]
fn reset_macro_options_keeps_macro_table() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    write(&path, r#"{"macro_enabled":false,"macro_digits":true,"macro_undo":false,"macros":{"ko":"không"}}"#);

    let mut doc = ConfigDocument::load(&path).unwrap();
    doc.reset_group(SettingsGroup::Macros);
    assert!(doc.config().macro_enabled);
    assert!(!doc.config().macro_digits);
    assert!(doc.config().macro_undo);
    assert_eq!(doc.config().macros.len(), 1);
}

#[test]
fn loads_utf8_bom_config_from_windows_editor() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend_from_slice(r#"{"input_method":"vni","macros":{"ko":"không"}}"#.as_bytes());
    fs::write(&path, bytes).unwrap();

    let mut doc = ConfigDocument::load(&path).unwrap();
    assert_eq!(doc.config().input_method, "vni");
    doc.draft_mut().config.spell_check = false;
    doc.mark(FieldId::SpellCheck);
    doc.save().unwrap().unwrap();

    let saved = fs::read(&path).unwrap();
    assert!(!saved.starts_with(b"\xEF\xBB\xBF"), "file ghi lại không còn BOM");
    let saved: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(saved["macros"]["ko"], json!("không"));
}
