//! Logo ViFastKey 96x96 RGBA do `build.rs` sinh.

const HEADER: usize = 8;

static DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/logo.bin"));

pub struct Logo {
    width: usize,
    height: usize,
    rgba: &'static [u8],
}

impl Logo {
    pub fn new() -> Self {
        assert_eq!(&DATA[0..4], b"VFKL", "logo sai magic");
        let width = u16::from_le_bytes([DATA[4], DATA[5]]) as usize;
        let height = u16::from_le_bytes([DATA[6], DATA[7]]) as usize;
        assert_eq!(
            DATA.len(),
            HEADER + width * height * 4,
            "logo sai kích thước"
        );
        Self {
            width,
            height,
            rgba: &DATA[HEADER..],
        }
    }

    /// Trả (r, g, b, a); sampler nearest-neighbor.
    pub fn sample(&self, x: usize, y: usize, out_w: usize, out_h: usize) -> [u8; 4] {
        let sx = (x * self.width / out_w.max(1)).min(self.width - 1);
        let sy = (y * self.height / out_h.max(1)).min(self.height - 1);
        let idx = (sy * self.width + sx) * 4;
        [
            self.rgba[idx],
            self.rgba[idx + 1],
            self.rgba[idx + 2],
            self.rgba[idx + 3],
        ]
    }
}

impl Default for Logo {
    fn default() -> Self {
        Self::new()
    }
}
