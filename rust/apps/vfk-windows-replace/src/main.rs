//! Adapter Windows hook (Replace fallback): WH_KEYBOARD_LL + engine ViFastKey.
//!
//! Preedit thật nằm ở `windows/tsf/` (TSF text service). Adapter này dùng cho
//! app/game không hỗ trợ TSF hoặc khi `windows.json` đặt `backend: "hook"`.
//! Config đọc từ `%APPDATA%\ViFastKey\config.json`; dùng SendInput Unicode/backspace.
//! Build trên Windows bằng `cargo build -p vfk-windows-replace --release` (binary `vifastkey-replace.exe`).
//!
//! Giới hạn: low-level hook yêu cầu app chạy trong desktop session và một số app
//! chống hook; TSF là hướng production. Không chạy được trên macOS/Linux.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("vifastkey-replace chỉ chạy trên Windows.");
}

#[cfg(windows)]
mod win {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::sync::OnceLock;
    use std::time::{Duration, Instant, SystemTime};

    use serde::Deserialize;
    use vfk_adapter_core::{plan_action, AdapterAction, Plan, PlanContext, PlanKind};
    use vfk_engine::testing::WordListDictionary;
    use vfk_engine::*;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, LPARAM, LRESULT, WPARAM,
    };
    use windows_sys::Win32::System::Memory::{
        MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_READ,
    };
    use windows_sys::Win32::System::Threading::{
        CreateMutexW, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyboardLayout, GetKeyboardState, SendInput, ToUnicodeEx, INPUT, INPUT_0,
        INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_BACK, VK_CAPITAL, VK_CONTROL,
        VK_LWIN, VK_MENU, VK_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
        KillTimer, SetTimer, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION,
        KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    thread_local! {
        static SESSION: RefCell<Option<ImeSession>> = const { RefCell::new(None) };
        static LAST_APP: RefCell<Option<String>> = const { RefCell::new(None) };
        static WINDOWS_CONFIG: RefCell<WindowsConfig> =
            const { RefCell::new(WindowsConfig { backend: None }) };
    }

    #[derive(Deserialize, Default)]
    struct WindowsConfig {
        /// hook | tsf. Mặc định tsf (preedit); auto là giá trị legacy.
        #[serde(default)]
        backend: Option<String>,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct TsfState {
        magic: u32,
        version: u32,
        pid: u32,
        active: u32,
        window: u64,
    }

    const TSF_STATE_MAGIC: u32 = 0x5646_4B54; // 'VFKT'
    static HOOK: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    #[derive(Clone)]
    struct MemStore(std::rc::Rc<RefCell<Config>>);
    impl ConfigStore for MemStore {
        fn load(&self) -> Config {
            self.0.borrow().clone()
        }
        fn save(&self, config: &Config) {
            *self.0.borrow_mut() = config.clone();
        }
    }

    thread_local! {
        static CONFIG_STORE: RefCell<Option<MemStore>> = const { RefCell::new(None) };
        static CONFIG_MTIME: RefCell<Option<SystemTime>> = const { RefCell::new(None) };
        static WINDOWS_MTIME: RefCell<Option<SystemTime>> = const { RefCell::new(None) };
        static LAST_CONFIG_CHECK: RefCell<Option<Instant>> = const { RefCell::new(None) };
    }
    struct ForegroundApp;
    impl AppContext for ForegroundApp {
        fn current_app(&self) -> Option<String> {
            process_name()
        }
    }
    struct NoSurrounding;
    impl SurroundingText for NoSurrounding {
        fn before_cursor(&self, _n: usize) -> Option<String> {
            None
        }
    }

    fn config_path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("ViFastKey").join("config.json")
    }

    fn load_config() -> Config {
        std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn windows_config_path() -> PathBuf {
        config_path().with_file_name("windows.json")
    }

    fn load_windows_config() -> WindowsConfig {
        std::fs::read_to_string(windows_config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn modified(path: &Path) -> Option<SystemTime> {
        std::fs::metadata(path).ok().and_then(|metadata| metadata.modified().ok())
    }

    /// UI cấu hình chạy ở process khác; hook poll mtime thay vì cần restart.
    /// Không reload khi đang có composition để không mất preedit.
    fn reload_runtime_config_if_changed() {
        let now = Instant::now();
        let should_check = LAST_CONFIG_CHECK.with(|last| {
            let mut last = last.borrow_mut();
            if last.map(|t| now.duration_since(t) < Duration::from_millis(500)).unwrap_or(false) {
                false
            } else {
                *last = Some(now);
                true
            }
        });
        if !should_check {
            return;
        }

        let config_mtime = modified(&config_path());
        let config_changed = CONFIG_MTIME.with(|mtime| {
            let mut mtime = mtime.borrow_mut();
            if *mtime == config_mtime {
                false
            } else {
                *mtime = config_mtime;
                true
            }
        });
        if config_changed {
            let idle = SESSION
                .with(|s| s.borrow().as_ref().map(|session| session.display().is_empty()).unwrap_or(true));
            if idle {
                let config = load_config();
                CONFIG_STORE.with(|store| {
                    if let Some(store) = store.borrow().as_ref() {
                        *store.0.borrow_mut() = config;
                    }
                });
                SESSION.with(|s| {
                    if let Some(session) = s.borrow_mut().as_mut() {
                        let _ = session.reload_config();
                    }
                });
            } else {
                // Thử lại sau 500 ms khi composition kết thúc.
                CONFIG_MTIME.with(|mtime| *mtime.borrow_mut() = None);
            }
        }

        let windows_mtime = modified(&windows_config_path());
        let windows_changed = WINDOWS_MTIME.with(|mtime| {
            let mut mtime = mtime.borrow_mut();
            if *mtime == windows_mtime {
                false
            } else {
                *mtime = windows_mtime;
                true
            }
        });
        if windows_changed {
            WINDOWS_CONFIG.with(|c| *c.borrow_mut() = load_windows_config());
        }
    }

    fn windows_backend() -> String {
        WINDOWS_CONFIG.with(|c| c.borrow().backend.clone().unwrap_or_else(|| "tsf".to_string()))
    }

    fn apply_letter_case(ch: char, shift: bool, caps: bool) -> char {
        if !ch.is_ascii_alphabetic() {
            return ch;
        }
        // GetKeyboardState trong WH_KEYBOARD_LL có thể chưa cập nhật Shift,
        // nên ToUnicodeEx trả chữ thường dù người dùng đang giữ Shift.
        if shift ^ caps {
            ch.to_ascii_uppercase()
        } else {
            ch.to_ascii_lowercase()
        }
    }

    fn is_terminal_app(app: &str) -> bool {
        matches!(app, "windowsterminal" | "openconsole" | "conhost" | "cmd" | "powershell" | "pwsh" | "wt")
    }

    /// TSF không được nạp trong Windows Terminal (MSIX/UWP), nên terminal cần
    /// hook Replace. Khi backend=tsf, chỉ cài hook lúc foreground là terminal;
    /// các app khác vẫn để TSF xử lý và không nhìn thấy keyboard hook.
    fn should_install_hook() -> bool {
        if windows_backend() == "hook" {
            return true;
        }
        if windows_backend() != "tsf" {
            return false;
        }
        foreground_info().map(|(_, _, app)| is_terminal_app(&app)).unwrap_or(false)
    }

    unsafe fn install_hook() {
        if !HOOK.load(Ordering::SeqCst).is_null() {
            return;
        }
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), std::ptr::null_mut(), 0);
        if hook.is_null() {
            eprintln!("Không cài được keyboard hook (error)");
        } else {
            HOOK.store(hook, Ordering::SeqCst);
        }
    }

    unsafe fn remove_hook() {
        let hook = HOOK.swap(std::ptr::null_mut(), Ordering::SeqCst);
        if !hook.is_null() {
            UnhookWindowsHookEx(hook);
            // Không giữ state dở khi hook ngừng theo dõi terminal.
            SESSION.with(|s| {
                if let Some(session) = s.borrow_mut().as_mut() {
                    session.reset();
                }
            });
            LAST_APP.with(|last| *last.borrow_mut() = None);
        }
    }

    unsafe fn update_hook() {
        if should_install_hook() {
            install_hook();
        } else {
            remove_hook();
        }
    }

    unsafe extern "system" fn timer_proc(_hwnd: *mut c_void, _msg: u32, _id: usize, _time: u32) {
        reload_runtime_config_if_changed();
        update_hook();
    }

    fn tsf_state_name() -> &'static [u16] {
        static NAME: OnceLock<Vec<u16>> = OnceLock::new();
        NAME.get_or_init(|| "Local\\ViFastKeyTsfState\0".encode_utf16().collect())
    }

    /// TSF ghi named mapping khi nó đang giữ focus. Hook thấy PID trùng thì
    /// pass-through để không xử lý kép.
    fn tsf_active_for(pid: u32, window: u64) -> bool {
        unsafe {
            let mapping = OpenFileMappingW(FILE_MAP_READ, 0, tsf_state_name().as_ptr());
            if mapping.is_null() {
                return false;
            }
            let view = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, std::mem::size_of::<TsfState>());
            let active = if view.Value.is_null() {
                false
            } else {
                let state = &*(view.Value as *const TsfState);
                state.magic == TSF_STATE_MAGIC
                    && state.version == 1
                    && state.active == 1
                    && (state.window == window || (state.window == 0 && state.pid == pid))
            };
            if !view.Value.is_null() {
                UnmapViewOfFile(view);
            }
            CloseHandle(mapping);
            active
        }
    }

    fn foreground_info() -> Option<(u32, u64, String)> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 {
                return None;
            }
            app_name_for_pid(pid).map(|name| (pid, hwnd as u64, name))
        }
    }

    fn app_name_for_pid(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
            CloseHandle(handle);
            if ok == 0 {
                return None;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            Some(path.rsplit(['\\', '/']).next().unwrap_or(&path).trim_end_matches(".exe").to_lowercase())
        }
    }

    fn process_name() -> Option<String> {
        foreground_info().map(|(_, _, name)| name)
    }

    fn send_key(vk: u16, up: bool) {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) };
    }

    fn type_text(text: &str) {
        for unit in text.encode_utf16() {
            let down = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            let up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            unsafe {
                SendInput(1, &down, std::mem::size_of::<INPUT>() as i32);
            }
            unsafe {
                SendInput(1, &up, std::mem::size_of::<INPUT>() as i32);
            }
        }
    }

    /// Thi hành plan từ adapter-core. Trả `true` nếu hook phải nuốt phím gốc.
    fn apply_plan(plan: &Plan) -> bool {
        match plan.kind {
            PlanKind::Pass | PlanKind::FailSafe => false,
            // Hook không có composition/TSF. Nếu ai đó bật PREEDIT cho hook,
            // đừng gõ thẳng text (sẽ nhân đôi); để phím đi tiếp và dùng TSF.
            PlanKind::Preedit => false,
            PlanKind::Commit => {
                type_text(&plan.text);
                !plan.pass_key
            }
            PlanKind::Replace => {
                // Windows text model dùng UTF-16, nhưng VK_BACK thường xoá một
                // grapheme; dùng delete_chars để giữ hành vi cũ với tiếng Việt
                // BMP. delete_utf16 vẫn có trong plan cho port khác.
                for _ in 0..plan.delete_chars {
                    send_key(VK_BACK, false);
                    send_key(VK_BACK, true);
                }
                type_text(&plan.text);
                !plan.pass_key
            }
        }
    }

    fn key_name(vk: u32) -> Option<&'static str> {
        match vk {
            0x08 => Some("Backspace"),
            0x0D => Some("Enter"),
            0x09 => Some("Tab"),
            0x1B => Some("Escape"),
            0x25 => Some("Left"),
            0x27 => Some("Right"),
            0x28 => Some("Down"),
            0x26 => Some("Up"),
            0x24 => Some("Home"),
            0x23 => Some("End"),
            0x2E => Some("Delete"),
            _ => None,
        }
    }

    fn open_settings() {
        let sibling = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("vifastkey-settings.exe")));
        if let Some(sibling) = sibling.filter(|path| path.exists()) {
            let _ = std::process::Command::new(sibling).args(["--tab", "apps"]).spawn();
            return;
        }
        let installed = std::env::var("LOCALAPPDATA")
            .ok()
            .map(|base| PathBuf::from(base).join("ViFastKey").join("vifastkey-settings.exe"));
        if let Some(installed) = installed.filter(|path| path.exists()) {
            let _ = std::process::Command::new(installed).args(["--tab", "apps"]).spawn();
            return;
        }
        let _ = std::process::Command::new("vifastkey-settings").args(["--tab", "apps"]).spawn();
    }

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code != HC_ACTION as i32 || (wparam as u32 != WM_KEYDOWN && wparam as u32 != WM_SYSKEYDOWN) {
            return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
        }
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        // Bỏ qua phím do chính SendInput của hook bơm ra, tránh xử lý kép.
        if kb.flags & LLKHF_INJECTED != 0 {
            return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
        }
        reload_runtime_config_if_changed();
        let ctrl = GetAsyncKeyState(VK_CONTROL as i32) < 0;
        let alt = GetAsyncKeyState(VK_MENU as i32) < 0;
        let meta = GetAsyncKeyState(VK_LWIN as i32) < 0;
        let shift = GetAsyncKeyState(VK_SHIFT as i32) < 0;
        let caps = GetAsyncKeyState(VK_CAPITAL as i32) & 1 != 0;

        let Some((foreground_pid, foreground_window, app)) = foreground_info() else {
            return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
        };
        let backend = windows_backend();
        let terminal = is_terminal_app(&app);
        // backend=tsf: app thường để TSF xử lý. Windows Terminal không nạp
        // third-party TSF, nên terminal dùng hook Replace khi TSF không giữ
        // context. Nếu TSF thật sự đang sở hữu context (conhost), pass-through.
        let tsf_owns = if backend == "tsf" && terminal {
            tsf_active_for(foreground_pid, foreground_window)
        } else {
            backend == "tsf" || tsf_active_for(foreground_pid, foreground_window)
        };

        if ctrl || alt || meta {
            if alt && !ctrl && !shift && !meta && kb.vkCode == 0xBC {
                if tsf_owns {
                    return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
                }
                // Alt+, (VK_OEM_COMMA) toggle (chỉ khi TSF không xử lý context này).
                SESSION.with(|s| {
                    if let Some(session) = s.borrow_mut().as_mut() {
                        session.toggle_vietnamese();
                    }
                });
                return 1;
            }
            if ctrl && shift && kb.vkCode == 0xBC {
                // Ctrl+Shift+, mở UI cấu hình (không có tray).
                open_settings();
                return 1;
            }
            return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
        }
        if tsf_owns {
            return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
        }
        let key = if let Some(name) = key_name(kb.vkCode) {
            name.to_string()
        } else {
            let mut state = [0u8; 256];
            GetKeyboardState(state.as_mut_ptr());
            let mut buf = [0u16; 8];
            let thread = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
            let layout = GetKeyboardLayout(thread);
            let n = ToUnicodeEx(kb.vkCode, kb.scanCode, state.as_ptr(), buf.as_mut_ptr(), 8, 0, layout);
            if n == 1 {
                match char::from_u32(buf[0] as u32) {
                    Some(c) => apply_letter_case(c, shift, caps).to_string(),
                    None => return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam),
                }
            } else {
                return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
            }
        };
        let app = Some(app);
        let mut swallowed = false;
        SESSION.with(|s| {
            if let Some(session) = s.borrow_mut().as_mut() {
                let changed = LAST_APP.with(|last| {
                    let is_changed = {
                        let last_ref = last.borrow();
                        last_ref.as_deref() != app.as_deref()
                    };
                    if is_changed {
                        *last.borrow_mut() = app.clone();
                    }
                    is_changed
                });
                if changed {
                    // Cập nhật app profile/excluded_apps khi đổi foreground
                    // window. REPLACE mode flush trả Pass nên không mất chữ.
                    let _ = session.focus_changed(app.as_deref());
                }
                let before = session.display();
                let action = session.process_key(&KeyEvent::parse(&key));
                let adapter_action = AdapterAction::from(action);
                let context = PlanContext {
                    field_kind: FieldKind::Text,
                    before_display: &before,
                    surrounding: None,
                    // Hook tự quản lý vùng text; không có surrounding text.
                    allow_unverified_replace: true,
                };
                let plan = plan_action(&adapter_action, &context, session.profile());
                if plan.kind == PlanKind::FailSafe {
                    session.reset();
                }
                swallowed = apply_plan(&plan);
            }
        });
        if swallowed {
            1
        } else {
            CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
        }
    }

    pub fn run() {
        // Một instance duy nhất: Run key + installer/settings có thể cùng gọi.
        static MUTEX_NAME: OnceLock<Vec<u16>> = OnceLock::new();
        let mutex_name = MUTEX_NAME.get_or_init(|| "Local\\ViFastKeyReplace\0".encode_utf16().collect());
        let _mutex = unsafe { CreateMutexW(std::ptr::null_mut(), 1, mutex_name.as_ptr()) };
        if _mutex.is_null() || unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return;
        }

        let config = load_config();
        WINDOWS_CONFIG.with(|c| *c.borrow_mut() = load_windows_config());
        let store = MemStore(std::rc::Rc::new(RefCell::new(config)));
        CONFIG_STORE.with(|slot| *slot.borrow_mut() = Some(store.clone()));
        CONFIG_MTIME.with(|mtime| *mtime.borrow_mut() = modified(&config_path()));
        WINDOWS_MTIME.with(|mtime| *mtime.borrow_mut() = modified(&windows_config_path()));
        let ports = Ports {
            config: Box::new(store),
            app: Box::new(ForegroundApp),
            surrounding: Box::new(NoSurrounding),
            dictionary: Box::new(WordListDictionary::default()),
        };
        SESSION.with(|s| *s.borrow_mut() = Some(ImeSession::new(ports, OutputMode::Replace)));
        unsafe {
            update_hook();
            let timer = SetTimer(std::ptr::null_mut(), 0, 250, Some(timer_proc));
            if timer == 0 {
                // Không có timer thì fallback hook toàn cục; vẫn hơn mất chức năng.
                install_hook();
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if timer != 0 {
                KillTimer(std::ptr::null_mut(), 0);
            }
            remove_hook();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::apply_letter_case;

        #[test]
        fn letter_case_honours_shift_and_caps_lock() {
            assert_eq!(apply_letter_case('o', true, false), 'O');
            assert_eq!(apply_letter_case('s', false, false), 's');
            assert_eq!(apply_letter_case('d', false, true), 'D');
            assert_eq!(apply_letter_case('d', true, true), 'd');
            assert_eq!(apply_letter_case('1', true, false), '1');
        }
    }
}

#[cfg(windows)]
fn main() {
    win::run();
}
