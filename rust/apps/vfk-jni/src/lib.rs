//! JNI tối giản cho Android IME. Java gọi các hàm `NativeEngine` và nhận Action JSON.
//! Không chứa logic tiếng Việt; dùng `vfk-engine` giống macOS/CLI.

use std::cell::RefCell;
use std::rc::Rc;

use jni::errors::{Result as JniResult, ThrowRuntimeExAndDefault};
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong};
use jni::{Env, EnvUnowned};
use serde_json::json;
use vfk_adapter_core::{
    char_to_utf16_offset, plan_action, AdapterAction, DeleteAmount, PlanContext, PlanKind,
    SurroundingSnapshot,
};
use vfk_engine::testing::WordListDictionary;
use vfk_engine::{
    Action, AppContext, Config, ConfigStore, FieldKind, ImeSession, KeyEvent, OutputMode, Ports,
    SurroundingText,
};

struct SharedConfig(Rc<RefCell<Config>>);
impl ConfigStore for SharedConfig {
    fn load(&self) -> Config {
        self.0.borrow().clone()
    }
    fn save(&self, config: &Config) {
        *self.0.borrow_mut() = config.clone();
    }
}

struct SharedApp(Rc<RefCell<Option<String>>>);
impl AppContext for SharedApp {
    fn current_app(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}

struct SharedSurrounding(Rc<RefCell<Option<String>>>);
impl SurroundingText for SharedSurrounding {
    fn before_cursor(&self, n: usize) -> Option<String> {
        self.0.borrow().as_ref().map(|text| {
            let chars: Vec<char> = text.chars().collect();
            chars[chars.len().saturating_sub(n)..].iter().collect()
        })
    }
}

struct Handle {
    session: ImeSession,
    app: Rc<RefCell<Option<String>>>,
    surrounding: Rc<RefCell<Option<String>>>,
    config: Rc<RefCell<Config>>,
}

fn action_json(action: &Action) -> serde_json::Value {
    match action {
        Action::PassThrough => json!({"kind": "pass"}),
        Action::Preedit { text, cursor } => json!({"kind": "preedit", "text": text, "cursor": cursor}),
        Action::Replace { delete_before, insert, pass_key } => json!({
            "kind": "replace", "delete": delete_before, "insert": insert, "pass": pass_key
        }),
        Action::Commit { text, pass_key } => json!({
            "kind": "commit", "text": text, "pass": pass_key
        }),
    }
}

fn to_jstring<'local>(env: &mut Env<'local>, value: serde_json::Value) -> JString<'local> {
    JString::from_str(env, value.to_string()).unwrap_or_else(|_| JString::null())
}

fn string_arg(value: &JString<'_>, env: &Env<'_>) -> String {
    value.try_to_string(env).unwrap_or_default()
}

fn field_kind_from_i32(value: jint) -> FieldKind {
    match value {
        1 => FieldKind::Password,
        2 => FieldKind::Number,
        3 => FieldKind::Email,
        4 => FieldKind::Url,
        5 => FieldKind::Terminal,
        _ => FieldKind::Text,
    }
}

fn adapter_action_from_json(value: &serde_json::Value) -> Option<AdapterAction> {
    let text = value["text"].as_str().or_else(|| value["insert"].as_str()).unwrap_or("").to_owned();
    let pass_key = value["pass"].as_bool().unwrap_or(false);
    match value["kind"].as_str().unwrap_or("pass") {
        "pass" => Some(AdapterAction::Pass),
        "preedit" => {
            let cursor = value["cursor"].as_u64().unwrap_or(0) as u32;
            let cursor_utf16 = char_to_utf16_offset(&text, cursor).unwrap_or(0);
            Some(AdapterAction::Preedit { text, cursor_utf16 })
        }
        "commit" => Some(AdapterAction::Commit { text, pass_key }),
        "replace" => {
            let delete = value["delete"].as_u64().unwrap_or(0) as u32;
            Some(AdapterAction::Replace { insert: text, delete: DeleteAmount::Chars(delete), pass_key })
        }
        _ => None,
    }
}

fn plan_value(plan: &vfk_adapter_core::Plan) -> serde_json::Value {
    let kind = match plan.kind {
        PlanKind::Pass => "pass",
        PlanKind::Preedit => "preedit",
        PlanKind::Commit => "commit",
        PlanKind::Replace => "replace",
        PlanKind::FailSafe => "fail_safe",
    };
    json!({
        "kind": kind,
        "text": plan.text,
        "delete_utf16": plan.delete_utf16,
        "delete_chars": plan.delete_chars,
        "cursor": plan.cursor_utf16,
        "wait": plan.wait_ms,
        "pass": plan.pass_key,
    })
}

