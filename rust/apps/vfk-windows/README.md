# ViFastKey Windows hook (Replace fallback)

Adapter Windows dùng **WH_KEYBOARD_LL + SendInput Unicode**. Engine là
`vfk-engine` (Rust), không phụ thuộc Python. Đây là backend **không có preedit**:
dùng cho app/game không hỗ trợ TSF, hoặc khi người dùng chọn `backend: hook`.

Preedit thật nằm ở [`windows/tsf`](../../windows/tsf/README.md) (TSF text service).
Hai backend dùng chung engine; TSF ghi named mapping `Local\ViFastKeyTsfState`,
hook đọc mapping và pass-through khi foreground window/PID trùng để tránh double input.

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
- Bỏ qua phím injected (`LLKHF_INJECTED`) để không xử lý lại chính SendInput.
- Ctrl/Alt/Win được pass; `Ctrl+Shift+F` toggle Việt/Anh khi hook đang xử lý context.
- Không có tray/GUI; config sửa tay rồi khởi động lại.
- Password/secure field và app-specific delay chưa xử lý; TSF là hướng production.

## Chọn backend

`%APPDATA%\ViFastKey\windows.json`:

```json
{ "backend": "auto" }
```

- `auto`: hook xử lý app thường; pass-through khi TSF đang active trong foreground process.
- `tsf`: hook luôn pass-through; chỉ TSF xử lý.
- `hook`: chỉ hook xử lý; TSF không ăn phím.

## Giới hạn / roadmap

- Không có TSF composition; đây là backend Replace.
- Chưa có tray, auto-start, auto-update, app exclusion runtime.
- Hook có thể bị một số app/game/anti-cheat chặn.
- Chưa test trên Windows thật trong môi trường macOS; cross-check bằng
  `cargo check --target x86_64-pc-windows-msvc`.
- Khi lên production: TSF cho app hiện đại + hook fallback, config/GUI + signing.
