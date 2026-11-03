//! Hợp đồng app profile dùng chung Python ↔ Rust.
//!
//! `conformance/app_profiles.json` là dữ liệu chuẩn cho alias/profile;
//! `conformance/app_profiles_behavior.jsonl` khóa hành vi terminal/soft-reset.

use std::path::PathBuf;

use serde_json::{json, Map, Value};
use vfk_engine::testing::Harness;
use vfk_engine::{app_aliases, default_app_profiles, Config, OutputMode};

fn conformance(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../conformance").join(name)
}

fn normalized(profile: &vfk_engine::AppProfile) -> Map<String, Value> {
    let mut map = serde_json::to_value(profile)
        .expect("AppProfile serializes")
        .as_object()
        .expect("AppProfile is an object")
        .clone();
    for (key, default) in [
        ("output_mode", Value::Null),
        ("backspace_delay_ms", json!(0)),
        ("delay_per_char_ms", json!(0)),
        ("wait_ack", json!(false)),
        ("edit_previous_word", Value::Null),
        ("restore_after_space", Value::Null),
        ("vietnamese", Value::Null),
        ("input_method", Value::Null),
        ("tone_style", Value::Null),
        ("spell_check", Value::Null),
        ("charset", Value::Null),
    ] {
        if map.get(key) == Some(&default) {
            map.remove(key);
        }
    }
    map
}

#[test]
fn app_profiles_contract() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../python/vfk_engine/app_profiles.json");
    let expected: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(app_aliases()).unwrap(), expected["aliases"]);

    let actual: Map<String, Value> = default_app_profiles()
        .iter()
        .map(|(name, profile)| (name.clone(), Value::Object(normalized(profile))))
        .collect();
    assert_eq!(Value::Object(actual), expected["profiles"]);
}

#[test]
fn terminal_profile_behavior() {
    let text = std::fs::read_to_string(conformance("app_profiles_behavior.jsonl")).unwrap();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let case: Value = serde_json::from_str(line).unwrap();
        let mode = match case["mode"].as_str().unwrap() {
            "preedit" => OutputMode::Preedit,
            "replace" => OutputMode::Replace,
            other => panic!("unknown mode {other}"),
        };
        let mut h = Harness::new(mode, Config::default(), case["app"].as_str().expect("app"), "");
        for key in case["keys"].as_array().unwrap() {
            h.field.key(&mut h.session, key.as_str().unwrap());
        }
        assert_eq!(h.text(), case["text"].as_str().unwrap(), "{}", case["name"]);
    }
}
