//! Lớp protocol Wayland: `zwp_input_method_v2` + `zwp_virtual_keyboard_v1`.
//!
//! Không chứa logic tiếng Việt. Mọi quyết định nằm ở [`crate::controller`].

use std::error::Error;
use std::os::fd::{AsFd, OwnedFd};
use std::path::PathBuf;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_keyboard, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2;
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_manager_v2::ZwpInputMethodManagerV2;
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_v2::ZwpInputMethodV2;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1;
use xkbcommon::xkb;

use crate::controller::{default_config_path, Backend, Controller, ForwardKey, KeyInput};
use vfk_engine::Key;

/// `zwp_input_method_v2` chưa có app_id; adapter standalone dùng cấu hình global.
struct WaylandBackend {
    im: ZwpInputMethodV2,
    vk: Option<ZwpVirtualKeyboardV1>,
}

impl Backend for WaylandBackend {
    fn preedit(&mut self, text: &str, cursor_bytes: i32, serial: u32) {
        self.im.set_preedit_string(text.to_string(), cursor_bytes, cursor_bytes);
        self.im.commit(serial);
    }

    fn clear_preedit(&mut self, serial: u32) {
        self.im.set_preedit_string(String::new(), -1, -1);
        self.im.commit(serial);
    }

    fn commit(&mut self, text: &str, serial: u32) {
        self.im.set_preedit_string(String::new(), -1, -1);
        self.im.commit_string(text.to_string());
        self.im.commit(serial);
    }

    fn replace(&mut self, text: &str, delete_bytes: u32, serial: u32) {
        self.im.set_preedit_string(String::new(), -1, -1);
        self.im.delete_surrounding_text(delete_bytes, 0);
        self.im.commit_string(text.to_string());
        self.im.commit(serial);
    }

    fn forward(&mut self, key: &ForwardKey) {
        let Some(vk) = self.vk.as_ref() else {
            log::debug!("forward key {} bỏ qua: compositor không có virtual-keyboard-v1", key.keycode);
            return;
        };
        vk.modifiers(key.depressed, key.latched, key.locked, key.group);
        let state =
            if key.pressed { wl_keyboard::KeyState::Pressed } else { wl_keyboard::KeyState::Released };
        vk.key(key.time, key.keycode, state.into());
    }
}

struct App {
    conn: Connection,
    controller: Controller<WaylandBackend>,
    vk: Option<ZwpVirtualKeyboardV1>,
    keyboard: Option<ZwpInputMethodKeyboardGrabV2>,
    xkb_context: xkb::Context,
    xkb_state: Option<xkb::State>,
    quit: Option<String>,
    debug: bool,
}

impl App {
    fn set_keymap(&mut self, format: WEnum<wl_keyboard::KeymapFormat>, fd: OwnedFd, size: u32) {
        let Ok(format) = format.into_result() else {
            log::warn!("keymap format không hợp lệ");
            return;
        };
        if format != wl_keyboard::KeymapFormat::XkbV1 {
            log::warn!("bỏ qua keymap không phải XKB");
            return;
        }
        let Ok(fd_for_xkb) = fd.try_clone() else {
            log::warn!("không clone được keymap fd");
            return;
        };
        // SAFETY: fd là keymap do compositor gửi, size đi kèm.
        let keymap = unsafe {
            xkb::Keymap::new_from_fd(
                &self.xkb_context,
                fd_for_xkb,
                size as usize,
                xkb::KEYMAP_FORMAT_TEXT_V1,
                xkb::KEYMAP_COMPILE_NO_FLAGS,
            )
        };
        match keymap {
            Ok(Some(keymap)) => {
                self.xkb_state = Some(xkb::State::new(&keymap));
                if let Some(vk) = self.vk.as_ref() {
                    vk.keymap(format.into(), fd.as_fd(), size);
                }
                log::debug!("đã nạp keymap {} byte", size);
            }
            Ok(None) => log::warn!("keymap rỗng/không compile được"),
            Err(error) => log::warn!("không đọc được keymap: {error}"),
        }
    }

