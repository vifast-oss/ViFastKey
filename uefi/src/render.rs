//! Renderer GOP: back buffer, text ô cố định, status bar và help.

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use uefi::boot::ScopedProtocol;
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput, PixelFormat};

use crate::editor::Editor;
use crate::font::{Font, CELL_H, CELL_W};
use crate::logo::Logo;
use crate::qr::HomeQr;

const BG: BltPixel = BltPixel::new(13, 17, 23);
const PANEL: BltPixel = BltPixel::new(23, 30, 38);
const TEXT: BltPixel = BltPixel::new(238, 232, 213);
const MUTED: BltPixel = BltPixel::new(142, 154, 168);
const ACCENT: BltPixel = BltPixel::new(232, 163, 61);
const CURSOR: BltPixel = BltPixel::new(246, 231, 193);
const WHITE: BltPixel = BltPixel::new(255, 255, 255);
const BLACK: BltPixel = BltPixel::new(0, 0, 0);

const MARGIN_X: usize = 1;
const HEADER_ROWS: usize = 2;
const STATUS_ROWS: usize = 2;

pub struct Renderer {
    gop: ScopedProtocol<GraphicsOutput>,
    buf: Vec<BltPixel>,
    width: usize,
    height: usize,
    cols: usize,
    rows: usize,
    text_top: usize,
    text_rows: usize,
    font: Font,
    logo: Logo,
    qr: HomeQr,
}

