//! Tray icon cho Windows/Linux (và macOS khi bật `--tray`).
//!
//! Windows/macOS dùng `tray-icon`; Linux dùng StatusNotifierItem thuần D-Bus
//! (`ksni`) để không kéo thêm GTK/AppIndicator vào binary. Mọi lỗi khởi tạo
//! được trả về UI, không làm app thoát.

use std::io::Cursor;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use egui::Context;

const TRAY_ICON_PNG: &[u8] = include_bytes!("../../../../resources/brand/favicon-32.png");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    Reload,
    Quit,
}

/// Kênh dùng chung cho cả vòng đời app: tắt/bật lại tray không làm mất handler
/// đã đăng ký với `tray-icon`/`ksni`.
fn channel() -> &'static (Sender<TrayAction>, Mutex<Receiver<TrayAction>>) {
    static CHANNEL: OnceLock<(Sender<TrayAction>, Mutex<Receiver<TrayAction>>)> = OnceLock::new();
    CHANNEL.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        (tx, Mutex::new(rx))
    })
}

fn sender() -> Sender<TrayAction> {
    channel().0.clone()
}

pub struct Tray {
    #[cfg(target_os = "linux")]
    _handle: Option<ksni::blocking::Handle<imp::VfkTray>>,
    #[cfg(not(target_os = "linux"))]
    _icon: Option<tray_icon::TrayIcon>,
    available: bool,
    error: Option<String>,
}

impl Tray {
    /// Bắt đầu tray. Không panic nếu desktop không hỗ trợ.
    pub fn start(ctx: Context) -> Self {
        let started = imp::start(&ctx);
        let (handle, error) = match started {
            Ok(handle) => (Some(handle), None),
            Err(err) => (None, Some(err)),
        };
        Tray {
            #[cfg(target_os = "linux")]
            _handle: handle,
            #[cfg(not(target_os = "linux"))]
            _icon: handle,
            available: error.is_none(),
            error,
        }
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn poll(&self) -> Vec<TrayAction> {
        let (_, rx) = channel();
        let Ok(rx) = rx.lock() else { return Vec::new() };
        let mut actions = Vec::new();
        while let Ok(action) = rx.try_recv() {
            actions.push(action);
        }
        actions
    }
}

fn load_icon_rgba() -> Result<(u32, u32, Vec<u8>), String> {
    let decoder = png::Decoder::new(Cursor::new(TRAY_ICON_PNG));
    let mut reader = decoder.read_info().map_err(|err| format!("không đọc được icon tray: {err}"))?;
    let size = reader.output_buffer_size().ok_or("icon tray quá lớn")?;
    let mut buffer = vec![0; size];
    let info =
        reader.next_frame(&mut buffer).map_err(|err| format!("không giải mã được icon tray: {err}"))?;
    buffer.truncate(info.buffer_size());

    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => {
            buffer.as_chunks::<3>().0.iter().flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255]).collect()
        }
        other => return Err(format!("icon tray phải là RGB/RGBA, gặp {other:?}")),
    };
    Ok((info.width, info.height, rgba))
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;

    pub struct VfkTray {
        tx: Sender<TrayAction>,
        ctx: Context,
        icon: Option<ksni::Icon>,
    }

    impl VfkTray {
        fn send(&self, action: TrayAction) {
            let _ = self.tx.send(action);
            self.ctx.request_repaint();
        }
    }

    impl ksni::Tray for VfkTray {
        fn id(&self) -> String {
            "vifastkey-settings".into()
        }

        fn title(&self) -> String {
            "ViFastKey".into()
        }

        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            self.icon.iter().cloned().collect()
        }

        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip {
                title: "ViFastKey".into(),
                description: "Bộ gõ tiếng Việt".into(),
                ..Default::default()
            }
        }

        fn activate(&mut self, _x: i32, _y: i32) {
            self.send(TrayAction::Show);
        }

        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            use ksni::menu::StandardItem;

            vec![
                StandardItem {
                    label: "Mở cấu hình".into(),
                    activate: Box::new(|this: &mut Self| this.send(TrayAction::Show)),
                    ..Default::default()
                }
                .into(),
                StandardItem {
                    label: "Tải lại cấu hình".into(),
                    activate: Box::new(|this: &mut Self| this.send(TrayAction::Reload)),
                    ..Default::default()
                }
                .into(),
                ksni::MenuItem::Separator,
                StandardItem {
                    label: "Thoát".into(),
                    activate: Box::new(|this: &mut Self| this.send(TrayAction::Quit)),
                    ..Default::default()
                }
                .into(),
            ]
        }
    }

    pub fn start(ctx: &Context) -> Result<ksni::blocking::Handle<VfkTray>, String> {
        use ksni::blocking::TrayMethods;

        let icon = load_icon_rgba().ok().map(|(width, height, rgba)| ksni::Icon {
            width: width as i32,
            height: height as i32,
            // ksni nhận ARGB32 network byte order.
            data: rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[3], p[0], p[1], p[2]]).collect(),
        });
        let tray = VfkTray { tx: sender(), ctx: ctx.clone(), icon };
        tray.spawn().map_err(|err| format!("không mở được tray: {err}"))
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};

    use super::*;

    pub fn start(ctx: &Context) -> Result<tray_icon::TrayIcon, String> {
        let (width, height, rgba) = load_icon_rgba()?;
        let icon = tray_icon::Icon::from_rgba(rgba, width, height)
            .map_err(|err| format!("icon tray không hợp lệ: {err}"))?;

        let menu = Menu::new();
        let open = MenuItem::with_id("open", "Mở cấu hình", true, None);
        let reload = MenuItem::with_id("reload", "Tải lại cấu hình", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit = MenuItem::with_id("quit", "Thoát", true, None);
        menu.append_items(&[&open, &reload, &separator, &quit])
            .map_err(|err| format!("không dựng được menu tray: {err}"))?;

        let tray = tray_icon::TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("ViFastKey — Bộ gõ tiếng Việt")
            .with_icon(icon)
            .build()
            .map_err(|err| format!("không tạo được tray: {err}"))?;

        let menu_tx = sender();
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = match event.id.0.as_str() {
                "open" => Some(TrayAction::Show),
                "reload" => Some(TrayAction::Reload),
                "quit" => Some(TrayAction::Quit),
                _ => None,
            };
            if let Some(action) = action {
                let _ = menu_tx.send(action);
                menu_ctx.request_repaint();
            }
        }));

        let click_tx = sender();
        let click_ctx = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            let show = matches!(
                event,
                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }
                    | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
            );
            if show {
                let _ = click_tx.send(TrayAction::Show);
                click_ctx.request_repaint();
            }
        }));

        Ok(tray)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_asset_decodes_to_rgba() {
        let (width, height, rgba) = load_icon_rgba().unwrap();
        assert_eq!((width, height), (32, 32));
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }
}