    fn translate_key(&self, keycode: u32, time: u32, pressed: bool) -> Option<KeyInput> {
        let state = self.xkb_state.as_ref()?;
        let code = keycode + 8;
        let sym = state.key_get_one_sym(code.into());
        let key = if let Some(key) = special_key(sym) {
            key
        } else {
            let text = state.key_get_utf8(code.into());
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if !c.is_control() => Key::Char(c),
                _ => Key::Other,
            }
        };
        let ctrl = state.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE);
        let alt = state.mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE);
        let meta = state.mod_name_is_active(xkb::MOD_NAME_LOGO, xkb::STATE_MODS_EFFECTIVE);
        Some(KeyInput { key, keycode, time, pressed, ctrl, alt, meta })
    }

    fn handle_key(&mut self, keycode: u32, time: u32, pressed: bool) {
        let Some(input) = self.translate_key(keycode, time, pressed) else {
            log::debug!("bỏ qua key {keycode}: chưa có keymap");
            return;
        };
        if self.debug {
            log::debug!(
                "key code={} {:?} pressed={} ctrl={} alt={} meta={}",
                keycode,
                input.key,
                pressed,
                input.ctrl,
                input.alt,
                input.meta
            );
        }
        self.controller.on_key(&input);
        if let Err(error) = self.conn.flush() {
            log::error!("flush Wayland lỗi: {error}");
            self.quit = Some("không flush được Wayland".into());
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for App {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpInputMethodManagerV2, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpInputMethodManagerV2,
        _event: <ZwpInputMethodManagerV2 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpVirtualKeyboardManagerV1,
        _event: <ZwpVirtualKeyboardManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardV1, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpVirtualKeyboardV1,
        _event: <ZwpVirtualKeyboardV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpInputMethodV2, ()> for App {
    fn event(
        state: &mut Self,
        proxy: &ZwpInputMethodV2,
        event: <ZwpInputMethodV2 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_v2::Event;
        match event {
            Event::Activate => {
                log::debug!("input method activate");
                state.controller.on_activate();
                if state.keyboard.is_none() {
                    state.keyboard = Some(proxy.grab_keyboard(qh, ()));
                }
            }
            Event::Deactivate => {
                log::debug!("input method deactivate");
                state.controller.on_deactivate();
                state.keyboard = None;
            }
            Event::SurroundingText { text, cursor, anchor } => {
                state.controller.on_surrounding(text, cursor, anchor);
            }
            Event::ContentType { hint, purpose } => {
                // text-input-v3 hint bit 7 = sensitive; coi như password.
                let hint: u32 = hint.into();
                let purpose: u32 = purpose.into();
                let sensitive = hint & (1 << 7) != 0;
                state.controller.on_content_type(if sensitive { 8 } else { purpose });
            }
            Event::Done => state.controller.on_done(),
            Event::Unavailable => {
                log::error!("compositor không cung cấp input-method-v2 cho seat này");
                state.quit = Some("input-method-v2 unavailable".into());
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwpInputMethodKeyboardGrabV2, ()> for App {
    fn event(
        state: &mut Self,
        _proxy: &ZwpInputMethodKeyboardGrabV2,
        event: <ZwpInputMethodKeyboardGrabV2 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_keyboard_grab_v2::Event;
        match event {
            Event::Keymap { format, fd, size } => state.set_keymap(format, fd, size),
            Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } => {
                state.controller.on_modifiers(mods_depressed, mods_latched, mods_locked, group);
                if let Some(xkb_state) = state.xkb_state.as_mut() {
                    xkb_state.update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                }
            }
            Event::Key { time, key, state: key_state, .. } => {
                let pressed = matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed));
                state.handle_key(key, time, pressed);
            }
            Event::RepeatInfo { .. } => {}
            _ => {}
        }
    }
}

#[allow(non_upper_case_globals)]
fn special_key(sym: xkb::Keysym) -> Option<Key> {
    use xkb::keysyms::*;
    Some(match sym.raw() {
        KEY_BackSpace => Key::Backspace,
        KEY_Return | KEY_KP_Enter => Key::Enter,
        KEY_Escape => Key::Escape,
        KEY_Tab => Key::Tab,
        KEY_Left => Key::Left,
        KEY_Right => Key::Right,
        KEY_Up => Key::Up,
        KEY_Down => Key::Down,
        KEY_Home => Key::Home,
        KEY_End => Key::End,
        KEY_Delete => Key::Delete,
        KEY_Page_Up => Key::PageUp,
        KEY_Page_Down => Key::PageDown,
        KEY_Shift_L | KEY_Shift_R | KEY_Control_L | KEY_Control_R | KEY_Alt_L | KEY_Alt_R | KEY_Meta_L
        | KEY_Meta_R | KEY_Super_L | KEY_Super_R | KEY_Caps_Lock => Key::Modifier,
        _ => return None,
    })
}

struct Args {
    config: PathBuf,
    debug: bool,
    check: bool,
    probe: bool,
}

fn parse_args() -> Args {
    let mut args = Args { config: default_config_path(), debug: false, check: false, probe: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--config" => {
                if let Some(path) = it.next() {
                    args.config = PathBuf::from(path);
                }
            }
            "--debug" => args.debug = true,
            "--check" => args.check = true,
            "--probe" => args.probe = true,
            "--version" => {
                println!("vifastkey-wayland {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                println!(
                    "vifastkey-wayland [--config PATH] [--debug] [--check] [--probe]\n\
                     \n\
                     Adapter Wayland standalone cho ViFastKey.\n\
                     Cần compositor hỗ trợ zwp_input_method_v2 + zwp_virtual_keyboard_v1\n\
                     (Sway, Hyprland, niri, wlroots khác)."
                );
                std::process::exit(0);
            }
            other => {
                eprintln!("tham số không hợp lệ: {other}");
                std::process::exit(2);
            }
        }
    }
    args
}

pub fn run() {
    let args = parse_args();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(if args.debug {
        "debug"
    } else {
        "info"
    }))
    .init();

    if args.check {
        match std::fs::read_to_string(&args.config) {
            Ok(json) => match serde_json::from_str::<vfk_engine::Config>(&json) {
                Ok(_) => println!("config OK: {}", args.config.display()),
                Err(error) => {
                    eprintln!("config lỗi {}: {error}", args.config.display());
                    std::process::exit(1);
                }
            },
            Err(error) => {
                // File thiếu là hợp lệ: adapter dùng default.
                println!("config chưa có ({}): dùng default ({error})", args.config.display());
            }
        }
        return;
    }

    if args.probe {
        match probe() {
            Ok(()) => return,
            Err(error) => {
                eprintln!("probe lỗi: {error}");
                std::process::exit(1);
            }
        }
    }

    if let Err(error) = run_inner(args.config, args.debug) {
        eprintln!("vifastkey-wayland lỗi: {error}");
        std::process::exit(1);
    }
}

fn probe() -> Result<(), Box<dyn Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, _queue) = registry_queue_init::<Probe>(&conn)?;
    let list = globals.contents().clone_list();
    let has = |name: &str| list.iter().any(|g| g.interface == name);
    println!("WAYLAND_DISPLAY={}", std::env::var("WAYLAND_DISPLAY").unwrap_or_default());
    println!("zwp_input_method_manager_v2: {}", has("zwp_input_method_manager_v2"));
    println!("zwp_virtual_keyboard_manager_v1: {}", has("zwp_virtual_keyboard_manager_v1"));
    println!("wl_seat: {}", has("wl_seat"));
    if !has("zwp_input_method_manager_v2") {
        return Err("compositor không hỗ trợ zwp_input_method_manager_v2".into());
    }
    Ok(())
}