fn handle<'a>(ptr: jlong) -> Option<&'a mut Handle> {
    if ptr == 0 {
        None
    } else {
        Some(unsafe { &mut *(ptr as *mut Handle) })
    }
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_create<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    config_json: JString<'caller>,
) -> jlong {
    unowned_env
        .with_env(|env| -> JniResult<jlong> {
            let config_text = string_arg(&config_json, env);
            let app = Rc::new(RefCell::new(None::<String>));
            let surrounding = Rc::new(RefCell::new(None::<String>));
            let config =
                Rc::new(RefCell::new(serde_json::from_str::<Config>(&config_text).unwrap_or_default()));
            let ports = Ports {
                config: Box::new(SharedConfig(config.clone())),
                app: Box::new(SharedApp(app.clone())),
                surrounding: Box::new(SharedSurrounding(surrounding.clone())),
                dictionary: Box::new(WordListDictionary::default()),
            };
            let handle =
                Handle { session: ImeSession::new(ports, OutputMode::Preedit), app, surrounding, config };
            Ok(Box::into_raw(Box::new(handle)) as jlong)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_free<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
) {
    if ptr != 0 {
        unsafe {
            drop(Box::from_raw(ptr as *mut Handle));
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_setConfig<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
    config_json: JString<'caller>,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let Some(h) = handle(ptr) else {
                return Ok(JString::null());
            };
            let text = string_arg(&config_json, env);
            let Some(config) = serde_json::from_str::<Config>(&text).ok() else {
                return Ok(to_jstring(env, json!({"kind": "pass"})));
            };
            *h.config.borrow_mut() = config;
            let action = h.session.reload_config();
            Ok(to_jstring(env, action_json(&action)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_setOutputMode<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
    mode: jint,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let Some(h) = handle(ptr) else {
                return Ok(JString::null());
            };
            let mode = if mode == 2 { OutputMode::Replace } else { OutputMode::Preedit };
            let action = h.session.set_output_mode(mode);
            Ok(to_jstring(env, action_json(&action)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_display<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let Some(h) = handle(ptr) else {
                return Ok(JString::null());
            };
            let text = h.session.display();
            Ok(JString::from_str(env, text).unwrap_or_else(|_| JString::null()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_suggest<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    _ptr: jlong,
    prefix: JString<'caller>,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let prefix = string_arg(&prefix, env);
            let values = vfk_suggest::suggest(prefix.trim(), 6);
            let json = serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string());
            Ok(JString::from_str(env, json).unwrap_or_else(|_| JString::null()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_autocorrect<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    word: JString<'caller>,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let word = string_arg(&word, env);
            let value = vfk_suggest::autocorrect(&word).unwrap_or_default();
            Ok(JString::from_str(env, value).unwrap_or_else(|_| JString::null()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_process<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
    key: JString<'caller>,
    ctrl: jboolean,
    alt: jboolean,
    meta: jboolean,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let Some(h) = handle(ptr) else {
                return Ok(JString::null());
            };
            let key = string_arg(&key, env);
            let mut event = KeyEvent::parse(&key);
            event.ctrl = ctrl;
            event.alt = alt;
            event.meta = meta;
            let action = h.session.process_key(&event);
            Ok(to_jstring(env, action_json(&action)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_plan<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
    action_json: JString<'caller>,
    field: jint,
    before_display: JString<'caller>,
    surrounding_text: JString<'caller>,
    cursor_utf16: jint,
    anchor_utf16: jint,
    allow_unverified_replace: jboolean,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let Some(h) = handle(ptr) else {
                return Ok(JString::null());
            };
            let action_text = string_arg(&action_json, env);
            let value: serde_json::Value =
                serde_json::from_str(&action_text).unwrap_or_else(|_| json!({"kind": "pass"}));
            let Some(action) = adapter_action_from_json(&value) else {
                return Ok(to_jstring(env, json!({"kind": "pass"})));
            };

            let before = string_arg(&before_display, env);
            let surrounding = string_arg(&surrounding_text, env);
            let snapshot = if surrounding.is_empty() {
                None
            } else {
                Some(SurroundingSnapshot {
                    text: &surrounding,
                    cursor_utf16: cursor_utf16.max(0) as u32,
                    anchor_utf16: anchor_utf16.max(0) as u32,
                })
            };
            let context = PlanContext {
                field_kind: field_kind_from_i32(field),
                before_display: &before,
                surrounding: snapshot,
                allow_unverified_replace,
            };
            let plan = plan_action(&action, &context, h.session.profile());
            Ok(to_jstring(env, plan_value(&plan)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_setSurrounding<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
    text: JString<'caller>,
) {
    unowned_env
        .with_env(|env| -> JniResult<()> {
            let Some(h) = handle(ptr) else {
                return Ok(());
            };
            let value = string_arg(&text, env);
            *h.surrounding.borrow_mut() = if value.is_empty() { None } else { Some(value) };
            Ok(())
        })
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_focus<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
    app: JString<'caller>,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let Some(h) = handle(ptr) else {
                return Ok(JString::null());
            };
            let app = string_arg(&app, env);
            *h.app.borrow_mut() = if app.is_empty() { None } else { Some(app.clone()) };
            let action = h.session.focus_changed(if app.is_empty() { None } else { Some(&app) });
            Ok(to_jstring(env, action_json(&action)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_cursorMoved<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let Some(h) = handle(ptr) else {
                return Ok(JString::null());
            };
            let action = h.session.cursor_moved();
            Ok(to_jstring(env, action_json(&action)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_toggle<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
) -> JString<'caller> {
    unowned_env
        .with_env(|env| -> JniResult<JString> {
            let Some(h) = handle(ptr) else {
                return Ok(JString::null());
            };
            let action = h.session.toggle_vietnamese();
            Ok(to_jstring(env, action_json(&action)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_isEnabled<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
) -> jboolean {
    handle(ptr).map(|h| h.session.is_enabled()).unwrap_or(false)
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_reset<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
) {
    if let Some(h) = handle(ptr) {
        h.session.reset();
    }
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_setField<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    ptr: jlong,
    field: jint,
) {
    let Some(h) = handle(ptr) else {
        return;
    };
    let kind = match field {
        1 => FieldKind::Password,
        2 => FieldKind::Number,
        3 => FieldKind::Email,
        4 => FieldKind::Url,
        5 => FieldKind::Terminal,
        _ => FieldKind::Text,
    };
    let _ = h.session.set_field_kind(kind);
}
