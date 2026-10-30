//! Token từ `design/tokens.json` -> egui Style/Visuals.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

pub const BG: Color32 = Color32::from_rgb(0xFB, 0xF6, 0xEE);
pub const SURFACE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const CREAM: Color32 = Color32::from_rgb(0xFF, 0xF8, 0xEC);
pub const INK: Color32 = Color32::from_rgb(0x1F, 0x24, 0x30);
pub const MUTED: Color32 = Color32::from_rgb(0x6B, 0x72, 0x80);
pub const ACCENT: Color32 = Color32::from_rgb(0xD4, 0x2A, 0x22);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0xB3, 0x14, 0x0F);
pub const GOLD: Color32 = Color32::from_rgb(0xFF, 0xB8, 0x00);
pub const BORDER: Color32 = Color32::from_rgb(0xE8, 0xDD, 0xD0);
pub const SUCCESS: Color32 = Color32::from_rgb(0x15, 0x80, 0x3D);
pub const WARNING: Color32 = Color32::from_rgb(0xB4, 0x53, 0x09);
pub const ERROR: Color32 = Color32::from_rgb(0xB9, 0x1C, 0x1C);

pub fn apply(ctx: &egui::Context, dark: bool) {
    let mut style = (*ctx.style()).clone();
    style.visuals = if dark { dark_visuals() } else { light_visuals() };
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 30.0;
    style.spacing.indent = 18.0;

    style.text_styles.insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));

    ctx.set_style(style);
}

fn light_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = CREAM;
    v.faint_bg_color = CREAM;
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(12);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.22);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.hyperlink_color = ACCENT;
    v.warn_fg_color = WARNING;
    v.error_fg_color = ERROR;
    v.widgets.noninteractive.bg_fill = SURFACE;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, INK);
    v.widgets.inactive.weak_bg_fill = SURFACE;
    v.widgets.inactive.bg_fill = CREAM;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, INK);
    v.widgets.hovered.weak_bg_fill = CREAM;
    v.widgets.hovered.bg_fill = CREAM;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, ACCENT_DARK);
    v.widgets.active.weak_bg_fill = CREAM;
    v.widgets.active.bg_fill = CREAM;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, ACCENT_DARK);
    v
}

fn dark_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::from_rgb(0x1A, 0x1D, 0x24);
    v.window_fill = Color32::from_rgb(0x22, 0x26, 0x2F);
    v.extreme_bg_color = Color32::from_rgb(0x15, 0x18, 0x1E);
    v.faint_bg_color = Color32::from_rgb(0x22, 0x26, 0x2F);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(12);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.hyperlink_color = GOLD;
    v.warn_fg_color = GOLD;
    v.error_fg_color = Color32::from_rgb(0xF8, 0x71, 0x71);
    v.widgets.noninteractive.bg_fill = Color32::from_rgb(0x22, 0x26, 0x2F);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x3A, 0x40, 0x4C));
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x2A, 0x2F, 0x3A);
    v.widgets.inactive.bg_fill = Color32::from_rgb(0x2A, 0x2F, 0x3A);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x39, 0x46);
    v.widgets.hovered.bg_fill = Color32::from_rgb(0x33, 0x39, 0x46);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0x3A, 0x40, 0x4C);
    v.widgets.active.bg_fill = Color32::from_rgb(0x3A, 0x40, 0x4C);
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v
}

pub fn severity_color(severity: vfk_config::Severity) -> Color32 {
    match severity {
        vfk_config::Severity::Error => ERROR,
        vfk_config::Severity::Warning => WARNING,
        vfk_config::Severity::Info => MUTED,
    }
}
