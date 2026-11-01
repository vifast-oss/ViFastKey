//! `vifastkey-wayland` — adapter Wayland standalone cho ViFastKey.
//!
//! Chỉ chạy trên Linux; phần state machine trong [`controller`] được test trên
//! mọi nền tảng.

// Trên macOS/Windows controller chỉ được dùng bởi test; Linux dùng cả binary.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod controller;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
fn main() {
    linux::run();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("vifastkey-wayland chỉ chạy trên Linux (Wayland).");
    std::process::exit(2);
}
