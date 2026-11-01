//! Trạng thái nhỏ do tiến trình hook ghi lại để UI/tray và toast đọc.
//!
//! File này chỉ chứa mode/input source, không chứa nội dung gõ.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookMode {
    Vietnamese,
    English,
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputSource {
    EnUs,
    Vietnamese,
    Japanese,
    Other,
    Unknown,
}

impl InputSource {
    pub fn label(self) -> &'static str {
        match self {
            InputSource::EnUs => "English US",
            InputSource::Vietnamese => "Vietnamese",
            InputSource::Japanese => "Japanese",
            InputSource::Other => "ngôn ngữ khác",
            InputSource::Unknown => "không xác định",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookState {
    pub pid: u32,
    pub mode: HookMode,
    pub input: InputSource,
    pub updated_ms: u128,
}

impl HookState {
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn now(pid: u32, mode: HookMode, input: InputSource) -> Self {
        let updated_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        Self { pid, mode, input, updated_ms }
    }
}

pub fn state_path() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("ViFastKey").join("hook-state.json")
}

pub fn show_request_path() -> PathBuf {
    state_path().with_file_name("show-ui")
}

#[cfg(windows)]
pub fn request_show() {
    if let Some(parent) = show_request_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(show_request_path(), b"1");
}

pub fn take_show_request() -> bool {
    let path = show_request_path();
    if path.exists() {
        let _ = std::fs::remove_file(path);
        true
    } else {
        false
    }
}

#[cfg(windows)]
pub fn toggle_request_path() -> PathBuf {
    state_path().with_file_name("toggle-request")
}

#[cfg(windows)]
pub fn request_toggle() {
    if let Some(parent) = toggle_request_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(toggle_request_path(), b"1");
}

#[cfg(windows)]
pub fn take_toggle_request() -> bool {
    let path = toggle_request_path();
    if path.exists() {
        let _ = std::fs::remove_file(path);
        true
    } else {
        false
    }
}

pub fn load() -> Option<HookState> {
    let bytes = std::fs::read(state_path()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(windows)]
pub fn save(state: &HookState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(bytes) = serde_json::to_vec_pretty(state) else { return };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, bytes).is_ok() {
        let _ = std::fs::rename(tmp, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_json() {
        let state = HookState::now(42, HookMode::External, InputSource::Japanese);
        let bytes = serde_json::to_vec(&state).unwrap();
        let decoded: HookState = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.mode, HookMode::External);
        assert_eq!(decoded.input, InputSource::Japanese);
    }
}
