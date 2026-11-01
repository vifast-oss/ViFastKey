//! State machine adapter Wayland, tách khỏi protocol để test được trên mọi OS.
//!
//! Lớp `linux.rs` chỉ dịch sự kiện `zwp_input_method_v2` thành [`KeyInput`],
//! [`SurroundingInput`], rồi thi hành [`Backend`]. Toàn bộ quyết định
//! preedit/commit/replace/pass nằm ở đây và dùng chung `vfk-adapter-core`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use vfk_adapter_core::{
    char_suffix, is_disabled, plan_action, utf16_len, utf16_to_byte_offset, AdapterAction, PlanContext,
    PlanKind, SurroundingSnapshot,
};
use vfk_engine::testing::{JsonFileConfigStore, StaticAppContext, WordListDictionary};
use vfk_engine::{
    Action, ConfigStore, FieldKind, ImeSession, Key, KeyEvent, OutputMode, Ports, SurroundingText,
};

/// Sự kiện bàn phím đã được lớp Wayland chuẩn hoá.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyInput {
    pub key: Key,
    /// Evdev keycode, dùng để forward qua virtual keyboard.
    pub keycode: u32,
    pub time: u32,
    pub pressed: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Phím cần chuyển tiếp nguyên bản cho app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForwardKey {
    pub keycode: u32,
    pub time: u32,
    pub pressed: bool,
    pub depressed: u32,
    pub latched: u32,
    pub locked: u32,
    pub group: u32,
}

/// Primitive mà lớp protocol phải cung cấp.
pub trait Backend {
    fn preedit(&mut self, text: &str, cursor_bytes: i32, serial: u32);
    fn clear_preedit(&mut self, serial: u32);
    fn commit(&mut self, text: &str, serial: u32);
    fn replace(&mut self, text: &str, delete_bytes: u32, serial: u32);
    fn forward(&mut self, key: &ForwardKey);
}

#[derive(Clone, Debug, Default)]
struct SurroundingState {
    text: String,
    cursor_bytes: u32,
    anchor_bytes: u32,
    valid: bool,
}

#[derive(Clone, Default)]
struct SharedSurrounding(Rc<RefCell<SurroundingState>>);

impl SharedSurrounding {
    fn set(&self, text: String, cursor: u32, anchor: u32) {
        let mut state = self.0.borrow_mut();
        let valid = cursor as usize <= text.len()
            && anchor as usize <= text.len()
            && text.is_char_boundary(cursor as usize)
            && text.is_char_boundary(anchor as usize);
        if !valid {
            *state = SurroundingState::default();
            return;
        }
        state.text = text;
        state.cursor_bytes = cursor;
        state.anchor_bytes = anchor;
        state.valid = true;
    }

    fn clear(&self) {
        *self.0.borrow_mut() = SurroundingState::default();
    }

    fn prefix(&self) -> Option<String> {
        let state = self.0.borrow();
        if !state.valid {
            return None;
        }
        state.text.get(..state.cursor_bytes as usize).map(str::to_string)
    }

    fn snapshot(&self) -> Option<(String, u32, u32)> {
        let state = self.0.borrow();
        if !state.valid {
            return None;
        }
        let cursor = utf16_len(state.text.get(..state.cursor_bytes as usize)?);
        let anchor = utf16_len(state.text.get(..state.anchor_bytes as usize)?);
        Some((state.text.clone(), cursor, anchor))
    }
}

impl SurroundingText for SharedSurrounding {
    fn before_cursor(&self, n: usize) -> Option<String> {
        let prefix = self.prefix()?;
        let chars: Vec<char> = prefix.chars().collect();
        let start = chars.len().saturating_sub(n);
        Some(chars[start..].iter().collect())
    }
}

/// Điều khiển engine + planner. Không biết Wayland là gì.
pub struct Controller<B: Backend> {
    session: ImeSession,
    backend: B,
    surrounding: SharedSurrounding,
    field: FieldKind,
    active: bool,
    serial: u32,
    consumed: Vec<u32>,
    config_path: Option<PathBuf>,
    config_mtime: Option<SystemTime>,
    depressed: u32,
    latched: u32,
    locked: u32,
    group: u32,
}

