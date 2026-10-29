//! Adapter mẫu + ô nhập giả lập. Port của `adapters/__init__.py`.
//!
//! Dùng cho test, CLI, và làm khuôn khi viết adapter thật (TSF, IMK, Fcitx5, Android...):
//! adapter thật có cùng vai trò: nhận phím từ OS, gọi `process_key`, rồi áp `Action` lên ô nhập.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ports::{AppContext, Config, ConfigStore, Dictionary, OutputMode, Ports, SurroundingText};
use crate::session::{Action, ImeSession, Key, KeyEvent};

// ---------------------------------------------------------------- port ra

#[derive(Default)]
pub struct MemoryConfigStore {
    c: RefCell<Config>,
}

impl MemoryConfigStore {
    pub fn new(config: Config) -> Self {
        MemoryConfigStore { c: RefCell::new(config) }
    }
}

impl ConfigStore for MemoryConfigStore {
    fn load(&self) -> Config {
        self.c.borrow().clone()
    }
    fn save(&self, config: &Config) {
        *self.c.borrow_mut() = config.clone();
    }
}

/// Dùng chung 1 store giữa session và code test (để đọc lại cấu hình đã lưu).
#[derive(Clone, Default)]
pub struct SharedConfigStore(pub Rc<MemoryConfigStore>);

impl ConfigStore for SharedConfigStore {
    fn load(&self) -> Config {
        self.0.load()
    }
    fn save(&self, config: &Config) {
        self.0.save(config)
    }
}

/// Lưu cấu hình ra file JSON (cùng định dạng bản Python).
pub struct JsonFileConfigStore {
    pub path: PathBuf,
}

impl JsonFileConfigStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        JsonFileConfigStore { path: path.as_ref().to_path_buf() }
    }
}

impl ConfigStore for JsonFileConfigStore {
    fn load(&self) -> Config {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self, config: &Config) {
        if let Ok(s) = serde_json::to_string_pretty(config) {
            let _ = std::fs::write(&self.path, s);
        }
    }
}

pub struct StaticAppContext(pub Option<String>);

impl StaticAppContext {
    pub fn new(app: &str) -> Self {
        StaticAppContext(Some(app.to_string()))
    }
}

impl AppContext for StaticAppContext {
    fn current_app(&self) -> Option<String> {
        self.0.clone()
    }
}

const ENGLISH_COMMON: &str = include_str!("../data/english_common.txt");
const VIETNAMESE_COMMON: &str = include_str!("../data/vietnamese_common.txt");

/// Port Dictionary dựa trên 2 danh sách từ. Bản thật nên nạp danh sách lớn
/// (vd ~7.000 âm tiết tiếng Việt phổ biến + ~20.000 từ tiếng Anh).
#[derive(Clone)]
pub struct WordListDictionary {
    pub english: HashSet<String>,
    pub vietnamese: HashSet<String>,
}

impl WordListDictionary {
    pub fn from_text(english: &str, vietnamese: &str) -> Self {
        WordListDictionary {
            english: english.split_whitespace().map(str::to_string).collect(),
            vietnamese: vietnamese.split_whitespace().map(str::to_string).collect(),
        }
    }
}

impl Default for WordListDictionary {
    fn default() -> Self {
        WordListDictionary::from_text(ENGLISH_COMMON, VIETNAMESE_COMMON)
    }
}

impl Dictionary for WordListDictionary {
    fn is_english(&self, word: &str) -> bool {
        self.english.contains(word)
    }
    fn is_vietnamese(&self, word: &str) -> bool {
        self.vietnamese.contains(word)
    }
}

// ---------------------------------------------------------------- ô nhập giả lập

/// Mô phỏng ô nhập của OS.
#[derive(Debug, Clone, Default)]
pub struct FakeTextField {
    pub committed: String,
    pub preedit: String,
}

fn drop_last_chars(s: &mut String, n: usize) {
    for _ in 0..n {
        if s.pop().is_none() {
            break;
        }
    }
}

impl FakeTextField {
    pub fn new(text: &str) -> Self {
        FakeTextField { committed: text.to_string(), preedit: String::new() }
    }

    pub fn text(&self) -> String {
        let mut s = self.committed.clone();
        s.push_str(&self.preedit);
        s
    }

