//! C ABI cho ViFastKey. Port mở rộng của `vfk_ffi/__init__.py`.
//!
//! Quy ước:
//! * Mọi chuỗi vào/ra là UTF-8, kết thúc bằng NUL.
//! * Độ dài xoá / vị trí con trỏ trả về theo **UTF-16 code unit** (Windows, macOS, Android, iOS, JS
//!   đều đếm UTF-16), adapter không phải tự đổi.
//! * `VfkAction.text` thuộc về session, còn hợp lệ tới lần gọi kế tiếp trên CÙNG session.
//! * Một session chỉ được dùng từ 1 luồng tại 1 thời điểm (không cần khoá nội bộ).
//! * Mọi hàm bắt panic (`catch_unwind`); khi có panic trả `PassThrough` và reset từ đang gõ.
//!
//! Header C: `include/vifastkey.h`.

#![allow(clippy::missing_safety_doc)]

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use vfk_adapter_core::{
    char_to_utf16_offset, is_word_codepoint, plan_action, AdapterAction, DeleteAmount, PlanContext, PlanKind,
    SurroundingSnapshot,
};
use vfk_core::charset;
use vfk_engine::testing::WordListDictionary;
use vfk_engine::*;

// ---------------------------------------------------------------- trạng thái dùng chung với port

struct Shared {
    config: Config,
    app: Option<String>,
    surrounding: Option<String>,
    /// Dictionary mặc định dùng chung giữa các session; copy-on-write khi
    /// adapter nạp user dictionary.
    dictionary: Arc<WordListDictionary>,
    /// Bộ đếm số lần lưu cấu hình (adapter so sánh để biết cần ghi file).
    config_version: u64,
}

fn shared_dictionary() -> Arc<WordListDictionary> {
    static DICTIONARY: OnceLock<Arc<WordListDictionary>> = OnceLock::new();
    DICTIONARY.get_or_init(|| Arc::new(WordListDictionary::default())).clone()
}

#[derive(Clone)]
struct SharedPorts(Rc<RefCell<Shared>>);

impl ConfigStore for SharedPorts {
    fn load(&self) -> Config {
        self.0.borrow().config.clone()
    }
    fn save(&self, config: &Config) {
        let mut s = self.0.borrow_mut();
        s.config = config.clone();
        s.config_version += 1;
    }
}

impl AppContext for SharedPorts {
    fn current_app(&self) -> Option<String> {
        self.0.borrow().app.clone()
    }
}

impl SurroundingText for SharedPorts {
    fn before_cursor(&self, n: usize) -> Option<String> {
        let s = self.0.borrow();
        let t = s.surrounding.as_ref()?;
        let cs: Vec<char> = t.chars().collect();
        Some(cs[cs.len().saturating_sub(n)..].iter().collect())
    }
}

impl Dictionary for SharedPorts {
    fn is_english(&self, word: &str) -> bool {
        self.0.borrow().dictionary.english.contains(word)
    }
    fn is_vietnamese(&self, word: &str) -> bool {
        self.0.borrow().dictionary.vietnamese.contains(word)
    }
}

// ---------------------------------------------------------------- kiểu C

pub const VFK_PASS: u32 = 0;
pub const VFK_PREEDIT: u32 = 1;
pub const VFK_REPLACE: u32 = 2;
pub const VFK_COMMIT: u32 = 3;

pub const VFK_MODE_PREEDIT: u32 = 0;
pub const VFK_MODE_REPLACE: u32 = 1;

pub const VFK_PLAN_PASS: u32 = 0;
pub const VFK_PLAN_PREEDIT: u32 = 1;
pub const VFK_PLAN_COMMIT: u32 = 2;
pub const VFK_PLAN_REPLACE: u32 = 3;
pub const VFK_PLAN_FAIL_SAFE: u32 = 4;

