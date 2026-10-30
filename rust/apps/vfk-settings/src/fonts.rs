//! Font tiếng Việt. Egui mặc định không phủ đủ dấu; nhúng Be Vietnam Pro (OFL).

pub const BE_VIETNAM_PRO: &[u8] = include_bytes!("../../../../resources/fonts/BeVietnamPro-Regular.ttf");

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "be_vietnam_pro".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(BE_VIETNAM_PRO)),
    );

    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "be_vietnam_pro".to_owned());

    // Macro key/JSON dùng monospace; thêm font Việt vào cuối để fallback dấu.
    fonts.families.entry(egui::FontFamily::Monospace).or_default().push("be_vietnam_pro".to_owned());

    ctx.set_fonts(fonts);
}
