//! UI cấu hình egui. Giữ toàn bộ state trong `SettingsApp`; egui chỉ render.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use vfk_config::{
    validate_macro_rows, ConfigDocument, FieldId, ImportPreview, Issue, SettingsGroup, Severity, ToneStyle,
    CHARSETS,
};
use vfk_engine::{resolve_profile, AppProfile, OutputMode, UnicodeForm};

use crate::{platform, preview, screenshot, theme, tray, tray::TrayAction, ui_prefs::UiPrefs};

const APP_TITLE: &str = "ViFastKey — Cấu hình";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tab {
    Overview,
    Typing,
    Macros,
    Apps,
    Advanced,
    Diagnostics,
}

impl Tab {
    pub(crate) const ALL: &'static [Tab] =
        &[Tab::Overview, Tab::Typing, Tab::Macros, Tab::Apps, Tab::Advanced, Tab::Diagnostics];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Tab::Overview => "Tổng quan",
            Tab::Typing => "Kiểu gõ",
            Tab::Macros => "Macro",
            Tab::Apps => "Ứng dụng",
            Tab::Advanced => "Nâng cao",
            Tab::Diagnostics => "Chẩn đoán",
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Typing => "typing",
            Tab::Macros => "macros",
            Tab::Apps => "apps",
            Tab::Advanced => "advanced",
            Tab::Diagnostics => "diagnostics",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "overview" | "tong-quan" => Some(Tab::Overview),
            "typing" | "kieu-go" => Some(Tab::Typing),
            "macros" | "macro" => Some(Tab::Macros),
            "apps" | "ung-dung" => Some(Tab::Apps),
            "advanced" | "nang-cao" => Some(Tab::Advanced),
            "diagnostics" | "chan-doan" => Some(Tab::Diagnostics),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Save,
    Reload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Confirm {
    Reload,
    Reset,
    ResetGroup(SettingsGroup),
    Close,
    Quit,
    Import(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MacroRow {
    key: String,
    value: String,
}

#[derive(Clone, Debug)]
struct AppEditor {
    app: String,
    has_preset: bool,
    inherit_preset: bool,
    excluded: bool,
    output_mode: Option<OutputMode>,
    vietnamese: Option<bool>,
    edit_previous_word: Option<bool>,
    input_method: Option<String>,
    tone_style: Option<ToneStyle>,
    spell_check: Option<bool>,
    charset: Option<String>,
    backspace_delay_ms: u32,
    delay_per_char_ms: u32,
    wait_ack: bool,
}

impl AppEditor {
    fn from_doc(doc: &ConfigDocument, app: &str) -> Self {
        let preset = doc.preset(app);
        let user = doc.profile(app);
        let profile = user.or(preset).cloned().unwrap_or_default();
        AppEditor {
            app: app.to_string(),
            has_preset: preset.is_some(),
            inherit_preset: user.is_none() && preset.is_some(),
            excluded: doc.config().excluded_apps.iter().any(|a| a == app),
            output_mode: profile.output_mode,
            vietnamese: profile.vietnamese,
            edit_previous_word: profile.edit_previous_word,
            input_method: profile.input_method,
            tone_style: profile.tone_style,
            spell_check: profile.spell_check,
            charset: profile.charset,
            backspace_delay_ms: profile.backspace_delay_ms,
            delay_per_char_ms: profile.delay_per_char_ms,
            wait_ack: profile.wait_ack,
        }
    }

    fn to_profile(&self) -> AppProfile {
        AppProfile {
            output_mode: self.output_mode,
            backspace_delay_ms: self.backspace_delay_ms,
            delay_per_char_ms: self.delay_per_char_ms,
            wait_ack: self.wait_ack,
            edit_previous_word: self.edit_previous_word,
            vietnamese: self.vietnamese,
            input_method: self.input_method.clone(),
            tone_style: self.tone_style,
            spell_check: self.spell_check,
            charset: self.charset.clone(),
        }
    }
}

pub struct SettingsApp {
    doc: ConfigDocument,
    read_only: bool,
    error: Option<String>,
    banner: Option<String>,
    external_change: bool,
    last_check: Instant,
    toast: Option<(String, Instant)>,
    last_save: Option<String>,
    tab: Tab,
    dark: bool,
    issues: Vec<Issue>,
    macros: Vec<MacroRow>,
    app_ids: Vec<String>,
    selected_app: Option<String>,
    app_editor: Option<AppEditor>,
    new_app: String,
    preview_input: String,
    preview_mode: OutputMode,
    preview_result: preview::PreviewResult,
    pending: Option<Pending>,
    confirm: Option<Confirm>,
    allow_close: bool,
    prefs: UiPrefs,
    windows_backend: String,
    hook_running: bool,
    import_preview: Option<(PathBuf, ImportPreview)>,
    app_filter: String,
    tray_enabled: bool,
    tray_attempted: bool,
    tray: Option<tray::Tray>,
    tray_error: Option<String>,
    close_to_tray: bool,
    screenshot: Option<screenshot::Plan>,
    title_dirty: bool,
}

impl SettingsApp {
    pub fn new(ctx: &egui::Context, path: std::path::PathBuf) -> Self {
        let (doc, read_only, banner) = match ConfigDocument::load(&path) {
            Ok(doc) => (doc, false, None),
            Err(err) => {
                let backup = match &err {
                    vfk_config::Error::Corrupt { backup: Some(path), .. } => {
                        format!(" (đã sao lưu: {})", path.display())
                    }
                    _ => String::new(),
                };
                (
                    ConfigDocument::new_default(&path),
                    true,
                    Some(format!("{err}{backup}\nĐang hiển thị giá trị mặc định; chưa ghi đè file.")),
                )
            }
        };

        let prefs = UiPrefs::load(&path);
        let dark = match prefs.theme.as_deref() {
            Some("dark") => true,
            Some("light") => false,
            _ => ctx.style().visuals.dark_mode,
        };
        theme::apply(ctx, dark);
        let initial_tab = prefs.last_tab.clone();
        let windows_backend = platform::load_windows_backend(&path);
        let mut app = SettingsApp {
            doc,
            read_only,
            error: None,
            banner,
            external_change: false,
            last_check: Instant::now(),
            toast: None,
            last_save: None,
            tab: Tab::Overview,
            dark,
            issues: Vec::new(),
            macros: Vec::new(),
            app_ids: Vec::new(),
            selected_app: None,
            app_editor: None,
            new_app: String::new(),
            preview_input: "Tieengs Vieejt".to_string(),
            preview_mode: OutputMode::Preedit,
            preview_result: preview::PreviewResult::default(),
            pending: None,
            confirm: None,
            allow_close: false,
            prefs,
            windows_backend,
            hook_running: false,
            import_preview: None,
            app_filter: String::new(),
            tray_enabled: false,
            tray_attempted: false,
            tray: None,
            tray_error: None,
            close_to_tray: true,
            screenshot: None,
            title_dirty: false,
        };
        app.tray_enabled = app.prefs.tray.unwrap_or(default_tray_enabled());
        app.close_to_tray = app.prefs.close_to_tray.unwrap_or(true);
        if app.windows_backend == "hook" {
            if let Err(err) = platform::ensure_replace_mode() {
                app.error = Some(err);
            }
            app.hook_running = platform::hook_running();
        } else if let Err(err) = platform::disable_replace_mode() {
            app.error = Some(err);
        }
        app.sync_from_doc();
        if let Some(tab) = initial_tab.as_deref() {
            app.set_tab(tab);
        }
        app.run_preview();
        app
    }

    /// Mở thẳng một tab, dùng cho menu/tray của adapter: `--tab apps`.
    pub fn set_tab(&mut self, name: &str) {
        if let Some(tab) = Tab::from_name(name) {
            self.tab = tab;
        }
    }

    pub fn set_tray_override(&mut self, enabled: Option<bool>) {
        if let Some(enabled) = enabled {
            self.tray_enabled = enabled;
        }
    }

    pub fn set_screenshot(&mut self, plan: Option<screenshot::Plan>) {
        if plan.is_some() {
            self.tray_enabled = false;
        }
        self.screenshot = plan;
    }

    fn persist_prefs(&mut self) {
        self.prefs.theme = Some(if self.dark { "dark" } else { "light" }.to_string());
        self.prefs.last_tab = Some(self.tab.name().to_string());
        self.prefs.tray = Some(self.tray_enabled);
        self.prefs.close_to_tray = Some(self.close_to_tray);
        self.prefs.save(self.doc.path());
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        self.refresh_issues();
        self.ensure_tray(ctx);
        self.poll_tray(ctx);
        self.handle_screenshot_event(ctx);
        self.check_external(ctx);
        self.handle_shortcuts(ctx);
        self.handle_close_request(ctx);
        self.update_title(ctx);

        self.top_bar(ctx);
        self.sidebar(ctx);
        self.bottom_bar(ctx);
        self.central(ctx);
        self.dialogs(ctx);
        self.toast_ui(ctx);
        self.process_pending(ctx);
        self.drive_screenshot(ctx);

        if self.external_change || self.toast.is_some() || self.tray.is_some() {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        if self.screenshot.is_some() {
            ctx.request_repaint();
        }
    }

    fn ensure_tray(&mut self, ctx: &egui::Context) {
        if !self.tray_enabled || self.tray_attempted {
            return;
        }
        self.tray_attempted = true;
        let tray = tray::Tray::start(ctx.clone());
        if tray.is_available() {
            self.tray_error = None;
            self.tray = Some(tray);
        } else {
            self.tray_error = Some(tray.error().unwrap_or("desktop không hỗ trợ tray").to_string());
            self.tray_enabled = false;
            self.tray = None;
        }
    }

    fn poll_tray(&mut self, ctx: &egui::Context) {
        let actions = self.tray.as_ref().map(tray::Tray::poll).unwrap_or_default();
        for action in actions {
            match action {
                TrayAction::Show => self.show_window(ctx),
                TrayAction::Reload => {
                    self.show_window(ctx);
                    self.request_reload();
                }
                TrayAction::Quit => {
                    self.show_window(ctx);
                    if self.doc.is_dirty() {
                        self.confirm = Some(Confirm::Quit);
                    } else {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let dirty = self.doc.is_dirty();
        if dirty != self.title_dirty {
            self.title_dirty = dirty;
            let title = if dirty { "ViFastKey — Cấu hình •" } else { APP_TITLE };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.to_string()));
        }
    }

    fn handle_screenshot_event(&mut self, ctx: &egui::Context) {
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = image else { return };
        let Some(plan) = self.screenshot.as_mut() else { return };
        let Some((tab, path)) = plan.current().map(|(tab, path)| (tab, path.to_path_buf())) else { return };
        match screenshot::save_png(&path, image.as_ref()) {
            Ok(()) => println!("Đã chụp {} (tab {})", path.display(), tab.name()),
            Err(err) => eprintln!("vifastkey-settings: {err}"),
        }
        plan.saved();
    }

    fn drive_screenshot(&mut self, ctx: &egui::Context) {
        let Some((target, _)) = self.screenshot.as_ref().and_then(|plan| plan.current()) else {
            if self.screenshot.is_some() {
                self.screenshot = None;
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        };
        if self.tab != target {
            self.tab = target;
            ctx.request_repaint();
            return;
        }
        let step = self.screenshot.as_mut().map(screenshot::Plan::tick).unwrap_or(screenshot::Step::Done);
        match step {
            screenshot::Step::Wait => ctx.request_repaint_after(Duration::from_millis(16)),
            screenshot::Step::Request => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                ctx.request_repaint_after(Duration::from_millis(50));
            }
            screenshot::Step::TimedOut => {
                eprintln!("vifastkey-settings: compositor không trả ảnh chụp sau 10 giây");
                self.screenshot = None;
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            screenshot::Step::Done => {
                self.screenshot = None;
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn sync_from_doc(&mut self) {
        self.macros = self
            .doc
            .config()
            .macros
            .iter()
            .map(|(key, value)| MacroRow { key: key.clone(), value: value.clone() })
            .collect();
        self.macros.sort_by(|a, b| a.key.cmp(&b.key));
        self.refresh_app_ids();
        self.refresh_issues();
    }

    fn refresh_issues(&mut self) {
        let mut issues = self.doc.validate();
        let rows: Vec<(String, String)> =
            self.macros.iter().map(|r| (r.key.clone(), r.value.clone())).collect();
        issues.extend(validate_macro_rows(&rows));
        self.issues = issues;
    }

    fn has_errors(&self) -> bool {
        self.issues.iter().any(Issue::is_error)
    }

    fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Error).count()
    }

    fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Warning).count()
    }

    fn check_external(&mut self, ctx: &egui::Context) {
        if self.last_check.elapsed() < Duration::from_secs(1) {
            ctx.request_repaint_after(Duration::from_secs(1));
            return;
        }
        self.last_check = Instant::now();
        if self.doc.has_external_change() {
            self.external_change = true;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let save = ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S))
        });
        if save {
            self.pending = Some(Pending::Save);
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if self.allow_close {
            return;
        }
        if self.doc.is_dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm = Some(Confirm::Close);
            return;
        }
        if self.close_to_tray && self.tray.as_ref().is_some_and(tray::Tray::is_available) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("ViFastKey").size(20.0).strong());
                ui.label(egui::RichText::new("Cấu hình").color(theme::MUTED));
                if self.doc.is_dirty() {
                    ui.label(egui::RichText::new("● Chưa lưu").color(theme::WARNING));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_save =
                        !self.read_only && (self.doc.is_dirty() || !self.doc.exists()) && !self.has_errors();
                    if ui
                        .add_enabled(
                            can_save,
                            egui::Button::new(
                                egui::RichText::new("Lưu").strong().color(egui::Color32::WHITE),
                            )
                            .fill(theme::ACCENT),
                        )
                        .clicked()
                    {
                        self.pending = Some(Pending::Save);
                    }
                    if ui.button("Tải lại").clicked() {
                        self.request_reload();
                    }
                    let theme_label = if self.dark { "Giao diện sáng" } else { "Giao diện tối" };
                    if ui.button(theme_label).clicked() {
                        self.dark = !self.dark;
                        theme::apply(ui.ctx(), self.dark);
                        self.persist_prefs();
                    }
                });
            });
            ui.add_space(4.0);
            self.banner_ui(ui);
        });
    }

    fn banner_ui(&mut self, ui: &mut egui::Ui) {
        if self.read_only {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme::ERROR, "File config hiện tại không đọc được. Đang ở chế độ chỉ đọc.");
                if ui.button("Dùng mặc định và cho phép lưu").clicked() {
                    self.read_only = false;
                    self.banner = None;
                    self.doc = ConfigDocument::new_default(self.doc.path().to_path_buf());
                    self.doc.reset();
                    self.sync_from_doc();
                }
            });
        }
        if self.external_change {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    theme::WARNING,
                    "File config đã thay đổi trên đĩa. Bản nháp của bạn có thể bị ghi đè.",
                );
                if ui.button("Tải lại").clicked() {
                    self.request_reload();
                }
                if ui.button("Bỏ qua").clicked() {
                    self.external_change = false;
                }
            });
        }
        if let Some(error) = self.error.clone() {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme::ERROR, error);
                if ui.button("Đóng").clicked() {
                    self.error = None;
                }
            });
        }
        if let Some(banner) = self.banner.clone() {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme::WARNING, banner);
                if ui.button("Đóng").clicked() {
                    self.banner = None;
                }
            });
        }
        if let Some(last) = &self.last_save {
            ui.label(egui::RichText::new(last).small().color(theme::MUTED));
        }
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav").exact_width(196.0).resizable(false).show(ctx, |ui| {
            ui.add_space(10.0);
            for &tab in Tab::ALL {
                let selected = self.tab == tab;
                let color = if self.dark {
                    if selected {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(0xD8, 0xDC, 0xE4)
                    }
                } else {
                    theme::INK
                };
                let text = egui::RichText::new(tab.label()).color(color);
                if ui.add_sized([176.0, 32.0], egui::Button::selectable(selected, text)).clicked() {
                    self.tab = tab;
                    self.persist_prefs();
                }
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .color(theme::MUTED),
                );
                ui.label(
                    egui::RichText::new(if self.read_only { "Chỉ đọc" } else { "Sẵn sàng" })
                        .small()
                        .color(if self.read_only { theme::ERROR } else { theme::SUCCESS }),
                );
            });
        });
    }

    fn bottom_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(self.doc.path().display().to_string()).small().color(theme::MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.error_count() > 0 {
                        ui.colored_label(theme::ERROR, format!("{} lỗi", self.error_count()));
                    }
                    if self.warning_count() > 0 {
                        ui.colored_label(theme::WARNING, format!("{} cảnh báo", self.warning_count()));
                    }
                    ui.label(
                        egui::RichText::new(format!("schema {}", self.doc.config().schema_version))
                            .small()
                            .color(theme::MUTED),
                    );
                });
            });
        });
    }

    fn central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt(self.tab.name()).auto_shrink([false, false]).show(
                ui,
                |ui| {
                    let tab = self.tab;
                    match tab {
                        Tab::Overview => self.overview_tab(ui),
                        Tab::Typing => self.typing_tab(ui),
                        Tab::Macros => self.macros_tab(ui),
                        Tab::Apps => self.apps_tab(ui),
                        Tab::Advanced => self.advanced_tab(ui),
                        Tab::Diagnostics => self.diagnostics_tab(ui),
                    }
                },
            );
        });
    }

    fn overview_tab(&mut self, ui: &mut egui::Ui) {
        let config = self.doc.config();
        let method = method_label(&config.input_method);
        let tone = tone_label(config.tone_style);
        let unicode = if config.unicode_form == UnicodeForm::Nfc { "NFC" } else { "NFD" };
        let macro_count = config.macros.len();
        let profile_count = self.doc.profiles().len();
        let excluded_count = config.excluded_apps.len();
        let path = self.doc.path().display().to_string();
        let exists = self.doc.exists();

        ui.heading("Tổng quan");
        ui.label("Cấu hình dùng chung cho mọi adapter: file JSON là nguồn sự thật.");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            card(ui, "Kiểu gõ", method);
            card(ui, "Đặt dấu", tone);
            card(ui, "Unicode", unicode);
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            card(ui, "Macro", &macro_count.to_string());
            card(ui, "App ghi đè", &profile_count.to_string());
            card(ui, "App loại trừ", &excluded_count.to_string());
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("File cấu hình").strong());
            ui.label(egui::RichText::new(path).monospace());
            ui.label(
                egui::RichText::new(if exists {
                    "Đã có file"
                } else {
                    "Chưa có file — sẽ tạo khi lưu"
                })
                .small()
                .color(theme::MUTED),
            );
            ui.horizontal(|ui| {
                if ui.button("Mở thư mục").clicked() {
                    if let Some(parent) = self.doc.path().parent() {
                        if let Err(err) = platform::open_path(parent) {
                            self.error = Some(format!("Không mở được thư mục: {err}"));
                        }
                    }
                }
                if ui.button("Sửa kiểu gõ").clicked() {
                    self.tab = Tab::Typing;
                }
                if ui.button("Thêm macro").clicked() {
                    self.tab = Tab::Macros;
                }
                if ui.button("Ứng dụng").clicked() {
                    self.tab = Tab::Apps;
                }
            });
        });

        ui.add_space(12.0);
        self.preview_panel(ui);
        self.issues_panel(ui);
    }

    fn typing_tab(&mut self, ui: &mut egui::Ui) {
        let issues = self.issues.clone();
        ui.heading("Kiểu gõ");
        ui.horizontal(|ui| {
            ui.label("Chọn cách gõ và các quy tắc áp dụng cho toàn bộ ứng dụng.");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Khôi phục nhóm").clicked() {
                    self.confirm = Some(Confirm::ResetGroup(SettingsGroup::Typing));
                }
            });
        });

        section(ui, "Kiểu gõ");
        {
            let draft = self.doc.draft_mut();
            let before = draft.config.input_method.clone();
            ui.horizontal(|ui| {
                ui.label("Kiểu gõ");
                let mut method = before.clone();
                egui::ComboBox::from_id_salt("input_method").selected_text(method_label(&method)).show_ui(
                    ui,
                    |ui| {
                        for (value, label) in [
                            ("telex", "Telex"),
                            ("simple_telex", "Simple Telex"),
                            ("vni", "VNI"),
                            ("viqr", "VIQR"),
                        ] {
                            ui.selectable_value(&mut method, value.to_string(), label);
                        }
                    },
                );
                if method != before {
                    draft.config.input_method = method;
                    draft.mark(FieldId::InputMethod);
                }
            });
            issues_for(ui, &issues, FieldId::InputMethod);

            let before = draft.config.tone_style;
            ui.horizontal(|ui| {
                ui.label("Đặt dấu");
                let mut tone = before;
                egui::ComboBox::from_id_salt("tone_style").selected_text(tone_label(tone)).show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(&mut tone, ToneStyle::New, "Mới (hoà)");
                        ui.selectable_value(&mut tone, ToneStyle::Old, "Cũ (hòa)");
                    },
                );
                if tone != before {
                    draft.config.tone_style = tone;
                    draft.mark(FieldId::ToneStyle);
                }
            });
        }

        section(ui, "Chính tả và Telex");
        {
            let draft = self.doc.draft_mut();
            let is_telex = matches!(draft.config.input_method.as_str(), "telex" | "simple_telex");
            if bool_row(
                ui,
                "Kiểm tra chính tả",
                "Chặn biến đổi sai chính tả và khôi phục raw.",
                &mut draft.config.spell_check,
            ) {
                draft.mark(FieldId::SpellCheck);
            }
            issues_for(ui, &issues, FieldId::SpellCheck);

            let changed = ui
                .add_enabled(is_telex, egui::Checkbox::new(&mut draft.config.w_as_u, "w -> ư khi đứng riêng"))
                .changed();
            if changed {
                draft.mark(FieldId::WAsU);
            }
            issues_for(ui, &issues, FieldId::WAsU);

            let changed = ui
                .add_enabled(is_telex, egui::Checkbox::new(&mut draft.config.bracket_keys, "[ ] -> ơ ư"))
                .changed();
            if changed {
                draft.mark(FieldId::BracketKeys);
            }
            issues_for(ui, &issues, FieldId::BracketKeys);

            let changed = ui
                .add_enabled(
                    is_telex,
                    egui::Checkbox::new(&mut draft.config.quick_telex, "Quick Telex (cc->ch, gg->gi…)"),
                )
                .changed();
            if changed {
                draft.mark(FieldId::QuickTelex);
            }
            issues_for(ui, &issues, FieldId::QuickTelex);

            if bool_row(
                ui,
                "Âm cuối k cho tên riêng",
                "Ví dụ Đắk Lắk; chỉ áp dụng với thanh sắc/nặng.",
                &mut draft.config.allow_coda_k,
            ) {
                draft.mark(FieldId::AllowCodaK);
            }
        }

        section(ui, "Từ và câu");
        {
            let draft = self.doc.draft_mut();
            if bool_row(
                ui,
                "Tự khôi phục tiếng Anh",
                "Dùng từ điển để trả lại từ tiếng Anh sau khi gõ.",
                &mut draft.config.auto_restore_english,
            ) {
                draft.mark(FieldId::AutoRestoreEnglish);
            }
            if bool_row(
                ui,
                "Viết hoa đầu câu",
                "Tự viết hoa chữ đầu câu khi bắt đầu từ mới.",
                &mut draft.config.auto_capitalize,
            ) {
                draft.mark(FieldId::AutoCapitalize);
            }
            if bool_row(
                ui,
                "Bảo vệ chữ HOA",
                "2 chữ hoa đầu từ thì gõ thẳng (JSON, API), trừ cặp tạo chữ Việt.",
                &mut draft.config.smart_caps,
            ) {
                draft.mark(FieldId::SmartCaps);
            }
            if bool_row(
                ui,
                "Sửa từ vừa gõ",
                "Dùng surrounding text để sửa từ đã commit (REPLACE).",
                &mut draft.config.edit_previous_word,
            ) {
                draft.mark(FieldId::EditPreviousWord);
            }
            if bool_row(
                ui,
                "Backspace sau dấu cách nạp lại từ trước",
                "Soft reset: xoá dấu cách rồi sửa tiếp từ vừa gõ.",
                &mut draft.config.restore_after_space,
            ) {
                draft.mark(FieldId::RestoreAfterSpace);
            }
            if bool_row(
                ui,
                "Mặc định tiếng Việt",
                "Chế độ bật/tắt mặc định khi chưa có ghi nhớ theo app.",
                &mut draft.config.default_vietnamese,
            ) {
                draft.mark(FieldId::DefaultVietnamese);
            }
        }
        ui.add_space(16.0);
    }

    fn macros_tab(&mut self, ui: &mut egui::Ui) {
        let issues = self.issues.clone();
        ui.heading("Macro");
        ui.horizontal(|ui| {
            ui.label("Macro chạy khi kết thúc từ (dấu cách, Enter, Tab, dấu câu).");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Khôi phục tùy chọn").clicked() {
                    self.confirm = Some(Confirm::ResetGroup(SettingsGroup::Macros));
                }
            });
        });

        section(ui, "Bật macro");
        {
            let draft = self.doc.draft_mut();
            if bool_row(ui, "Bật macro", "Tắt để bỏ qua toàn bộ macro.", &mut draft.config.macro_enabled)
            {
                draft.mark(FieldId::MacroEnabled);
            }
            let enabled = draft.config.macro_enabled;
            let changed = ui
                .add_enabled(
                    enabled,
                    egui::Checkbox::new(&mut draft.config.macro_digits, "Giữ chữ số trong từ (mp3, k8s)"),
                )
                .changed();
            if changed {
                draft.mark(FieldId::MacroDigits);
            }
            let changed = ui
                .add_enabled(
                    enabled,
                    egui::Checkbox::new(&mut draft.config.macro_undo, "Backspace hoàn tác macro"),
                )
                .changed();
            if changed {
                draft.mark(FieldId::MacroUndo);
            }
            let changed = ui
                .add_enabled(
                    enabled,
                    egui::Checkbox::new(
                        &mut draft.config.macro_in_off_mode,
                        "Vẫn dùng macro khi tắt tiếng Việt",
                    ),
                )
                .changed();
            if changed {
                draft.mark(FieldId::MacroInOffMode);
            }
        }

        section(ui, "Bảng macro");
        ui.label(
            egui::RichText::new("Key được tra lowercase. Key có dấu cách là macro nhiều từ.")
                .small()
                .color(theme::MUTED),
        );
        let mut rows = self.macros.clone();
        let mut remove = None;
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(180.0, 20.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(egui::RichText::new("Trigger").strong());
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(360.0, 20.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(egui::RichText::new("Thay thế").strong());
                },
            );
            ui.label("");
        });
        for (index, row) in rows.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add_sized([180.0, 26.0], egui::TextEdit::singleline(&mut row.key));
                ui.add_sized([360.0, 26.0], egui::TextEdit::singleline(&mut row.value));
                if ui.small_button("Xoá").clicked() {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = remove {
            rows.remove(index);
        }
        ui.horizontal(|ui| {
            if ui.button("+ Thêm dòng").clicked() {
                rows.push(MacroRow { key: String::new(), value: String::new() });
            }
            if ui.button("Sao chép JSON").clicked() {
                let map = rows_to_map(&rows);
                let json = serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".into());
                ui.ctx().copy_text(json);
                self.toast("Đã sao chép macro dạng JSON");
            }
            ui.label(egui::RichText::new(format!("{} dòng", rows.len())).small().color(theme::MUTED));
        });
        if rows != self.macros {
            self.macros = rows;
            self.doc.set_macros(rows_to_map(&self.macros));
        }
        issues_for(ui, &issues, FieldId::Macros);
        ui.add_space(16.0);
    }

    fn apps_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Ứng dụng");
        ui.horizontal(|ui| {
            ui.label(
                "Ghi đè cách gõ, output và độ trễ theo từng app. Loại trừ hoàn toàn thắng mọi cấu hình khác.",
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Khôi phục nhóm").clicked() {
                    self.confirm = Some(Confirm::ResetGroup(SettingsGroup::Apps));
                }
            });
        });

        section(ui, "Chọn app");
        let filter = self.app_filter.trim().to_lowercase();
        let app_ids: Vec<String> =
            self.app_ids.iter().filter(|app| filter.is_empty() || app.contains(&filter)).cloned().collect();
        let mut selected = self.selected_app.clone();
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.app_filter).hint_text("Tìm app…").desired_width(140.0),
            );
            egui::ComboBox::from_id_salt("selected_app")
                .selected_text(selected.clone().unwrap_or_else(|| "— chọn app —".into()))
                .show_ui(ui, |ui| {
                    for app in &app_ids {
                        ui.selectable_value(&mut selected, Some(app.clone()), app);
                    }
                    if app_ids.is_empty() {
                        ui.label("Không có app khớp.");
                    }
                });
            if selected != self.selected_app {
                if let Some(app) = selected.clone() {
                    self.select_app(app);
                }
            }
            ui.separator();
            ui.add(
                egui::TextEdit::singleline(&mut self.new_app)
                    .hint_text("app id mới, vd: zed")
                    .desired_width(150.0),
            );
            if ui.button("Thêm").clicked() {
                let app = vfk_engine::normalize_app_id(Some(&self.new_app));
                if app.is_empty() {
                    self.error = Some("App id không hợp lệ.".into());
                } else {
                    self.new_app.clear();
                    self.app_filter.clear();
                    if !self.app_ids.contains(&app) {
                        self.app_ids.push(app.clone());
                        self.app_ids.sort();
                    }
                    self.select_app(app);
                }
            }
        });

        let Some(mut editor) = self.app_editor.clone() else {
            self.issues_panel(ui);
            return;
        };

        section(ui, &format!("Cấu hình cho {}", editor.app));
        let user_override = self.doc.profile(&editor.app).is_some();
        ui.label(
            egui::RichText::new(if user_override {
                "Nguồn: ghi đè của bạn"
            } else if editor.has_preset {
                "Nguồn: preset có sẵn"
            } else {
                "Nguồn: mặc định"
            })
            .small()
            .color(theme::MUTED),
        );

        let mut changed = false;
        if ui.checkbox(&mut editor.excluded, "Loại trừ hoàn toàn (luôn tiếng Anh)").changed() {
            changed = true;
        }
        if editor.has_preset {
            if ui.checkbox(&mut editor.inherit_preset, "Kế thừa preset (không ghi đè)").changed() {
                changed = true;
            }
            if editor.inherit_preset {
                ui.label(
                    egui::RichText::new("Đang dùng preset; bật một field bên dưới để tạo ghi đè.")
                        .small()
                        .color(theme::MUTED),
                );
            }
        }

        ui.scope(|ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Output");
                let before = editor.output_mode;
                if combo(
                    ui,
                    "output_mode",
                    &mut editor.output_mode,
                    &[
                        (None, "Mặc định (Preedit)"),
                        (Some(OutputMode::Preedit), "Preedit"),
                        (Some(OutputMode::Replace), "Replace"),
                    ],
                ) {
                    changed = true;
                }
                if editor.output_mode != before {
                    editor.inherit_preset = false;
                }
            });
            ui.label(
                egui::RichText::new(
                    "Preedit là mặc định. Replace chỉ dùng cho app không nhận preedit (macOS/Fcitx5); Windows chọn Preedit/Replace ở tab Nâng cao.",
                )
                .small()
                .color(theme::MUTED),
            );
            ui.horizontal(|ui| {
                ui.label("Tiếng Việt");
                if tri_bool(ui, "app_vietnamese", &mut editor.vietnamese, "Theo mặc định") {
                    changed = true;
                    editor.inherit_preset = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Sửa từ cũ");
                if tri_bool(ui, "app_edit_prev", &mut editor.edit_previous_word, "Theo mặc định") {
                    changed = true;
                    editor.inherit_preset = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Kiểm tra chính tả");
                if tri_bool(ui, "app_spell", &mut editor.spell_check, "Theo mặc định") {
                    changed = true;
                    editor.inherit_preset = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Kiểu gõ");
                if combo_optional_string(
                    ui,
                    "app_method",
                    &mut editor.input_method,
                    &[("telex", "Telex"), ("simple_telex", "Simple Telex"), ("vni", "VNI"), ("viqr", "VIQR")],
                ) {
                    changed = true;
                    editor.inherit_preset = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Đặt dấu");
                if combo(
                    ui,
                    "app_tone",
                    &mut editor.tone_style,
                    &[
                        (None, "Kế thừa"),
                        (Some(ToneStyle::Old), "Cũ (hòa)"),
                        (Some(ToneStyle::New), "Mới (hoà)"),
                    ],
                ) {
                    changed = true;
                    editor.inherit_preset = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Bảng mã gợi ý");
                if combo_optional_string(ui, "app_charset", &mut editor.charset, &charset_options()) {
                    changed = true;
                    editor.inherit_preset = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Backspace delay");
                if ui
                    .add(egui::DragValue::new(&mut editor.backspace_delay_ms).range(0..=2_000).suffix(" ms"))
                    .changed()
                {
                    changed = true;
                    editor.inherit_preset = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Delay mỗi ký tự");
                if ui
                    .add(egui::DragValue::new(&mut editor.delay_per_char_ms).range(0..=500).suffix(" ms"))
                    .changed()
                {
                    changed = true;
                    editor.inherit_preset = false;
                }
            });
            if ui.checkbox(&mut editor.wait_ack, "Chờ app xử lý backspace (ACK)").changed() {
                changed = true;
                editor.inherit_preset = false;
            }
        });

        ui.add_space(6.0);
        let mut apply = changed;
        ui.horizontal(|ui| {
            if editor.has_preset && !editor.inherit_preset && ui.button("Bỏ ghi đè, dùng preset").clicked()
            {
                editor.inherit_preset = true;
                apply = true;
            }
            ui.label(
                egui::RichText::new("Thay đổi được áp dụng vào bản nháp ngay; bấm Lưu để ghi file.")
                    .small()
                    .color(theme::MUTED),
            );
        });

        if apply {
            self.app_editor = Some(editor);
            self.apply_app_editor();
        }

        ui.add_space(10.0);
        self.effective_profile_ui(ui);
        self.app_modes_ui(ui);
        self.issues_panel(ui);
    }

    fn effective_profile_ui(&self, ui: &mut egui::Ui) {
        let Some(app) = &self.selected_app else { return };
        let config = self.doc.effective_config();
        let profile = resolve_profile(&config, Some(app));
        let excluded = config.excluded_apps.iter().any(|a| a == app);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Hiệu lực hiện tại").strong());
            if excluded {
                ui.colored_label(theme::ERROR, "App đang bị loại trừ hoàn toàn (luôn tiếng Anh).");
            }
            ui.label(format!(
                "Output: {} · Delay: {} ms + {} ms/ký tự · ACK: {}",
                output_label(profile.output_mode),
                profile.backspace_delay_ms,
                profile.delay_per_char_ms,
                if profile.wait_ack { "có" } else { "không" }
            ));
            ui.label(format!(
                "Tiếng Việt: {} · Sửa từ cũ: {} · Chính tả: {} · Charset: {}",
                tri_label(profile.vietnamese),
                tri_label(profile.edit_previous_word),
                tri_label(profile.spell_check),
                profile.charset.as_deref().unwrap_or("Unicode"),
            ));
        });
    }

    fn app_modes_ui(&mut self, ui: &mut egui::Ui) {
        let modes: Vec<(String, bool)> =
            self.doc.config().app_modes.iter().map(|(app, vi)| (app.clone(), *vi)).collect();
        if modes.is_empty() {
            return;
        }
        ui.add_space(6.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Trạng thái do adapter ghi nhớ").strong());
            ui.label(
                egui::RichText::new("Đây là runtime state, không phải cấu hình người dùng.")
                    .small()
                    .color(theme::MUTED),
            );
            let mut clear = None;
            for (app, vi) in &modes {
                ui.horizontal(|ui| {
                    ui.label(app);
                    ui.label(if *vi { "Tiếng Việt" } else { "Tiếng Anh" });
                    if ui.small_button("Quên").clicked() {
                        clear = Some(app.clone());
                    }
                });
            }
            if let Some(app) = clear {
                self.doc.clear_app_mode(&app);
            }
        });
    }

    fn advanced_tab(&mut self, ui: &mut egui::Ui) {
        let issues = self.issues.clone();
        ui.heading("Nâng cao");
        ui.horizontal(|ui| {
            ui.label("Tùy chọn đầu ra, đường dẫn và các thao tác trên file cấu hình.");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Khôi phục nhóm").clicked() {
                    self.confirm = Some(Confirm::ResetGroup(SettingsGroup::Advanced));
                }
            });
        });

        section(ui, "Đầu ra");
        {
            let draft = self.doc.draft_mut();
            if bool_row(
                ui,
                "Tự khôi phục tiếng Anh",
                "Dùng từ điển để trả lại từ tiếng Anh.",
                &mut draft.config.auto_restore_english,
            ) {
                draft.mark(FieldId::AutoRestoreEnglish);
            }
            ui.horizontal(|ui| {
                ui.label("Dạng Unicode");
                if combo(
                    ui,
                    "unicode_form",
                    &mut draft.config.unicode_form,
                    &[(UnicodeForm::Nfc, "NFC (dựng sẵn)"), (UnicodeForm::Nfd, "NFD (tổ hợp)")],
                ) {
                    draft.mark(FieldId::UnicodeForm);
                }
            });
            issues_for(ui, &issues, FieldId::UnicodeForm);
            ui.horizontal(|ui| {
                ui.label("Schema version");
                ui.label(egui::RichText::new(draft.config.schema_version.to_string()).monospace());
            });
        }

        section(ui, "Giao diện");
        if ui.checkbox(&mut self.dark, "Giao diện tối").changed() {
            theme::apply(ui.ctx(), self.dark);
            self.persist_prefs();
        }

        section(ui, "Khay hệ thống");
        let tray_changed =
            ui.checkbox(&mut self.tray_enabled, "Hiện biểu tượng ViFastKey trong khay hệ thống").changed();
        if tray_changed {
            if !self.tray_enabled {
                self.tray = None;
            } else {
                self.tray_attempted = false;
            }
            self.persist_prefs();
        }
        let close_changed = ui
            .add_enabled(
                self.tray_enabled,
                egui::Checkbox::new(&mut self.close_to_tray, "Đóng cửa sổ thì ẩn xuống khay thay vì thoát"),
            )
            .changed();
        if close_changed {
            self.persist_prefs();
        }
        if self.tray.as_ref().is_some_and(tray::Tray::is_available) {
            ui.colored_label(theme::SUCCESS, "Tray đang chạy.");
        } else if let Some(err) = &self.tray_error {
            ui.colored_label(theme::WARNING, format!("Tray chưa bật được: {err}"));
        }
        ui.label(
            egui::RichText::new(
                "Trên Linux cần desktop hỗ trợ StatusNotifierItem (GNOME cần extension AppIndicator).",
            )
            .small()
            .color(theme::MUTED),
        );

        section(ui, "File và sao lưu");
        ui.label(egui::RichText::new(self.doc.path().display().to_string()).monospace());
        ui.horizontal(|ui| {
            if ui.button("Mở thư mục").clicked() {
                if let Some(parent) = self.doc.path().parent() {
                    if let Err(err) = platform::open_path(parent) {
                        self.error = Some(format!("Không mở được thư mục: {err}"));
                    }
                }
            }
            if ui.button("Sao chép JSON hiệu lực").clicked() {
                ui.ctx().copy_text(self.doc.pretty_effective_json());
                self.toast("Đã sao chép JSON");
            }
            if ui.button("Tải lại từ đĩa").clicked() {
                self.request_reload();
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Khôi phục mặc định").color(egui::Color32::WHITE))
                        .fill(theme::ERROR),
                )
                .clicked()
            {
                self.confirm = Some(Confirm::Reset);
            }
            ui.label(egui::RichText::new("Tạo backup trước khi ghi đè.").small().color(theme::MUTED));
        });

        section(ui, "Nhập / Xuất");
        ui.horizontal(|ui| {
            if ui.button("Nhập từ file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    self.request_import(path);
                }
            }
            if ui.button("Xuất toàn bộ…").clicked() {
                self.export_dialog();
            }
            ui.label(
                egui::RichText::new("Nhập chỉ thay bản nháp; bấm Lưu mới ghi file.")
                    .small()
                    .color(theme::MUTED),
            );
        });
        egui::CollapsingHeader::new("Xem JSON hiệu lực (dùng khi xuất)").show(ui, |ui| {
            let mut text = self.doc.pretty_effective_json();
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .code_editor()
                    .desired_rows(14)
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        });

        if cfg!(target_os = "windows") {
            section(ui, "Windows: Preedit / Replace");
            ui.horizontal(|ui| {
                ui.label("Chế độ gõ");
                let mut backend = self.windows_backend.clone();
                let selected = if backend == "hook" { "Replace (Hook)" } else { "Preedit (TSF)" };
                egui::ComboBox::from_id_salt("windows_backend").selected_text(selected).show_ui(ui, |ui| {
                    ui.selectable_value(&mut backend, "tsf".to_string(), "Preedit (TSF)");
                    ui.selectable_value(&mut backend, "hook".to_string(), "Replace (Hook)");
                });
                if backend != self.windows_backend {
                    match platform::save_windows_backend(self.doc.path(), &backend) {
                        Ok(()) => {
                            let mode = if backend == "hook" {
                                platform::ensure_replace_mode()
                            } else {
                                platform::disable_replace_mode()
                            };
                            self.windows_backend = backend;
                            match mode {
                                Ok(()) => {
                                    self.hook_running = platform::hook_running();
                                    self.toast("Đã lưu windows.json");
                                }
                                Err(err) => self.error = Some(err),
                            }
                        }
                        Err(err) => self.error = Some(format!("Không ghi được windows.json: {err}")),
                    }
                }
            });
            if self.windows_backend == "hook" {
                if self.hook_running {
                    ui.colored_label(
                        theme::SUCCESS,
                        "Replace: hook nền đang chạy và tự khởi động cùng Windows.",
                    );
                } else {
                    ui.colored_label(
                        theme::WARNING,
                        "Replace: hook nền chưa chạy; bấm Kiểm tra/khởi động lại.",
                    );
                }
                if ui.button("Kiểm tra hook").clicked() {
                    if let Err(err) = platform::ensure_replace_mode() {
                        self.error = Some(err);
                    }
                    self.hook_running = platform::hook_running();
                }
            } else {
                ui.colored_label(theme::SUCCESS, "Preedit (TSF) là mặc định và an toàn cho đa số app.");
            }
            ui.label(
                egui::RichText::new(
                    "Preedit hợp với Office/trình duyệt/terminal; Replace dùng cho app hoặc game không nhận TSF. Đổi chế độ có hiệu lực trong khoảng 0,5 giây; nếu đang có preedit dở, bấm ra ngoài một lần.",
                )
                .small()
                .color(theme::MUTED),
            );
        }

        ui.add_space(16.0);
    }

    fn diagnostics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Chẩn đoán");
        let path = self.doc.path().display().to_string();
        let fingerprint = self
            .doc
            .fingerprint()
            .map(|f| format!("{}… ({} bytes)", &f.hash[..12.min(f.hash.len())], f.len))
            .unwrap_or_else(|| "file chưa tồn tại".into());
        let schema = self.doc.config().schema_version;
        let dirty = self.doc.is_dirty();

        ui.group(|ui| {
            ui.label(egui::RichText::new("Môi trường").strong());
            ui.label(format!("OS: {}", std::env::consts::OS));
            ui.label(format!("Phiên bản: {}", env!("CARGO_PKG_VERSION")));
            ui.label(format!("Config: {path}"));
            ui.label(format!("Fingerprint: {fingerprint}"));
            ui.label(format!("Schema: {schema} · Dirty: {}", if dirty { "có" } else { "không" }));
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Kiểm tra lại").clicked() {
                self.refresh_issues();
                self.toast("Đã kiểm tra cấu hình");
            }
            if ui.button("Gõ thử mẫu").clicked() {
                self.preview_input = "Tieengs Vieejt".into();
                self.run_preview();
            }
            if ui.button("Sao chép chẩn đoán").clicked() {
                let text = format!(
                    "ViFastKey settings {}\nOS: {}\nconfig: {path}\nschema: {schema}\ndirty: {dirty}\nerrors: {}\nwarnings: {}\n",
                    env!("CARGO_PKG_VERSION"),
                    std::env::consts::OS,
                    self.error_count(),
                    self.warning_count(),
                );
                ui.ctx().copy_text(text);
                self.toast("Đã sao chép chẩn đoán");
            }
        });

        self.preview_panel(ui);
        self.issues_panel(ui);
        ui.add_space(16.0);
    }

    fn preview_panel(&mut self, ui: &mut egui::Ui) {
        section(ui, "Gõ thử");
        ui.label(
            egui::RichText::new("Cú pháp raw key: < Backspace, ~ Escape, | Enter. Preview chỉ kiểm tra engine, không kiểm tra adapter.")
                .small()
                .color(theme::MUTED),
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.preview_input)
                    .desired_width(360.0)
                    .hint_text("tieengs<"),
            );
            egui::ComboBox::from_id_salt("preview_mode")
                .selected_text(output_label(Some(self.preview_mode)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.preview_mode, OutputMode::Preedit, "Preedit");
                    ui.selectable_value(&mut self.preview_mode, OutputMode::Replace, "Replace");
                });
            if ui.button("Chạy thử").clicked() {
                self.run_preview();
            }
        });
        if !self.preview_result.output.is_empty() || self.preview_result.error.is_some() {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Kết quả").strong());
                if let Some(error) = &self.preview_result.error {
                    ui.colored_label(theme::ERROR, error);
                }
                ui.label(egui::RichText::new(&self.preview_result.output).monospace().size(16.0));
                egui::CollapsingHeader::new("Action trace").show(ui, |ui| {
                    for action in &self.preview_result.actions {
                        ui.label(egui::RichText::new(action).small().monospace());
                    }
                });
            });
        }
    }

    fn issues_panel(&self, ui: &mut egui::Ui) {
        if self.issues.is_empty() {
            ui.add_space(8.0);
            ui.colored_label(theme::SUCCESS, "Không có lỗi cấu hình.");
            return;
        }
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Kiểm tra cấu hình").strong());
            for issue in &self.issues {
                ui.colored_label(theme::severity_color(issue.severity), format!("• {}", issue.message));
            }
        });
    }

    fn request_reload(&mut self) {
        if self.doc.is_dirty() {
            self.confirm = Some(Confirm::Reload);
        } else {
            self.pending = Some(Pending::Reload);
        }
    }

    fn select_app(&mut self, app: String) {
        self.selected_app = Some(app.clone());
        self.app_editor = Some(AppEditor::from_doc(&self.doc, &app));
    }

    fn refresh_app_ids(&mut self) {
        self.app_ids = self.doc.app_ids();
        self.app_ids.sort();
        if let Some(selected) = &self.selected_app {
            if !self.app_ids.iter().any(|a| a == selected) {
                self.selected_app = self.app_ids.first().cloned();
                self.app_editor = self.selected_app.as_ref().map(|app| AppEditor::from_doc(&self.doc, app));
            }
        } else if let Some(app) = self.app_ids.first() {
            self.select_app(app.clone());
        }
    }

    fn apply_app_editor(&mut self) {
        let Some(editor) = self.app_editor.clone() else { return };
        let app = editor.app.clone();
        let mut excluded = self.doc.config().excluded_apps.clone();
        if editor.excluded {
            if !excluded.iter().any(|a| a == &app) {
                excluded.push(app.clone());
            }
        } else {
            excluded.retain(|a| a != &app);
        }
        self.doc.set_excluded_apps(excluded);

        if editor.inherit_preset {
            self.doc.remove_profile(&app);
        } else {
            self.doc.set_profile(app.clone(), editor.to_profile());
        }
        self.refresh_app_ids();
        self.app_editor = Some(AppEditor::from_doc(&self.doc, &app));
        self.selected_app = Some(app.clone());
        self.toast(format!("Đã cập nhật cấu hình cho {app}"));
    }

    fn run_preview(&mut self) {
        let config = self.doc.effective_config();
        let app = self.selected_app.clone().unwrap_or_default();
        self.preview_result = preview::run(config, self.preview_mode, &app, &self.preview_input);
    }

    fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        let Some((text, at)) = self.toast.clone() else { return };
        if at.elapsed() > Duration::from_secs(4) {
            self.toast = None;
            return;
        }
        egui::Window::new("toast")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .show(ctx, |ui| {
                ui.label(text);
            });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm.clone() else { return };
        let mut chosen = false;
        let mut cancel = false;
        let (title, confirm_label) = match &action {
            Confirm::Reload => ("Tải lại cấu hình?", "Tải lại"),
            Confirm::Reset => ("Khôi phục mặc định?", "Khôi phục"),
            Confirm::ResetGroup(_) => ("Khôi phục nhóm?", "Khôi phục nhóm"),
            Confirm::Close => ("Đóng mà không lưu?", "Đóng không lưu"),
            Confirm::Quit => ("Thoát ViFastKey?", "Thoát"),
            Confirm::Import(_) => ("Nhập cấu hình?", "Nhập"),
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .default_width(620.0)
            .max_width(760.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                match &action {
                    Confirm::Reload => {
                        ui.label("Các thay đổi chưa lưu sẽ bị bỏ.");
                    }
                    Confirm::Reset => {
                        ui.label("Toàn bộ cấu hình người dùng sẽ bị xoá. File hiện tại được sao lưu trước khi ghi.");
                    }
                    Confirm::ResetGroup(group) => {
                        ui.label(group_reset_body(*group));
                    }
                    Confirm::Close => {
                        ui.label("Bạn có thay đổi chưa lưu. Đóng bây giờ sẽ bỏ các thay đổi đó.");
                    }
                    Confirm::Quit => {
                        ui.label("Thoát ứng dụng cấu hình? Bộ gõ vẫn tiếp tục chạy.");
                        if self.doc.is_dirty() {
                            ui.colored_label(theme::WARNING, "Có thay đổi chưa lưu sẽ bị bỏ.");
                        }
                    }
                    Confirm::Import(path) => self.import_dialog_ui(ui, path),
                }
                ui.add_space(8.0);
                let can_confirm = match &action {
                    Confirm::Import(_) => self.import_preview.as_ref().is_some_and(|(_, p)| !p.has_errors()),
                    _ => true,
                };
                ui.horizontal(|ui| {
                    if ui.add_enabled(can_confirm, egui::Button::new(confirm_label)).clicked() {
                        chosen = true;
                    }
                    if ui.button("Huỷ").clicked() {
                        cancel = true;
                    }
                });
            });
        if chosen {
            self.confirm = None;
            self.apply_confirm(ctx, action);
        } else if cancel {
            self.confirm = None;
            if matches!(action, Confirm::Import(_)) {
                self.import_preview = None;
            }
        }
    }

    fn import_dialog_ui(&self, ui: &mut egui::Ui, path: &Path) {
        let Some((_, preview)) = &self.import_preview else {
            ui.colored_label(theme::ERROR, "Không có preview để hiển thị.");
            return;
        };
        ui.label(format!(
            "Nạp {} sẽ thay thế bản nháp hiện tại. File hiện tại chỉ bị ghi khi bấm Lưu; key lạ của file nhập được giữ.",
            path.display()
        ));
        ui.label(
            egui::RichText::new(format!(
                "{} thay đổi · {} lỗi · {} cảnh báo · {} bytes",
                preview.change_count(),
                preview.error_count(),
                preview.warning_count(),
                preview.bytes
            ))
            .small()
            .color(theme::MUTED),
        );
        ui.add_space(4.0);
        if preview.changes.is_empty() {
            ui.label("Giá trị hiệu lực không đổi.");
        } else {
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                for change in preview.changes.iter().take(200) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(&change.path).monospace().strong());
                        ui.label("->");
                        ui.label(egui::RichText::new(format_change(change)).small());
                    });
                }
                if preview.changes.len() > 200 {
                    ui.label(
                        egui::RichText::new(format!("… và {} thay đổi nữa", preview.changes.len() - 200))
                            .small()
                            .color(theme::MUTED),
                    );
                }
            });
        }
        let visible_issues: Vec<_> =
            preview.issues.iter().filter(|issue| issue.severity != Severity::Info).collect();
        if !visible_issues.is_empty() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Kiểm tra cấu hình nhập").strong());
            for issue in visible_issues.iter().take(40) {
                ui.colored_label(theme::severity_color(issue.severity), format!("• {}", issue.message));
            }
            if visible_issues.len() > 40 {
                ui.label(
                    egui::RichText::new(format!("… và {} cảnh báo nữa", visible_issues.len() - 40))
                        .small()
                        .color(theme::MUTED),
                );
            }
        }
    }

    fn apply_confirm(&mut self, ctx: &egui::Context, action: Confirm) {
        match action {
            Confirm::Reload => self.pending = Some(Pending::Reload),
            Confirm::Reset => {
                self.doc.reset();
                self.macros.clear();
                self.selected_app = None;
                self.app_editor = None;
                self.sync_from_doc();
                self.pending = Some(Pending::Save);
            }
            Confirm::ResetGroup(group) => {
                let changes = self.doc.reset_group(group);
                self.sync_from_doc();
                if let Some(app) = self.selected_app.clone() {
                    self.select_app(app);
                }
                self.toast(format!("Đã khôi phục nhóm ({} thay đổi)", changes));
            }
            Confirm::Close | Confirm::Quit => {
                self.persist_prefs();
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Confirm::Import(_) => self.apply_import(),
        }
    }

    fn request_import(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(text) => match self.doc.preview_import(&text) {
                Ok(preview) => {
                    self.import_preview = Some((path.clone(), preview));
                    self.confirm = Some(Confirm::Import(path));
                }
                Err(err) => self.error = Some(format!("File nhập không hợp lệ: {err}")),
            },
            Err(err) => self.error = Some(format!("Không đọc được {}: {err}", path.display())),
        }
    }

    fn apply_import(&mut self) {
        let Some((path, preview)) = self.import_preview.take() else { return };
        match self.doc.apply_import(preview) {
            Ok(()) => {
                self.read_only = false;
                self.banner = None;
                self.external_change = false;
                self.sync_from_doc();
                self.toast(format!("Đã nạp {} — bấm Lưu để ghi", path.display()));
            }
            Err(err) => self.error = Some(format!("Không nạp được {}: {err}", path.display())),
        }
    }

    fn export_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("vifastkey-config.json")
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return;
        };
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            if let Err(err) = std::fs::create_dir_all(parent) {
                self.error = Some(format!("Không tạo được thư mục: {err}"));
                return;
            }
        }
        match std::fs::write(&path, self.doc.pretty_effective_json()) {
            Ok(()) => self.toast(format!("Đã xuất cấu hình ra {}", path.display())),
            Err(err) => self.error = Some(format!("Không xuất được {}: {err}", path.display())),
        }
    }

    fn process_pending(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending.take() else { return };
        match pending {
            Pending::Save => self.save(),
            Pending::Reload => match ConfigDocument::load(self.doc.path()) {
                Ok(doc) => {
                    self.doc = doc;
                    self.read_only = false;
                    self.external_change = false;
                    self.sync_from_doc();
                    self.toast("Đã tải lại cấu hình");
                }
                Err(err) => self.error = Some(format!("Không tải lại được: {err}")),
            },
        }
        ctx.request_repaint();
    }

    fn save(&mut self) {
        if self.read_only {
            self.error = Some("Đang ở chế độ chỉ đọc; chưa thể lưu.".into());
            return;
        }
        let current: BTreeMap<String, String> =
            self.doc.config().macros.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let rows = rows_to_map(&self.macros);
        if current != rows {
            self.doc.set_macros(rows);
        }

        // Recompute sau khi đồng bộ macro để Ctrl+S không bỏ qua lỗi vừa sửa.
        let row_issues = validate_macro_rows(
            &self.macros.iter().map(|r| (r.key.clone(), r.value.clone())).collect::<Vec<_>>(),
        );
        let mut issues = self.doc.validate();
        issues.extend(row_issues);
        self.issues = issues;
        if self.has_errors() {
            self.error = Some("Còn lỗi cấu hình; sửa trước khi lưu.".into());
            return;
        }

        match self.doc.save() {
            Ok(Some(outcome)) => {
                self.external_change = false;
                self.last_save = Some(format!(
                    "Đã lưu {} ({} bytes){}",
                    outcome.path.display(),
                    outcome.bytes,
                    outcome
                        .backup
                        .as_ref()
                        .map(|p| format!(" · backup: {}", p.display()))
                        .unwrap_or_default()
                ));
                self.toast("Đã lưu cấu hình");
                self.sync_from_doc();
            }
            Ok(None) => self.toast("Không có thay đổi để lưu"),
            Err(err) => {
                self.error = Some(format!("Không lưu được: {err}"));
                if matches!(err, vfk_config::Error::Conflict { .. }) {
                    self.external_change = true;
                }
            }
        }
    }
}

