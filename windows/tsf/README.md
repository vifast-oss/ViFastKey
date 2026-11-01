> **Legacy / reference only.** Từ bản Windows hook-only, TSF không còn được
> build/đóng gói/cài mặc định. Giữ source để tham khảo hoặc cho một bản preedit
> riêng sau này. Xem `docs/INSTALL.md` cho bản Windows hiện tại.

# ViFastKey Windows TSF (preedit)

Text service TSF cho Windows: composition/preedit thật (vùng gạch chân), commit
nguyên tử, sửa từ đã gõ qua surrounding text. DLL gọi C ABI `vfk-ffi` nên dùng
chung engine, adapter-core và conformance với các nền tảng khác.

Adapter hook (`rust/apps/vfk-windows-replace`) vẫn là fallback Replace cho app không dùng
TSF. Hai backend không xử lý cùng một context: TSF ghi named mapping
`Local\ViFastKeyTsfState`, hook đọc mapping và pass-through khi PID trùng.

## Core dùng chung và phần TSF-specific

- **Dùng chung** qua `vfk-ffi` + `vfk-engine` + `vfk-adapter-core`: action/plan
  preedit-commit-replace, kiểm tra surrounding, quy đổi UTF-16, timing và
  conformance. Android/JNI, macOS, Linux và Windows đều đi qua phần này.
- **TSF-specific**, nằm trong `text_service.cpp`, không đưa vào core vì phụ thuộc
  `ITfContext`/`ITfRange`: `ActivateEx`/`EnsureContext` khi đổi IME, giữ caret sau
  `EndComposition`, throttle reload config và routing Preedit/Replace. Các port
  khác có primitive tương đương nhưng không dùng chung code COM.

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
  Chrome/Edge, Discord/Slack, VS Code thường hỗ trợ; nhiều game dùng DirectInput/
  Raw Input hoặc render UI riêng nên không nhận preedit, nhất là fullscreen exclusive.
- **Windows Terminal/OpenConsole không nạp third-party TSF per-user chưa ký**:
  Microsoft Vietnamese/Pinyin (TIP hệ thống, đã ký) vẫn preedit được, nhưng DLL
  `vifastkey_tsf.dll` không xuất hiện trong process `WindowsTerminal`/`OpenConsole`
  (kiểm tra `tasklist /m vifastkey_tsf.dll`). Vì vậy terminal dùng đường hook
  Replace riêng, không cần đổi toàn bộ Windows sang Replace.
- **Replace (hook)** bắt phím bằng `WH_KEYBOARD_LL` rồi gửi backspace + Unicode
  bằng `SendInput`. Nó có thể gõ được trong terminal và ô chat của một số game
  không hỗ trợ TSF, nhưng không có gạch chân preedit; game dùng anti-cheat có thể
  chặn hook/SendInput, và fullscreen exclusive vẫn có thể nuốt phím.
- Hook process chạy nền ở cả Preedit, nhưng **chỉ cài keyboard hook khi foreground
  là terminal/console** (Windows Terminal, OpenConsole, conhost, cmd, PowerShell).
  App thường vẫn do TSF xử lý và không bị hook can thiệp. Chọn `backend: hook` thì
  hook xử lý toàn cục như cũ.
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

Không cần khởi động lại khi sửa `config.json`/`windows.json`: TSF kiểm tra lại mỗi
500 ms khi có phím và khi focus đổi; hook hot-reload theo mtime. Nếu đang có preedit
dở, bấm ra ngoài một lần để chốt trước khi đổi chế độ.

**Mặc định là Preedit (TSF).** Hook process vẫn chạy nền để xử lý riêng
Windows Terminal/console; nó chỉ cài keyboard hook khi terminal focus. Để dùng
Replace cho toàn bộ app/game không nhận TSF, mở `vifastkey-settings` → tab
**Nâng cao** → **Windows: Preedit / Replace** → chọn **Replace (Hook)**. Không
cần bật `auto`.

## Hành vi

- Preedit = một composition của TSF; gõ `tieengs` → `tiếng`, Space/Enter/Dấu câu
  commit theo `Action` của engine.
- Backspace sửa trong composition; click/focus khác chốt composition, không mất từ.
- Surrounding text lấy từ `ITfRange` trước caret; `Replace` chỉ chạy khi plan xác
  minh được hậu tố.
- Password/PIN/URL/email/số bị tắt qua `ITfInputScope`; terminal nhận field
  `Terminal` (Windows Terminal dùng hook vì không nạp TSF per-user).
- Display attribute: gạch chân xanh cho composition.
- `Alt+,` toggle Việt/Anh (khi TSF đang xử lý context).

## Giới hạn / còn lại

- Chưa test runtime trên Windows thật trong môi trường repo (macOS); CI build MSVC
  chỉ kiểm tra compile. Cần smoke test Notepad/Chrome/Office/Windows Terminal.
- Tray icon nằm trong `vifastkey-settings`; installer đăng ký TSF per-user, cài hook
  và tạo shortcut `ViFastKey Settings`. Hook process có Run key riêng; ở Preedit
  nó chỉ cài keyboard hook khi terminal focus.
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
6. `backend: hook` → hook xử lý mọi app; `backend: tsf` (mặc định) → TSF cho app thường, hook chỉ cho terminal/console.
