//! WASM binding cho ViFastKey.
//!
//! Trả Action dưới dạng JSON để JS không phải biết layout Rust:
//! `{"type":"pass"}` · `{"type":"preedit","text":...,"cursor":...}` ·
//! `{"type":"replace","delete_before":N,"insert":...,"pass_key":bool}` ·
//! `{"type":"commit","text":...,"pass_key":bool}`.
//!
//! `delete_before` tính bằng **code point** (JS tự đổi sang UTF-16 khi cần).

use std::cell::RefCell;
use std::rc::Rc;

use vfk_core::charset;
use vfk_core::{to_raw, transform_str, InputMethodSpec, TransformOptions, SIMPLE_TELEX, TELEX, VIQR, VNI};
use vfk_engine::testing::WordListDictionary;
use vfk_engine::*;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
struct SharedConfig(Rc<RefCell<Config>>);

impl ConfigStore for SharedConfig {
    fn load(&self) -> Config {
        self.0.borrow().clone()
    }
    fn save(&self, config: &Config) {
        *self.0.borrow_mut() = config.clone();
    }
}

#[derive(Clone)]
struct SharedApp(Rc<RefCell<Option<String>>>);

impl AppContext for SharedApp {
    fn current_app(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}

#[derive(Clone)]
struct SharedSurrounding(Rc<RefCell<Option<String>>>);

impl SurroundingText for SharedSurrounding {
    fn before_cursor(&self, n: usize) -> Option<String> {
        let text = self.0.borrow().clone()?;
        let chars: Vec<char> = text.chars().collect();
        Some(chars[chars.len().saturating_sub(n)..].iter().collect())
    }
}

#[derive(Clone)]
struct SharedDictionary(Rc<RefCell<WordListDictionary>>);

impl Dictionary for SharedDictionary {
    fn is_english(&self, word: &str) -> bool {
        self.0.borrow().english.contains(word)
    }
    fn is_vietnamese(&self, word: &str) -> bool {
        self.0.borrow().vietnamese.contains(word)
    }
}

fn action_json(action: &Action) -> String {
    match action {
        Action::PassThrough => r#"{"type":"pass"}"#.to_string(),
        Action::Preedit { text, cursor } => serde_json::json!({
            "type": "preedit",
            "text": text,
            "cursor": cursor,
        })
        .to_string(),
        Action::Replace { delete_before, insert, pass_key } => serde_json::json!({
            "type": "replace",
            "delete_before": delete_before,
            "insert": insert,
            "pass_key": pass_key,
        })
        .to_string(),
        Action::Commit { text, pass_key } => serde_json::json!({
            "type": "commit",
            "text": text,
            "pass_key": pass_key,
        })
        .to_string(),
    }
}

#[wasm_bindgen]
pub struct WasmSession {
    inner: ImeSession,
    config: SharedConfig,
    app: SharedApp,
    surrounding: SharedSurrounding,
    dictionary: SharedDictionary,
}

#[wasm_bindgen]
impl WasmSession {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: Option<String>, mode: &str) -> Result<WasmSession, JsValue> {
        let config: Config = match config_json.as_deref() {
            None | Some("") => Config::default(),
            Some(text) => serde_json::from_str(text)
                .map_err(|e| JsValue::from_str(&format!("config JSON không hợp lệ: {e}")))?,
        };
        let config = SharedConfig(Rc::new(RefCell::new(config)));
        let app = SharedApp(Rc::new(RefCell::new(Some("browser".to_string()))));
        let surrounding = SharedSurrounding(Rc::new(RefCell::new(None)));
        let dictionary = SharedDictionary(Rc::new(RefCell::new(WordListDictionary::default())));
        let ports = Ports {
            config: Box::new(config.clone()),
            app: Box::new(app.clone()),
            surrounding: Box::new(surrounding.clone()),
            dictionary: Box::new(dictionary.clone()),
        };
        let output_mode = match mode {
            "preedit" => OutputMode::Preedit,
            _ => OutputMode::Replace,
        };
        Ok(WasmSession { inner: ImeSession::new(ports, output_mode), config, app, surrounding, dictionary })
    }

    /// Xử lý một phím. `key` là 1 ký tự hoặc tên phím
    /// ("Backspace", "Enter", "Escape", "Left", ...).
    pub fn process_key(&mut self, key: &str, ctrl: bool, alt: bool, meta: bool) -> String {
        let ev = KeyEvent { key: Key::parse(key), ctrl, alt, meta };
        action_json(&self.inner.process_key(&ev))
    }

    /// Cập nhật phần văn bản ngay trước con trỏ (surrounding text).
    pub fn set_surrounding(&mut self, text: Option<String>) {
        *self.surrounding.0.borrow_mut() = text;
    }

    /// Đổi app đang focus; có thể trả COMMIT khi đang có preedit.
    pub fn focus_changed(&mut self, app_id: Option<String>) -> String {
        *self.app.0.borrow_mut() = app_id.clone();
        action_json(&self.inner.focus_changed(app_id.as_deref()))
    }

    /// Người dùng click chuột / con trỏ đổi chỗ.
    pub fn cursor_moved(&mut self) -> String {
        action_json(&self.inner.cursor_moved())
    }