// ---------------------------------------------------------------- helpers

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(egui::RichText::new(title).size(16.0).strong());
    ui.add_space(2.0);
}

fn card(ui: &mut egui::Ui, title: &str, value: &str) {
    ui.group(|ui| {
        ui.set_min_width(150.0);
        ui.label(egui::RichText::new(title).small().color(theme::MUTED));
        ui.label(egui::RichText::new(value).size(16.0).strong());
    });
}

fn bool_row(ui: &mut egui::Ui, label: &str, help: &str, value: &mut bool) -> bool {
    let changed = ui.checkbox(value, label).changed();
    ui.label(egui::RichText::new(help).small().color(theme::MUTED));
    changed
}

fn issues_for(ui: &mut egui::Ui, issues: &[Issue], field: FieldId) {
    for issue in issues.iter().filter(|i| i.field == Some(field)) {
        ui.colored_label(theme::severity_color(issue.severity), format!("• {}", issue.message));
    }
}

fn combo<T: PartialEq + Copy>(ui: &mut egui::Ui, id: &str, current: &mut T, options: &[(T, &str)]) -> bool {
    let before = *current;
    let selected = options.iter().find(|(value, _)| value == current).map(|(_, label)| *label).unwrap_or("—");
    egui::ComboBox::from_id_salt(id).selected_text(selected).show_ui(ui, |ui| {
        for (value, label) in options {
            ui.selectable_value(current, *value, *label);
        }
    });
    *current != before
}

