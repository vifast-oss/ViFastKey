# ViFastKey Windows Replace hook

Binary phát hành: **`vifastkey-replace.exe`** (crate/package nội bộ: `vfk-windows-replace`).

Adapter Windows dùng **WH_KEYBOARD_LL + SendInput Unicode**. Engine là
`vfk-engine` (Rust), không phụ thuộc Python. Đây là backend **không có preedit**:
dùng cho app/game không hỗ trợ TSF, hoặc khi người dùng chọn `backend: hook`.

Preedit thật nằm ở [`windows/tsf`](../../windows/tsf/README.md) (TSF text service).
Hai backend dùng chung engine; TSF ghi named mapping `Local\ViFastKeyTsfState`,
hook đọc mapping và pass-through khi foreground window/PID trùng để tránh double input.

## Build

```powershell
cd rust
cargo build -p vfk-windows-replace --release
# -> target\release\vifastkey-replace.exe
```

Yêu cầu Windows 10/11 x64, Rust stable. Chạy exe trong desktop session; hook là global.

## Hành vi

- Gõ Telex/VNI/… theo `%APPDATA%\ViFastKey\config.json`; mặc định Telex.
- Backspace/Enter/Tab/Esc/arrow được map sang engine.
- Action REPLACE: gửi backspace + text Unicode qua `SendInput`.
- Bỏ qua phím injected (`LLKHF_INJECTED`) để không xử lý lại chính SendInput.
- Ctrl/Alt/Win được pass; `Ctrl+Shift+F` toggle Việt/Anh khi hook đang xử lý context.
- `Ctrl+Shift+,` mở `vifastkey-settings.exe --tab apps`.
- Hook đọc và hot-reload `config.json` + `windows.json` theo mtime, không cần restart.
- Password/secure field và app-specific delay chưa xử lý; TSF là hướng production.

## Chọn backend

`%APPDATA%\ViFastKey\windows.json`:

```json
{ "backend": "tsf" }
```

- `tsf` (mặc định): hook luôn pass-through; chỉ TSF preedit.
- `hook`: chỉ hook xử lý; TSF không ăn phím.

UI `vifastkey-settings` → **Nâng cao** → **Windows: Preedit / Replace** là cách
chọn an toàn: chọn Replace sẽ chạy hook nền + thêm Run key; chọn lại Preedit sẽ
dừng hook + xoá Run key. Giá trị `auto` cũ chỉ còn để tương thích file cũ và được
UI đọc như `tsf`.

## Giới hạn / roadmap

- Không có TSF composition; đây là backend Replace.
- Chưa có tray icon riêng; UI quản lý chạy/Run key của hook khi chọn Replace.
- Chưa có auto-update, app exclusion runtime.
- Hook có thể bị một số app/game/anti-cheat chặn.
- Chưa test trên Windows thật trong môi trường macOS; cross-check bằng
  `cargo check --target x86_64-pc-windows-msvc`.
- Khi lên production: TSF cho app hiện đại + hook fallback, signing, auto-update.
