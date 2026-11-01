//! Font bitmap ô cố định do `build.rs` sinh ra.

pub const CELL_W: usize = 14;
pub const CELL_H: usize = 26;

const HEADER: usize = 8;
const ENTRY: usize = 4 + CELL_W * CELL_H;
const MAGIC: &[u8; 4] = b"VFKF";

static DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vietnamese_font.bin"));
static BLANK: [u8; CELL_W * CELL_H] = [0u8; CELL_W * CELL_H];

pub struct Font {
    count: usize,
}

impl Font {
    pub fn new() -> Self {
        assert!(DATA.len() >= HEADER, "font atlas quá nhỏ");
        assert_eq!(&DATA[0..4], MAGIC, "font atlas sai magic");
        let cell_w = DATA[4] as usize;
        let cell_h = DATA[5] as usize;
        let count = u16::from_le_bytes([DATA[6], DATA[7]]) as usize;
        assert_eq!(cell_w, CELL_W, "font atlas sai cell width");
        assert_eq!(cell_h, CELL_H, "font atlas sai cell height");
        assert_eq!(
            DATA.len(),
            HEADER + count * ENTRY,
            "font atlas sai kích thước"
        );
        Self { count }
    }

    /// `None` nếu font không có glyph; renderer sẽ dùng dấu hỏi.
    pub fn glyph(&self, ch: char) -> Option<&'static [u8]> {
        let cp = ch as u32;
        let mut lo = 0usize;
        let mut hi = self.count;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let off = HEADER + mid * ENTRY;
            let found =
                u32::from_le_bytes([DATA[off], DATA[off + 1], DATA[off + 2], DATA[off + 3]]);
            if found < cp {
                lo = mid + 1;
            } else if found > cp {
                hi = mid;
            } else {
                return Some(&DATA[off + 4..off + ENTRY]);
            }
        }
        None
    }

    pub fn glyph_or_blank(&self, ch: char) -> &'static [u8] {
        self.glyph(ch).or_else(|| self.glyph('?')).unwrap_or(&BLANK)
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::new()
    }
}