fn tri_bool(ui: &mut egui::Ui, id: &str, current: &mut Option<bool>, default_label: &str) -> bool {
    combo(ui, id, current, &[(None, default_label), (Some(true), "Bật"), (Some(false), "Tắt")])
}

fn combo_optional_string(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut Option<String>,
    options: &[(&str, &str)],
) -> bool {
    let before = current.clone();
    let mut selected = current.clone().unwrap_or_default();
    let selected_label = if selected.is_empty() {
        "Kế thừa".to_string()
    } else {
        options
            .iter()
            .find(|(key, _)| *key == selected)
            .map(|(_, label)| (*label).to_string())
            .unwrap_or_else(|| selected.clone())
    };
    egui::ComboBox::from_id_salt(id).selected_text(selected_label).show_ui(ui, |ui| {
        if ui.selectable_label(selected.is_empty(), "Kế thừa").clicked() {
            selected.clear();
        }
        for (key, label) in options {
            if ui.selectable_label(selected == *key, *label).clicked() {
                selected = (*key).to_string();
            }
        }
    });
    let after = if selected.is_empty() { None } else { Some(selected) };
    if after != before {
        *current = after;
        true
    } else {
        false
    }
}

fn charset_options() -> Vec<(&'static str, &'static str)> {
    CHARSETS.iter().map(|name| (*name, *name)).collect()
}

