#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod fonts;
mod hook;
mod hook_state;
mod platform;
mod preview;
mod screenshot;
mod theme;
mod toast;
mod tray;
mod ui_prefs;

use std::path::PathBuf;
use std::process::ExitCode;

use vfk_config::{ConfigDocument, Severity};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("vifastkey-settings: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, String> {
    if std::env::args().any(|arg| arg == "--hook") {
        #[cfg(target_os = "windows")]
        {
            hook::run();
            return Ok(ExitCode::SUCCESS);
        }
        #[cfg(not(target_os = "windows"))]
        return Err("--hook chỉ dùng trên Windows".into());
    }
    let args = Args::parse()?;
    if args.help {
        println!("{}", Args::HELP);
        return Ok(ExitCode::SUCCESS);
    }

    let path = args.config.clone().unwrap_or_else(vfk_config::paths::config_path);

    if args.check {
        return check(&path);
    }
    if args.self_test {
        return self_test(&path);
    }
    if let Some(export) = &args.export {
        return export_config(&path, export);
    }
    if let Some(import) = &args.import {
        return import_config(&path, import);
    }

    if !platform::ensure_ui_instance() {
        return Ok(ExitCode::SUCCESS);
    }

    let inner_size = args.window_size.unwrap_or([1040.0, 720.0]);
    let mut viewport =
        egui::ViewportBuilder::default().with_inner_size(inner_size).with_min_inner_size([900.0, 600.0]);
    if let Some(icon) = tray::icon_data() {
        viewport = viewport.with_icon(icon);
    }
    // Windows: cửa sổ cấu hình là app tray, không chiếm taskbar/Alt+Tab.
    // Tray "Mở cấu hình" vẫn ShowWindow/SetForegroundWindow bình thường.
    #[cfg(target_os = "windows")]
    {
        viewport = viewport.with_taskbar(false);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };

    let tab = args.tab.clone();
    let tray = args.tray;
    let hidden = args.hidden;
    let screenshot = match (&args.screenshot, &args.screenshots) {
        (Some(path), None) => {
            let tab = tab.as_deref().and_then(app::Tab::from_name).unwrap_or(app::Tab::Overview);
            Some(screenshot::Plan::single(path.clone(), tab))
        }
        (None, Some(dir)) => Some(screenshot::Plan::all(dir, app::Tab::ALL)),
        _ => None,
    };
    let screenshot_count = screenshot.as_ref().map(screenshot::Plan::count).unwrap_or(0);

    eframe::run_native(
        app::window_title(),
        options,
        Box::new(move |cc| {
            fonts::install(&cc.egui_ctx);
            theme::apply(&cc.egui_ctx, cc.egui_ctx.style().visuals.dark_mode);
            let mut app = app::SettingsApp::new(&cc.egui_ctx, path);
            app.start_hook();
            if let Some(tab) = tab.as_deref() {
                app.set_tab(tab);
            }
            app.set_tray_override(tray);
            app.set_start_hidden(hidden);
            app.set_screenshot(screenshot);
            Ok(Box::new(app))
        }),
    )
    .map_err(|err| format!("Không mở được cửa sổ: {err}"))?;

    if screenshot_count > 0 {
        println!("Đã chụp {screenshot_count} ảnh.");
    }
    Ok(ExitCode::SUCCESS)
}

