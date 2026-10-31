# vifastkey-settings

UI cấu hình ViFastKey cho desktop, dùng `eframe`/`egui` 0.32.3.
Windows/Linux là UI chính; binary cũng chạy trên macOS để kiểm thử/portable
(macOS vẫn có preferences AppKit native trong `macos/`).

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
cargo run -p vifastkey-settings -- --screenshot /tmp/settings.png --tab advanced
cargo run -p vifastkey-settings -- --screenshots /tmp/settings-shots
cargo run -p vifastkey-settings -- --tray      # bật tray trên macOS
cargo run -p vifastkey-settings -- --no-tray   # tắt tray
```

`--check` in JSON và exit 1 nếu có lỗi; `--self-test` load config, dựng UI lần lượt
6 tab rồi thoát, dùng cho CI. `--screenshots` chụp framebuffer của cửa sổ ra
`overview.png`, `typing.png`… không cần `screencapture`/X11 screenshot tool.

## Tray

- Windows/macOS: `tray-icon` (Win32 Shell_NotifyIcon / NSStatusItem).
- Linux: StatusNotifierItem thuần D-Bus qua `ksni`; không cần GTK/AppIndicator
  trong binary. Desktop phải có StatusNotifierWatcher; GNOME cần extension
  **AppIndicator and KStatusNotifierItem Support**.
- Chuột phải mở menu (mở cấu hình, tải lại, thoát); click trái mở/đưa cửa sổ lên.
- Đóng cửa sổ (khi không có thay đổi chưa lưu) ẩn xuống tray thay vì thoát;
  tắt trong tab **Nâng cao**.
- macOS mặc định tắt tray; dùng `--tray` khi muốn kiểm thử.

## Config

Đọc/ghi cùng file với adapter:

- macOS: `~/Library/Application Support/ViFastKey/config.json`
- Windows: `%APPDATA%\ViFastKey\config.json`
- Linux: `$XDG_CONFIG_HOME/vifastkey/config.json` hoặc `~/.config/vifastkey/config.json`
- Override: `$VIFASTKEY_CONFIG`

UI chỉ ghi field đã sửa, giữ key lạ, merge từng key cho `macros`/`app_profiles`/
`app_modes`, ghi atomic và tạo `config.json.bak`.

## Cấu trúc

- `src/app.rs` — state, tab, dialog, tray actions.
- `src/tray.rs` — tray Windows/macOS (`tray-icon`) và Linux (`ksni`).
- `src/screenshot.rs` — chụp PNG qua `ViewportCommand::Screenshot`.
- `src/theme.rs` — token từ `design/tokens.json`; đổi theme sáng/tối.
- `src/fonts.rs` — Be Vietnam Pro (OFL).
- `src/preview.rs` — gõ thử bằng `vfk-engine::testing::Harness`.
- `src/platform.rs` — mở thư mục/file, `windows.json`.
- `src/ui_prefs.rs` — theme/tab/tray trong `settings-ui.json`.

Logic đọc/ghi/validate/diff/reset nằm trong `crates/vfk-config`, không phụ thuộc egui.

## Linux build deps

```bash
sudo apt-get install -y pkg-config libxkbcommon-dev libwayland-dev \
  libx11-dev libxi-dev libgl1-mesa-dev libegl1-mesa-dev
```

Tray không cần thêm thư viện hệ thống (D-Bus có sẵn trên desktop Linux).

## Đã có

- Import có **preview diff** (thay đổi hiệu lực + lỗi/cảnh báo) trước khi nạp.
- Export JSON hiệu lực, xem trước JSON ngay trong UI.
- Khôi phục mặc định **từng nhóm**: kiểu gõ, tuỳ chọn macro, ứng dụng, nâng cao.
- Tìm app theo tên; sửa app profile được áp dụng vào bản nháp ngay.
- Tray + ẩn xuống tray khi đóng; lưu theme/tab/tray/bật-ẩn vào `settings-ui.json`.
- Tab Windows backend (`windows.json`).
- macOS IMK, Windows hook/TSF, Fcitx5 và IBus poll mtime để reload config.
- `--screenshots` cho kiểm thử giao diện trên CI/desktop headless.

## Giới hạn hiện tại

- UI tiếng Việt; không có i18n.
- `app_profiles` override hiện thay thế toàn bộ preset của app đó (schema 1).
- Tray Linux phụ thuộc StatusNotifierWatcher của desktop.
- Windows: đã test render 6 tab, self-test, import/export, BOM UTF-8 và close-to-tray
  trên Windows 11; chưa test tương tác menu tray bằng chuột.
- macOS dùng preferences AppKit làm UI chính.