#[allow(dead_code)]
impl<B: Backend> Controller<B> {
    /// Tạo controller với file config (có thể chưa tồn tại).
    pub fn new(backend: B, config_path: impl AsRef<Path>) -> Self {
        let path = config_path.as_ref().to_path_buf();
        Self::with_store(backend, Box::new(JsonFileConfigStore::new(&path)), Some(path))
    }

    /// Dùng store tuỳ ý (test / nhúng).
    pub fn with_store(backend: B, store: Box<dyn ConfigStore>, config_path: Option<PathBuf>) -> Self {
        let surrounding = SharedSurrounding::default();
        let ports = Ports {
            config: store,
            app: Box::new(StaticAppContext(None)),
            surrounding: Box::new(surrounding.clone()),
            dictionary: Box::new(WordListDictionary::default()),
        };
        let session = ImeSession::new(ports, OutputMode::Preedit);
        let config_mtime = config_path.as_deref().and_then(mtime);
        Controller {
            session,
            backend,
            surrounding,
            field: FieldKind::Text,
            active: false,
            serial: 0,
            consumed: Vec::new(),
            config_path,
            config_mtime,
            depressed: 0,
            latched: 0,
            locked: 0,
            group: 0,
        }
    }

    pub fn session(&self) -> &ImeSession {
        &self.session
    }

