//! Port ra (driven ports) của hexagon + cấu hình. Port của `vfk_engine/ports.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vi_syllable::ToneStyle;

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    CONFIG_SCHEMA_VERSION
}

/// Mô hình xuất chữ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// TSF, IMK, Fcitx5, IBus, Android, iOS (chữ gạch chân).
    Preedit,
    /// Hook + gửi backspace (kiểu UniKey/OpenKey).
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum UnicodeForm {
    #[default]
    Nfc,
    Nfd,
}

/// Gợi ý cho ADAPTER theo từng app (engine chỉ dùng `edit_previous_word`, `vietnamese`).
/// Rút ra từ lỗi thực tế: vnkey (terminal, Electron), fcitx5-lotus (ack Chrome, độ trễ theo độ dài).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppProfile {
    pub output_mode: Option<OutputMode>,
    /// Trễ cơ sở giữa các backspace giả.
    pub backspace_delay_ms: u32,
    /// Cộng thêm theo số ký tự bị xoá.
    pub delay_per_char_ms: u32,
    /// Chờ app xử lý xong backspace rồi mới chèn.
    pub wait_ack: bool,
    pub edit_previous_word: Option<bool>,
    /// Ép bật/tắt (game, ...).
    pub vietnamese: Option<bool>,
    /// Ghi đè kiểu gõ theo app.
    pub input_method: Option<String>,
    /// Ghi đè kiểu đặt dấu theo app.
    pub tone_style: Option<ToneStyle>,
    /// Ghi đè kiểm tra chính tả theo app.
    pub spell_check: Option<bool>,
    /// Gợi ý bảng mã cho adapter.
    pub charset: Option<String>,
}

pub fn app_aliases() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("google-chrome", "chrome"),
        ("google-chrome-stable", "chrome"),
        ("google-chrome-beta", "chrome"),
        ("microsoft-edge", "msedge"),
        ("microsoft-edge-stable", "msedge"),
        ("vscode", "code"),
        ("code-url-handler", "code"),
        ("soffice.bin", "libreoffice"),
        ("gnome-terminal-server", "gnome-terminal"),
    ])
}

pub fn default_app_profiles() -> HashMap<String, AppProfile> {
    let term = AppProfile {
        output_mode: Some(OutputMode::Preedit),
        edit_previous_word: Some(false),
        ..Default::default()
    };
    let chromium = AppProfile { wait_ack: true, delay_per_char_ms: 2, ..Default::default() };
    let electron =
        AppProfile { backspace_delay_ms: 3, output_mode: Some(OutputMode::Preedit), ..Default::default() };
    let mut m = HashMap::new();
    for k in ["terminal", "wezterm", "kitty", "alacritty", "iterm2", "windowsterminal"] {
        m.insert(k.to_string(), term.clone());
    }
    for k in ["chrome", "chromium", "brave", "msedge"] {
        m.insert(k.to_string(), chromium.clone());
    }
    m.insert(
        "firefox".into(),
        AppProfile {
            backspace_delay_ms: 4,
            edit_previous_word: Some(false),
            output_mode: Some(OutputMode::Preedit),
            ..Default::default()
        },
    );
    for k in ["code", "slack", "discord"] {
        m.insert(k.to_string(), electron.clone());
    }
    m.insert(
        "notion".into(),
        AppProfile { backspace_delay_ms: 5, output_mode: Some(OutputMode::Preedit), ..Default::default() },
    );
    m.insert("minecraft".into(), AppProfile { vietnamese: Some(false), ..Default::default() });
    m
}