struct Probe;
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Probe {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

fn run_inner(config: PathBuf, debug: bool) -> Result<(), Box<dyn Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<App>(&conn)?;
    let qh = queue.handle();

    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ())?;
    let im_manager: ZwpInputMethodManagerV2 = globals.bind(&qh, 1..=1, ())?;
    let vk_manager: Option<ZwpVirtualKeyboardManagerV1> = globals.bind(&qh, 1..=1, ()).ok();
    if vk_manager.is_none() {
        log::warn!(
            "compositor không có zwp_virtual_keyboard_manager_v1: \
             phím pass-through/Backspace sẽ không tới app"
        );
    }

    let im = im_manager.get_input_method(&seat, &qh, ());
    let vk = vk_manager.as_ref().map(|manager| manager.create_virtual_keyboard(&seat, &qh, ()));

    let backend = WaylandBackend { im: im.clone(), vk: vk.clone() };
    let controller = Controller::new(backend, &config);

    let mut app = App {
        conn: conn.clone(),
        controller,
        vk,
        keyboard: None,
        xkb_context: xkb::Context::new(xkb::CONTEXT_NO_DEFAULT_INCLUDES),
        xkb_state: None,
        quit: None,
        debug,
    };

    log::info!("vifastkey-wayland đang chạy (config {})", config.display());
    while app.quit.is_none() {
        if let Err(error) = queue.blocking_dispatch(&mut app) {
            return Err(Box::new(error));
        }
    }
    let reason = app.quit.unwrap_or_else(|| "stopped".into());
    Err(reason.into())
}
