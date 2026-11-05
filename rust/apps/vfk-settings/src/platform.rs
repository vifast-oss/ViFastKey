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

/// Một app đang có cửa sổ, dùng cho picker thay vì bắt người dùng gõ id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningApp {
    pub id: String,
    pub label: String,
}

/// Liệt kê app đang chạy có cửa sổ nhìn thấy. Windows dùng Win32; Linux đọc
/// tiến trình của user hiện tại và đối chiếu với các file `.desktop`.
#[cfg(target_os = "windows")]
pub fn running_apps() -> Vec<RunningApp> {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
    };

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        // Bỏ tool window/overlay; vẫn giữ app thường và game fullscreen.
        if GetWindowLongW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW as i32 != 0 {
            return 1;
        }
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return 1;
        }
        let mut title_buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
        if copied <= 0 {
            return 1;
        }
        let title = String::from_utf16_lossy(&title_buf[..copied as usize]);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return 1;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return 1;
        }
        let mut path_buf = [0u16; 1024];
        let mut path_len = path_buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, path_buf.as_mut_ptr(), &mut path_len);
        CloseHandle(handle);
        if ok == 0 {
            return 1;
        }
        let path = String::from_utf16_lossy(&path_buf[..path_len as usize]);
        let name = path.rsplit(['\\', '/']).next().unwrap_or(&path);
        let id = vfk_engine::normalize_app_id(Some(name));
        if id.is_empty() {
            return 1;
        }
        let title: String = title.chars().take(100).collect();
        let label = format!("{id}  ·  {title}");
        let apps = &mut *(lparam as *mut Vec<RunningApp>);
        if !apps.iter().any(|app| app.id == id && app.label == label) {
            apps.push(RunningApp { id, label });
        }
        1
    }

    let mut apps: Vec<RunningApp> = Vec::new();
    unsafe {
        EnumWindows(Some(collect), &mut apps as *mut Vec<RunningApp> as LPARAM);
    }
    apps.sort_by(|a: &RunningApp, b: &RunningApp| a.id.cmp(&b.id).then(a.label.cmp(&b.label)));
    apps
}

#[cfg(target_os = "linux")]
pub fn running_apps() -> Vec<RunningApp> {
    use std::collections::BTreeMap;
    use std::os::unix::fs::MetadataExt;

    // Chỉ lấy tiến trình của user đang chạy Settings, tránh thấy app của user khác.
    let current_uid = std::fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(0);
    let current_exe = std::env::current_exe().ok().and_then(|p| std::fs::canonicalize(p).ok());
    let desktop_names = linux_desktop_exec_names();
    let mut found = BTreeMap::<String, RunningApp>::new();

    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let proc_dir = entry.path();
        if std::fs::metadata(&proc_dir).map(|m| m.uid()).ok() != Some(current_uid) {
            continue;
        }
        let exe_path = std::fs::read_link(proc_dir.join("exe")).ok();
        if current_exe.is_some() && exe_path == current_exe {
            continue;
        }
        let exe_name = exe_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let comm = std::fs::read_to_string(proc_dir.join("comm")).unwrap_or_default();
        let comm = comm.trim().to_ascii_lowercase();
        let name = if !exe_name.is_empty() { exe_name.as_str() } else { comm.as_str() };
        if !linux_is_gui_process(name, &desktop_names) {
            continue;
        }
        let id = vfk_engine::normalize_app_id(Some(name));
        if id.is_empty() || found.contains_key(&id) {
            continue;
        }
        let label =
            if comm.is_empty() || comm == name { name.to_string() } else { format!("{name}  ·  {comm}") };
        let _ = pid; // giữ PID trong tương lai nếu cần hiển thị
        found.insert(id.clone(), RunningApp { id, label });
    }
    found.into_values().collect()
}