pub const VFK_FIELD_TEXT: u32 = 0;
pub const VFK_FIELD_PASSWORD: u32 = 1;
pub const VFK_FIELD_NUMBER: u32 = 2;
pub const VFK_FIELD_EMAIL: u32 = 3;
pub const VFK_FIELD_URL: u32 = 4;
pub const VFK_FIELD_TERMINAL: u32 = 5;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VfkPlan {
    pub kind: u32,
    pub delete_utf16: u32,
    pub delete_chars: u32,
    pub cursor_utf16: u32,
    pub wait_ms: u32,
    pub pass_key: bool,
    pub text: *const c_char,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VfkAction {
    /// VFK_PASS | VFK_PREEDIT | VFK_REPLACE | VFK_COMMIT
    pub kind: u32,
    /// REPLACE: số UTF-16 code unit cần xoá lùi trước con trỏ.
    pub delete_utf16: u32,
    /// PREEDIT: vị trí con trỏ trong preedit (UTF-16).
    pub cursor_utf16: u32,
    /// REPLACE/COMMIT: sau khi áp, có cho phím gốc đi tiếp không.
    pub pass_key: bool,
    /// PREEDIT: chuỗi preedit; REPLACE: chuỗi chèn; COMMIT: chuỗi commit. NULL nếu PASS.
    pub text: *const c_char,
}

impl VfkAction {
    const PASS: VfkAction =
        VfkAction { kind: VFK_PASS, delete_utf16: 0, cursor_utf16: 0, pass_key: false, text: ptr::null() };
}

pub struct VfkSession {
    session: ImeSession,
    shared: SharedPorts,
    /// Giữ chuỗi của Action gần nhất.
    text_buf: CString,
    /// Giữ chuỗi display hiện tại cho adapter kiểm tra surrounding text.
    display_buf: CString,
    /// Giữ text của VfkPlan gần nhất.
    plan_buf: CString,
    /// Giữ chuỗi trả về bởi các hàm *_json.
    json_buf: CString,
    /// Giữ chuỗi trả về bởi các hàm charset (tách khỏi config JSON).
    charset_buf: CString,
    /// Giữ bytes trả về bởi vfk_charset_encode_bytes.
    bytes_buf: Vec<u8>,
}

fn utf16_len(s: &str) -> u32 {
    s.chars().map(|c| c.len_utf16() as u32).sum()
}

/// Đổi Action (đơn vị char) sang VfkAction (đơn vị UTF-16). `before` = display() trước khi xử lý.
fn to_c(s: &mut VfkSession, act: Action, before: &str) -> VfkAction {
    let mut set_text = |t: &str| -> *const c_char {
        s.text_buf = CString::new(t.replace('\0', "")).unwrap_or_default();
        s.text_buf.as_ptr()
    };
    match act {
        Action::PassThrough => VfkAction::PASS,
        Action::Preedit { text, cursor } => {
            let cur: String = text.chars().take(cursor).collect();
            VfkAction {
                kind: VFK_PREEDIT,
                delete_utf16: 0,
                cursor_utf16: utf16_len(&cur),
                pass_key: false,
                text: set_text(&text),
            }
        }
        Action::Commit { text, pass_key } => {
            VfkAction { kind: VFK_COMMIT, delete_utf16: 0, cursor_utf16: 0, pass_key, text: set_text(&text) }
        }
        Action::Replace { delete_before, insert, pass_key } => {
            let cs: Vec<char> = before.chars().collect();
            let deleted: String = cs[cs.len().saturating_sub(delete_before)..].iter().collect();
            VfkAction {
                kind: VFK_REPLACE,
                delete_utf16: utf16_len(&deleted),
                cursor_utf16: 0,
                pass_key,
                text: set_text(&insert),
            }
        }
    }
}

/// Display hiện tại; nếu rỗng nhưng engine đang adopt từ trước con trỏ, dùng
/// surrounding text để `Replace` biết chính xác hậu tố cần xoá.
fn before_display(s: &VfkSession) -> String {
    let display = s.session.display();
    if !display.is_empty() {
        return display;
    }
    s.shared.0.borrow().surrounding.clone().unwrap_or_default()
}

unsafe fn opt_str(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    Some(CStr::from_ptr(p).to_string_lossy().into_owned())
}

fn field_kind_from_u32(value: u32) -> FieldKind {
    match value {
        1 => FieldKind::Password,
        2 => FieldKind::Number,
        3 => FieldKind::Email,
        4 => FieldKind::Url,
        5 => FieldKind::Terminal,
        _ => FieldKind::Text,
    }
}

fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

unsafe fn with_session<T>(h: *mut VfkSession, fallback: T, f: impl FnOnce(&mut VfkSession) -> T) -> T {
    if h.is_null() {
        return fallback;
    }
    match catch_unwind(AssertUnwindSafe(|| f(&mut *h))) {
        Ok(v) => v,
        Err(_) => {
            // trạng thái có thể dở dang: xoá từ đang gõ cho an toàn
            let _ = catch_unwind(AssertUnwindSafe(|| (*h).session.reset()));
            fallback
        }
    }
}

// ---------------------------------------------------------------- vòng đời

/// Tạo session. `config_json` có thể NULL (dùng mặc định) hoặc JSON một phần (thiếu trường -> mặc định).
/// `mode`: VFK_MODE_PREEDIT | VFK_MODE_REPLACE. Trả NULL nếu JSON sai.
#[no_mangle]
pub unsafe extern "C" fn vfk_session_new(config_json: *const c_char, mode: u32) -> *mut VfkSession {
    guard(ptr::null_mut(), || {
        let config: Config = match opt_str(config_json) {
            None => Config::default(),
            Some(s) => match serde_json::from_str(&s) {
                Ok(c) => c,
                Err(_) => return ptr::null_mut(),
            },
        };
        let shared = SharedPorts(Rc::new(RefCell::new(Shared {
            config,
            app: None,
            surrounding: None,
            dictionary: shared_dictionary(),
            config_version: 0,
        })));
        let ports = Ports {
            config: Box::new(shared.clone()),
            app: Box::new(shared.clone()),
            surrounding: Box::new(shared.clone()),
            dictionary: Box::new(shared.clone()),
        };
        let mode = if mode == VFK_MODE_REPLACE { OutputMode::Replace } else { OutputMode::Preedit };
        Box::into_raw(Box::new(VfkSession {
            session: ImeSession::new(ports, mode),
            shared,
            text_buf: CString::default(),
            display_buf: CString::default(),
            plan_buf: CString::default(),
            json_buf: CString::default(),
            charset_buf: CString::default(),
            bytes_buf: Vec::new(),
        }))
    })
}

#[no_mangle]
pub unsafe extern "C" fn vfk_session_free(h: *mut VfkSession) {
    if !h.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(h))));
    }
}

