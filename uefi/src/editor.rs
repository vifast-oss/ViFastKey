//! Trình soạn thảo dòng tối giản, tích hợp Telex/VNI.

use alloc::string::String;
use alloc::vec::Vec;

use vfk_core::{Composer, InputMethodSpec, Status, ToneStyle, TransformOptions, TELEX, VNI};

#[cfg_attr(feature = "selftest", allow(dead_code))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Action {
    None,
    Redraw,
    Exit,
}

pub struct Editor {
    /// Các dòng đã commit. Từ đang gõ nằm trong `active_text`, chưa nằm ở đây.
    lines: Vec<Vec<char>>,
    /// Dòng chứa con trỏ.
    cx: usize,
    /// Vị trí char trong dòng.
    cy: usize,
    active: Option<Composer>,
    active_text: String,
    raw: String,
    status: Status,
    method: &'static InputMethodSpec,
    options: TransformOptions,
    vietnamese: bool,
    help: bool,
    cols: usize,
    rows: usize,
    scroll: usize,
}

#[cfg_attr(feature = "selftest", allow(dead_code))]
impl Editor {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            lines: alloc::vec![Vec::new()],
            cx: 0,
            cy: 0,
            active: None,
            active_text: String::new(),
            raw: String::new(),
            status: Status::Valid,
            method: &TELEX,
            options: TransformOptions {
                tone_style: ToneStyle::New,
                ..Default::default()
            },
            vietnamese: true,
            help: false,
            cols: cols.max(8),
            rows: rows.max(1),
            scroll: 0,
        }
    }

    // ------------------------------------------------------------ truy cập cho renderer

    pub fn lines(&self) -> &[Vec<char>] {
        &self.lines
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cx, self.cy)
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn active_text(&self) -> &str {
        &self.active_text
    }

    /// (dòng, cột bắt đầu, độ dài) của từ đang gõ, nếu có.
    pub fn active_range(&self) -> Option<(usize, usize, usize)> {
        if self.active.is_none() || self.active_text.is_empty() {
            return None;
        }
        let len = self.active_text.chars().count();
        // Từ đang gõ luôn nằm sau phần đã commit của dòng hiện tại.
        let start = self.lines[self.cx].len();
        Some((self.cx, start, len))
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn status_text(&self) -> &'static str {
        match self.status {
            Status::Valid => "Hợp lệ",
            Status::Partial => "Dở dang",
            Status::Invalid => "Sai",
        }
    }

    pub fn lang_label(&self) -> &'static str {
        if self.vietnamese {
            "Việt"
        } else {
            "Anh"
        }
    }

    pub fn method_label(&self) -> &'static str {
        if self.method.name == "vni" {
            "VNI"
        } else {
            "Telex"
        }
    }

    pub fn help(&self) -> bool {
        self.help
    }

    // ------------------------------------------------------------ phím

    pub fn on_char(&mut self, ch: char) -> Action {
        if self.help {
            self.help = false;
            return Action::Redraw;
        }
        match ch {
            '\r' | '\n' => return self.on_enter(),
            '\u{8}' => return self.on_backspace(),
            '\t' => {
                self.commit_active();
                for _ in 0..4 {
                    self.insert_char(' ');
                }
                return Action::Redraw;
            }
            _ => {}
        }

        let composing = self.vietnamese
            && self.is_word_char(ch)
            && (self.active.is_some() || self.at_line_end());
        if composing {
            self.push_compose(ch);
        } else {
            self.commit_active();
            self.insert_char(ch);
        }
        Action::Redraw
    }

    pub fn on_backspace(&mut self) -> Action {
        if self.active.is_some() {
            let empty = {
                let composer = self.active.as_mut().unwrap();
                composer.pop();
                composer.raw().is_empty()
            };
            if empty {
                self.active = None;
                self.active_text.clear();
                self.raw.clear();
                self.status = Status::Valid;
            } else {
                self.refresh_active();
            }
            return Action::Redraw;
        }

        if self.cy > 0 {
            self.cy -= 1;
            self.lines[self.cx].remove(self.cy);
        } else if self.cx > 0 {
            let line = self.lines.remove(self.cx);
            self.cx -= 1;
            self.cy = self.lines[self.cx].len();
            self.lines[self.cx].extend(line);
        }
        self.ensure_visible();
        Action::Redraw
    }

    pub fn on_enter(&mut self) -> Action {
        self.commit_active();
        let tail = self.lines[self.cx].split_off(self.cy);
        self.lines.insert(self.cx + 1, tail);
        self.cx += 1;
        self.cy = 0;
        self.ensure_visible();
        Action::Redraw
    }

    pub fn on_delete(&mut self) -> Action {
        self.commit_active();
        if self.cy < self.lines[self.cx].len() {
            self.lines[self.cx].remove(self.cy);
        } else if self.cx + 1 < self.lines.len() {
            let next = self.lines.remove(self.cx + 1);
            self.lines[self.cx].extend(next);
        }
        Action::Redraw
    }

    pub fn on_up(&mut self) -> Action {
        self.commit_active();
        if self.cx > 0 {
            self.cx -= 1;
            self.cy = self.cy.min(self.lines[self.cx].len());
        }
        self.ensure_visible();
        Action::Redraw
    }

    pub fn on_down(&mut self) -> Action {
        self.commit_active();
        if self.cx + 1 < self.lines.len() {
            self.cx += 1;
            self.cy = self.cy.min(self.lines[self.cx].len());
        }
        self.ensure_visible();
        Action::Redraw
    }

    pub fn on_left(&mut self) -> Action {
        self.commit_active();
        if self.cy > 0 {
            self.cy -= 1;
        } else if self.cx > 0 {
            self.cx -= 1;
            self.cy = self.lines[self.cx].len();
        }
        self.ensure_visible();
        Action::Redraw
    }

    pub fn on_right(&mut self) -> Action {
        self.commit_active();
        if self.cy < self.lines[self.cx].len() {
            self.cy += 1;
        } else if self.cx + 1 < self.lines.len() {
            self.cx += 1;
            self.cy = 0;
        }
        self.ensure_visible();
        Action::Redraw
    }

    pub fn on_home(&mut self) -> Action {
        self.commit_active();
        self.cy = 0;
        Action::Redraw
    }

    pub fn on_end(&mut self) -> Action {
        self.commit_active();
        self.cy = self.lines[self.cx].len();
        Action::Redraw
    }

    pub fn on_escape(&mut self) -> Action {
        self.active = None;
        self.active_text.clear();
        self.raw.clear();
        self.status = Status::Valid;
        self.help = false;
        Action::Redraw
    }

    pub fn on_toggle_language(&mut self) -> Action {
        self.commit_active();
        self.vietnamese = !self.vietnamese;
        Action::Redraw
    }

    pub fn on_toggle_method(&mut self) -> Action {
        self.commit_active();
        self.method = if self.method.name == "vni" {
            &TELEX
        } else {
            &VNI
        };
        Action::Redraw
    }

    pub fn on_help(&mut self) -> Action {
        self.help = !self.help;
        Action::Redraw
    }

    pub fn on_exit(&mut self) -> Action {
        Action::Exit
    }

    // ------------------------------------------------------------ nội bộ

    fn is_word_char(&self, ch: char) -> bool {
        ch.is_ascii_alphabetic() || (self.method.name == "vni" && ch.is_ascii_digit() && ch != '0')
    }

    fn at_line_end(&self) -> bool {
        self.cy >= self.lines[self.cx].len()
    }

    fn push_compose(&mut self, ch: char) {
        if self.active.is_none() {
            self.active = Some(Composer::new(self.method, self.options));
        }
        self.active.as_mut().unwrap().push(ch);
        self.refresh_active();
        self.ensure_visible();
    }

    fn refresh_active(&mut self) {
        let Some(composer) = self.active.as_mut() else {
            return;
        };
        let result = composer.result(false);
        self.active_text = result.text;
        self.raw = composer.raw().iter().collect();
        self.status = result.status;
    }

    fn commit_active(&mut self) {
        let Some(composer) = self.active.take() else {
            return;
        };
        let text = composer.result(true).text;
        self.active_text.clear();
        self.raw.clear();
        self.status = Status::Valid;
        for ch in text.chars() {
            self.insert_char(ch);
        }
    }

    fn insert_char(&mut self, ch: char) {
        if self.cy >= self.lines[self.cx].len() && self.lines[self.cx].len() >= self.cols {
            self.lines.insert(self.cx + 1, Vec::new());
            self.cx += 1;
            self.cy = 0;
        }
        let line = &mut self.lines[self.cx];
        if self.cy > line.len() {
            self.cy = line.len();
        }
        line.insert(self.cy, ch);
        self.cy += 1;
        self.ensure_visible();
    }

    fn ensure_visible(&mut self) {
        if self.cx < self.scroll {
            self.scroll = self.cx;
        } else if self.cx >= self.scroll + self.rows {
            self.scroll = self.cx + 1 - self.rows;
        }
        let max_scroll = self.lines.len().saturating_sub(self.rows);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new(80, 20)
    }
}
