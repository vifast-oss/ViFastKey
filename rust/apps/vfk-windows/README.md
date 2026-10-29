# ViFastKey Windows (prototype)

Adapter Windows thực nghiệm dùng **WH_KEYBOARD_LL + SendInput Unicode**. Engine là
`vfk-engine` (Rust), không phụ thuộc Python.

## Build

```powershell
cd rust
cargo build -p vfk-windows --release
# -> target\release\vfk-windows.exe
```

Yêu cầu Windows 10/11 x64, Rust stable. Chạy exe trong desktop session; hook là global.

## Hành vi

- Gõ Telex/VNI/… theo `%APPDATA%\ViFastKey\config.json`; mặc định Telex.
- Backspace/Enter/Tab/Esc/arrow được map sang engine.
- Action REPLACE: gửi backspace + text Unicode qua `SendInput`.
- Ctrl/Alt/Win được pass; `Ctrl+Shift+F` toggle Việt/Anh.
- Không có tray/GUI; config sửa tay rồi khởi động lại.
- Password/secure field và app-specific delay chưa xử lý; đây là lý do TSF là hướng production.

## Giới hạn / roadmap

- Chưa có TSF, tray, auto-start, auto-update, app exclusion runtime.
- Hook có thể bị một số app/game/anti-cheat chặn.
- Chưa test trên Windows thật trong môi trường macOS; cross-check bằng
  `cargo check --target x86_64-pc-windows-msvc`.
- Khi lên production: chuyển sang TSF + config/GUI + signing; giữ hook làm fallback.