// ---------------------------------------------------------------- phím

/// Xử lý 1 phím. `key`: 1 ký tự UTF-8 ("a", "Ư") hoặc tên phím
/// ("Backspace", "Enter", "Escape", "Tab", "Left", "Right", "Up", "Down", "Home", "End", "Delete",
///  "PageUp", "PageDown", "Shift", "Control", "Alt", "Meta", "Super", "CapsLock", "Option", "Command", "Fn").
#[no_mangle]
pub unsafe extern "C" fn vfk_process_key(
    h: *mut VfkSession,
    key: *const c_char,
    ctrl: bool,
    alt: bool,
    meta: bool,
) -> VfkAction {
    let Some(name) = opt_str(key) else { return VfkAction::PASS };
    with_session(h, VfkAction::PASS, |s| {
        let before = before_display(s);
        let ev = KeyEvent { key: Key::parse(&name), ctrl, alt, meta };
        let act = s.session.process_key(&ev);
        to_c(s, act, &before)
    })
}

/// Giống `vfk_process_key` nhưng nhận mã Unicode (tiện cho Android/JNI, Win32 WM_CHAR).
#[no_mangle]
pub unsafe extern "C" fn vfk_process_char(
    h: *mut VfkSession,
    codepoint: u32,
    ctrl: bool,
    alt: bool,
    meta: bool,
) -> VfkAction {
    let Some(c) = char::from_u32(codepoint) else { return VfkAction::PASS };
    with_session(h, VfkAction::PASS, |s| {
        let before = before_display(s);
        let ev = KeyEvent { key: Key::Char(c), ctrl, alt, meta };
        let act = s.session.process_key(&ev);
        to_c(s, act, &before)
    })
}

// ---------------------------------------------------------------- ngữ cảnh

/// Đổi app có focus (`app_id` có thể NULL). Có thể trả COMMIT (chốt preedit).
#[no_mangle]
pub unsafe extern "C" fn vfk_focus_changed(h: *mut VfkSession, app_id: *const c_char) -> VfkAction {
    let app = opt_str(app_id);
    with_session(h, VfkAction::PASS, |s| {
        let before = s.session.display();
        s.shared.0.borrow_mut().app = app.clone();
        let act = s.session.focus_changed(app.as_deref());
        to_c(s, act, &before)
    })
}

/// Cập nhật văn bản ngay trước con trỏ (surrounding text). Gọi trước `vfk_process_key`
/// nếu nền tảng hỗ trợ; NULL = không biết.
#[no_mangle]
pub unsafe extern "C" fn vfk_set_surrounding_text(h: *mut VfkSession, before_cursor: *const c_char) {
    let t = opt_str(before_cursor);
    with_session(h, (), |s| {
        s.shared.0.borrow_mut().surrounding = t;
    })
}

/// Người dùng click chuột / con trỏ đổi chỗ. Có thể trả COMMIT.
#[no_mangle]
pub unsafe extern "C" fn vfk_cursor_moved(h: *mut VfkSession) -> VfkAction {
    with_session(h, VfkAction::PASS, |s| {
        let before = s.session.display();
        let act = s.session.cursor_moved();
        to_c(s, act, &before)
    })
}

/// 0 text, 1 password, 2 number, 3 email, 4 url, 5 terminal.
#[no_mangle]
pub unsafe extern "C" fn vfk_set_field_kind(h: *mut VfkSession, kind: u32) -> VfkAction {
    let k = field_kind_from_u32(kind);
    with_session(h, VfkAction::PASS, |s| {
        let before = s.session.display();
        let act = s.session.set_field_kind(k);
        to_c(s, act, &before)
    })
}

#[no_mangle]
pub unsafe extern "C" fn vfk_toggle_vietnamese(h: *mut VfkSession) -> VfkAction {
    with_session(h, VfkAction::PASS, |s| {
        let before = s.session.display();
        let act = s.session.toggle_vietnamese();
        to_c(s, act, &before)
    })
}

#[no_mangle]
pub unsafe extern "C" fn vfk_is_enabled(h: *mut VfkSession) -> bool {
    with_session(h, false, |s| s.session.is_enabled())
}

/// Xoá từ đang gõ mà không phát Action (vd khi adapter tự huỷ composition).
#[no_mangle]
pub unsafe extern "C" fn vfk_reset(h: *mut VfkSession) {
    with_session(h, (), |s| s.session.reset())
}

/// Chuỗi engine tin là đang hiển thị (display của từ hiện tại).
///
/// Adapter dùng để xác minh surrounding text trước khi áp `REPLACE`. Chuỗi thuộc
/// về session, hợp lệ tới lần gọi kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn vfk_display_text(h: *mut VfkSession) -> *const c_char {
    with_session(h, ptr::null(), |s| {
        s.display_buf = CString::new(s.session.display().replace('\0', "")).unwrap_or_default();
        s.display_buf.as_ptr()
    })
}

