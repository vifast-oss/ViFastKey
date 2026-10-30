//! Port vào (driving port): `ImeSession`. Port nguyên văn `vfk_engine/session.py`.
//!
//! Bất biến (có test):
//! * Chữ trên màn hình của từ đang gõ == `display()` sau khi áp mọi `Action`.
//! * Bên trong luôn dùng NFC; chỉ chuẩn hoá NFC/NFD tại đầu ra.
//! * Mọi độ dài tính theo `char` của chuỗi ĐẦU RA (FFI đổi sang UTF-16).

use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;
use vfk_core::{diff, rebuild, Composer, InputMethodSpec, TransformOptions, MAX_RAW, TELEX};
use vi_syllable::upper;

use crate::ports::{normalize_app_id, resolve_profile, AppProfile, Config, OutputMode, Ports, UnicodeForm};

// ---------------------------------------------------------------- input / output

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Backspace,
    Enter,
    Escape,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Delete,
    PageUp,
    PageDown,
    /// Shift, Control, Alt, Meta, Super, CapsLock, Option, Command, Fn nhấn riêng.
    Modifier,
    /// Phím khác (F1, Insert, ...): coi là ranh giới từ.
    Other,
}

impl Key {
    /// Tên phím giống bản Python ("a", "Backspace", "Enter", "Shift", ...).
    pub fn parse(name: &str) -> Key {
        let mut it = name.chars();
        if let (Some(c), None) = (it.next(), it.next()) {
            return Key::Char(c);
        }
        match name {
            "Backspace" => Key::Backspace,
            "Enter" => Key::Enter,
            "Escape" => Key::Escape,
            "Tab" => Key::Tab,
            "Left" => Key::Left,
            "Right" => Key::Right,
            "Up" => Key::Up,
            "Down" => Key::Down,
            "Home" => Key::Home,
            "End" => Key::End,
            "Delete" => Key::Delete,
            "PageUp" => Key::PageUp,
            "PageDown" => Key::PageDown,
            "Shift" | "Control" | "Alt" | "Meta" | "Super" | "CapsLock" | "Option" | "Command" | "Fn" => {
                Key::Modifier
            }
            _ => Key::Other,
        }
    }

