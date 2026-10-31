# ViFastKey Windows TSF (preedit)

Text service TSF cho Windows: composition/preedit thật (vùng gạch chân), commit
nguyên tử, sửa từ đã gõ qua surrounding text. DLL gọi C ABI `vfk-ffi` nên dùng
chung engine, adapter-core và conformance với các nền tảng khác.

Adapter hook (`rust/apps/vfk-windows`) vẫn là fallback Replace cho app không dùng
TSF. Hai backend không xử lý cùng một context: TSF ghi named mapping
`Local\ViFastKeyTsfState`, hook đọc mapping và pass-through khi PID trùng.

## Build (Windows 10/11)

```powershell
# 1. Engine C ABI (x64)
cd rust
cargo build --release -p vfk-ffi
cd ..

# 2. TSF DLL
cmake -S windows/tsf -B build/tsf -A x64 -DVFK_RUST_PROFILE=release
cmake --build build/tsf --config Release

# 3. Cài per-user + đăng ký TSF profile
powershell -ExecutionPolicy Bypass -File windows/installer/install.ps1 `
  -BuildDir build/tsf
```

`install.ps1` copy `vifastkey_tsf.dll` + `vifastkey.dll` vào
`%LOCALAPPDATA%\ViFastKey\tsf\<arch>` rồi gọi `regsvr32` đúng view (SysWOW64 cho
x86). Chạy PowerShell **không** elevated để đăng ký profile cho user hiện tại.

Bản zip phát hành có `x64\` và `x86\`; cài đúng kiến trúc:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -BuildDir x64
```

### 32-bit

TSF bị nạp vào process của app, nên app 32-bit cần DLL 32-bit:

```powershell
rustup target add i686-pc-windows-msvc
cd rust
cargo build --release -p vfk-ffi --target i686-pc-windows-msvc
cd ..
cmake -S windows/tsf -B build/tsf-x86 -A Win32 `
  -DVFK_RUST_PROFILE=release `
  -DVFK_FFI_LIB_DIR="$PWD/rust/target/i686-pc-windows-msvc/release"
cmake --build build/tsf-x86 --config Release
```

Mỗi lần chạy `install.ps1` sẽ copy DLL vào `tsf\x64` hoặc `tsf\x86` và đăng ký
đúng registry view (SysWOW64 cho x86), nên có thể cài cả hai kiến trúc cho app
64-bit và 32-bit.

## Chọn bộ gõ

1. Settings → Time & language → Language & region → Vietnamese → Language options
   → Add a keyboard → **ViFastKey**.
2. Hoặc dùng language bar (nếu bật) để chọn ViFastKey.
3. Nếu chưa thấy profile: đăng xuất/đăng nhập lại, hoặc kiểm tra
   `regsvr32` đã chạy đúng user.

## Preedit và Replace khi chơi game

- **Preedit (TSF)** cần app có text store/composition của Windows. Notepad, Office,
  trình duyệt, Discord/Slack thường hỗ trợ; nhiều game dùng DirectInput/Raw Input
  hoặc render UI riêng nên không nhận preedit, nhất là fullscreen exclusive.
- **Replace (hook)** bắt phím bằng `WH_KEYBOARD_LL` rồi gửi backspace + Unicode
  bằng `SendInput`. Nó có thể gõ được trong ô chat của một số game không hỗ trợ TSF,
  nhưng không có gạch chân preedit; game dùng anti-cheat có thể chặn hook/SendInput,
  và fullscreen exclusive vẫn có thể nuốt phím.
- Vì vậy mặc định là **Preedit**; chỉ chuyển **Replace** cho app/game cụ thể khi
  preedit không hoạt động. Không bật hook nền ở chế độ Preedit để tránh anti-cheat
  nhìn thấy keyboard hook không cần thiết.
- Linux IBus/IMK/Fcitx5 dùng preedit; Fcitx5/macOS có Replace theo app nhưng cần
  surrounding text và không phải giải pháp cho game.

## Cấu hình

- `%APPDATA%\ViFastKey\config.json`: cấu hình engine (Telex/VNI/VIQR, macro,
  app profile...). Xem `docs/CONFIG.md`.
- UI cấu hình: `vifastkey-settings.exe` (Start Menu shortcut hoặc `Ctrl+Shift+,` khi hook đang chạy).
- `%APPDATA%\ViFastKey\windows.json`:

```json
{
  "backend": "tsf"
}
```

Mẫu: `windows/tsf/windows.example.json`.

| backend | Hành vi |
|---|---|
| `tsf` (mặc định) | Hook luôn pass; chỉ TSF preedit |
| `hook` | TSF không ăn phím; chỉ hook Replace xử lý |

Không cần khởi động lại khi sửa `config.json`: TSF đọc lại khi focus đổi; hook
hot-reload theo mtime. Đổi `windows.json` có hiệu lực ngay với hook; TSF có thể cần
đổi focus hoặc khởi động lại text service.

**Mặc định là Preedit (TSF).** Để dùng Replace cho game/app không nhận TSF, mở
`vifastkey-settings` → tab **Nâng cao** → **Windows: Preedit / Replace** → chọn
**Replace (Hook)**. UI sẽ chạy hook nền và thêm Run key cho user hiện tại; chuyển
lại **Preedit (TSF)** sẽ dừng hook và xoá Run key. Không cần bật `auto`.

## Hành vi

- Preedit = một composition của TSF; gõ `tieengs` → `tiếng`, Space/Enter/Dấu câu
  commit theo `Action` của engine.
- Backspace sửa trong composition; click/focus khác chốt composition, không mất từ.
- Surrounding text lấy từ `ITfRange` trước caret; `Replace` chỉ chạy khi plan xác
  minh được hậu tố.
- Password/PIN/URL/email/số bị tắt qua `ITfInputScope`; terminal nhận field
  `Terminal`.
- Display attribute: gạch chân xanh cho composition.
- Ctrl+Shift+F toggle Việt/Anh (khi TSF đang xử lý context).

## Giới hạn / còn lại

- Chưa test runtime trên Windows thật trong môi trường repo (macOS); CI build MSVC
  chỉ kiểm tra compile. Cần smoke test Notepad/Chrome/Office/Windows Terminal.
- Tray icon nằm trong `vifastkey-settings`; installer đăng ký TSF per-user, cài hook
  và tạo shortcut `ViFastKey Settings`. Auto-start hook do UI quản lý khi chọn Replace.
- Macro hiện chỉ chạy ở `REPLACE` trong engine; TSF `PREEDIT` chưa mở macro.
- Charset legacy chưa áp ở đầu ra TSF (luôn chèn Unicode).
- App elevated: TSF per-user thường không nạp vào process elevated; cần bản HKLM
  hoặc chạy app cùng mức quyền. Hook cũng bị UIPI giới hạn.
- Chưa ký Authenticode; SmartScreen sẽ cảnh báo.
- `windows.json` được hot-reload trong hook; TSF đọc lại khi focus đổi.
- Nếu app không hỗ trợ TSF/không nhận composition: dùng backend `hook`.

## Kiểm thử tay

1. Mở Notepad, chọn ViFastKey, gõ `tieengs` → `tiếng`, Space → commit.
2. Backspace trong lúc preedit, Enter, dấu câu, click chuột giữa từ.
3. Chrome address bar + Google Docs; Office; Windows Terminal.
4. Field password/URL/number: không có composition, phím đi thẳng.
5. Chuyển app qua lại: không mất từ, không nhân đôi chữ.
6. `backend: hook` → chỉ hook chạy; `backend: tsf` (mặc định) → chỉ TSF preedit.