/// Chốt composition hiện tại. Trả `COMMIT` nếu đang ở PREEDIT, `PASS` nếu không
/// còn gì để commit. Adapter nên áp Action này trước `vfk_reset`/focus-out.
/// Đổi offset char (Unicode codepoint) sang UTF-16 code unit.
/// Trả `UINT32_MAX` nếu text/offset không hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn vfk_char_to_utf16_offset(text: *const c_char, chars: u32) -> u32 {
    let Some(text) = opt_str(text) else { return u32::MAX };
    char_to_utf16_offset(&text, chars).unwrap_or(u32::MAX)
}

/// Có phải ký tự thuộc từ (chữ/số/underscore/combining mark) không.
#[no_mangle]
pub extern "C" fn vfk_is_word_codepoint(codepoint: u32) -> bool {
    char::from_u32(codepoint).is_some_and(is_word_codepoint)
}

/// Lập kế hoạch thi hành Action bằng logic adapter dùng chung.
///
/// `before_display` là `vfk_display_text()` trước khi xử lý phím.
/// `surrounding_text` là snapshot đầy đủ app cung cấp (nullable), offset tính
/// bằng UTF-16 code unit. `allow_unverified_replace` chỉ dùng cho port tự quản
/// lý toàn bộ vùng text (Windows hook), nơi không có surrounding text.
#[no_mangle]
pub unsafe extern "C" fn vfk_plan_action(
    h: *mut VfkSession,
    action: *const VfkAction,
    field_kind: u32,
    before_display: *const c_char,
    surrounding_text: *const c_char,
    cursor_utf16: u32,
    anchor_utf16: u32,
    allow_unverified_replace: bool,
    out: *mut VfkPlan,
) -> bool {
    if action.is_null() || out.is_null() {
        return false;
    }
    let raw = &*action;
    let text = opt_str(raw.text).unwrap_or_default();
    let adapter_action = match raw.kind {
        VFK_PASS => AdapterAction::Pass,
        VFK_PREEDIT => AdapterAction::Preedit { text, cursor_utf16: raw.cursor_utf16 },
        VFK_COMMIT => AdapterAction::Commit { text, pass_key: raw.pass_key },
        VFK_REPLACE => AdapterAction::Replace {
            insert: text,
            delete: DeleteAmount::Utf16(raw.delete_utf16),
            pass_key: raw.pass_key,
        },
        _ => return false,
    };

    let before = opt_str(before_display).unwrap_or_default();
    let surrounding_owned = opt_str(surrounding_text);
    let surrounding =
        surrounding_owned.as_deref().map(|text| SurroundingSnapshot { text, cursor_utf16, anchor_utf16 });
    let context = PlanContext {
        field_kind: field_kind_from_u32(field_kind),
        before_display: &before,
        surrounding,
        allow_unverified_replace,
    };

    with_session(h, false, |s| {
        let plan = plan_action(&adapter_action, &context, s.session.profile());
        let kind = match plan.kind {
            PlanKind::Pass => VFK_PLAN_PASS,
            PlanKind::Preedit => VFK_PLAN_PREEDIT,
            PlanKind::Commit => VFK_PLAN_COMMIT,
            PlanKind::Replace => VFK_PLAN_REPLACE,
            PlanKind::FailSafe => VFK_PLAN_FAIL_SAFE,
        };
        let mut result = VfkPlan {
            kind,
            delete_utf16: plan.delete_utf16,
            delete_chars: plan.delete_chars,
            cursor_utf16: plan.cursor_utf16,
            wait_ms: plan.wait_ms,
            pass_key: plan.pass_key,
            text: ptr::null(),
        };
        if !plan.text.is_empty() {
            s.plan_buf = CString::new(plan.text.replace('\0', "")).unwrap_or_default();
            result.text = s.plan_buf.as_ptr();
        }
        *out = result;
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn vfk_flush(h: *mut VfkSession) -> VfkAction {
    with_session(h, VfkAction::PASS, |s| {
        let before = s.session.display();
        let act = s.session.flush_composition();
        to_c(s, act, &before)
    })
}

/// Đổi chế độ xuất chữ (VFK_MODE_PREEDIT/VFK_MODE_REPLACE).
///
/// Có thể trả Action `COMMIT`/`PASS` để adapter chốt composition cũ trước khi
/// đổi mode. Adapter phải áp Action trả về trước khi xử lý phím tiếp theo.
#[no_mangle]
pub unsafe extern "C" fn vfk_set_output_mode(h: *mut VfkSession, mode: u32) -> VfkAction {
    let mode = if mode == VFK_MODE_REPLACE { OutputMode::Replace } else { OutputMode::Preedit };
    with_session(h, VfkAction::PASS, |s| {
        let before = s.session.display();
        let act = s.session.set_output_mode(mode);
        to_c(s, act, &before)
    })
}

/// Gợi ý timing/ACK của AppProfile hiện tại.
#[no_mangle]
pub unsafe extern "C" fn vfk_profile_backspace_delay_ms(h: *mut VfkSession) -> u32 {
    with_session(h, 0, |s| s.session.profile().backspace_delay_ms)
}

#[no_mangle]
pub unsafe extern "C" fn vfk_profile_delay_per_char_ms(h: *mut VfkSession) -> u32 {
    with_session(h, 0, |s| s.session.profile().delay_per_char_ms)
}

#[no_mangle]
pub unsafe extern "C" fn vfk_profile_wait_ack(h: *mut VfkSession) -> bool {
    with_session(h, false, |s| s.session.profile().wait_ack)
}

/// 0 = không đặt, 1 = PREEDIT, 2 = REPLACE.
#[no_mangle]
pub unsafe extern "C" fn vfk_profile_output_mode(h: *mut VfkSession) -> u32 {
    with_session(h, 0, |s| match s.session.profile().output_mode {
        Some(OutputMode::Preedit) => 1,
        Some(OutputMode::Replace) => 2,
        None => 0,
    })
}

// ---------------------------------------------------------------- cấu hình / từ điển

/// Áp JSON cấu hình và trả Action cần áp trước khi dùng session tiếp.
///
/// Trả false nếu JSON sai/không đọc được; khi đó `out_action` không bị thay đổi.
/// Khi thành công, `out_action` nhận Action do `reload_config` sinh ra (thường là
/// `COMMIT` nếu đang có composition).
#[no_mangle]
pub unsafe extern "C" fn vfk_apply_config_json(
    h: *mut VfkSession,
    json: *const c_char,
    out_action: *mut VfkAction,
) -> bool {
    let Some(text) = opt_str(json) else { return false };
    with_session(h, false, |s| {
        let before = s.session.display();
        let base = s.shared.0.borrow().config.clone();
        let Some(c) = base.apply_json_patch(&text) else { return false };
        {
            let mut shared = s.shared.0.borrow_mut();
            shared.config = c;
        }
        let act = s.session.reload_config();
        if !out_action.is_null() {
            *out_action = to_c(s, act, &before);
        }
        true
    })
}

/// Thay cấu hình bằng JSON (một phần cũng được). Trả false nếu JSON sai. Có thể chốt preedit:
/// adapter nên gọi `vfk_flush` trước nếu đang có preedit.
#[no_mangle]
pub unsafe extern "C" fn vfk_set_config_json(h: *mut VfkSession, json: *const c_char) -> bool {
    let Some(text) = opt_str(json) else { return false };
    with_session(h, false, |s| {
        let base = s.shared.0.borrow().config.clone();
        match base.apply_json_patch(&text) {
            Some(c) => {
                s.shared.0.borrow_mut().config = c;
                s.session.reload_config();
                true
            }
            None => false,
        }
    })
}

/// Cấu hình hiện tại dạng JSON. Chuỗi thuộc về session, hợp lệ tới lần gọi *_json kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn vfk_config_json(h: *mut VfkSession) -> *const c_char {
    with_session(h, ptr::null(), |s| {
        let j = serde_json::to_string(s.session.config()).unwrap_or_default();
        s.json_buf = CString::new(j).unwrap_or_default();
        s.json_buf.as_ptr()
    })
}

/// Tăng mỗi khi engine tự lưu cấu hình (vd nhớ Việt/Anh theo app). Adapter so sánh để ghi file.
#[no_mangle]
pub unsafe extern "C" fn vfk_config_version(h: *mut VfkSession) -> u64 {
    with_session(h, 0, |s| s.shared.0.borrow().config_version)
}

/// Gợi ý theo app hiện tại, dạng JSON AppProfile (output_mode, backspace_delay_ms, delay_per_char_ms, wait_ack...).
#[no_mangle]
pub unsafe extern "C" fn vfk_app_profile_json(h: *mut VfkSession) -> *const c_char {
    with_session(h, ptr::null(), |s| {
        let j = serde_json::to_string(s.session.profile()).unwrap_or_default();
        s.json_buf = CString::new(j).unwrap_or_default();
        s.json_buf.as_ptr()
    })
}

/// Thêm từ vào dictionary dùng chung (copy-on-write cho session này).
/// `vietnamese` true: thêm vào danh sách tiếng Việt; false: tiếng Anh.
#[no_mangle]
pub unsafe extern "C" fn vfk_add_dictionary_words(
    h: *mut VfkSession,
    words: *const c_char,
    vietnamese: bool,
) {
    let Some(text) = opt_str(words) else { return };
    with_session(h, (), |s| {
        let mut sh = s.shared.0.borrow_mut();
        let dict = Arc::make_mut(&mut sh.dictionary);
        let target = if vietnamese { &mut dict.vietnamese } else { &mut dict.english };
        for word in text.split_whitespace() {
            let word = word.trim().to_lowercase();
            if !word.is_empty() {
                target.insert(word);
            }
        }
    })
}

/// Nạp từ điển (danh sách từ cách nhau bởi khoảng trắng/xuống dòng). NULL = giữ nguyên danh sách đó.
#[no_mangle]
pub unsafe extern "C" fn vfk_load_dictionary(
    h: *mut VfkSession,
    english: *const c_char,
    vietnamese: *const c_char,
) {
    let en = opt_str(english);
    let vi = opt_str(vietnamese);
    with_session(h, (), |s| {
        if en.is_none() && vi.is_none() {
            return;
        }
        let mut sh = s.shared.0.borrow_mut();
        let dict = Arc::make_mut(&mut sh.dictionary);
        if let Some(e) = en {
            dict.english = e.split_whitespace().map(|w| w.to_lowercase()).collect();
        }
        if let Some(v) = vi {
            dict.vietnamese = v.split_whitespace().map(|w| w.to_lowercase()).collect();
        }
    })
}

// ---------------------------------------------------------------- bảng mã

/// Danh sách bảng mã, mỗi dòng `key<TAB>tên`.
#[no_mangle]
pub unsafe extern "C" fn vfk_charset_list() -> *const c_char {
    static LIST: OnceLock<CString> = OnceLock::new();
    let list = LIST.get_or_init(|| {
        let mut s = String::new();
        for key in vfk_core::CHARSETS {
            s.push_str(key);
            s.push('\t');
            s.push_str(charset::display_name(key).unwrap_or(key));
            s.push('\n');
        }
        CString::new(s).unwrap_or_default()
    });
    list.as_ptr()
}

/// Mã hoá Unicode -> bảng mã. Trả chuỗi UTF-8 của các codepoint legacy; với bảng một byte,
/// adapter có thể gửi từng codepoint như một byte, hoặc dùng `vfk_charset_encode_bytes`.
/// Chuỗi thuộc session, hợp lệ tới lần gọi charset kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn vfk_charset_encode(
    h: *mut VfkSession,
    name: *const c_char,
    text: *const c_char,
) -> *const c_char {
    let Some(name) = opt_str(name) else { return ptr::null() };
    let Some(text) = opt_str(text) else { return ptr::null() };
    with_session(h, ptr::null(), |s| {
        let out = charset::encode(&text, &name);
        s.charset_buf = CString::new(out).unwrap_or_default();
        s.charset_buf.as_ptr()
    })
}

/// Giải mã bảng mã -> Unicode NFC. Chuỗi thuộc session.
#[no_mangle]
pub unsafe extern "C" fn vfk_charset_decode(
    h: *mut VfkSession,
    name: *const c_char,
    text: *const c_char,
) -> *const c_char {
    let Some(name) = opt_str(name) else { return ptr::null() };
    let Some(text) = opt_str(text) else { return ptr::null() };
    with_session(h, ptr::null(), |s| {
        let out = charset::decode(&text, &name);
        s.charset_buf = CString::new(out).unwrap_or_default();
        s.charset_buf.as_ptr()
    })
}

/// Mã hoá sang bytes cho bảng một byte. Trả NULL nếu bảng không phải một byte.
/// `out_len` nhận số byte. Con trỏ thuộc session, hợp lệ tới lần gọi kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn vfk_charset_encode_bytes(
    h: *mut VfkSession,
    name: *const c_char,
    text: *const c_char,
    out_len: *mut usize,
) -> *const u8 {
    let Some(name) = opt_str(name) else { return ptr::null() };
    let Some(text) = opt_str(text) else { return ptr::null() };
    with_session(h, ptr::null(), |s| {
        let Some(bytes) = charset::encode_bytes(&text, &name) else { return ptr::null() };
        s.bytes_buf = bytes;
        if !out_len.is_null() {
            *out_len = s.bytes_buf.len();
        }
        s.bytes_buf.as_ptr()
    })
}

