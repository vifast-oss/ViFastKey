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

/// `windows.json` là file legacy; giữ đọc để migration khỏi TSF.
pub fn windows_backend_path(config_path: &Path) -> PathBuf {
    config_path.with_file_name("windows.json")
}

#[allow(dead_code)]
pub fn load_windows_backend(config_path: &Path) -> String {
    let path = windows_backend_path(config_path);
    let backend = std::fs::read(&path)
        .ok()
        .and_then(|bytes| vfk_config::raw::parse_object(&bytes).ok())
        .and_then(|value| value.get("backend").and_then(|v| v.as_str()).map(str::to_string))
        .unwrap_or_else(|| "hook".to_string());
    if backend == "tsf" {
        "tsf".into()
    } else {
        "hook".into()
    }
}

#[allow(dead_code)]
pub fn save_windows_backend(config_path: &Path, backend: &str) -> io::Result<()> {
    let path = windows_backend_path(config_path);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let backend = if backend == "tsf" { "tsf" } else { "hook" };
    let bytes =
        serde_json::to_vec_pretty(&serde_json::json!({ "backend": backend })).map_err(io::Error::other)?;
    std::fs::write(path, bytes)
}

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Trả false nếu đã có một UI Settings đang chạy; khi đó ghi request để UI cũ hiện lên.
#[cfg(target_os = "windows")]
pub fn ensure_ui_instance() -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    static NAME: &str = "Local\\ViFastKeySettingsUi\0";
    let name: Vec<u16> = NAME.encode_utf16().collect();
    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 1, name.as_ptr()) };
    if handle.is_null() {
        return true; // không chặn app nếu mutex lỗi
    }
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if already {
        // Ưu tiên hiện cửa sổ đang tồn tại; nếu process ẩn chưa render, ghi request
        // để UI cũ tự bật khi vòng lặp thức.
        show_existing_settings_window();
        crate::hook_state::request_show();
        false
    } else {
        // Không CloseHandle: mutex sống tới khi process kết thúc.
        true
    }
}

#[cfg(target_os = "windows")]
fn show_existing_settings_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };
    let title: Vec<u16> = "ViFastKey Settings\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_ui_instance() -> bool {
    true
}

#[cfg(target_os = "windows")]
fn hidden_command<P: AsRef<std::ffi::OsStr>>(program: P) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut command = std::process::Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

/// Chạy hook nền bằng chính binary Settings (`--hook`) và đặt autostart `--hidden`.
#[cfg(target_os = "windows")]
pub fn ensure_hook_service() -> Result<(), String> {
    use std::time::Duration;

    let exe = std::env::current_exe().map_err(|err| format!("không tìm được exe: {err}"))?;
    // Dọn hook legacy (khác tên với Settings) để không chạy hai bộ gõ.
    for image in ["vifastkey-replace.exe", "vfk-windows.exe"] {
        let _ = hidden_command("taskkill").args(["/IM", image, "/F"]).output();
    }
    if !hook_running() {
        hidden_command(&exe).arg("--hook").spawn().map_err(|err| format!("không chạy được hook: {err}"))?;
        std::thread::sleep(Duration::from_millis(250));
    }
    set_autostart(Some(&exe))?;
    Ok(())
}

/// Bật hook: giữ tên cũ cho code/CLI đang gọi.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn ensure_replace_mode() -> Result<(), String> {
    ensure_hook_service()
}

#[cfg(target_os = "windows")]
pub fn hook_running() -> bool {
    let Some(state) = crate::hook_state::load() else {
        return false;
    };
    process_running_pid(state.pid)
}

#[cfg(target_os = "windows")]
fn process_running_pid(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    hidden_command("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .map(|output| {
            let text = String::from_utf8_lossy(&output.stdout);
            text.contains(&format!("\"{pid}\""))
        })
        .unwrap_or(false)
}

/// Dừng hook theo PID trong state file; không kill nhầm process Settings.
#[cfg(target_os = "windows")]
pub fn stop_hook() {
    if let Some(state) = crate::hook_state::load() {
        if state.pid != 0 {
            let _ = hidden_command("taskkill").args(["/PID", &state.pid.to_string(), "/F"]).output();
        }
    }
    let _ = std::fs::remove_file(crate::hook_state::state_path());
}

/// Tắt hook: bỏ autostart và dừng process hook.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn disable_replace_mode() -> Result<(), String> {
    set_autostart(None)?;
    stop_hook();
    // Dọn process legacy từ bản trước.
    for image in ["vifastkey-replace.exe", "vfk-windows.exe"] {
        let _ = hidden_command("taskkill").args(["/IM", image, "/F"]).output();
    }
    if hook_running() {
        return Err("không dừng được hook; mở Task Manager để kết thúc.".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_autostart(exe: Option<&Path>) -> Result<(), String> {
    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let output = match exe {
        Some(exe) => {
            let _ = hidden_command("reg").args(["DELETE", KEY, "/v", "ViFastKeyHook", "/f"]).output();
            hidden_command("reg")
                .args([
                    "ADD",
                    KEY,
                    "/v",
                    "ViFastKey",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &format!("\"{}\" --hidden", exe.display()),
                    "/f",
                ])
                .output()
        }
        None => {
            let _ = hidden_command("reg").args(["DELETE", KEY, "/v", "ViFastKeyHook", "/f"]).output();
            hidden_command("reg").args(["DELETE", KEY, "/v", "ViFastKey", "/f"]).output()
        }
    }
    .map_err(|err| format!("không cập nhật Run key: {err}"))?;

    if exe.is_some() && !output.status.success() {
        return Err(format!(
            "không thêm được ViFastKey vào Run key: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_hook_service() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn ensure_replace_mode() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn disable_replace_mode() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn hook_running() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn stop_hook() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_backend_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.json");
        assert_eq!(load_windows_backend(&config), "hook");
        save_windows_backend(&config, "hook").unwrap();
        assert_eq!(load_windows_backend(&config), "hook");
        std::fs::write(windows_backend_path(&config), br#"{"backend":"tsf"}"#).unwrap();
        assert_eq!(load_windows_backend(&config), "tsf");
    }
}
