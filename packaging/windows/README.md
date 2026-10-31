# ViFastKey cho Windows

ViFastKey là bộ gõ tiếng Việt cho Windows 10/11, gồm **TSF preedit** (mặc định)
và **hook Replace** cho app/game không nhận preedit. Gói này chứa bộ cài per-user,
UI cấu hình và hook Replace.

## Tính năng

- Telex, Simple Telex, VNI, VIQR; kiểu đặt dấu cũ/mới.
- Preedit thật bằng TSF: gạch chân, sửa trong composition, commit khi Space/Enter/dấu câu.
- Sửa từ đã gõ, macro nhiều từ, macro số (`mp3`, `k8s`), macro undo.
- Tự khôi phục tiếng Anh, viết hoa đầu câu, bảo vệ chữ HOA.
- App profile: output, delay/ACK, charset, kiểu gõ/chính tả theo app.
- UI cấu hình egui: gõ thử, import/export có preview diff, reset từng nhóm,
  tìm app, theme sáng/tối, tray.
- Mặc định **Preedit**; **Replace** là tùy chọn cho game/app không hỗ trợ TSF.
- Hỗ trợ app 64-bit và 32-bit (TSF x64/x86).

## Trong gói

```text
x64/                    TSF + engine cho app 64-bit
x86/                    TSF + engine cho app 32-bit
vifastkey-settings.exe  UI cấu hình
vfk-windows.exe         hook Replace (chỉ chạy khi chọn Replace)
install.ps1             cài per-user + đăng ký TSF
uninstall.ps1           gỡ cài đặt
windows.example.json    mẫu cấu hình backend
README.md               file này
DEVELOPER.md            tài liệu build/kỹ thuật TSF
LICENSE
THIRD_PARTY_NOTICES.md
```

## Cài đặt

1. Giải nén gói zip ra một thư mục.
2. Mở **PowerShell** tại thư mục đó. Không cần chạy Administrator.
3. Cài cho app 64-bit:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1 -BuildDir x64
```

Nếu còn dùng app 32-bit, cài thêm:

```powershell
.\install.ps1 -BuildDir x86
```

4. Khởi động lại các app đang mở (hoặc đăng xuất/đăng nhập) để Windows nạp DLL TSF mới.
5. Thêm bàn phím:
   **Settings → Time & language → Language & region → Vietnamese → Language options
   → Add a keyboard → ViFastKey**.
   Nếu chưa thấy, đăng xuất/đăng nhập rồi thử lại.

Mặc định sau khi cài là **Preedit (TSF)**. Không có hook nền chạy ở chế độ này.

## Sử dụng

- Chuyển Việt/Anh bằng language bar hoặc `Ctrl+Shift+F` khi ViFastKey đang xử lý.
- Mở cấu hình: Start Menu → **ViFastKey Settings**, hoặc `Ctrl+Shift+,` khi hook đang chạy.
- Tray: chuột phải để mở cấu hình/tải lại/thoát; click trái để mở cửa sổ.
- Đóng cửa sổ cấu hình sẽ ẩn xuống tray (tắt được trong tab **Nâng cao**).

## Preedit và Replace khi chơi game

**Preedit (TSF)** là mặc định:
- Có composition/gạch chân, an toàn với anti-cheat hơn vì không dùng global hook.
- Hợp với Notepad, Office, Chrome/Edge/Firefox, Discord/Slack, VS Code, Windows Terminal.
- Nhiều game dùng DirectInput/Raw Input, UI riêng hoặc fullscreen exclusive nên
  **không nhận TSF**.

**Replace (Hook)**:
- Bắt phím bằng hook và gửi backspace + Unicode bằng `SendInput`; có thể gõ được
  trong ô chat của một số game không hỗ trợ TSF.
- Không có gạch chân preedit; anti-cheat có thể chặn/flag hook hoặc synthetic input;
  fullscreen exclusive vẫn có thể nuốt phím.
- Chỉ bật khi app/game cụ thể không nhận Preedit. Không bật hook nền mặc định.

Cách đổi: **ViFastKey Settings → Nâng cao → Windows: Preedit / Replace**.
- Chọn **Replace (Hook)**: app chạy `vfk-windows.exe` và thêm Run key cho user.
- Chọn lại **Preedit (TSF)**: app dừng hook và xoá Run key.
- Đổi chế độ có hiệu lực trong khoảng 0,5 giây; nếu đang có preedit dở, bấm ra ngoài một lần.

## Cấu hình

- `%APPDATA%\ViFastKey\config.json` — cấu hình engine dùng chung.
- `%APPDATA%\ViFastKey\windows.json` — `{"backend":"tsf"}` (Preedit) hoặc `{"backend":"hook"}` (Replace).
- `%APPDATA%\ViFastKey\settings-ui.json` — theme/tab/tray của UI (không thuộc engine).
- UI ghi atomic, tạo `config.json.bak`, giữ key lạ.

Không cần sửa JSON nếu dùng UI. File cấu hình cũ vẫn được giữ khi nâng cấp.

## Gỡ cài đặt

```powershell
.\uninstall.ps1
```

Gỡ TSF, hook, shortcut và `windows.json`; **giữ** `config.json` và macro của bạn.
Muốn xoá sạch, xoá thêm `%APPDATA%\ViFastKey`.

## Xử lý sự cố

| Hiện tượng | Cách xử lý |
|---|---|
| Không thấy ViFastKey trong danh sách bàn phím | Đăng xuất/đăng nhập; cài lại đúng x64/x86; kiểm tra `%LOCALAPPDATA%\ViFastKey\tsf\x64\vifastkey_tsf.dll` |
| Có ViFastKey nhưng gõ không ra tiếng Việt | Kiểm tra đang chọn ViFastKey; thử Notepad; xem `windows.json` là `tsf` |
| Game không nhận preedit | Chuyển Replace (Hook), hoặc tắt bộ gõ nếu game có anti-cheat mạnh |
| Replace không gõ được | Mở Nâng cao → **Kiểm tra hook**; kiểm tra `vfk-windows.exe` còn trong `%LOCALAPPDATA%\ViFastKey` |
| SmartScreen cảnh báo | Bản phát hành chưa ký Authenticode; chọn “More info → Run anyway” nếu tin nguồn |
| App chạy Administrator không nhận TSF | TSF per-user không nạp vào app elevated; chạy app cùng mức quyền hoặc dùng bản HKLM |

## Giới hạn

- Chưa ký Authenticode.
- Macro hiện chạy ở chế độ Replace; TSF preedit chưa mở macro.
- Charset legacy chưa áp ở đầu ra TSF (luôn Unicode).
- Hook có thể bị anti-cheat chặn; không nên dùng trong game có anti-cheat nghiêm ngặt.
- Chưa có auto-update.

## Giấy phép

Mã nguồn MIT. Xem `LICENSE` và `THIRD_PARTY_NOTICES.md`.
