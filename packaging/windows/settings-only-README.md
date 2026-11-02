# ViFastKey Settings (chỉ UI cấu hình)

Gói này **chỉ chứa `vifastkey-settings.exe`**, không chứa hook.
Để gõ tiếng Việt trên Windows, hãy dùng gói **`vifastkey-windows-hook-0.1.1.zip`** —
gói đó có Settings + hook + installer và hướng dẫn đầy đủ.

## Tính năng UI

- Telex/Simple Telex/VNI/VIQR, kiểu đặt dấu, chính tả, Telex nâng cao.
- Macro nhiều từ, macro số, macro undo; import/export có preview diff.
- App profile: output, delay/ACK, charset, kiểu gõ/chính tả theo app.
- Gõ thử bằng engine, reset từng nhóm, tìm app, theme sáng/tối, tray.
- Mặc định Preedit; chọn Replace (Hook) trong tab Nâng cao nếu đã cài hook.

## Chạy

```powershell
.\vifastkey-settings.exe
.\vifastkey-settings.exe --tab apps
.\vifastkey-settings.exe --check
```

Nếu đã cài gói hook, mở từ Start Menu **ViFastKey Settings**.
Cấu hình nằm tại `%APPDATA%\ViFastKey\config.json`.

Xem `DEVELOPER.md` để build/tùy chỉnh. Mã nguồn MIT; xem `LICENSE` và
`THIRD_PARTY_NOTICES.md`.