#[cfg(target_os = "linux")]
fn linux_desktop_exec_names() -> std::collections::BTreeSet<String> {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    let mut dirs: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ];
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        dirs.push(Path::new(&data_home).join("applications"));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push(Path::new(&home).join(".local/share/applications"));
    }
    if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in data_dirs.split(':').filter(|d| !d.is_empty()) {
            dirs.push(Path::new(dir).join("applications"));
        }
    }

    let mut names = BTreeSet::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.insert(stem.to_ascii_lowercase());
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let mut exec_seen = false;
            for line in text.lines() {
                let Some((key, value)) = line.split_once('=') else { continue };
                match key.trim() {
                    "Exec" if !exec_seen => {
                        exec_seen = true;
                        linux_insert_exec_name(&mut names, value);
                    }
                    "StartupWMClass" => {
                        let value = value.trim().trim_matches(['"', '\'']).to_ascii_lowercase();
                        if !value.is_empty() {
                            names.insert(value);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    names
}

#[cfg(target_os = "linux")]
fn linux_insert_exec_name(names: &mut std::collections::BTreeSet<String>, value: &str) {
    let mut parts = value.split_whitespace();
    let mut token = parts.next().unwrap_or("");
    if token == "env" {
        token = parts.next().unwrap_or("");
    }
    let token = token.trim_matches(['"', '\'']);
    let base = token.rsplit('/').next().unwrap_or(token).to_ascii_lowercase();
    if !base.is_empty() && !base.starts_with('%') && base != "sh" && base != "bash" && base != "env" {
        names.insert(base);
    }
}

#[cfg(target_os = "linux")]
fn linux_is_gui_process(name: &str, desktop_names: &std::collections::BTreeSet<String>) -> bool {
    if name.is_empty() || name.contains(char::is_whitespace) || name.starts_with('.') {
        return false;
    }
    const NOISE: &[&str] = &[
        "systemd",
        "dbus-daemon",
        "dbus-broker",
        "pipewire",
        "wireplumber",
        "gnome-shell",
        "gnome-session",
        "gjs",
        "xwayland",
        "xorg",
        "ibus-",
        "fcitx5",
        "goa-",
        "python",
        "gdm",
        "sshd",
        "bash",
        "sh",
        "zsh",
        "fish",
        "xdg-desktop-portal",
        "evolution-alarm-notify",
        "update-notifier",
        "at-spi",
        "snapd",
        "cupsd",
        "packagekitd",
        "tracker-miner",
        "gnome-keyring",
    ];
    if NOISE.iter().any(|noise| name.starts_with(noise)) {
        return false;
    }
    if desktop_names.contains(name) {
        return true;
    }
    // Một số app snap/Electron đổi tên binary khỏi Exec trong .desktop.
    const ALIASES: &[&str] = &[
        "chrome",
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "firefox",
        "firefox-esr",
        "code",
        "code-insiders",
        "codium",
        "zed",
        "alacritty",
        "kitty",
        "wezterm",
        "gnome-terminal-server",
        "konsole",
        "xterm",
        "slack",
        "discord",
        "telegram-desktop",
        "thunderbird",
        "soffice.bin",
        "gimp",
        "inkscape",
        "krita",
        "obs",
        "vlc",
        "mpv",
        "steam",
        "lutris",
        "java",
    ];
    ALIASES.contains(&name)
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn running_apps() -> Vec<RunningApp> {
    Vec::new()
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
        show_main_window();
        crate::hook_state::request_show();
        false
    } else {
        // Không CloseHandle: mutex sống tới khi process kết thúc.
        true
    }
}

/// Đánh thức cửa sổ Settings đang chạy. Tìm cả title có dấu `*` khi form dirty.
#[cfg(target_os = "windows")]
pub fn show_main_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    for title in ["ViFastKey Settings", "ViFastKey Settings *"] {
        let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if !hwnd.is_null() {
            unsafe {
                ShowWindow(hwnd, SW_RESTORE);
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
            }
            return;
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
