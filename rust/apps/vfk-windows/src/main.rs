//! Adapter Windows prototype: WH_KEYBOARD_LL + engine ViFastKey.
//!
//! Đây là adapter thực nghiệm: không có tray/GUI, config đọc từ
//! `%APPDATA%\ViFastKey\config.json`; dùng SendInput Unicode/backspace.
//! Build trên Windows bằng `cargo build -p vfk-windows --release`.
//!
//! Giới hạn: low-level hook yêu cầu app chạy trong desktop session và một số app
//! chống hook; TSF là hướng production. Không chạy được trên macOS/Linux.

#[cfg(not(windows))]
fn main() {
    eprintln!("vfk-windows chỉ chạy trên Windows.");
}

#[cfg(windows)]
mod win {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use vfk_engine::testing::WordListDictionary;
    use vfk_engine::*;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyboardLayout, GetKeyboardState, SendInput, ToUnicodeEx, INPUT, INPUT_0,
        INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_BACK, VK_CONTROL, VK_LWIN,
        VK_MENU, VK_RETURN, VK_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
        SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    thread_local! {
        static SESSION: RefCell<Option<ImeSession>> = const { RefCell::new(None) };
    }

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

    fn process_name() -> Option<String> {
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
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
            windows_sys::Win32::Foundation::CloseHandle(handle);
            if ok == 0 {
                return None;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            Some(path.rsplit(['\\', '/']).next().unwrap_or(&path).trim_end_matches(".exe").to_lowercase())
        }
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

    fn text_of(action: &Action) -> (&str, bool) {
        match action {
            Action::Commit { text, pass_key } => (text, *pass_key),
            Action::Replace { insert, pass_key, .. } => (insert, *pass_key),
            _ => ("", false),
        }
    }

    fn apply(action: Action) {
        match action {
            Action::PassThrough => {}
            Action::Preedit { text, .. } => {
                // Không có preedit trong hook; commit trực tiếp.
                type_text(&text);
            }
            Action::Commit { text, pass_key } => {
                type_text(&text);
                if pass_key {
                    send_key(VK_RETURN, false);
                }
            }
            Action::Replace { delete_before, insert, pass_key } => {
                for _ in 0..delete_before {
                    send_key(VK_BACK, false);
                    send_key(VK_BACK, true);
                }
                type_text(&insert);
                if pass_key { /* phím gốc do hook nuốt; Enter/space đã nằm trong insert */ }
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

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code != HC_ACTION as i32 || (wparam as u32 != WM_KEYDOWN && wparam as u32 != WM_SYSKEYDOWN) {
            return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
        }
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        let ctrl = GetAsyncKeyState(VK_CONTROL as i32) < 0;
        let alt = GetAsyncKeyState(VK_MENU as i32) < 0;
        let meta = GetAsyncKeyState(VK_LWIN as i32) < 0;
        let shift = GetAsyncKeyState(VK_SHIFT as i32) < 0;
        if ctrl || alt || meta {
            if ctrl && shift && kb.vkCode == 0x46 {
                // Ctrl+Shift+F toggle
                SESSION.with(|s| {
                    if let Some(session) = s.borrow_mut().as_mut() {
                        session.toggle_vietnamese();
                    }
                });
                return 1;
            }
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
                    Some(c) => c.to_string(),
                    None => return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam),
                }
            } else {
                return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
            }
        };
        let mut swallowed = false;
        SESSION.with(|s| {
            if let Some(session) = s.borrow_mut().as_mut() {
                let action = session.process_key(&KeyEvent::parse(&key));
                let (text, pass_key) = text_of(&action);
                if !text.is_empty() || matches!(action, Action::Replace { .. } | Action::Commit { .. }) {
                    apply(action);
                    swallowed = !pass_key;
                }
            }
        });
        if swallowed {
            1
        } else {
            CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
        }
    }

    pub fn run() {
        let config = load_config();
        let store = MemStore(std::rc::Rc::new(RefCell::new(config)));
        let ports = Ports {
            config: Box::new(store),
            app: Box::new(ForegroundApp),
            surrounding: Box::new(NoSurrounding),
            dictionary: Box::new(WordListDictionary::default()),
        };
        SESSION.with(|s| *s.borrow_mut() = Some(ImeSession::new(ports, OutputMode::Replace)));
        unsafe {
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), std::ptr::null_mut(), 0);
            if hook.is_null() {
                eprintln!("Không cài được keyboard hook (error)");
                return;
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            UnhookWindowsHookEx(hook);
        }
    }
}

#[cfg(windows)]
fn main() {
    win::run();
}