fn method_label(method: &str) -> &'static str {
    match method {
        "simple_telex" => "Simple Telex",
        "vni" => "VNI",
        "viqr" => "VIQR",
        _ => "Telex",
    }
}

fn tone_label(tone: ToneStyle) -> &'static str {
    match tone {
        ToneStyle::Old => "Cũ (hòa)",
        ToneStyle::New => "Mới (hoà)",
    }
}

fn output_label(mode: Option<OutputMode>) -> &'static str {
    match mode {
        Some(OutputMode::Preedit) => "Preedit",
        Some(OutputMode::Replace) => "Replace",
        None => "Mặc định",
    }
}

fn tri_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "Bật",
        Some(false) => "Tắt",
        None => "Theo mặc định",
    }
}

fn group_reset_body(group: SettingsGroup) -> &'static str {
    match group {
        SettingsGroup::Typing => {
            "Đưa kiểu gõ, đặt dấu, chính tả và các tùy chọn Telex về mặc định. Bảng macro và app profile không đổi."
        }
        SettingsGroup::Macros => {
            "Đưa các tùy chọn macro về mặc định. Bảng macro hiện có được giữ nguyên."
        }
        SettingsGroup::Apps => {
            "Xoá toàn bộ app profile ghi đè và danh sách app loại trừ. Trạng thái Việt/Anh do adapter ghi nhớ vẫn được giữ."
        }
        SettingsGroup::Advanced => {
            "Đưa các tùy chọn đầu ra, viết hoa, sửa từ và Unicode về mặc định."
        }
    }
}