    fn is_nav(&self) -> bool {
        matches!(
            self,
            Key::Tab
                | Key::Left
                | Key::Right
                | Key::Up
                | Key::Down
                | Key::Home
                | Key::End
                | Key::Delete
                | Key::PageUp
                | Key::PageDown
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub key: Key,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl KeyEvent {
    pub fn new(key: Key) -> Self {
        KeyEvent { key, ctrl: false, alt: false, meta: false }
    }

    pub fn ch(c: char) -> Self {
        KeyEvent::new(Key::Char(c))
    }

    pub fn parse(name: &str) -> Self {
        KeyEvent::new(Key::parse(name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FieldKind {
    #[default]
    Text,
    /// Luôn tắt bộ gõ (tránh lộ mật khẩu trong preedit, sai mật khẩu).
    Password,
    Number,
    Email,
    Url,
    /// Adapter nên dùng PREEDIT (commit nguyên tử) cho terminal.
    Terminal,
}

impl FieldKind {
    fn disabled(self) -> bool {
        matches!(self, FieldKind::Password | FieldKind::Number | FieldKind::Email)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Engine không xử lý, trả phím cho hệ điều hành.
    PassThrough,
    Preedit {
        text: String,
        cursor: usize,
    },
    /// `delete_before`: số char của chuỗi đầu ra cần xoá lùi.
    /// `pass_key`: sau khi sửa, để phím gốc đi tiếp (dấu cách, Enter...).
    Replace {
        delete_before: usize,
        insert: String,
        pass_key: bool,
    },
    Commit {
        text: String,
        pass_key: bool,
    },
}

/// Trạng thái từ vừa kết thúc, cho soft reset (Backspace ngay sau dấu cách).
#[derive(Debug, Clone)]
struct LastWord {
    raw: Vec<char>,
    word: String,
    literal: bool,
    sep: char,
}

fn is_sentence_end(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '…')
}

fn is_sentence_closer(c: char) -> bool {
    matches!(c, '"' | '\'' | ')' | ']' | '»' | '”' | '’' | '›')
}

/// `str.isupper()` của Python: có ít nhất 1 ký tự có hoa/thường và mọi ký tự đó đều hoa.
fn py_isupper(s: &str) -> bool {
    let mut cased = false;
    for c in s.chars() {
        if c.is_lowercase() {
            return false;
        }
        if c.is_uppercase() {
            cased = true;
        }
    }
    cased
}

// ---------------------------------------------------------------- session

/// Mỗi ô nhập / input context có 1 session. Đồng bộ, không thread, không async.
pub struct ImeSession {
    ports: Ports,
    mode: OutputMode,
    config: Config,
    profile: AppProfile,
    raw: Vec<char>,
    /// NFC, chữ đang hiển thị của từ hiện tại.
    word: String,
    /// Từ đang ở chế độ thô (sau Backspace vào từ không tái tạo được).
    literal: bool,
    /// Từng gõ escape đôi phím thanh (pharr, vissa).
    escaped: bool,
    /// Viết hoa đầu câu.
    cap_next: bool,
    /// Vừa gõ dấu kết câu, chờ dấu cách.
    pending_end: bool,
    enabled: bool,
    field: FieldKind,
    last: Option<LastWord>,
    /// (text đã chèn, phím thô cần khôi phục) ngay sau macro.
    last_macro: Option<(String, String)>,
    /// Text đã commit gần đây, cho macro nhiều từ.
    history: String,
    /// Phím đang gõ khi tắt tiếng Việt (macro off-mode).
    off_raw: Vec<char>,
    composer: Option<Composer>,
    /// (từ cũ đã nạp lại, số phím thô của nó).
    adopted: Option<(String, usize)>,
    /// Cho phép nạp lại từ trước con trỏ (tắt ngay sau click/focus).
    adopt_allowed: bool,
}

impl ImeSession {
    pub fn new(ports: Ports, mode: OutputMode) -> Self {
        let config = ports.config.load();
        let mut s = ImeSession {
            ports,
            mode,
            config,
            profile: AppProfile::default(),
            raw: Vec::new(),
            word: String::new(),
            literal: false,
            escaped: false,
            cap_next: false,
            pending_end: false,
            enabled: true,
            field: FieldKind::Text,
            last: None,
            last_macro: None,
            history: String::new(),
            off_raw: Vec::new(),
            composer: None,
            adopted: None,
            adopt_allowed: true,
        };
        let app = s.ports.app.current_app();
        s.focus_changed(app.as_deref());
        s
    }

    // ---------- truy cập

    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn profile(&self) -> &AppProfile {
        &self.profile
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Từ hiện tại đang ở chế độ thô (auto-restore/sticky) không?
    pub fn is_literal(&self) -> bool {
        self.literal
    }

    pub fn ports(&self) -> &Ports {
        &self.ports
    }

    /// Chữ của từ đang gõ, ở dạng đầu ra (NFC/NFD).
    pub fn display(&self) -> String {
        self.out(&self.word)
    }

    /// Chuỗi phím thô của từ hiện tại (để adapter/debug hiển thị).
    pub fn raw_string(&self) -> String {
        self.raw.iter().collect()
    }

    // ---------- tiện ích

    fn im(&self) -> &'static InputMethodSpec {
        let name = self.profile.input_method.as_deref().unwrap_or(&self.config.input_method);
        InputMethodSpec::by_name(name).unwrap_or(&TELEX)
    }

    /// URL/omnibox: vẫn cho gõ tiếng Việt (search), nhưng tắt các biến đổi
    /// dễ phá URL (`w` → `ư`, `[`/`]`, quick telex, macro).
    fn url_safe(&self) -> bool {
        self.field == FieldKind::Url
    }

    fn macro_on(&self) -> bool {
        self.config.macro_enabled && !self.url_safe() && !self.literal
    }

    /// Từ đứng ngay sau dấu phân cách URL/email (`.`, `@`, `/`, `:`, `-`, `_`)
    /// được giữ nguyên, không áp Telex/macro. Vd: `vietnam.vn`, `foo.bar`,
    /// `nguyen.van@example.com`, `src/main.rs`.
    fn starts_url_segment(&self) -> bool {
        matches!(self.history.chars().last(), Some('.' | '@' | '/' | ':' | '\\' | '-' | '_'))
    }

    fn opt(&self) -> TransformOptions {
        let c = &self.config;
        let url_safe = self.url_safe();
        TransformOptions {
            tone_style: self.profile.tone_style.unwrap_or(c.tone_style),
            spell_check: self.profile.spell_check.unwrap_or(c.spell_check),
            w_as_u: c.w_as_u && !url_safe,
            bracket_keys: c.bracket_keys && !url_safe,
            quick_telex: c.quick_telex && !url_safe,
            allow_coda_k: c.allow_coda_k,
        }
    }

    fn out(&self, s: &str) -> String {
        match self.config.unicode_form {
            UnicodeForm::Nfc => s.nfc().collect(),
            UnicodeForm::Nfd => s.nfd().collect(),
        }
    }

    fn compute(&mut self, is_final: bool) -> String {
        if self.literal || self.smart_caps_literal() {
            return self.raw.iter().collect();
        }
        let stale = match &self.composer {
            Some(c) => c.raw() != self.raw.as_slice(),
            None => true,
        };
        if stale {
            // raw bị thay thế (rebuild / soft reset / đổi cấu hình)
            self.composer = Some(Composer::with_raw(self.im(), self.opt(), &self.raw));
        }
        let res = match &self.composer {
            Some(c) => c.result(is_final),
            None => unreachable!(),
        };
        if !res.rendered.is_empty() {
            self.escaped = true;
        }
        if res.restored {
            self.literal = true; // dính chế độ thô: text + s -> texts
            if let Some((word, n)) = &self.adopted {
                // 'Việt' + 'f' -> 'Việtf', không phải 'vieetjf'
                let mut s = word.clone();
                s.extend(self.raw.get(*n..).unwrap_or(&[]).iter());
                return s;
            }
        }
        res.text
    }

    fn smart_caps_literal(&self) -> bool {
        let r = &self.raw;
        if !(self.config.smart_caps && r.len() >= 2 && r[0].is_uppercase() && r[1].is_uppercase()) {
            return false;
        }
        // Các cặp tạo chữ Việt bằng dấu phụ vẫn phải biến đổi, kể cả all-caps:
        // DD → Đ, AA → Â, AW → Ă, EE → Ê, OO → Ô, OW → Ơ, UW → Ư.
        !matches!(
            (r[0], r[1]),
            ('D', 'D') | ('A', 'A') | ('A', 'W') | ('E', 'E') | ('O', 'O') | ('O', 'W') | ('U', 'W')
        )
    }

    /// Xoá từ đang gõ (không phát Action).
    pub fn reset(&mut self) {
        self.raw.clear();
        self.word.clear();
        self.literal = false;
        self.escaped = false;
        self.composer = None;
        self.adopted = None;
    }

    /// Chốt composition hiện tại và trả Action tương ứng.
    ///
    /// Adapter dùng hàm này trước reset/focus-out để không làm mất từ đang gõ.
    /// Trong PREEDIT, Action là `Commit`; trong REPLACE, chữ đã nằm trên màn hình
    /// nên Action là `PassThrough` và state được xoá.
    pub fn flush_composition(&mut self) -> Action {
        self.flush()
    }

    /// Đổi chế độ xuất chữ sau khi chốt composition hiện tại.
    ///
    /// Adapter phải áp Action trả về trước khi xử lý phím tiếp theo. Hàm không
    /// tự gọi UI/preedit; đó là việc của adapter.
    pub fn set_output_mode(&mut self, mode: OutputMode) -> Action {
        let act = self.flush();
        self.mode = mode;
        self.composer = None;
        act
    }

    fn is_word_char(&self, ch: char) -> bool {
        if vi_syllable::lower(ch).is_ascii_lowercase() {
            return true;
        }
        if !self.raw.is_empty() && self.im().extra_word_key(ch) {
            return true; // VIQR: ^ ( + ' ` ? ~ . 0 thuộc từ
        }
        if self.config.bracket_keys && self.im().insert_for(ch).is_some() {
            return true;
        }
        if self.config.macro_enabled
            && self.config.macro_digits
            && !self.im().digit_keys
            && ch.is_ascii_digit()
            && !self.raw.is_empty()
        {
            return true; // 'mp3', 'k8s': giữ số trong từ để macro hoạt động
        }
        self.im().digit_keys && ch.is_ascii_digit() && !self.raw.is_empty()
    }

    // ---------- cấu hình / ngữ cảnh

    pub fn reload_config(&mut self) -> Action {
        let act = self.flush();
        self.config = self.ports.config.load();
        self.composer = None;
        self.last_macro = None;
        act
    }

    pub fn focus_changed(&mut self, app_id: Option<&str>) -> Action {
        let act = self.flush();
        self.cap_next = false;
        self.pending_end = false;
        self.profile = resolve_profile(&self.config, app_id);
        let key = normalize_app_id(app_id);
        let c = &self.config;
        let excluded = |a: &String| normalize_app_id(Some(a)) == key;
        self.enabled = if !key.is_empty() && c.excluded_apps.iter().any(excluded) {
            false
        } else if let (Some(v), false) = (self.profile.vietnamese, c.app_modes.contains_key(&key)) {
            v
        } else {
            c.app_modes.get(&key).copied().unwrap_or(c.default_vietnamese)
        };
        act
    }

    pub fn set_field_kind(&mut self, kind: FieldKind) -> Action {
        let act = self.flush();
        self.field = kind;
        act
    }

    /// Adapter gọi khi người dùng click chuột / con trỏ đổi chỗ (fcitx5-unikey #53, ibus-bamboo #607).
    pub fn cursor_moved(&mut self) -> Action {
        self.cap_next = false;
        self.pending_end = false;
        self.adopt_allowed = false;
        self.flush()
    }

    pub fn toggle_vietnamese(&mut self) -> Action {
        let act = self.flush();
        self.enabled = !self.enabled;
        let app = normalize_app_id(self.ports.app.current_app().as_deref());
        if !app.is_empty() {
            self.config.app_modes.insert(app, self.enabled);
            self.ports.config.save(&self.config);
        }
        act
    }

    /// Chốt từ đang gõ dở trước khi đổi ngữ cảnh (không áp macro/khôi phục).
    fn flush(&mut self) -> Action {
        let text = self.display();
        let adopted = self.adopted.is_some();
        self.reset();
        self.last = None;
        self.last_macro = None;
        self.history.clear();
        self.off_raw.clear();
        if self.mode == OutputMode::Preedit && !text.is_empty() && !adopted {
            return Action::Commit { text, pass_key: false };
        }
        Action::PassThrough
    }

    // ---------- xử lý phím

    pub fn process_key(&mut self, ev: &KeyEvent) -> Action {
        if self.field.disabled() {
            return Action::PassThrough;
        }
        if !self.enabled {
            return self.off_mode_key(ev);
        }
        if ev.key == Key::Modifier {
            // Shift/Option nhấn riêng: không được làm mất từ đang gõ
            return Action::PassThrough;
        }
        if ev.key != Key::Backspace {
            self.last_macro = None; // macro undo chỉ có hiệu lực ngay sau macro
        }
        if ev.ctrl || ev.alt || ev.meta || ev.key.is_nav() {
            self.cap_next = false;
            self.pending_end = false;
            self.last = None;
            return self.boundary_passthrough();
        }
        match ev.key {
            Key::Backspace => self.backspace(),
            Key::Escape => self.escape(),
            Key::Enter => {
                let act = self.word_end(None);
                self.cap_next = self.config.auto_capitalize;
                self.pending_end = false;
                act
            }
            Key::Char(c) if self.is_word_char(c) => self.letter(c),
            Key::Char(c) => self.separator(c),
            _ => self.boundary_passthrough(),
        }
    }

    fn off_mode_key(&mut self, ev: &KeyEvent) -> Action {
        if ev.key == Key::Modifier || ev.ctrl || ev.alt || ev.meta || ev.key.is_nav() {
            self.off_raw.clear();
            self.last_macro = None;
            return Action::PassThrough;
        }
        let macro_on = self.macro_on() && self.config.macro_in_off_mode;
        if macro_on {
            if let Key::Char(ch) = ev.key {
                if ch.is_alphabetic() || ch.is_ascii_digit() {
                    if self.off_raw.len() < MAX_RAW {
                        self.off_raw.push(ch);
                    }
                    return Action::PassThrough;
                }
            }
        }
        if macro_on && !self.off_raw.is_empty() {
            let raw: String = self.off_raw.iter().collect();
            let mac = self.macro_text(&raw, &raw);
            self.off_raw.clear();
            if let Some(m) = mac {
                let sep = if let Key::Char(ch) = ev.key { Some(ch) } else { None };
                if self.mode == OutputMode::Replace {
                    self.history.push_str(&m);
                    if let Some(s) = sep {
                        self.history.push(s);
                    }
                    let key_len = raw.chars().count();
                    return Action::Replace {
                        delete_before: key_len,
                        insert: m,
                        pass_key: sep.is_some() || ev.key == Key::Enter,
                    };
                }
            }
        }
        self.off_raw.clear();
        Action::PassThrough
    }

    fn separator(&mut self, ch: char) -> Action {
        let act = self.word_end(Some(ch));
        if is_sentence_end(ch) {
            self.pending_end = true;
        } else if is_sentence_closer(ch) {
            // ngoặc kép/đóng không làm mất trạng thái câu
        } else if ch.is_whitespace() {
            if self.pending_end || matches!(ch, '\n' | '\r') {
                self.cap_next = self.config.auto_capitalize;
            }
            self.pending_end = false;
        } else {
            self.pending_end = false;
            self.cap_next = false;
        }
        act
    }

    fn show(&mut self, new: String, typed: Option<char>) -> Action {
        let old_out = self.display();
        self.word = new;
        let new_out = self.display();
        if self.mode == OutputMode::Preedit {
            if self.adopted.is_some() {
                // Sửa từ đã commit: Replace thay vì preedit.
                let e = diff(&old_out, &new_out);
                return Action::Replace { delete_before: e.delete_before, insert: e.insert, pass_key: false };
            }
            let cursor = new_out.chars().count();
            return Action::Preedit { text: new_out, cursor };
        }
        let e = diff(&old_out, &new_out);
        if let Some(t) = typed {
            let mut it = e.insert.chars();
            if e.delete_before == 0 && it.next() == Some(t) && it.next().is_none() {
                return Action::PassThrough; // chữ thường: để OS tự gõ, đỡ nháy
            }
        }
        Action::Replace { delete_before: e.delete_before, insert: e.insert, pass_key: false }
    }

    fn letter(&mut self, key: char) -> Action {
        let mut ch = key;
        self.last = None;
        if self.raw.is_empty() {
            if self.cap_next {
                ch = upper(ch);
            }
            self.cap_next = false;
            self.pending_end = false;
            self.adopt_previous_word();
            self.literal = self.raw.is_empty() && self.starts_url_segment();
        }
        self.raw.push(ch);
        let n = self.raw.len();
        if let Some(c) = self.composer.as_mut() {
            if c.raw() == &self.raw[..n - 1] {
                c.push(ch); // đường nhanh: tăng dần
            }
        }
        if self.raw.len() > MAX_RAW {
            self.literal = true;
        }
        let text = self.compute(false);
        self.show(text, if ch == key { Some(ch) } else { None })
    }

    /// Con trỏ đứng ngay sau 1 từ đã gõ: nạp lại từ đó để sửa tiếp (vd thêm dấu).
    fn adopt_previous_word(&mut self) {
        let mut allowed = self.config.edit_previous_word;
        if let Some(p) = self.profile.edit_previous_word {
            allowed = allowed && p;
        }
        if !allowed || !self.adopt_allowed {
            return;
        }
        let before = match self.ports.surrounding.before_cursor(MAX_RAW + 1) {
            Some(b) if !b.is_empty() => b,
            _ => return,
        };
        let cs: Vec<char> = before.chars().collect();
        let mut i = cs.len();
        while i > 0 && (cs[i - 1].is_alphabetic() || canonical_combining_class(cs[i - 1]) != 0) {
            i -= 1;
        }
        let on_screen: String = cs[i..].iter().collect();
        if on_screen.is_empty() || self.out(&on_screen) != on_screen {
            // chữ trên màn hình khác dạng NFC/NFD đầu ra -> số ký tự xoá sẽ sai, bỏ qua
            return;
        }
        let word: String = on_screen.nfc().collect();
        if word.chars().count() > MAX_RAW / 2 {
            return;
        }
        if let Some(raw) = rebuild(&word, self.im(), &self.opt()) {
            if !raw.is_empty() {
                let n = raw.len();
                self.raw = raw;
                self.word = word.clone();
                self.adopted = Some((word, n));
            }
        }
    }

    /// Xoá 1 ký tự HIỂN THỊ (tiếng -> tiến), giữ dấu ở phần còn lại như UniKey/OpenKey.
    fn backspace(&mut self) -> Action {
        if self.raw.is_empty() {
            self.history.clear();
            self.off_raw.clear();
            if self.config.macro_undo && self.mode == OutputMode::Replace {
                if let Some((inserted, restored)) = self.last_macro.take() {
                    let n = inserted.chars().count();
                    if let Some(before) = self.ports.surrounding.before_cursor(n) {
                        if self.out(&before) == inserted {
                            return Action::Replace { delete_before: n, insert: restored, pass_key: false };
                        }
                    }
                }
            }
            if let Some(last) = self.last.take() {
                if self.config.restore_after_space
                    && self.mode == OutputMode::Replace
                    && last.sep.len_utf8() > 0
                {
                    self.raw = last.raw;
                    self.word = last.word;
                    self.literal = last.literal;
                    self.adopted = None;
                }
            }
            return Action::PassThrough; // OS tự xoá dấu cách
        }
        let mut t: Vec<char> = self.word.chars().collect();
        t.pop();
        let target: String = t.iter().collect();
        if target.is_empty() {
            let act = self.show(String::new(), None);
            self.reset();
            return act;
        }
        // Bỏ phím lặp ở cuối từ literal và thử biến đổi phần còn lại.
        // Làm được nhiều lần liên tiếp: nhuwnggg -> BS -> nhuwngg -> BS -> nhưng;
        // textt -> BS -> text (vẫn literal), không biến thành tẽ.
        if self.literal
            && self.raw.len() >= 2
            && self.raw[self.raw.len() - 1].eq_ignore_ascii_case(&self.raw[self.raw.len() - 2])
        {
            let candidate: Vec<char> = self.raw[..self.raw.len() - 1].to_vec();
            let probe = Composer::with_raw(self.im(), self.opt(), &candidate);
            let res = probe.result(false);
            self.adopted = None;
            self.composer = None;
            if !res.restored {
                self.raw = candidate;
                self.literal = false;
                self.escaped = false;
                let text = self.compute(false);
                return self.show(text, None);
            }
            self.raw = candidate;
            self.literal = true;
            self.escaped = false;
            let text = self.compute(false);
            return self.show(text, None);
        }
        // Từ đã escape đôi phím thanh phải quay lại được dấu khi xoá và gõ tiếp
        // (pharr -> phar -> phả). Từ literal thường vẫn giữ raw.
        let raw = if self.escaped || !self.literal { rebuild(&target, self.im(), &self.opt()) } else { None };
        self.adopted = None;
        match raw {
            Some(r) => {
                self.raw = r;
                self.literal = false;
                self.escaped = false;
            }
            None => {
                self.raw = t;
                self.literal = true;
            }
        }
        let text = self.compute(false);
        self.show(text, None)
    }

    /// Esc: huỷ biến đổi, trả về đúng phím đã gõ.
    fn escape(&mut self) -> Action {
        if self.raw.is_empty() {
            return Action::PassThrough;
        }
        let raw: String = self.raw.iter().collect();
        self.finish(raw, None, false)
    }

    fn word_end(&mut self, sep: Option<char>) -> Action {
        self.last_macro = None;
        if self.raw.is_empty() {
            self.last = None;
            if let Some(s) = sep {
                self.history.push(s);
                if self.history.chars().count() > 256 {
                    self.history =
                        self.history.chars().rev().take(256).collect::<Vec<_>>().into_iter().rev().collect();
                }
            }
            return Action::PassThrough;
        }
        let mut fin = self.compute(true);
        let raw_s: String = self.raw.iter().collect();
        if self.macro_on() && self.mode == OutputMode::Replace {
            let phrase = format!("{}{}", self.history, fin);
            if let Some((key, expansion, prefix)) = self.phrase_macro(&phrase) {
                let sep_s = sep.map(|c| c.to_string()).unwrap_or_default();
                let inserted = format!("{expansion}{sep_s}");
                self.last_macro = sep.map(|_| (inserted.clone(), format!("{key}{sep_s}")));
                self.history = format!("{prefix}{inserted}");
                self.reset();
                return Action::Replace {
                    delete_before: key.chars().count(),
                    insert: expansion,
                    pass_key: sep.is_some(),
                };
            }
        }
        let mac =
            if self.macro_on() { self.macro_of(&self.word).or_else(|| self.macro_of(&raw_s)) } else { None };
        if let Some(m) = mac {
            if let Some(s) = sep {
                if self.config.macro_undo {
                    let mut inserted = self.out(&m);
                    inserted.push(s);
                    let mut restored = raw_s.clone();
                    restored.push(s);
                    self.last_macro = Some((inserted, restored));
                }
            }
            fin = m;
        } else if self.config.auto_restore_english
            && !self.literal
            && !self.escaped
            && fin != raw_s
            && self.ports.dictionary.is_english(&raw_s.to_lowercase())
            && !self.ports.dictionary.is_vietnamese(&fin.to_lowercase())
        {
            fin = raw_s; // 'data' -> 'dât' -> 'data'; nhưng 'thí' (this) giữ nguyên
        }
        self.finish(fin, sep, true)
    }

    fn finish(&mut self, fin: String, sep: Option<char>, pass_key: bool) -> Action {
        let old_out = self.display();
        let final_out = self.out(&fin);
        self.last = match sep {
            // chỉ soft reset khi từ không bị macro/khôi phục đổi
            Some(s) if fin == self.word => Some(LastWord {
                raw: self.raw.clone(),
                word: self.word.clone(),
                literal: self.literal,
                sep: s,
            }),
            _ => None,
        };
        if let Some(s) = sep {
            self.history.push_str(&final_out);
            self.history.push(s);
            if self.history.chars().count() > 256 {
                let keep: String =
                    self.history.chars().rev().take(256).collect::<Vec<_>>().into_iter().rev().collect();
                self.history = keep;
            }
        }
        let adopted = self.adopted.is_some();
        self.reset();
        if self.mode == OutputMode::Preedit {
            if adopted {
                let mut out = final_out;
                if let Some(s) = sep {
                    out.push(s);
                }
                let e = diff(&old_out, &out);
                return Action::Replace {
                    delete_before: e.delete_before,
                    insert: e.insert,
                    pass_key: pass_key && sep.is_none(),
                };
            }
            return match sep {
                None => Action::Commit { text: final_out, pass_key },
                Some(s) => {
                    let mut text = final_out;
                    text.push(s);
                    Action::Commit { text, pass_key: false }
                }
            };
        }
        if final_out == old_out {
            return Action::PassThrough;
        }
        let e = diff(&old_out, &final_out);
        Action::Replace { delete_before: e.delete_before, insert: e.insert, pass_key }
    }

    fn boundary_passthrough(&mut self) -> Action {
        let text = self.display();
        let adopted = self.adopted.is_some();
        self.reset();
        if self.mode == OutputMode::Preedit && !text.is_empty() && !adopted {
            return Action::Commit { text, pass_key: true };
        }
        Action::PassThrough
    }

    fn macro_text(&self, matched: &str, key: &str) -> Option<String> {
        let m = self.config.macros.get(&key.to_lowercase())?;
        if m.is_empty() {
            return None;
        }
        if py_isupper(matched) && matched.chars().count() > 1 {
            return Some(m.to_uppercase());
        }
        if matched.chars().next().is_some_and(|c| c.is_uppercase()) {
            let mut it = m.chars();
            let first: String = it.next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
            return Some(first + it.as_str());
        }
        Some(m.clone())
    }

    fn phrase_macro(&self, phrase: &str) -> Option<(String, String, String)> {
        let low = phrase.to_lowercase();
        let current_len = self.word.chars().count();
        let mut best: Option<(usize, String, String)> = None;
        for key in self.config.macros.keys() {
            let k = key.to_lowercase();
            if k.is_empty() || !low.ends_with(&k) {
                continue;
            }
            let prefix = &phrase[..phrase.len() - k.len()];
            if let Some(last) = prefix.chars().last() {
                if last.is_alphanumeric() || last == '_' || matches!(last, '.' | '@' | '/' | ':' | '\\' | '-')
                {
                    continue;
                }
            }
            let len = k.chars().count();
            if len <= current_len {
                continue;
            }
            if best.as_ref().is_none_or(|(best_len, _, _)| len > *best_len) {
                best = Some((len, key.clone(), prefix.to_string()));
            }
        }
        let (_, key, prefix) = best?;
        let matched = &phrase[phrase.len() - key.len()..];
        let expansion = self.macro_text(matched, &key)?;
        Some((key, expansion, prefix))
    }

    /// Macro giữ kiểu hoa/thường: ko -> không, Ko -> Không, KO -> KHÔNG.
    fn macro_of(&self, word: &str) -> Option<String> {
        let m = self.config.macros.get(&word.to_lowercase())?;
        if m.is_empty() {
            return None; // Python: chuỗi rỗng là falsy
        }
        if py_isupper(word) && word.chars().count() > 1 {
            return Some(m.to_uppercase());
        }
        if word.chars().next().is_some_and(|c| c.is_uppercase()) {
            let mut it = m.chars();
            let first: String = it.next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
            return Some(first + it.as_str());
        }
        Some(m.clone())
    }
}
