//! Windows hook-only backend: WH_KEYBOARD_LL + engine ViFastKey.
//!
//! Chỉ xử lý khi input source foreground là English (US). VI/JP/ngôn ngữ khác
//! được pass-through để nhường Microsoft Vietnamese/Japanese IME. Trạng thái
//! mode/input được ghi ra `hook-state.json` cho tray/toast của Settings.

#[cfg(windows)]
mod win {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::sync::OnceLock;
    use std::time::{Duration, Instant, SystemTime};

    use vfk_adapter_core::{plan_action, AdapterAction, Plan, PlanContext, PlanKind};
    use vfk_engine::testing::WordListDictionary;
    use vfk_engine::*;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, LPARAM, LRESULT, WPARAM,
    };
    use windows_sys::Win32::System::Threading::{
        CreateMutexW, GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_QUERY_LIMITED_INFORMATION,
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

    use crate::hook_state::{save, HookMode, HookState, InputSource};

    thread_local! {
        static SESSION: RefCell<Option<ImeSession>> = const { RefCell::new(None) };
        static LAST_APP: RefCell<Option<String>> = const { RefCell::new(None) };
        static LAST_REPORTED: RefCell<Option<(HookMode, InputSource)>> = const { RefCell::new(None) };
    }
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

    unsafe extern "system" fn timer_proc(_hwnd: *mut c_void, _msg: u32, _id: usize, _time: u32) {
        reload_runtime_config_if_changed();
        let input = input_source();
        if crate::hook_state::take_toggle_request() && input == InputSource::EnUs {
            SESSION.with(|s| {
                if let Some(session) = s.borrow_mut().as_mut() {
                    session.toggle_vietnamese();
                }
            });
        }
        report_mode(mode_for_input(input), input);
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
        // settings đang sở hữu hook process; đánh thức UI hiện có thay vì mở instance thứ hai.
        if let Some(parent) = crate::hook_state::show_request_path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(crate::hook_state::show_request_path(), b"1");
    }

    fn input_from_langid(lang: u16) -> InputSource {
        match lang {
            0x0409 => InputSource::EnUs,
            0x042a => InputSource::Vietnamese,
            0x0411 => InputSource::Japanese,
            _ => InputSource::Other,
        }
    }

    fn input_source() -> InputSource {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return InputSource::Unknown;
            }
            let tid = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let hkl = GetKeyboardLayout(tid);
            input_from_langid((hkl as usize & 0xffff) as u16)
        }
    }

    fn mode_for_input(input: InputSource) -> HookMode {
        if input != InputSource::EnUs {
            return HookMode::External;
        }
        let enabled =
            SESSION.with(|s| s.borrow().as_ref().map(|session| session.is_enabled()).unwrap_or(true));
        if enabled {
            HookMode::Vietnamese
        } else {
            HookMode::English
        }
    }

    fn report_mode(mode: HookMode, input: InputSource) {
        let changed = LAST_REPORTED.with(|last| {
            let mut last = last.borrow_mut();
            if *last == Some((mode, input)) {
                false
            } else {
                *last = Some((mode, input));
                true
            }
        });
        if changed {
            save(&HookState::now(unsafe { GetCurrentProcessId() }, mode, input));
        }
    }

    fn report_current_input() {
        let input = input_source();
        let mode = mode_for_input(input);
        report_mode(mode, input);
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

        let input = input_source();
        if input != InputSource::EnUs {
            // VI/JP/ngôn ngữ khác: nhường hoàn toàn cho IME của Windows.
            report_mode(HookMode::External, input);
            return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
        }
        report_mode(mode_for_input(input), input);

        let Some((_foreground_pid, _foreground_window, app)) = foreground_info() else {
            return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
        };

        if ctrl || alt || meta {
            if alt && !ctrl && !shift && !meta && kb.vkCode == 0xBC {
                // Alt+, chỉ toggle khi EN-US; hook sở hữu input source này.
                SESSION.with(|s| {
                    if let Some(session) = s.borrow_mut().as_mut() {
                        session.toggle_vietnamese();
                    }
                });
                report_mode(mode_for_input(input), input);
                return 1;
            }
            if ctrl && shift && kb.vkCode == 0xBC {
                open_settings();
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
        let store = MemStore(std::rc::Rc::new(RefCell::new(config)));
        CONFIG_STORE.with(|slot| *slot.borrow_mut() = Some(store.clone()));
        CONFIG_MTIME.with(|mtime| *mtime.borrow_mut() = modified(&config_path()));
        let ports = Ports {
            config: Box::new(store),
            app: Box::new(ForegroundApp),
            surrounding: Box::new(NoSurrounding),
            dictionary: Box::new(WordListDictionary::default()),
        };
        SESSION.with(|s| *s.borrow_mut() = Some(ImeSession::new(ports, OutputMode::Replace)));
        unsafe {
            report_current_input();
            install_hook();
            let timer = SetTimer(std::ptr::null_mut(), 0, 250, Some(timer_proc));
            if timer == 0 {
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
        use super::{apply_letter_case, input_from_langid};
        use crate::hook_state::InputSource;

        #[test]
        fn input_language_ids() {
            assert_eq!(input_from_langid(0x0409), InputSource::EnUs);
            assert_eq!(input_from_langid(0x042a), InputSource::Vietnamese);
            assert_eq!(input_from_langid(0x0411), InputSource::Japanese);
            assert_eq!(input_from_langid(0x0809), InputSource::Other);
        }

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
pub fn run() {
    win::run();
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn run() {}
