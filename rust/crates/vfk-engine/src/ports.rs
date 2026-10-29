//! Port ra (driven ports) của hexagon + cấu hình. Port của `vfk_engine/ports.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vi_syllable::ToneStyle;

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
}

pub fn default_app_profiles() -> HashMap<String, AppProfile> {
    let term = AppProfile { output_mode: Some(OutputMode::Preedit), edit_previous_word: Some(false), ..Default::default() };
    let chromium = AppProfile { wait_ack: true, delay_per_char_ms: 2, ..Default::default() };
    let electron = AppProfile { backspace_delay_ms: 3, ..Default::default() };
    let mut m = HashMap::new();
    for k in ["terminal", "wezterm", "kitty", "alacritty", "iterm2", "windowsterminal"] {
        m.insert(k.to_string(), term.clone());
    }
    for k in ["chrome", "chromium", "brave", "msedge"] {
        m.insert(k.to_string(), chromium.clone());
    }
    m.insert(
        "firefox".into(),
        AppProfile { backspace_delay_ms: 4, edit_previous_word: Some(false), ..Default::default() },
    );
    for k in ["code", "slack", "discord"] {
        m.insert(k.to_string(), electron.clone());
    }
    m.insert("notion".into(), AppProfile { backspace_delay_ms: 5, ..Default::default() });
    m.insert("minecraft".into(), AppProfile { vietnamese: Some(false), ..Default::default() });
    m
}

/// Cấu hình. Tên trường và giá trị JSON trùng bản Python (dùng chung config.json / conformance).
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
    // --- sau khi kết thúc từ
    pub auto_restore_english: bool,
    pub macro_enabled: bool,
    pub macros: HashMap<String, String>,
    pub auto_capitalize: bool,
    // --- ngữ cảnh
    pub excluded_apps: Vec<String>,
    pub app_modes: HashMap<String, bool>,
    pub default_vietnamese: bool,
    pub edit_previous_word: bool,
    pub app_profiles: HashMap<String, AppProfile>,
    /// Backspace ngay sau dấu cách: nạp lại trạng thái từ trước (soft reset).
    pub restore_after_space: bool,
    /// 2 chữ hoa đầu từ (trừ DD) -> gõ thẳng.
    pub smart_caps: bool,
    // --- đầu ra
    pub unicode_form: UnicodeForm,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input_method: "telex".into(),
            tone_style: ToneStyle::Old,
            spell_check: true,
            w_as_u: true,
            bracket_keys: false,
            quick_telex: false,
            auto_restore_english: true,
            macro_enabled: true,
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
        }
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
    /// Chữ tiếng Việt, thường, NFC: "dât".
    fn is_vietnamese(&self, word: &str) -> bool;
}

pub struct Ports {
    pub config: Box<dyn ConfigStore>,
    pub app: Box<dyn AppContext>,
    pub surrounding: Box<dyn SurroundingText>,
    pub dictionary: Box<dyn Dictionary>,
}

/// So khớp không phân biệt hoa thường, theo tên tiến trình rút gọn (bỏ đường dẫn, .exe, .app).
pub fn resolve_profile(config: &Config, app_id: Option<&str>) -> AppProfile {
    let Some(app) = app_id else { return AppProfile::default() };
    if app.is_empty() {
        return AppProfile::default();
    }
    let lo = app.to_lowercase();
    let k = lo.rsplit('/').next().unwrap_or("");
    let k = k.rsplit('\\').next().unwrap_or("");
    let k = k.strip_suffix(".exe").unwrap_or(k);
    let k = k.strip_suffix(".app").unwrap_or(k);
    config.app_profiles.get(k).cloned().unwrap_or_default()
}
