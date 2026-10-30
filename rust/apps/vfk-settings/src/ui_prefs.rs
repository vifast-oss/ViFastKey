//! State riêng của UI (không thuộc schema engine).

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UiPrefs {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub last_tab: Option<String>,
}

impl UiPrefs {
    pub fn load(config_path: &Path) -> Self {
        let path = vfk_config::paths::ui_path(config_path);
        std::fs::read(&path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
    }

    pub fn save(&self, config_path: &Path) {
        let path = vfk_config::paths::ui_path(config_path);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(path, bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_next_to_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.json");
        let prefs = UiPrefs { theme: Some("dark".into()), last_tab: Some("apps".into()) };
        prefs.save(&config);
        let loaded = UiPrefs::load(&config);
        assert_eq!(loaded.theme.as_deref(), Some("dark"));
        assert_eq!(loaded.last_tab.as_deref(), Some("apps"));
    }
}
