//! JNI tối giản cho Android IME. Java gọi các hàm `NativeEngine` và nhận Action JSON.
//! Không chứa logic tiếng Việt; dùng `vfk-engine` giống macOS/CLI.

use std::cell::RefCell;
use std::rc::Rc;

use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jlong, jstring};
use jni::JNIEnv;
use serde_json::json;
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

fn to_jstring(env: &mut JNIEnv, value: serde_json::Value) -> jstring {
    env.new_string(value.to_string()).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn handle<'a>(ptr: jlong) -> Option<&'a mut Handle> {
    if ptr == 0 {
        None
    } else {
        Some(unsafe { &mut *(ptr as *mut Handle) })
    }
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_create(_env: JNIEnv, _class: JClass) -> jlong {
    let app = Rc::new(RefCell::new(None::<String>));
    let surrounding = Rc::new(RefCell::new(None::<String>));
    let config = Rc::new(RefCell::new(Config::default()));
    let ports = Ports {
        config: Box::new(SharedConfig(config.clone())),
        app: Box::new(SharedApp(app.clone())),
        surrounding: Box::new(SharedSurrounding(surrounding.clone())),
        dictionary: Box::new(WordListDictionary::default()),
    };
    let handle = Handle { session: ImeSession::new(ports, OutputMode::Preedit), app, surrounding, config };
    Box::into_raw(Box::new(handle)) as jlong
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_free(_env: JNIEnv, _class: JClass, ptr: jlong) {
    if ptr != 0 {
        unsafe {
            drop(Box::from_raw(ptr as *mut Handle));
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_setConfig(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    config_json: JString,
) -> jstring {
    let Some(h) = handle(ptr) else {
        return std::ptr::null_mut();
    };
    let text: String = env.get_string(&config_json).map(|s| s.into()).unwrap_or_default();
    let Some(config) = serde_json::from_str::<Config>(&text).ok() else {
        return to_jstring(&mut env, json!({"kind": "pass"}));
    };
    *h.config.borrow_mut() = config;
    let action = h.session.reload_config();
    to_jstring(&mut env, action_json(&action))
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_display(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    let Some(h) = handle(ptr) else {
        return std::ptr::null_mut();
    };
    let text = h.session.display();
    env.new_string(text).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_suggest(
    mut env: JNIEnv,
    _class: JClass,
    _ptr: jlong,
    prefix: JString,
) -> jstring {
    let prefix: String = env.get_string(&prefix).map(|s| s.into()).unwrap_or_default();
    let values = vfk_suggest::suggest(prefix.trim(), 6);
    let json = serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string());
    env.new_string(json).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_process(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    key: JString,
    ctrl: jboolean,
    alt: jboolean,
    meta: jboolean,
) -> jstring {
    let Some(h) = handle(ptr) else {
        return std::ptr::null_mut();
    };
    let key: String = match env.get_string(&key) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    let mut event = KeyEvent::parse(&key);
    event.ctrl = ctrl != 0;
    event.alt = alt != 0;
    event.meta = meta != 0;
    let action = h.session.process_key(&event);
    to_jstring(&mut env, action_json(&action))
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_setSurrounding(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    text: JString,
) {
    let Some(h) = handle(ptr) else {
        return;
    };
    let value: String = env.get_string(&text).map(|s| s.into()).unwrap_or_default();
    *h.surrounding.borrow_mut() = if value.is_empty() { None } else { Some(value) };
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_focus(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    app: JString,
) -> jstring {
    let Some(h) = handle(ptr) else {
        return std::ptr::null_mut();
    };
    let app: String = env.get_string(&app).map(|s| s.into()).unwrap_or_default();
    *h.app.borrow_mut() = if app.is_empty() { None } else { Some(app.clone()) };
    let action = h.session.focus_changed(if app.is_empty() { None } else { Some(&app) });
    to_jstring(&mut env, action_json(&action))
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_cursorMoved(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    let Some(h) = handle(ptr) else {
        return std::ptr::null_mut();
    };
    let action = h.session.cursor_moved();
    to_jstring(&mut env, action_json(&action))
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_toggle(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    let Some(h) = handle(ptr) else {
        return std::ptr::null_mut();
    };
    let action = h.session.toggle_vietnamese();
    to_jstring(&mut env, action_json(&action))
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_isEnabled(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jboolean {
    handle(ptr).map(|h| if h.session.is_enabled() { 1 } else { 0 }).unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_reset(_env: JNIEnv, _class: JClass, ptr: jlong) {
    if let Some(h) = handle(ptr) {
        h.session.reset();
    }
}

#[no_mangle]
pub extern "system" fn Java_app_vifastkey_NativeEngine_setField(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    field: jni::sys::jint,
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