/// Cấu hình. Tên trường và giá trị JSON trùng bản Python (dùng chung config.json / conformance).
/// File người dùng chỉ cần chứa app đã ghi đè; preset còn lại vẫn được giữ.
/// Entry user hiện thay thế toàn bộ preset của cùng app (schema 1).
fn deserialize_app_profiles<'de, D>(deserializer: D) -> Result<HashMap<String, AppProfile>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let user = HashMap::<String, AppProfile>::deserialize(deserializer)?;
    let mut profiles = default_app_profiles();
    profiles.extend(user);
    Ok(profiles)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // --- kiểu gõ
    /// telex | simple_telex | vni
    pub input_method: String,
    pub tone_style: ToneStyle,
    pub spell_check: bool,
    pub w_as_u: bool,
    pub bracket_keys: bool,
    pub quick_telex: bool,
    /// Âm cuối k cho tên riêng (Đắk Lắk); chỉ đi với thanh sắc/nặng.
    pub allow_coda_k: bool,
    // --- sau khi kết thúc từ
    pub macro_enabled: bool,
    /// Chữ số thuộc từ để macro có số hoạt động (mp3, k8s); cần `macro_enabled`.
    pub macro_digits: bool,
    /// Backspace ngay sau macro: khôi phục phím gõ.
    pub macro_undo: bool,
    /// Vẫn mở macro khi đang tắt tiếng Việt.
    pub macro_in_off_mode: bool,
    pub macros: HashMap<String, String>,
    pub auto_capitalize: bool,
    // --- ngữ cảnh
    pub excluded_apps: Vec<String>,
    pub app_modes: HashMap<String, bool>,
    pub default_vietnamese: bool,
    pub edit_previous_word: bool,
    #[serde(default, deserialize_with = "deserialize_app_profiles")]
    pub app_profiles: HashMap<String, AppProfile>,
    /// Backspace ngay sau dấu cách: nạp lại trạng thái từ trước (soft reset).
    pub restore_after_space: bool,
    /// 2 chữ hoa đầu từ (trừ DD) -> gõ thẳng.
    pub smart_caps: bool,
    // --- đầu ra
    pub unicode_form: UnicodeForm,
    /// Phiên bản tài liệu cấu hình; file cũ thiếu trường này được hiểu là 1.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input_method: "telex".into(),
            tone_style: ToneStyle::New,
            spell_check: true,
            w_as_u: false,
            bracket_keys: false,
            quick_telex: false,
            allow_coda_k: false,
            macro_enabled: true,
            macro_digits: false,
            macro_undo: true,
            macro_in_off_mode: false,
            macros: HashMap::new(),
            auto_capitalize: false,
            excluded_apps: Vec::new(),
            app_modes: HashMap::new(),
            default_vietnamese: true,
            edit_previous_word: true,
            app_profiles: default_app_profiles(),
            restore_after_space: true,
            smart_caps: false,
            unicode_form: UnicodeForm::Nfc,
            schema_version: CONFIG_SCHEMA_VERSION,
        }
    }
}

impl Config {
    /// Trộn một JSON một phần lên cấu hình hiện tại. Trả None nếu JSON sai.
    /// Dùng chung cho C ABI và WASM; khoá lạ bị bỏ qua như khi deserialize.
    pub fn apply_json_patch(&self, patch: &str) -> Option<Config> {
        let mut base = match serde_json::to_value(self) {
            Ok(serde_json::Value::Object(m)) => m,
            _ => return None,
        };
        let patch: HashMap<String, serde_json::Value> = serde_json::from_str(patch).ok()?;
        for (k, v) in patch {
            base.insert(k, v);
        }
        serde_json::from_value(serde_json::Value::Object(base)).ok()
    }
}

// ---------------------------------------------------------------- port ra

pub trait ConfigStore {
    fn load(&self) -> Config;
    fn save(&self, config: &Config);
}

pub trait AppContext {
    fn current_app(&self) -> Option<String>;
}

pub trait SurroundingText {
    /// Tối đa `n` ký tự (Unicode scalar) ngay trước con trỏ.
    fn before_cursor(&self, n: usize) -> Option<String>;
}

pub trait Dictionary {
    /// Phím thô, chữ thường: "data".
    fn is_english(&self, word: &str) -> bool;
    /// Chữ tiếng Việt, thường, NFC: "nhưng".
    fn is_vietnamese(&self, word: &str) -> bool;
}

pub struct Ports {
    pub config: Box<dyn ConfigStore>,
    pub app: Box<dyn AppContext>,
    pub surrounding: Box<dyn SurroundingText>,
    pub dictionary: Box<dyn Dictionary>,
}

/// Chuẩn hoá app id: bỏ path/.exe/.app/.desktop, lowercase, lấy segment cuối của bundle id.
pub fn normalize_app_id(app_id: Option<&str>) -> String {
    let Some(app) = app_id else { return String::new() };
    let mut key = app.trim().to_lowercase().replace('\\', "/");
    key = key.rsplit('/').next().unwrap_or("").to_string();
    for suffix in [".exe", ".app", ".desktop"] {
        if let Some(stripped) = key.strip_suffix(suffix) {
            key = stripped.to_string();
        }
    }
    if let Some((head, _)) = key.split_once(':') {
        key = head.to_string();
    }
    let head = key.split('.').next().unwrap_or("");
    if matches!(head, "com" | "org" | "io" | "net" | "app") {
        key = key.rsplit('.').next().unwrap_or("").to_string();
    }
    app_aliases().get(key.as_str()).map(|v| (*v).to_string()).unwrap_or(key)
}

/// So khớp app id đã chuẩn hoá; fallback theo segment cuối của bundle id.
pub fn resolve_profile(config: &Config, app_id: Option<&str>) -> AppProfile {
    let key = normalize_app_id(app_id);
    if key.is_empty() {
        return AppProfile::default();
    }
    config.app_profiles.get(&key).cloned().unwrap_or_default()
}
