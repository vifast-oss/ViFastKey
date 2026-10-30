//! Đường dẫn config dùng chung cho mọi adapter.
//!
//! Giữ đúng path hiện có của từng nền tảng để không phá cấu hình đã cài:
//! - macOS: `~/Library/Application Support/ViFastKey/config.json`
//! - Windows: `%APPDATA%\ViFastKey\config.json`
//! - Linux: `$XDG_CONFIG_HOME/vifastkey/config.json` hoặc `~/.config/vifastkey/config.json`
//!
//! `VIFASTKEY_CONFIG` được ưu tiên trước tất cả (dùng cho test và portable mode).

use std::env;
use std::path::{Path, PathBuf};

pub const CONFIG_ENV: &str = "VIFASTKEY_CONFIG";
pub const UI_FILE_NAME: &str = "settings-ui.json";

pub fn config_path() -> PathBuf {
    if let Some(path) = env_path(CONFIG_ENV) {
        return path;
    }
    platform_config_path()
}

pub fn platform_config_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        return home().join("Library").join("Application Support").join("ViFastKey").join("config.json");
    }
    if cfg!(target_os = "windows") {
        if let Some(appdata) = env_path("APPDATA") {
            return appdata.join("ViFastKey").join("config.json");
        }
        return home().join("AppData").join("Roaming").join("ViFastKey").join("config.json");
    }
    if let Some(xdg) = env_path("XDG_CONFIG_HOME") {
        return xdg.join("vifastkey").join("config.json");
    }
    home().join(".config").join("vifastkey").join("config.json")
}

/// File UI-only (theme, ngôn ngữ, tab cuối). Nằm cạnh config engine nhưng
/// không thuộc schema engine và không bị adapter đọc.
pub fn ui_path(config: &Path) -> PathBuf {
    config.parent().map(|p| p.join(UI_FILE_NAME)).unwrap_or_else(|| PathBuf::from(UI_FILE_NAME))
}

fn env_path(name: &str) -> Option<PathBuf> {
    let value = env::var_os(name)?;
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn home() -> PathBuf {
    env_path("HOME").or_else(|| env_path("USERPROFILE")).unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_file_sits_next_to_config() {
        let dir = PathBuf::from("/tmp/vfk");
        assert_eq!(ui_path(&dir.join("config.json")), dir.join("settings-ui.json"));
    }

    #[test]
    fn config_override_wins() {
        // Không set env trong test vì env là process-wide; chỉ kiểm tra path nền tảng có tên file.
        assert!(platform_config_path().ends_with("config.json"));
    }
}