    /// 0 text, 1 password, 2 number, 3 email, 4 url, 5 terminal.
    pub fn set_field_kind(&mut self, kind: &str) -> String {
        let kind = match kind {
            "password" => FieldKind::Password,
            "number" => FieldKind::Number,
            "email" => FieldKind::Email,
            "url" => FieldKind::Url,
            "terminal" => FieldKind::Terminal,
            _ => FieldKind::Text,
        };
        action_json(&self.inner.set_field_kind(kind))
    }

    /// Bật/tắt tiếng Việt; có thể trả COMMIT.
    pub fn toggle_vietnamese(&mut self) -> String {
        action_json(&self.inner.toggle_vietnamese())
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }

    /// Cấu hình hiện tại dạng JSON.
    pub fn config_json(&self) -> String {
        serde_json::to_string(self.inner.config()).unwrap_or_default()
    }

    /// Cập nhật một phần cấu hình (merge với cấu hình hiện tại).
    pub fn set_config(&mut self, json: &str) -> bool {
        let Some(c) = self.config.0.borrow().apply_json_patch(json) else {
            return false;
        };
        *self.config.0.borrow_mut() = c;
        self.inner.reload_config();
        true
    }

    /// Nạp từ điển từ 2 danh sách (mỗi từ cách nhau bởi khoảng trắng/xuống dòng).
    pub fn load_dictionary(&mut self, english: &str, vietnamese: &str) {
        let mut d = self.dictionary.0.borrow_mut();
        d.english = english.split_whitespace().map(|w| w.to_lowercase()).collect();
        d.vietnamese = vietnamese.split_whitespace().map(|w| w.to_lowercase()).collect();
    }

    /// Chữ đang hiển thị của từ hiện tại.
    pub fn display(&self) -> String {
        self.inner.display()
    }

    /// Chuỗi phím thô của từ hiện tại (để hiển thị debug).
    pub fn raw(&self) -> String {
        self.inner.raw_string()
    }

    /// Xoá từ đang gõ mà không phát Action.
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

fn input_method(name: &str) -> &'static InputMethodSpec {
    match name {
        "simple_telex" => &SIMPLE_TELEX,
        "vni" => &VNI,
        "viqr" => &VIQR,
        _ => &TELEX,
    }
}

fn word_char(ch: char) -> bool {
    ch.is_alphabetic() || (ch as u32) >= 0x0300 && (ch as u32) <= 0x036f
}

/// Chuẩn hoá một đoạn văn bản: chuyển Telex/VNI thô thành chữ Việt và
/// chuẩn hoá các từ đã có dấu. Giữ nguyên dấu câu/khoảng trắng.
/// Từ tiếng Anh đã biết trong lexicon được giữ nguyên để tránh `data` -> `dât`.
#[wasm_bindgen]
pub fn normalize_text(text: &str, method: &str, tone_style: &str) -> String {
    let im = input_method(method);
    let opt = TransformOptions {
        tone_style: if tone_style == "new" { vfk_core::ToneStyle::New } else { vfk_core::ToneStyle::Old },
        ..TransformOptions::default()
    };

    fn normalize_word(word: &str, im: &InputMethodSpec, opt: &TransformOptions) -> String {
        let marked = word.chars().any(|ch| {
            let d = vi_syllable::decompose_char(ch);
            d.tone != 0 || d.mark.is_some()
        });
        if marked {
            let Some(raw) = to_raw(word, im) else { return word.to_string() };
            let result = transform_str(&raw, im, opt, true);
            if result.status != vfk_core::Status::Invalid && !result.text.is_empty() {
                result.text
            } else {
                word.to_string()
            }
        } else if vfk_suggest::contains(&word.to_lowercase()) {
            word.to_string()
        } else {
            let result = transform_str(word, im, opt, true);
            if result.status != vfk_core::Status::Invalid && !result.text.is_empty() {
                result.text
            } else {
                word.to_string()
            }
        }
    }

    let mut out = String::new();
    let mut word = String::new();
    for ch in text.chars() {
        if word_char(ch) {
            word.push(ch);
        } else {
            if !word.is_empty() {
                out.push_str(&normalize_word(&word, im, &opt));
                word.clear();
            }
            out.push(ch);
        }
    }
    if !word.is_empty() {
        out.push_str(&normalize_word(&word, im, &opt));
    }
    out
}

/// Danh sách bảng mã dạng JSON: [{key,name}].
#[wasm_bindgen]
pub fn charset_list() -> String {
    serde_json::to_string(
        &vfk_core::CHARSETS
            .iter()
            .map(|key| serde_json::json!({"key": key, "name": charset::display_name(key).unwrap_or(key)}))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string())
}

#[wasm_bindgen]
pub fn charset_encode(name: &str, text: &str) -> String {
    charset::encode(text, name)
}

#[wasm_bindgen]
pub fn charset_decode(name: &str, text: &str) -> String {
    charset::decode(text, name)
}

/// Bytes thô cho bảng một byte; mảng rỗng nếu bảng không hỗ trợ.
#[wasm_bindgen]
pub fn charset_encode_bytes(name: &str, text: &str) -> Vec<u8> {
    charset::encode_bytes(text, name).unwrap_or_default()
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
