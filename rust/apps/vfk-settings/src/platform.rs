//! Mở thư mục/file bằng ứng dụng mặc định của hệ điều hành.

use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

pub fn open_path(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

/// `windows.json` nằm cạnh `config.json`, không thuộc schema engine.
pub fn windows_backend_path(config_path: &Path) -> PathBuf {
    config_path.with_file_name("windows.json")
}

pub fn load_windows_backend(config_path: &Path) -> String {
    let path = windows_backend_path(config_path);
    std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value.get("backend").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "tsf".to_string())
}

pub fn save_windows_backend(config_path: &Path, backend: &str) -> io::Result<()> {
    let path = windows_backend_path(config_path);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let bytes =
        serde_json::to_vec_pretty(&serde_json::json!({ "backend": backend })).map_err(io::Error::other)?;
    std::fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_backend_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.json");
        assert_eq!(load_windows_backend(&config), "tsf");
        save_windows_backend(&config, "tsf").unwrap();
        assert_eq!(load_windows_backend(&config), "tsf");
    }
}
