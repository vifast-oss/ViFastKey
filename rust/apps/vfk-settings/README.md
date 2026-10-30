# vifastkey-settings

UI cấu hình ViFastKey cho Windows/Linux, dùng `eframe`/`egui` 0.32.3.

macOS tiếp tục dùng preferences AppKit trong `macos/Sources/PreferencesWindowController.swift`.

## Chạy

```bash
cd rust
cargo run -p vifastkey-settings
cargo run -p vifastkey-settings -- --config /tmp/config.json
cargo run -p vifastkey-settings -- --tab apps
cargo run -p vifastkey-settings -- --check --config /tmp/config.json
cargo run -p vifastkey-settings -- --self-test --config /tmp/config.json
cargo run -p vifastkey-settings -- --export /tmp/vifastkey-config.json
cargo run -p vifastkey-settings -- --import /tmp/vifastkey-config.json
```

`--check` in JSON và exit 1 nếu có lỗi; `--self-test` load config, dựng UI một frame
không cần cửa sổ, dùng cho CI.

## Config

Đọc/ghi cùng file với adapter:

- macOS: `~/Library/Application Support/ViFastKey/config.json`
- Windows: `%APPDATA%\ViFastKey\config.json`
- Linux: `$XDG_CONFIG_HOME/vifastkey/config.json` hoặc `~/.config/vifastkey/config.json`
- Override: `$VIFASTKEY_CONFIG`

UI chỉ ghi field đã sửa, giữ key lạ, merge từng key cho `macros`/`app_profiles`/
`app_modes`, ghi atomic và tạo `config.json.bak`.

## Cấu trúc

- `src/app.rs` — state, tab, dialog.
- `src/theme.rs` — token từ `design/tokens.json`.
- `src/fonts.rs` — Be Vietnam Pro (OFL).
- `src/preview.rs` — gõ thử bằng `vfk-engine::testing::Harness`.
- `src/platform.rs` — mở thư mục/file.

Logic đọc/ghi/validate nằm trong `crates/vfk-config`, không phụ thuộc egui.

## Linux build deps

```bash
sudo apt-get install -y pkg-config libxkbcommon-dev libwayland-dev \
  libx11-dev libxi-dev libgl1-mesa-dev libegl1-mesa-dev
```

## Đã có

- Import/export JSON qua file dialog; CLI `--import`/`--export`.
- Lưu theme/tab cuối vào `settings-ui.json`.
- Tab Windows backend (`windows.json`).
- macOS IMK và Windows hook poll mtime để reload config.

## Giới hạn hiện tại

- UI tiếng Việt; không có i18n.
- `app_profiles` override hiện thay thế toàn bộ preset của app đó (schema 1).
- Chưa có tray icon; Windows hook dùng `Ctrl+Shift+,`, macOS dùng menu “Cấu hình…”,
  Linux dùng desktop entry.
