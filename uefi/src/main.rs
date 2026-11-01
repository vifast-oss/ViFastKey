#![no_main]
#![no_std]

extern crate alloc;

mod editor;

#[cfg(not(feature = "selftest"))]
mod font;
#[cfg(not(feature = "selftest"))]
mod logo;
#[cfg(not(feature = "selftest"))]
mod qr;
#[cfg(not(feature = "selftest"))]
mod render;

#[cfg(feature = "selftest")]
mod debugcon;
#[cfg(feature = "selftest")]
mod selftest;

use uefi::prelude::*;

#[cfg(not(feature = "selftest"))]
use editor::{Action, Editor};
#[cfg(not(feature = "selftest"))]
use render::Renderer;
#[cfg(not(feature = "selftest"))]
use uefi::proto::console::gop::GraphicsOutput;
#[cfg(not(feature = "selftest"))]
use uefi::proto::console::text::{Key, ScanCode};

#[cfg(feature = "selftest")]
#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    let ok = selftest::run();
    debugcon::line(if ok { "SELFTEST PASS" } else { "SELFTEST FAIL" });
    if ok {
        Status::SUCCESS
    } else {
        Status::ABORTED
    }
}

#[cfg(not(feature = "selftest"))]
#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    match run() {
        Ok(()) => Status::SUCCESS,
        Err(err) => {
            uefi::println!("vfk-uefi: {:?}", err.status());
            err.status()
        }
    }
}

#[cfg(not(feature = "selftest"))]
fn run() -> uefi::Result<()> {
    let handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
    let gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle)?;
    let mut renderer = Renderer::new(gop)?;

    // The editor owns only platform-independent text; vfk-core does the
    // Vietnamese transformation.
    let mut editor = Editor::new(renderer.cols(), renderer.text_rows());
    renderer.draw(&editor)?;

    system::with_stdin(|input| -> uefi::Result<()> {
        loop {
            // WaitForKey + ReadKeyStroke: the simplest portable UEFI input path.
            let event = input.wait_for_key_event()?;
            boot::wait_for_event(&[event]).discard_errdata()?;
            let Some(key) = input.read_key()? else {
                continue;
            };

            let action = match key {
                Key::Printable(ch) => editor.on_char(char::from(ch)),
                Key::Special(ScanCode::ESCAPE) => editor.on_escape(),
                Key::Special(ScanCode::FUNCTION_1) => editor.on_help(),
                Key::Special(ScanCode::FUNCTION_2) => editor.on_toggle_language(),
                Key::Special(ScanCode::FUNCTION_3) => editor.on_toggle_method(),
                Key::Special(ScanCode::FUNCTION_10) => editor.on_exit(),
                Key::Special(ScanCode::UP) => editor.on_up(),
                Key::Special(ScanCode::DOWN) => editor.on_down(),
                Key::Special(ScanCode::LEFT) => editor.on_left(),
                Key::Special(ScanCode::RIGHT) => editor.on_right(),
                Key::Special(ScanCode::HOME) => editor.on_home(),
                Key::Special(ScanCode::END) => editor.on_end(),
                Key::Special(ScanCode::DELETE) => editor.on_delete(),
                _ => Action::None,
            };

            match action {
                Action::Exit => break,
                Action::Redraw => renderer.draw(&editor)?,
                Action::None => {}
            }
        }
        Ok(())
    })
}