/// Giải mã bytes của bảng một byte -> Unicode NFC.
#[no_mangle]
pub unsafe extern "C" fn vfk_charset_decode_bytes(
    h: *mut VfkSession,
    name: *const c_char,
    data: *const u8,
    len: usize,
) -> *const c_char {
    let Some(name) = opt_str(name) else { return ptr::null() };
    if data.is_null() {
        return ptr::null();
    }
    let slice = std::slice::from_raw_parts(data, len);
    with_session(h, ptr::null(), |s| {
        let out = charset::decode_bytes(slice, &name);
        s.charset_buf = CString::new(out).unwrap_or_default();
        s.charset_buf.as_ptr()
    })
}

/// Phiên bản thư viện (chuỗi tĩnh).
#[no_mangle]
pub extern "C" fn vfk_version() -> *const c_char {
    static V: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    V.as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn key(h: *mut VfkSession, k: &str) -> (VfkAction, Option<String>) {
        let c = CString::new(k).unwrap();
        let a = vfk_process_key(h, c.as_ptr(), false, false, false);
        let t =
            if a.text.is_null() { None } else { Some(CStr::from_ptr(a.text).to_string_lossy().into_owned()) };
        (a, t)
    }

    #[test]
    fn utf16_replace() {
        unsafe {
            let h = vfk_session_new(ptr::null(), VFK_MODE_REPLACE);
            assert!(!h.is_null());
            for k in ["t", "i", "e", "e", "n", "g"] {
                key(h, k);
            }
            let (a, t) = key(h, "s");
            assert_eq!(a.kind, VFK_REPLACE);
            assert_eq!(a.delete_utf16, 3);
            assert_eq!(t.as_deref(), Some("ếng"));
            vfk_session_free(h);
        }
    }

    #[test]
    fn utf16_nfd_backspace() {
        unsafe {
            let cfg = CString::new(r#"{"unicode_form":"NFD"}"#).unwrap();
            let h = vfk_session_new(cfg.as_ptr(), VFK_MODE_REPLACE);
            for k in ["v", "i", "e", "e", "j"] {
                key(h, k);
            }
            let (a, _) = key(h, "t");
            assert_eq!(a.kind, VFK_PASS);
            let (a, _) = key(h, "Backspace");
            assert_eq!((a.kind, a.delete_utf16), (VFK_REPLACE, 1));
            let (a, _) = key(h, "Backspace"); // việ -> vi: 'ệ' NFD = 3 code unit
            assert_eq!(a.delete_utf16, 3);
            vfk_session_free(h);
        }
    }

    #[test]
    fn preedit_and_focus() {
        unsafe {
            let h = vfk_session_new(ptr::null(), VFK_MODE_PREEDIT);
            for k in ["v", "i", "e", "e", "j"] {
                key(h, k);
            }
            let (a, t) = key(h, "t");
            assert_eq!(a.kind, VFK_PREEDIT);
            assert_eq!(t.as_deref(), Some("việt"));
            assert_eq!(a.cursor_utf16, 4);
            let app = CString::new("kitty").unwrap();
            let a = vfk_focus_changed(h, app.as_ptr());
            assert_eq!(a.kind, VFK_COMMIT);
            let p = CStr::from_ptr(vfk_app_profile_json(h)).to_string_lossy().into_owned();
            assert!(p.contains("\"preedit\""), "{p}");
            vfk_session_free(h);
        }
    }

    #[test]
    fn bad_json_and_null() {
        unsafe {
            let bad = CString::new("{nope").unwrap();
            assert!(vfk_session_new(bad.as_ptr(), 0).is_null());
            let a = vfk_process_key(ptr::null_mut(), ptr::null(), false, false, false);
            assert_eq!(a.kind, VFK_PASS);
        }
    }

    #[test]
    fn new_options_reach_engine_through_ffi() {
        unsafe {
            let cfg =
                CString::new(r#"{"allow_coda_k":true,"macro_digits":true,"macros":{"mp3":"máy nghe nhạc"}}"#)
                    .unwrap();
            let h = vfk_session_new(cfg.as_ptr(), VFK_MODE_REPLACE);
            assert!(!h.is_null());
            for k in ["D", "d", "a", "w", "s", "k"] {
                key(h, k);
            }
            assert_eq!((*h).session.display(), "Đắk");
            let (a, _) = key(h, " ");
            assert_eq!(a.kind, VFK_PASS); // chữ cuối không đổi, dấu cách do OS chèn
            for k in ["m", "p", "3"] {
                key(h, k);
            }
            let (a, t) = key(h, " ");
            assert_eq!(a.kind, VFK_REPLACE);
            assert_eq!(t.as_deref(), Some("áy nghe nhạc")); // diff giữ tiền tố "m"
            vfk_session_free(h);
        }
    }

    #[test]
    fn charset_ffi() {
        unsafe {
            let h = vfk_session_new(ptr::null(), VFK_MODE_REPLACE);
            assert!(!h.is_null());
            let cs = CString::new("viqr").unwrap();
            let text = CString::new("Tiếng Việt").unwrap();
            let p = vfk_charset_encode(h, cs.as_ptr(), text.as_ptr());
            let encoded = CStr::from_ptr(p).to_string_lossy().into_owned();
            assert_eq!(encoded, "Tie^'ng Vie^.t");
            let p = vfk_charset_decode(h, cs.as_ptr(), p);
            assert_eq!(CStr::from_ptr(p).to_string_lossy(), "Tiếng Việt");

            let cs = CString::new("tcvn3").unwrap();
            let mut n = 0usize;
            let p = vfk_charset_encode_bytes(h, cs.as_ptr(), text.as_ptr(), &mut n);
            assert_eq!(std::slice::from_raw_parts(p, n), b"Ti\xd5ng Vi\xd6t");
            let p = vfk_charset_decode_bytes(h, cs.as_ptr(), p, n);
            assert_eq!(CStr::from_ptr(p).to_string_lossy(), "Tiếng Việt");
            vfk_session_free(h);
        }
    }

    #[test]
    fn config_json_patch_keeps_other_fields() {
        unsafe {
            let cfg = CString::new(r#"{"macros":{"ko":"không"},"tone_style":"old"}"#).unwrap();
            let h = vfk_session_new(cfg.as_ptr(), VFK_MODE_REPLACE);
            assert!(!h.is_null());
            let patch = CString::new(r#"{"tone_style":"new"}"#).unwrap();
            assert!(vfk_set_config_json(h, patch.as_ptr()));
            let c = CStr::from_ptr(vfk_config_json(h)).to_string_lossy().into_owned();
            assert!(c.contains("\"tone_style\":\"new\""), "{c}");
            assert!(c.contains("\"ko\":\"không\""), "{c}");
            let bad = CString::new(r#"{"tone_style":"không có"}"#).unwrap();
            assert!(!vfk_set_config_json(h, bad.as_ptr()));
            vfk_session_free(h);
        }
    }

    #[test]
    fn plan_action_shared_guard() {
        unsafe {
            let h = vfk_session_new(ptr::null(), VFK_MODE_PREEDIT);
            assert!(!h.is_null());
            let before = CString::new("tiếng").unwrap();
            let surrounding = CString::new("tiếng").unwrap();
            let insert = CString::new("ềng").unwrap();
            let action = VfkAction {
                kind: VFK_REPLACE,
                delete_utf16: 3,
                cursor_utf16: 0,
                pass_key: false,
                text: insert.as_ptr(),
            };
            let mut plan = VfkPlan {
                kind: 0,
                delete_utf16: 0,
                delete_chars: 0,
                cursor_utf16: 0,
                wait_ms: 0,
                pass_key: false,
                text: ptr::null(),
            };
            assert!(vfk_plan_action(
                h,
                &action,
                VFK_FIELD_TEXT,
                before.as_ptr(),
                surrounding.as_ptr(),
                5,
                5,
                false,
                &mut plan,
            ));
            assert_eq!(plan.kind, VFK_PLAN_REPLACE);
            assert_eq!(plan.delete_chars, 3);
            assert_eq!(CStr::from_ptr(plan.text).to_string_lossy(), "ềng");

            // Selection -> fail-safe.
            let mut stale = plan;
            assert!(vfk_plan_action(
                h,
                &action,
                VFK_FIELD_TEXT,
                before.as_ptr(),
                surrounding.as_ptr(),
                5,
                2,
                false,
                &mut stale,
            ));
            assert_eq!(stale.kind, VFK_PLAN_FAIL_SAFE);

            // Password -> pass.
            let mut password = plan;
            assert!(vfk_plan_action(
                h,
                &action,
                VFK_FIELD_PASSWORD,
                before.as_ptr(),
                surrounding.as_ptr(),
                5,
                5,
                false,
                &mut password,
            ));
            assert_eq!(password.kind, VFK_PLAN_PASS);
            vfk_session_free(h);
        }
    }

    #[test]
    fn preedit_edits_previous_word_from_surrounding() {
        unsafe {
            let h = vfk_session_new(ptr::null(), VFK_MODE_PREEDIT);
            assert!(!h.is_null());
            let app = CString::new("notepad").unwrap();
            vfk_focus_changed(h, app.as_ptr());
            let surrounding = CString::new("tiếng").unwrap();
            vfk_set_surrounding_text(h, surrounding.as_ptr());
            let f = CString::new("f").unwrap();
            let a = vfk_process_key(h, f.as_ptr(), false, false, false);
            assert_eq!(a.kind, VFK_REPLACE);
            // Diff giữ tiền tố "ti", chỉ thay "ếng" -> "ềng".
            assert_eq!(a.delete_utf16, 3);
            assert_eq!(CStr::from_ptr(a.text).to_string_lossy(), "ềng");
            assert_eq!(CStr::from_ptr(vfk_display_text(h)).to_string_lossy(), "tiềng");
            vfk_session_free(h);
        }
    }

    #[test]
    fn display_flush_mode_and_profile() {
        unsafe {
            let h = vfk_session_new(ptr::null(), VFK_MODE_PREEDIT);
            assert!(!h.is_null());
            for k in ["v", "i", "e", "e", "j", "t"] {
                key(h, k);
            }
            let shown = CStr::from_ptr(vfk_display_text(h)).to_string_lossy().into_owned();
            assert_eq!(shown, "việt");

            let a = vfk_flush(h);
            assert_eq!(a.kind, VFK_COMMIT);
            assert_eq!(CStr::from_ptr(a.text).to_string_lossy(), "việt");
            assert_eq!(CStr::from_ptr(vfk_display_text(h)).to_string_lossy(), "");

            let app = CString::new("chrome").unwrap();
            vfk_focus_changed(h, app.as_ptr());
            assert!(vfk_profile_wait_ack(h));
            assert_eq!(vfk_profile_delay_per_char_ms(h), 2);
            assert_eq!(vfk_profile_output_mode(h), 0);

            let mode_action = vfk_set_output_mode(h, VFK_MODE_REPLACE);
            assert_eq!(mode_action.kind, VFK_PASS);
            assert_eq!(vfk_profile_output_mode(h), 0);

            let cfg = CString::new(r#"{"tone_style":"new"}"#).unwrap();
            let mut out = VfkAction::PASS;
            assert!(vfk_apply_config_json(h, cfg.as_ptr(), &mut out));
            let json = CStr::from_ptr(vfk_config_json(h)).to_string_lossy().into_owned();
            assert!(json.contains("\"tone_style\":\"new\""), "{json}");
            vfk_session_free(h);
        }
    }
}
