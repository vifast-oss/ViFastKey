//! Chụp framebuffer của cửa sổ ra PNG, dùng cho kiểm thử giao diện/CI.
//!
//! Không dùng `screencapture` của macOS hay API compositor: eframe trả ảnh qua
//! `Event::Screenshot`, hoạt động cả khi cửa sổ không hiển thị trên màn hình.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui::ColorImage;

use crate::app::Tab;

pub struct Plan {
    targets: Vec<(Tab, PathBuf)>,
    index: usize,
    frames: u32,
    requested: bool,
    requested_at: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Wait,
    Request,
    TimedOut,
    Done,
}

impl Plan {
    pub fn single(path: PathBuf, tab: Tab) -> Self {
        Plan { targets: vec![(tab, path)], index: 0, frames: 8, requested: false, requested_at: None }
    }

    pub fn all(dir: &Path, tabs: &[Tab]) -> Self {
        let targets = tabs.iter().map(|tab| (*tab, dir.join(format!("{}.png", tab.name())))).collect();
        Plan { targets, index: 0, frames: 8, requested: false, requested_at: None }
    }

    pub fn count(&self) -> usize {
        self.targets.len()
    }

    pub fn current(&self) -> Option<(Tab, &Path)> {
        self.targets.get(self.index).map(|(tab, path)| (*tab, path.as_path()))
    }

    /// Gọi mỗi frame sau khi vẽ. `Request` nghĩa là gửi `ViewportCommand::Screenshot`.
    pub fn tick(&mut self) -> Step {
        if self.index >= self.targets.len() {
            return Step::Done;
        }
        if self.frames > 0 {
            self.frames -= 1;
            return Step::Wait;
        }
        if !self.requested {
            self.requested = true;
            self.requested_at = Some(Instant::now());
            return Step::Request;
        }
        if self.requested_at.is_some_and(|at| at.elapsed() > Duration::from_secs(10)) {
            return Step::TimedOut;
        }
        Step::Wait
    }

    pub fn saved(&mut self) {
        self.index += 1;
        self.frames = 4;
        self.requested = false;
        self.requested_at = None;
    }
}

pub fn save_png(path: &Path, image: &ColorImage) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("không tạo được {}: {err}", parent.display()))?;
    }
    let file = File::create(path).map_err(|err| format!("không tạo được {}: {err}", path.display()))?;
    let width = image.size[0] as u32;
    let height = image.size[1] as u32;

    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer =
        encoder.write_header().map_err(|err| format!("không ghi được PNG {}: {err}", path.display()))?;

    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        rgba.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b(), pixel.a()]);
    }
    writer.write_image_data(&rgba).map_err(|err| format!("không ghi được PNG {}: {err}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use egui::{Color32, ColorImage};

    use super::*;

    #[test]
    fn plan_waits_then_requests_and_advances() {
        let dir = tempfile::tempdir().unwrap();
        let mut plan = Plan::all(dir.path(), &[Tab::Overview, Tab::Typing]);
        assert_eq!(plan.count(), 2);
        for _ in 0..8 {
            assert_eq!(plan.tick(), Step::Wait);
        }
        assert_eq!(plan.tick(), Step::Request);
        assert_eq!(plan.tick(), Step::Wait);
        plan.saved();
        assert_eq!(plan.current().unwrap().0, Tab::Typing);
    }

    #[test]
    fn times_out_when_compositor_never_returns_a_screenshot() {
        let dir = tempfile::tempdir().unwrap();
        let mut plan = Plan::all(dir.path(), &[Tab::Overview]);
        for _ in 0..8 {
            assert_eq!(plan.tick(), Step::Wait);
        }
        assert_eq!(plan.tick(), Step::Request);
        plan.requested_at = Some(Instant::now() - Duration::from_secs(11));
        assert_eq!(plan.tick(), Step::TimedOut);
    }

    #[test]
    fn writes_rgba_png() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shot.png");
        let image = ColorImage::new([2, 1], vec![Color32::RED, Color32::GREEN]);
        save_png(&path, &image).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert!(bytes.len() > 20);
    }
}
