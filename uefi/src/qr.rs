//! QR trang chủ (đã gồm quiet zone) do `build.rs` sinh.

const HEADER: usize = 6;

static DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/homepage_qr.bin"));

pub struct HomeQr {
    size: usize,
    modules: &'static [u8],
}

impl HomeQr {
    pub fn new() -> Self {
        assert_eq!(&DATA[0..4], b"VFKQ", "QR sai magic");
        let size = u16::from_le_bytes([DATA[4], DATA[5]]) as usize;
        assert_eq!(DATA.len(), HEADER + size * size, "QR sai kích thước");
        Self {
            size,
            modules: &DATA[HEADER..],
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn module(&self, x: usize, y: usize) -> bool {
        self.modules[y * self.size + x] != 0
    }
}

impl Default for HomeQr {
    fn default() -> Self {
        Self::new()
    }
}
