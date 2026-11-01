# ViFastKey cho Windows — Hook-only

ViFastKey dùng **keyboard hook** toàn cục, không cài TSF, không cần ký Authenticode.
Windows nên để input source **English (US)**; khi chọn Vietnamese/Japanese,
ViFastKey tự nhường cho bộ gõ Windows.

## Trong gói

```text
vifastkey-settings.exe  UI + hook trong cùng một file
install.ps1             cài per-user, thêm autostart, chạy ẩn vào tray
uninstall.ps1           gỡ app/hook/TSF legacy
README.md               file này
INSTALL.md              hướng dẫn mọi nền tảng
LICENSE / THIRD_PARTY_NOTICES.md
```

## Cài đặt

1. Giải nén ZIP ra thư mục bất kỳ.
2. Mở PowerShell tại thư mục đó (không cần Administrator):

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1
```

3. Trong Windows, chọn input source **English (US)**:

```text
Settings → Time & language → Language & region
→ Add a language → English (United States)
→ Win+Space chọn ENG
```

4. Mở **ViFastKey Settings** từ Start Menu hoặc `Ctrl+Shift+,`.

Nếu SmartScreen cảnh báo: **More info → Run anyway** (bản chưa ký).

## Sử dụng

- **Logo ViFastKey** trong tray: đang gõ tiếng Việt.
- **E**: ViFastKey đang English.
- **Logo mờ**: đang nhường cho Microsoft Vietnamese/Japanese hoặc Game Mode.
- **Alt+,**: đổi Việt ↔ Anh khi Windows đang ở English US.
- **Click trái tray**: đổi Việt ↔ Anh; **double-click** mở Settings; chuột phải mở menu.
- **Ctrl+Shift+,**: mở Settings.
- **X**: ẩn xuống tray (mặc định). Thoát hẳn: menu tray → **Thoát**.
- Toast nhỏ hiện khi đổi chế độ; tray icon có thể bị Windows ẩn trong overflow.

## Input source và tránh gõ đôi

Hook chỉ xử lý khi `GetKeyboardLayout` của foreground là **English US (0x0409)**.

| Windows input | ViFastKey |
|---|---|
| English US | hook gõ tiếng Việt, Alt+, đổi V/E |
| Vietnamese | pass-through, nhường Microsoft Vietnamese |
| Japanese | pass-through cả English/Japanese mode |
| Khác | pass-through |

Vì vậy:

- Muốn dùng ViFastKey: chọn **English US**.
- Muốn dùng Microsoft Vietnamese: chọn **Vietnamese**; ViFastKey không chèn.
- Muốn dùng Japanese: chọn **Japanese**; ViFastKey không phá chế độ English của JP.
- Không nên bật Microsoft Vietnamese và ViFastKey cùng lúc trong cùng input source.

`0x0409` là language ID English (US), không đảm bảo layout QWERTY; nếu dùng
Dvorak/US International, nên chuyển về US QWERTY.

## Game

- Thêm game vào **Ứng dụng → loại trừ** hoặc tạm chuyển English trước khi chơi.
- Hook có thể bị anti-cheat chặn; không nên dùng trong game có anti-cheat nghiêm ngặt.
- Game fullscreen exclusive thường không có IME/preedit; WASD có thể bị hook nuốt nếu
  engine đang bật tiếng Việt. Loại trừ game để pass toàn bộ phím.

## Giới hạn

- Không có preedit/TSF; chữ được chèn bằng Unicode/backspace.
- App chạy Administrator có thể cần chạy ViFastKey bằng Administrator.
- Không có ký Authenticode; SmartScreen/antivirus có thể cảnh báo.
- Hook có thể bị một số app bảo vệ/anti-cheat chặn.
- `0x0409` có thể là US Dvorak/International; chỉ hỗ trợ tốt US QWERTY.

## Gỡ

```powershell
.\uninstall.ps1
```

Giữ `config.json`, macro và `settings-ui.json`. Xoá sạch thêm
`%APPDATA%\ViFastKey`.

## Cấu hình

```text
%APPDATA%\ViFastKey\config.json       engine/macro/app profile
%APPDATA%\ViFastKey\settings-ui.json  theme/tab/tray/close-to-tray
%APPDATA%\ViFastKey\hook-state.json   trạng thái hook (không chứa nội dung gõ)
```