fn check(path: &std::path::Path) -> Result<ExitCode, String> {
    match ConfigDocument::load(path) {
        Ok(doc) => {
            let issues = doc.validate();
            let errors = issues.iter().filter(|i| i.severity == Severity::Error).count();
            let warnings = issues.iter().filter(|i| i.severity == Severity::Warning).count();
            let report = serde_json::json!({
                "ok": errors == 0,
                "path": path.display().to_string(),
                "schema_version": doc.config().schema_version,
                "errors": errors,
                "warnings": warnings,
                "issues": issues.iter().map(|i| serde_json::json!({
                    "severity": match i.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                        Severity::Info => "info",
                    },
                    "field": i.field.map(|f| f.as_key()),
                    "message": i.message,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            Ok(if errors == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE })
        }
        Err(err) => {
            let report = serde_json::json!({
                "ok": false,
                "path": path.display().to_string(),
                "error": err.to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            Ok(ExitCode::FAILURE)
        }
    }
}

fn export_config(config_path: &std::path::Path, out: &std::path::Path) -> Result<ExitCode, String> {
    let doc = ConfigDocument::load(config_path).map_err(|err| err.to_string())?;
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    std::fs::write(out, doc.pretty_effective_json()).map_err(|err| err.to_string())?;
    println!("Đã xuất cấu hình ra {}", out.display());
    Ok(ExitCode::SUCCESS)
}

fn import_config(config_path: &std::path::Path, input: &std::path::Path) -> Result<ExitCode, String> {
    let mut doc = ConfigDocument::load(config_path).map_err(|err| err.to_string())?;
    let text = std::fs::read_to_string(input).map_err(|err| err.to_string())?;
    let preview = doc.preview_import(&text).map_err(|err| err.to_string())?;
    let errors: Vec<_> = preview.issues.iter().filter(|issue| issue.severity == Severity::Error).collect();
    if !errors.is_empty() {
        return Err(format!(
            "Config nhập có lỗi: {}",
            errors.iter().map(|issue| issue.message.as_str()).collect::<Vec<_>>().join("; ")
        ));
    }
    let changes = preview.change_count();
    doc.apply_import(preview).map_err(|err| err.to_string())?;
    match doc.save().map_err(|err| err.to_string())? {
        Some(outcome) => println!(
            "Đã nhập {} thay đổi từ {} vào {} ({} bytes)",
            changes,
            input.display(),
            outcome.path.display(),
            outcome.bytes
        ),
        None => println!("Không có thay đổi"),
    }
    Ok(ExitCode::SUCCESS)
}

fn self_test(path: &std::path::Path) -> Result<ExitCode, String> {
    let doc = match ConfigDocument::load(path) {
        Ok(doc) => doc,
        Err(err) => {
            println!("SELFTEST FAIL config: {err}");
            return Ok(ExitCode::FAILURE);
        }
    };
    let issues = doc.validate();
    if issues.iter().any(|i| i.severity == Severity::Error) {
        println!("SELFTEST FAIL validate:");
        for issue in issues.iter().filter(|i| i.severity == Severity::Error) {
            println!("  - {}", issue.message);
        }
        return Ok(ExitCode::FAILURE);
    }

    let ctx = egui::Context::default();
    fonts::install(&ctx);
    theme::apply(&ctx, false);
    let mut app = app::SettingsApp::new(&ctx, path.to_path_buf());
    app.set_tray_override(Some(false));
    for tab in app::Tab::ALL {
        app.set_tab(tab.name());
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| app.render(ctx));
        }
    }

    println!(
        "SELFTEST OK path={} schema={} tabs={}",
        path.display(),
        doc.config().schema_version,
        app::Tab::ALL.len()
    );
    Ok(ExitCode::SUCCESS)
}

fn parse_window_size(value: &str) -> Result<[f32; 2], String> {
    let (width, height) = value
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("--window-size phải có dạng WxH, nhận “{value}”"))?;
    let width: f32 = width.parse().map_err(|_| format!("chiều rộng không hợp lệ: {width}"))?;
    let height: f32 = height.parse().map_err(|_| format!("chiều cao không hợp lệ: {height}"))?;
    if !(900.0..=4096.0).contains(&width) || !(600.0..=2160.0).contains(&height) {
        return Err("--window-size ngoài khoảng 900..4096 x 600..2160".into());
    }
    Ok([width, height])
}

struct Args {
    config: Option<PathBuf>,
    tab: Option<String>,
    import: Option<PathBuf>,
    export: Option<PathBuf>,
    screenshot: Option<PathBuf>,
    screenshots: Option<PathBuf>,
    window_size: Option<[f32; 2]>,
    tray: Option<bool>,
    hidden: bool,
    check: bool,
    self_test: bool,
    help: bool,
}

impl Args {
    const HELP: &'static str = "\
vifastkey-settings — UI cấu hình ViFastKey

  vifastkey-settings                    mở UI
  vifastkey-settings --config <path>    dùng file config khác
  vifastkey-settings --tab <name>       mở tab: overview|typing|macros|apps|advanced|diagnostics
  vifastkey-settings --check            kiểm tra config, in JSON, exit 1 nếu lỗi
  vifastkey-settings --self-test        load config + dựng UI tất cả tab, headless
  vifastkey-settings --import <path>    nhập JSON vào config hiện tại (có backup)
  vifastkey-settings --export <path>    xuất config hiệu lực ra JSON
  vifastkey-settings --screenshot <png> chụp tab hiện tại (--tab) rồi thoát
  vifastkey-settings --screenshots <dir> chụp lần lượt 6 tab rồi thoát
  vifastkey-settings --window-size <WxH> kích thước cửa sổ, mặc định 1040x720
  vifastkey-settings --tray             bật/tắt tray: --tray hoặc --no-tray
  vifastkey-settings --hidden            chạy ẩn vào tray (dùng khi autostart)
  vifastkey-settings --hook              chạy hook nền (nội bộ)
  vifastkey-settings --help

Mặc định: $VIFASTKEY_CONFIG hoặc path theo nền tảng.
";

    fn parse() -> Result<Self, String> {
        let mut args = Args {
            config: None,
            tab: None,
            import: None,
            export: None,
            screenshot: None,
            screenshots: None,
            window_size: None,
            tray: None,
            hidden: false,
            check: false,
            self_test: false,
            help: false,
        };
        let mut iter = std::env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--help" | "-h" => args.help = true,
                "--check" => args.check = true,
                "--self-test" => args.self_test = true,
                "--config" => {
                    let path = iter.next().ok_or("--config cần đường dẫn")?;
                    args.config = Some(PathBuf::from(path));
                }
                "--tab" => {
                    let tab = iter.next().ok_or("--tab cần tên tab")?;
                    args.tab = Some(tab);
                }
                "--import" => {
                    args.import = Some(PathBuf::from(iter.next().ok_or("--import cần đường dẫn")?));
                }
                "--export" => {
                    args.export = Some(PathBuf::from(iter.next().ok_or("--export cần đường dẫn")?));
                }
                "--screenshot" => {
                    args.screenshot =
                        Some(PathBuf::from(iter.next().ok_or("--screenshot cần đường dẫn PNG")?));
                }
                "--screenshots" => {
                    args.screenshots = Some(PathBuf::from(iter.next().ok_or("--screenshots cần thư mục")?));
                }
                "--window-size" => {
                    let size = iter.next().ok_or("--window-size cần dạng WxH")?;
                    args.window_size = Some(parse_window_size(&size)?);
                }
                "--tray" => args.tray = Some(true),
                "--no-tray" => args.tray = Some(false),
                "--hidden" => args.hidden = true,
                other if other.starts_with("--config=") => {
                    args.config = Some(PathBuf::from(&other["--config=".len()..]));
                }
                other if other.starts_with("--tab=") => {
                    args.tab = Some(other["--tab=".len()..].to_string());
                }
                other if other.starts_with("--import=") => {
                    args.import = Some(PathBuf::from(&other["--import=".len()..]));
                }
                other if other.starts_with("--export=") => {
                    args.export = Some(PathBuf::from(&other["--export=".len()..]));
                }
                other if other.starts_with("--screenshot=") => {
                    args.screenshot = Some(PathBuf::from(&other["--screenshot=".len()..]));
                }
                other if other.starts_with("--screenshots=") => {
                    args.screenshots = Some(PathBuf::from(&other["--screenshots=".len()..]));
                }
                other if other.starts_with("--window-size=") => {
                    args.window_size = Some(parse_window_size(&other["--window-size=".len()..])?);
                }
                other => return Err(format!("tham số không hỗ trợ: {other}")),
            }
        }
        if args.check && args.self_test {
            return Err("--check và --self-test không dùng cùng lúc".into());
        }
        if args.import.is_some() && args.export.is_some() {
            return Err("--import và --export không dùng cùng lúc".into());
        }
        if args.screenshot.is_some() && args.screenshots.is_some() {
            return Err("--screenshot và --screenshots không dùng cùng lúc".into());
        }
        let headless = args.check || args.self_test || args.import.is_some() || args.export.is_some();
        if (args.screenshot.is_some() || args.screenshots.is_some()) && headless {
            return Err("--screenshot/--screenshots không dùng cùng chế độ headless".into());
        }
        if headless && args.tray.is_some() {
            return Err("--tray chỉ dùng khi mở UI".into());
        }
        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_headless_without_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"input_method":"vni","macros":{"ko":"không"}}"#).unwrap();

        let ctx = egui::Context::default();
        super::fonts::install(&ctx);
        super::theme::apply(&ctx, false);
        let mut app = super::app::SettingsApp::new(&ctx, path);
        app.set_tray_override(Some(false));
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.render(ctx));
    }

    #[test]
    fn parses_window_size() {
        assert_eq!(parse_window_size("1040x720").unwrap(), [1040.0, 720.0]);
        assert_eq!(parse_window_size("1200X900").unwrap(), [1200.0, 900.0]);
        assert!(parse_window_size("100x100").is_err());
        assert!(parse_window_size("abc").is_err());
    }
}