impl Renderer {
    pub fn new(mut gop: ScopedProtocol<GraphicsOutput>) -> uefi::Result<Self> {
        // Prefer a mode with enough room; keep the current one if it is
        // already reasonable.
        let (mut width, mut height) = gop.current_mode_info().resolution();
        if width < 1024 || height < 768 {
            let mut best = None;
            let mut best_area = 0usize;
            for mode in gop.modes() {
                let info = mode.info();
                let (w, h) = info.resolution();
                if info.pixel_format() != PixelFormat::BltOnly
                    && w >= 1024
                    && h >= 768
                    && w * h > best_area
                {
                    best = Some(mode);
                    best_area = w * h;
                }
            }
            if let Some(mode) = best {
                gop.set_mode(&mode)?;
                (width, height) = gop.current_mode_info().resolution();
            }
        }

        let cols = width / CELL_W;
        let rows = height / CELL_H;
        if cols < 20 || rows < 8 {
            return Err(uefi::Status::UNSUPPORTED.into());
        }
        let text_top = HEADER_ROWS;
        let text_rows = rows.saturating_sub(HEADER_ROWS + STATUS_ROWS);

        Ok(Self {
            gop,
            buf: vec![BG; width * height],
            width,
            height,
            cols,
            rows,
            text_top,
            text_rows,
            font: Font::new(),
            logo: Logo::new(),
            qr: HomeQr::new(),
        })
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn text_rows(&self) -> usize {
        self.text_rows
    }

    pub fn draw(&mut self, editor: &Editor) -> uefi::Result<()> {
        self.fill_all(BG);

        // Header: logo + tên + link trang chủ.
        self.fill_rect(0, 0, self.width, HEADER_ROWS * CELL_H, PANEL);
        self.draw_logo(8, 4, 40);
        self.draw_text(4, 0, "ViFastKey UEFI", ACCENT);
        self.draw_text(
            4,
            1,
            "F2 Việt/Anh   F3 Telex/VNI   F1 trợ giúp   F10 thoát",
            MUTED,
        );
        self.draw_text_right(
            self.cols.saturating_sub(1),
            0,
            "key.vifast.org",
            MUTED,
        );

        // Committed lines plus the active (not yet committed) word.
        let scroll = editor.scroll();
        let active = editor.active_range();
        for row in 0..self.text_rows {
            let line_index = scroll + row;
            let Some(line) = editor.lines().get(line_index) else {
                break;
            };
            let text: String = line.iter().collect();
            self.draw_text(MARGIN_X, self.text_top + row, &text, TEXT);
            if let Some((active_line, active_start, _)) = active {
                if active_line == line_index {
                    self.draw_text(
                        MARGIN_X + active_start,
                        self.text_top + row,
                        editor.active_text(),
                        TEXT,
                    );
                }
            }
        }

        // Underline the word being composed.
        if let Some((line, col, len)) = active {
            if line >= scroll && line < scroll + self.text_rows {
                let x = (MARGIN_X + col) * CELL_W;
                let y = (self.text_top + line - scroll) * CELL_H + CELL_H - 3;
                self.fill_rect(x, y, len * CELL_W, 2, ACCENT);
            }
        }

        // Cursor sits after the active word when composing.
        let (cursor_line, cursor_col) = editor.cursor();
        let display_col = match active {
            Some((line, start, len)) if line == cursor_line => start + len,
            _ => cursor_col,
        };
        if cursor_line >= scroll && cursor_line < scroll + self.text_rows {
            let x = (MARGIN_X + display_col) * CELL_W;
            let y = (self.text_top + cursor_line - scroll) * CELL_H + 2;
            self.fill_rect(x, y, 2, CELL_H - 4, CURSOR);
        }

        // Status bar.
        let status_top = self.rows - STATUS_ROWS;
        self.fill_rect(
            0,
            status_top * CELL_H,
            self.width,
            STATUS_ROWS * CELL_H,
            PANEL,
        );
        let raw = format!(
            "phím thô: {}   trạng thái: {}   {} / {}",
            editor.raw(),
            editor.status_text(),
            editor.lang_label(),
            editor.method_label()
        );
        self.draw_text(MARGIN_X, status_top, &raw, TEXT);
        self.draw_text(
            MARGIN_X,
            status_top + 1,
            "Backspace xoá phím   Enter xuống dòng   mũi tên di chuyển",
            MUTED,
        );

        if editor.help() {
            self.draw_help();
        }

        self.blt()
    }

    fn draw_help(&mut self) {
        let x = 3 * CELL_W;
        let y = 4 * CELL_H;
        let w = self.width.saturating_sub(6 * CELL_W);
        let h = 12 * CELL_H;
        self.fill_rect(x, y, w, h, PANEL);
        self.fill_rect(x, y, w, 2, ACCENT);
        let lines = [
            "ViFastKey UEFI - bản demo",
            "Telex:  tieengs  ->  Tiếng",
            "VNI:    Vie65t   ->  Việt",
            "F2: Việt/Anh     F3: Telex/VNI",
            "Backspace: xoá phím thô cuối",
            "F1 lần nữa: đóng trợ giúp",
            "Esc: huỷ từ đang gõ",
            "F10: thoát",
            "Trang chủ: key.vifast.org",
        ];
        for (i, line) in lines.iter().enumerate() {
            self.draw_text(
                MARGIN_X + 2,
                5 + i,
                line,
                if i == 0 { ACCENT } else { TEXT },
            );
        }

        // QR trang chủ: UEFI không mở browser, nhưng điện thoại quét được.
        let qr_scale = 4;
        let qr_px = self.qr.size() * qr_scale;
        let qr_x = self.width.saturating_sub(6 * CELL_W + qr_px + 24);
        let qr_y = y + 28;
        self.draw_qr(qr_x, qr_y, qr_scale);
        self.draw_text_right(
            self.cols.saturating_sub(1),
            (qr_y + qr_px + 10) / CELL_H,
            "Quét QR để mở trang chủ",
            MUTED,
        );
    }

    fn draw_text_right(&mut self, right_cell: usize, y_cell: usize, text: &str, fg: BltPixel) {
        let len = text.chars().count();
        if let Some(x) = right_cell.checked_sub(len) {
            self.draw_text(x, y_cell, text, fg);
        }
    }

    fn draw_logo(&mut self, x: usize, y: usize, size: usize) {
        for py in 0..size {
            let dy = y + py;
            if dy >= self.height {
                break;
            }
            for px in 0..size {
                let dx = x + px;
                if dx >= self.width {
                    break;
                }
                let [r, g, b, a] = self.logo.sample(px, py, size, size);
                if a == 0 {
                    continue;
                }
                let idx = dy * self.width + dx;
                self.buf[idx] = blend(self.buf[idx], BltPixel::new(r, g, b), a);
            }
        }
    }

    fn draw_qr(&mut self, x: usize, y: usize, scale: usize) {
        let qr_size = self.qr.size();
        let px = qr_size * scale;
        self.fill_rect(x, y, px, px, WHITE);
        for my in 0..qr_size {
            for mx in 0..qr_size {
                if self.qr.module(mx, my) {
                    self.fill_rect(x + mx * scale, y + my * scale, scale, scale, BLACK);
                }
            }
        }
    }

    fn blt(&mut self) -> uefi::Result<()> {
        self.gop.blt(BltOp::BufferToVideo {
            buffer: &self.buf,
            src: BltRegion::Full,
            dest: (0, 0),
            dims: (self.width, self.height),
        })
    }

    fn fill_all(&mut self, color: BltPixel) {
        self.buf.fill(color);
    }

    fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: BltPixel) {
        let x1 = x.min(self.width);
        let y1 = y.min(self.height);
        let x2 = x.saturating_add(w).min(self.width);
        let y2 = y.saturating_add(h).min(self.height);
        for row in y1..y2 {
            let start = row * self.width;
            for px in &mut self.buf[start + x1..start + x2] {
                *px = color;
            }
        }
    }

    fn draw_text(&mut self, x_cell: usize, y_cell: usize, text: &str, fg: BltPixel) {
        if y_cell >= self.rows {
            return;
        }
        let y = y_cell * CELL_H;
        for (i, ch) in text.chars().enumerate() {
            let cell = x_cell + i;
            if cell >= self.cols {
                break;
            }
            let glyph = self.font.glyph_or_blank(ch);
            let x = cell * CELL_W;
            for row in 0..CELL_H {
                let py = y + row;
                if py >= self.height {
                    break;
                }
                for col in 0..CELL_W {
                    let alpha = glyph[row * CELL_W + col];
                    if alpha == 0 {
                        continue;
                    }
                    let px = x + col;
                    if px >= self.width {
                        break;
                    }
                    let idx = py * self.width + px;
                    self.buf[idx] = blend(self.buf[idx], fg, alpha);
                }
            }
        }
    }
}

fn blend(bg: BltPixel, fg: BltPixel, alpha: u8) -> BltPixel {
    let a = alpha as u32;
    let inv = 255 - a;
    BltPixel::new(
        ((fg.red as u32 * a + bg.red as u32 * inv) / 255) as u8,
        ((fg.green as u32 * a + bg.green as u32 * inv) / 255) as u8,
        ((fg.blue as u32 * a + bg.blue as u32 * inv) / 255) as u8,
    )
}
