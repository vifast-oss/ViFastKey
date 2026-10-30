#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod fonts;
mod platform;
mod preview;
mod theme;
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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 720.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    let tab = args.tab.clone();
    eframe::run_native(
        app::window_title(),
        options,
        Box::new(move |cc| {
            fonts::install(&cc.egui_ctx);
            theme::apply(&cc.egui_ctx, cc.egui_ctx.style().visuals.dark_mode);
            let mut app = app::SettingsApp::new(&cc.egui_ctx, path);
            if let Some(tab) = tab.as_deref() {
                app.set_tab(tab);
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|err| format!("Không mở được cửa sổ: {err}"))?;

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
    doc.import_json(&text).map_err(|err| err.to_string())?;
    let errors: Vec<_> =
        doc.validate().into_iter().filter(|issue| issue.severity == Severity::Error).collect();
    if !errors.is_empty() {
        return Err(format!(
            "Config nhập có lỗi: {}",
            errors.iter().map(|issue| issue.message.as_str()).collect::<Vec<_>>().join("; ")
        ));
    }
    match doc.save().map_err(|err| err.to_string())? {
        Some(outcome) => {
            println!("Đã nhập {} vào {} ({} bytes)", input.display(), outcome.path.display(), outcome.bytes)
        }
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
    let _ = ctx.run(egui::RawInput::default(), |ctx| app.render(ctx));

    println!("SELFTEST OK path={} schema={}", path.display(), doc.config().schema_version);
    Ok(ExitCode::SUCCESS)
}

struct Args {
    config: Option<PathBuf>,
    tab: Option<String>,
    import: Option<PathBuf>,
    export: Option<PathBuf>,
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
  vifastkey-settings --self-test        load config + dựng UI một frame, headless
  vifastkey-settings --import <path>    nhập JSON vào config hiện tại (có backup)
  vifastkey-settings --export <path>    xuất config hiệu lực ra JSON
  vifastkey-settings --help

Mặc định: $VIFASTKEY_CONFIG hoặc path theo nền tảng.
";

    fn parse() -> Result<Self, String> {
        let mut args = Args {
            config: None,
            tab: None,
            import: None,
            export: None,
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
                other => return Err(format!("tham số không hỗ trợ: {other}")),
            }
        }
        if args.check && args.self_test {
            return Err("--check và --self-test không dùng cùng lúc".into());
        }
        if args.import.is_some() && args.export.is_some() {
            return Err("--import và --export không dùng cùng lúc".into());
        }
        if (args.import.is_some() || args.export.is_some()) && (args.check || args.self_test) {
            return Err("--import/--export không dùng cùng --check/--self-test".into());
        }
        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_headless_without_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"input_method":"vni","macros":{"ko":"không"}}"#).unwrap();

        let ctx = egui::Context::default();
        super::fonts::install(&ctx);
        super::theme::apply(&ctx, false);
        let mut app = super::app::SettingsApp::new(&ctx, path);
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.render(ctx));
    }
}
