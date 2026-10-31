//! Mở thư mục/file bằng ứng dụng mặc định của hệ điều hành.

use std::io;
use std::path::{Path, PathBuf};

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
    let backend = std::fs::read(&path)
        .ok()
        .and_then(|bytes| vfk_config::raw::parse_object(&bytes).ok())
        .and_then(|value| value.get("backend").and_then(|v| v.as_str()).map(str::to_string))
        .unwrap_or_else(|| "tsf".to_string());
    // `auto` là giá trị legacy; UI hiện chỉ còn Preedit/Replace và mặc định Preedit.
    if backend == "hook" {
        "hook".into()
    } else {
        "tsf".into()
    }
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

/// Tên process/binary Replace. `vfk-windows` cũ là tên legacy cần dọn khi nâng cấp.
#[cfg(target_os = "windows")]
const HOOK_EXE: &str = "vifastkey-replace.exe";
#[cfg(target_os = "windows")]
const LEGACY_HOOK_EXE: &str = "vfk-windows.exe";

/// Bật Replace: chạy hook nền và thêm vào Run key của user hiện tại.
#[cfg(target_os = "windows")]
pub fn ensure_replace_mode() -> Result<(), String> {
    use std::process::Command;
    use std::time::Duration;

    stop_process(LEGACY_HOOK_EXE);
    let exe = std::env::current_exe().map_err(|err| format!("không tìm được exe: {err}"))?;
    let hook = exe.with_file_name(HOOK_EXE);
    if !hook.exists() {
        return Err(format!("Không thấy {}; cài lại gói ViFastKey có {HOOK_EXE}.", hook.display()));
    }

    if !hook_running() {
        Command::new(&hook).spawn().map_err(|err| format!("không chạy được {}: {err}", hook.display()))?;
        std::thread::sleep(Duration::from_millis(200));
    }
    set_hook_autostart(Some(&hook))?;
    Ok(())
}

/// Tắt Replace: bỏ Run key và dừng hook nền (kể cả process legacy).
#[cfg(target_os = "windows")]
pub fn disable_replace_mode() -> Result<(), String> {
    set_hook_autostart(None)?;
    stop_process(HOOK_EXE);
    stop_process(LEGACY_HOOK_EXE);
    if hook_running() {
        return Err(format!("không dừng được {HOOK_EXE}; mở Task Manager để kết thúc."));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn hook_running() -> bool {
    process_running(HOOK_EXE)
}

#[cfg(target_os = "windows")]
fn process_running(image: &str) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {image}"), "/NH"])
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout).to_ascii_lowercase().contains(&image.to_ascii_lowercase())
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn stop_process(image: &str) {
    let _ = std::process::Command::new("taskkill").args(["/IM", image, "/F"]).output();
}

#[cfg(target_os = "windows")]
fn set_hook_autostart(hook: Option<&Path>) -> Result<(), String> {
    use std::process::Command;

    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let output = match hook {
        Some(hook) => Command::new("reg")
            .args([
                "ADD",
                KEY,
                "/v",
                "ViFastKeyHook",
                "/t",
                "REG_SZ",
                "/d",
                &format!("\"{}\"", hook.display()),
                "/f",
            ])
            .output(),
        None => Command::new("reg").args(["DELETE", KEY, "/v", "ViFastKeyHook", "/f"]).output(),
    }
    .map_err(|err| format!("không cập nhật Run key: {err}"))?;

    if hook.is_some() && !output.status.success() {
        return Err(format!(
            "không thêm được ViFastKeyHook vào Run key: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_replace_mode() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn disable_replace_mode() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn hook_running() -> bool {
    false
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

        std::fs::write(windows_backend_path(&config), b"\xEF\xBB\xBF{\"backend\":\"hook\"}").unwrap();
        assert_eq!(load_windows_backend(&config), "hook");

        std::fs::write(windows_backend_path(&config), br#"{"backend":"auto"}"#).unwrap();
        assert_eq!(load_windows_backend(&config), "tsf", "auto legacy được đọc như Preedit");
    }
}
