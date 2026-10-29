# ViFastKey Fcitx5 (Linux)

Addon C++ mỏng dùng C ABI Rust, phù hợp Fcitx5. Logic tiếng Việt nằm trong
`rust/crates/vfk-ffi`; addon chỉ chuyển `KeyEvent`/`Action` và preedit.

## Build (Ubuntu/Debian)

```bash
sudo apt install cmake pkg-config libfcitx5core-dev libfcitx5utils-dev
./linux/fcitx5/build.sh
sudo cmake --install linux/fcitx5/build
fcitx5 -r
```

Thêm **ViFastKey** trong Fcitx5 Configuration → Input Method.

## Hành vi

- PREEDIT mặc định, phù hợp terminal/Chromium/Electron.
- Config `~/.config/vifastkey/config.json` (cùng schema với macOS/IBus).
- Sửa từ trước con trỏ dùng `Replace` + surrounding text (nếu app hỗ trợ).
- `Ctrl+Shift+F` toggle Việt/Anh.
- Chưa có preferences UI riêng; sửa JSON rồi restart Fcitx5.

## Kiểm thử

CI Ubuntu cài `libfcitx5core-dev`, build Rust staticlib và biên dịch addon.
Chưa chạy GUI Fcitx5 trong CI; cần test runtime trên desktop Linux thật.

## Giới hạn

- `delete_utf16` của C ABI được dùng trực tiếp; với chữ Việt BMP tương đương số
  code point của Fcitx5, nhưng emoji/astral có thể lệch.
- Chưa có tray/status actions/cấu hình UI.
- Fcitx5 có thể cache addon; sau khi cài chạy `fcitx5 -r` hoặc logout/login.