    pub fn before_cursor(&self, n: usize) -> String {
        let cs: Vec<char> = self.committed.chars().collect();
        cs[cs.len().saturating_sub(n)..].iter().collect()
    }

    fn os_default(&mut self, ev: &KeyEvent) {
        if ev.ctrl || ev.alt || ev.meta {
            return;
        }
        match ev.key {
            Key::Backspace => drop_last_chars(&mut self.committed, 1),
            Key::Enter => self.committed.push('\n'),
            Key::Char(c) => self.committed.push(c),
            _ => {}
        }
    }

    /// Áp `Action` lên ô nhập. `ev` = phím gốc (để mô phỏng OS khi PassThrough / pass_key).
    pub fn apply(&mut self, act: &Action, ev: Option<&KeyEvent>) {
        match act {
            Action::PassThrough => {
                if let Some(e) = ev {
                    self.os_default(e);
                }
            }
            Action::Preedit { text, .. } => self.preedit = text.clone(),
            Action::Commit { text, pass_key } => {
                self.committed.push_str(text);
                self.preedit.clear();
                if *pass_key {
                    if let Some(e) = ev {
                        self.os_default(e);
                    }
                }
            }
            Action::Replace { delete_before, insert, pass_key } => {
                drop_last_chars(&mut self.committed, *delete_before);
                self.committed.push_str(insert);
                if *pass_key {
                    if let Some(e) = ev {
                        self.os_default(e);
                    }
                }
            }
        }
    }
}

/// Ô nhập dùng chung giữa test và session (session đọc qua port SurroundingText).
#[derive(Clone, Default)]
pub struct SharedField(pub Rc<RefCell<FakeTextField>>);

impl SharedField {
    pub fn new(text: &str) -> Self {
        SharedField(Rc::new(RefCell::new(FakeTextField::new(text))))
    }

    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn apply(&self, act: &Action, ev: Option<&KeyEvent>) {
        self.0.borrow_mut().apply(act, ev)
    }

    /// Gõ 1 phím: gọi session rồi áp kết quả.
    pub fn key(&self, session: &mut ImeSession, name: &str) -> Action {
        let ev = KeyEvent::parse(name);
        let act = session.process_key(&ev);
        self.apply(&act, Some(&ev));
        act
    }

    /// Gõ cả chuỗi, mỗi ký tự là 1 phím.
    pub fn type_str(&self, session: &mut ImeSession, s: &str) {
        for c in s.chars() {
            let ev = KeyEvent::ch(c);
            let act = session.process_key(&ev);
            self.apply(&act, Some(&ev));
        }
    }

    /// Gõ danh sách phím có tên ("a", "Backspace", ...).
    pub fn type_keys(&self, session: &mut ImeSession, keys: &[&str]) {
        for k in keys {
            self.key(session, k);
        }
    }
}

impl SurroundingText for SharedField {
    fn before_cursor(&self, n: usize) -> Option<String> {
        Some(self.0.borrow().before_cursor(n))
    }
}

/// Tạo session + ô nhập + store dùng chung. Tiện cho test và CLI.
pub struct Harness {
    pub session: ImeSession,
    pub field: SharedField,
    pub store: SharedConfigStore,
}

impl Harness {
    pub fn new(mode: OutputMode, config: Config, app: &str, text: &str) -> Self {
        Self::with_dictionary(mode, config, app, text, WordListDictionary::default())
    }

    pub fn with_dictionary(
        mode: OutputMode,
        config: Config,
        app: &str,
        text: &str,
        dict: WordListDictionary,
    ) -> Self {
        let field = SharedField::new(text);
        let store = SharedConfigStore(Rc::new(MemoryConfigStore::new(config)));
        let ports = Ports {
            config: Box::new(store.clone()),
            app: Box::new(StaticAppContext::new(app)),
            surrounding: Box::new(field.clone()),
            dictionary: Box::new(dict),
        };
        Harness { session: ImeSession::new(ports, mode), field, store }
    }

    pub fn type_str(&mut self, s: &str) -> &mut Self {
        self.field.type_str(&mut self.session, s);
        self
    }

    pub fn type_keys(&mut self, keys: &[&str]) -> &mut Self {
        self.field.type_keys(&mut self.session, keys);
        self
    }

    pub fn text(&self) -> String {
        self.field.text()
    }
}