fn format_change(change: &vfk_config::Change) -> String {
    let before = change.before.as_ref().map(compact_json).unwrap_or_else(|| "(không có)".into());
    let after = change.after.as_ref().map(compact_json).unwrap_or_else(|| "(xoá)".into());
    format!("{before} -> {after}")
}

fn compact_json(value: &serde_json::Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 90 {
        let mut out: String = text.chars().take(87).collect();
        out.push('…');
        out
    } else {
        text
    }
}

fn default_tray_enabled() -> bool {
    cfg!(any(target_os = "windows", target_os = "linux"))
}

fn rows_to_map(rows: &[MacroRow]) -> BTreeMap<String, String> {
    rows.iter()
        .filter_map(|row| {
            let key = row.key.trim().to_lowercase();
            if key.is_empty() {
                None
            } else {
                Some((key, row.value.clone()))
            }
        })
        .collect()
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(ctx);
    }
}

pub fn window_title() -> &'static str {
    APP_TITLE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app(contents: &str) -> (tempfile::TempDir, std::path::PathBuf, egui::Context, SettingsApp) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, contents).unwrap();
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        theme::apply(&ctx, false);
        let mut app = SettingsApp::new(&ctx, path.clone());
        app.tray_enabled = false;
        (dir, path, ctx, app)
    }

    fn render_frames(ctx: &egui::Context, app: &mut SettingsApp, count: usize) {
        for _ in 0..count {
            let _ = ctx.run(egui::RawInput::default(), |ctx| app.render(ctx));
        }
    }

    #[test]
    fn renders_import_preview_dialog() {
        let (_dir, _path, ctx, mut app) = test_app(r#"{"input_method":"telex","macros":{"ko":"không"}}"#);
        let imported = r#"{"input_method":"vni","macros":{"ko":"không","vn":"Việt Nam"}}"#;
        let preview = app.doc.preview_import(imported).unwrap();
        let source = PathBuf::from("/tmp/import.json");
        app.import_preview = Some((source.clone(), preview));
        app.confirm = Some(Confirm::Import(source));
        render_frames(&ctx, &mut app, 2);
    }

    #[test]
    fn app_editor_applies_profile_immediately() {
        let (_dir, _path, ctx, mut app) = test_app(r#"{"app_profiles":{"kitty":{"output_mode":"preedit"}}}"#);
        app.select_app("kitty".into());
        let mut editor = app.app_editor.clone().unwrap();
        editor.output_mode = Some(OutputMode::Replace);
        app.app_editor = Some(editor);
        app.apply_app_editor();
        assert_eq!(app.doc.profile("kitty").unwrap().output_mode, Some(OutputMode::Replace));
        render_frames(&ctx, &mut app, 1);
    }

    #[test]
    fn renders_each_tab_and_group_reset_dialog() {
        let (_dir, path, ctx, mut app) = test_app(
            r#"{"input_method":"vni","tone_style":"old","excluded_apps":["minecraft"],"app_profiles":{"kitty":{"output_mode":"preedit"}}}"#,
        );
        for tab in Tab::ALL {
            app.set_tab(tab.name());
            render_frames(&ctx, &mut app, 1);
        }
        app.confirm = Some(Confirm::ResetGroup(SettingsGroup::Apps));
        render_frames(&ctx, &mut app, 2);
        assert!(path.exists());
    }
}