    pub fn field_kind(&self) -> FieldKind {
        self.field
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn serial(&self) -> u32 {
        self.serial
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// `zwp_input_method_v2.activate`
    pub fn on_activate(&mut self) {
        self.active = true;
        self.consumed.clear();
        self.surrounding.clear();
        self.session.focus_changed(None);
        self.session.reset();
        self.backend.clear_preedit(self.serial);
    }

    /// `zwp_input_method_v2.deactivate`
    pub fn on_deactivate(&mut self) {
        if !self.active {
            return;
        }
        self.backend.clear_preedit(self.serial);
        self.session.reset();
        self.consumed.clear();
        self.active = false;
    }

    /// `zwp_input_method_v2.done` — serial dùng cho request `commit`.
    pub fn on_done(&mut self) {
        self.serial = self.serial.saturating_add(1);
    }

    /// `zwp_input_method_v2.surrounding_text` (offset là byte).
    pub fn on_surrounding(&mut self, text: String, cursor: u32, anchor: u32) {
        self.surrounding.set(text, cursor, anchor);
    }

    /// `zwp_input_method_v2.content_type` (purpose là enum text-input-v3).
    pub fn on_content_type(&mut self, purpose: u32) {
        let kind = field_from_purpose(purpose);
        if kind == self.field {
            return;
        }
        let before = self.session.display();
        let action = self.session.set_field_kind(kind);
        self.field = kind;
        if is_disabled(kind) {
            // Không commit composition cũ vào password/email/number.
            self.session.reset();
            self.backend.clear_preedit(self.serial);
            self.consumed.clear();
            return;
        }
        self.apply_action(action, &before, None);
    }

    pub fn on_modifiers(&mut self, depressed: u32, latched: u32, locked: u32, group: u32) {
        self.depressed = depressed;
        self.latched = latched;
        self.locked = locked;
        self.group = group;
    }

    /// Một key event từ keyboard grab. `key` đã được xkbcommon dịch.
    pub fn on_key(&mut self, input: &KeyInput) {
        if !input.pressed {
            if let Some(pos) = self.consumed.iter().position(|k| *k == input.keycode) {
                self.consumed.swap_remove(pos);
                return;
            }
            self.forward(input);
            return;
        }

        if !self.active || is_disabled(self.field) {
            self.forward(input);
            return;
        }

        self.reload_config_if_needed();

        // Alt+, : toggle Việt/Anh giống Fcitx5/IBus, không gửi phím cho app.
        if input.alt && !input.ctrl && !input.meta && input.key == Key::Char(',') {
            let before = self.session.display();
            let action = self.session.toggle_vietnamese();
            self.apply_action(action, &before, None);
            self.mark_consumed(input.keycode);
            return;
        }

        let display = self.session.display();
        let before = if display.is_empty() { self.surrounding.prefix().unwrap_or_default() } else { display };
        let action = self.session.process_key(&KeyEvent {
            key: input.key.clone(),
            ctrl: input.ctrl,
            alt: input.alt,
            meta: input.meta,
        });
        self.apply_action(action, &before, Some(input));
    }

    fn apply_action(&mut self, action: Action, before_display: &str, input: Option<&KeyInput>) {
        let adapter = AdapterAction::from(action);
        let snapshot = self.surrounding.snapshot();
        let profile = self.session.profile().clone();
        let plan = {
            let context = PlanContext {
                field_kind: self.field,
                before_display,
                surrounding: snapshot.as_ref().map(|(text, cursor, anchor)| SurroundingSnapshot {
                    text,
                    cursor_utf16: *cursor,
                    anchor_utf16: *anchor,
                }),
                allow_unverified_replace: false,
            };
            plan_action(&adapter, &context, &profile)
        };

        match plan.kind {
            PlanKind::Pass => {
                if let Some(input) = input {
                    self.forward(input);
                }
            }
            PlanKind::Preedit => {
                if plan.text.is_empty() {
                    self.backend.clear_preedit(self.serial);
                } else {
                    let cursor = utf16_to_byte_offset(&plan.text, plan.cursor_utf16)
                        .map(|b| b as i32)
                        .unwrap_or(plan.text.len() as i32);
                    self.backend.preedit(&plan.text, cursor, self.serial);
                }
                self.mark_input(input);
            }
            PlanKind::Commit => {
                self.backend.clear_preedit(self.serial);
                self.backend.commit(&plan.text, self.serial);
                self.update_after_commit(&plan.text);
                if plan.pass_key {
                    if let Some(input) = input {
                        // Phím gốc (space/Enter) vẫn phải tới app; release thật
                        // sẽ được forward vì keycode không nằm trong consumed.
                        self.forward(input);
                    }
                } else {
                    self.mark_input(input);
                }
            }
            PlanKind::Replace => {
                let source = if before_display.is_empty() {
                    self.surrounding.prefix().unwrap_or_default()
                } else {
                    before_display.to_string()
                };
                let deleted = char_suffix(&source, plan.delete_chars).unwrap_or("");
                let delete_bytes = deleted.len() as u32;
                if delete_bytes == 0 {
                    self.backend.clear_preedit(self.serial);
                    self.backend.commit(&plan.text, self.serial);
                    self.update_after_commit(&plan.text);
                } else {
                    self.backend.clear_preedit(self.serial);
                    self.backend.replace(&plan.text, delete_bytes, self.serial);
                    self.update_after_replace(delete_bytes, &plan.text);
                }
                if plan.pass_key {
                    if let Some(input) = input {
                        self.forward(input);
                    }
                } else {
                    self.mark_input(input);
                }
            }
            PlanKind::FailSafe => {
                // Không xoá mù. Chốt chữ đang soạn (nếu có) rồi reset; nếu
                // không có gì để chốt thì để phím gốc đi tiếp.
                let text = self.session.display();
                if !text.is_empty() {
                    self.backend.clear_preedit(self.serial);
                    self.backend.commit(&text, self.serial);
                    self.update_after_commit(&text);
                    self.session.reset();
                    // Phím gốc chưa được áp: chốt từ rồi vẫn chuyển phím cho
                    // app (giống Fcitx5/IBus fail-safe).
                    if let Some(input) = input {
                        self.forward(input);
                    }
                } else {
                    self.session.reset();
                    self.backend.clear_preedit(self.serial);
                    if let Some(input) = input {
                        self.forward(input);
                    }
                }
            }
        }
    }

    fn mark_input(&mut self, input: Option<&KeyInput>) {
        if let Some(input) = input {
            self.mark_consumed(input.keycode);
        }
    }

    fn mark_consumed(&mut self, keycode: u32) {
        if !self.consumed.contains(&keycode) {
            self.consumed.push(keycode);
        }
    }

    fn forward(&mut self, input: &KeyInput) {
        self.backend.forward(&ForwardKey {
            keycode: input.keycode,
            time: input.time,
            pressed: input.pressed,
            depressed: self.depressed,
            latched: self.latched,
            locked: self.locked,
            group: self.group,
        });
    }

    fn update_after_commit(&mut self, insert: &str) {
        if insert.is_empty() {
            return;
        }
        let mut state = self.surrounding.0.borrow_mut();
        if !state.valid {
            return;
        }
        let cursor = state.cursor_bytes as usize;
        if cursor > state.text.len() || !state.text.is_char_boundary(cursor) {
            *state = SurroundingState::default();
            return;
        }
        let mut text = state.text[..cursor].to_string();
        text.push_str(insert);
        text.push_str(&state.text[cursor..]);
        let new_cursor = cursor + insert.len();
        state.text = text;
        state.cursor_bytes = new_cursor as u32;
        state.anchor_bytes = new_cursor as u32;
    }

    fn update_after_replace(&mut self, delete_bytes: u32, insert: &str) {
        let mut state = self.surrounding.0.borrow_mut();
        if !state.valid {
            return;
        }
        let cursor = state.cursor_bytes as usize;
        let delete = delete_bytes as usize;
        if delete > cursor
            || cursor > state.text.len()
            || !state.text.is_char_boundary(cursor)
            || !state.text.is_char_boundary(cursor - delete)
        {
            *state = SurroundingState::default();
            return;
        }
        let start = cursor - delete;
        let mut text = state.text[..start].to_string();
        text.push_str(insert);
        text.push_str(&state.text[cursor..]);
        let new_cursor = start + insert.len();
        state.text = text;
        state.cursor_bytes = new_cursor as u32;
        state.anchor_bytes = new_cursor as u32;
    }

    fn reload_config_if_needed(&mut self) {
        let Some(path) = self.config_path.as_deref() else {
            return;
        };
        let current = mtime(path);
        if current == self.config_mtime {
            return;
        }
        if !self.session.display().is_empty() {
            // Đang soạn: để lượt sau, tránh đổi cấu hình giữa từ.
            return;
        }
        self.config_mtime = current;
        let action = self.session.reload_config();
        self.apply_action(action, "", None);
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Map `zwp_text_input_v3.content_purpose` sang field kind của engine.
pub fn field_from_purpose(purpose: u32) -> FieldKind {
    match purpose {
        2 | 3 | 4 | 9 => FieldKind::Number, // digits / number / phone / pin
        5 => FieldKind::Url,
        6 => FieldKind::Email,
        8 => FieldKind::Password,
        13 => FieldKind::Terminal,
        _ => FieldKind::Text,
    }
}

/// Config mặc định cho adapter standalone.
pub fn default_config_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("vifastkey").join("config.json");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("vifastkey").join("config.json");
    }
    PathBuf::from("vifastkey.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfk_engine::Config;

    #[derive(Debug)]
    enum Ev {
        Preedit(String, i32),
        Clear,
        Commit(String, u32),
        Replace(String, u32, u32),
        Forward(u32, bool),
    }

    #[derive(Default)]
    struct FakeBackend {
        events: Vec<Ev>,
    }

    impl Backend for FakeBackend {
        fn preedit(&mut self, text: &str, cursor_bytes: i32, _serial: u32) {
            self.events.push(Ev::Preedit(text.into(), cursor_bytes));
        }
        fn clear_preedit(&mut self, _serial: u32) {
            self.events.push(Ev::Clear);
        }
        fn commit(&mut self, text: &str, serial: u32) {
            self.events.push(Ev::Commit(text.into(), serial));
        }
        fn replace(&mut self, text: &str, delete_bytes: u32, serial: u32) {
            self.events.push(Ev::Replace(text.into(), delete_bytes, serial));
        }
        fn forward(&mut self, key: &ForwardKey) {
            self.events.push(Ev::Forward(key.keycode, key.pressed));
        }
    }

    fn ctrl() -> Controller<FakeBackend> {
        Controller::with_store(
            FakeBackend::default(),
            Box::new(vfk_engine::testing::MemoryConfigStore::new(Config::default())),
            None,
        )
    }

    fn press(ctrl: &mut Controller<FakeBackend>, key: char, code: u32) {
        ctrl.on_key(&KeyInput {
            key: Key::Char(key),
            keycode: code,
            time: 0,
            pressed: true,
            ctrl: false,
            alt: false,
            meta: false,
        });
    }

    #[test]
    fn telex_preedit_then_space_commits() {
        let mut c = ctrl();
        c.on_activate();
        for (ch, code) in [('t', 20), ('i', 23), ('e', 18), ('e', 18), ('n', 49), ('g', 34), ('s', 31)] {
            press(&mut c, ch, code);
        }
        assert!(c.backend.events.iter().any(|e| matches!(e, Ev::Preedit(t, _) if t == "tiếng")));
        c.on_done();
        c.on_key(&KeyInput {
            key: Key::Char(' '),
            keycode: 57,
            time: 1,
            pressed: true,
            ctrl: false,
            alt: false,
            meta: false,
        });
        assert!(c.backend.events.iter().any(|e| matches!(e, Ev::Commit(t, 1) if t == "tiếng ")));
        assert!(!c.backend.events.iter().any(|e| matches!(e, Ev::Forward(57, true))));
    }

    #[test]
    fn release_of_consumed_key_is_not_forwarded() {
        let mut c = ctrl();
        c.on_activate();
        press(&mut c, 'a', 30);
        let presses = c.backend.events.iter().filter(|e| matches!(e, Ev::Forward(30, true))).count();
        c.on_key(&KeyInput {
            key: Key::Char('a'),
            keycode: 30,
            time: 1,
            pressed: false,
            ctrl: false,
            alt: false,
            meta: false,
        });
        let releases = c.backend.events.iter().filter(|e| matches!(e, Ev::Forward(30, false))).count();
        assert_eq!(presses, 0);
        assert_eq!(releases, 0);
    }

    #[test]
    fn password_field_passes_through() {
        let mut c = ctrl();
        c.on_activate();
        c.on_content_type(8); // password
        press(&mut c, 'a', 30);
        assert!(c.backend.events.iter().any(|e| matches!(e, Ev::Forward(30, true))));
        assert!(!c.backend.events.iter().any(|e| matches!(e, Ev::Preedit(_, _))));
    }

    #[test]
    fn surrounding_replace_previous_word() {
        let mut c = ctrl();
        c.on_activate();
        c.on_surrounding("Chào".into(), "Chào".len() as u32, "Chào".len() as u32);
        press(&mut c, 's', 31);
        let replaced = c.backend.events.iter().find_map(|e| match e {
            Ev::Replace(text, bytes, _) => Some((text.clone(), *bytes)),
            _ => None,
        });
        let (text, bytes) = replaced.expect("expected Replace");
        assert_eq!(text, "áo");
        assert_eq!(bytes, "ào".len() as u32);
    }

    #[test]
    fn fail_safe_commits_and_forwards_key() {
        let mut c = ctrl();
        c.on_activate();
        press(&mut c, 'a', 30);
        let input = KeyInput {
            key: Key::Char('s'),
            keycode: 31,
            time: 2,
            pressed: true,
            ctrl: false,
            alt: false,
            meta: false,
        };
        // Replace không có surrounding -> fail-safe: chốt "a", vẫn gửi 's'
        // cho app thay vì nuốt phím.
        c.apply_action(
            Action::Replace { delete_before: 1, insert: "á".into(), pass_key: false },
            "a",
            Some(&input),
        );
        assert!(c.backend.events.iter().any(|e| matches!(e, Ev::Commit(t, _) if t == "a")));
        assert!(c.backend.events.iter().any(|e| matches!(e, Ev::Forward(31, true))));
        assert!(c.session.display().is_empty());
    }

    #[test]
    fn deactivate_clears_preedit() {
        let mut c = ctrl();
        c.on_activate();
        press(&mut c, 'a', 30);
        c.on_deactivate();
        assert!(!c.is_active());
        assert!(c.backend.events.iter().any(|e| matches!(e, Ev::Clear)));
    }

    #[test]
    fn serial_is_used_in_commit() {
        let mut c = ctrl();
        c.on_activate();
        c.on_done();
        c.on_done();
        press(&mut c, 'a', 30);
        c.on_key(&KeyInput {
            key: Key::Char(' '),
            keycode: 57,
            time: 1,
            pressed: true,
            ctrl: false,
            alt: false,
            meta: false,
        });
        assert!(c.backend.events.iter().any(|e| matches!(e, Ev::Commit(_, 2))));
    }

    #[test]
    fn url_field_still_types_vietnamese() {
        let mut c = ctrl();
        c.on_activate();
        c.on_content_type(5); // url
        press(&mut c, 'a', 30);
        press(&mut c, 's', 31);
        assert!(c.backend.events.iter().any(|e| matches!(e, Ev::Preedit(t, _) if t == "á")));
    }
}
