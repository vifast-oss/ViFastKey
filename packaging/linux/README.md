# ViFastKey Settings cho Linux

UI cấu hình desktop cho bộ gõ tiếng Việt ViFastKey, dùng egui/eframe.

Gói này chỉ chứa **UI cấu hình**. Bộ gõ thực tế chạy qua **IBus** hoặc **Fcitx5**
(cài riêng theo README của adapter tương ứng). UI đọc/ghi cùng file config với engine.

## Tính năng

- Telex, Simple Telex, VNI, VIQR; kiểu đặt dấu cũ/mới.
- Bật/tắt chính tả, `w → ư`, `[ ] → ơ ư`, Quick Telex, âm cuối `k`.
- Sửa từ đã gõ, gõ tiếng Anh bằng escape Telex, viết hoa, bảo vệ chữ HOA.
- Macro nhiều từ, macro số, macro undo, macro khi tắt tiếng Việt.
- App profile: output, delay/ACK, charset, kiểu gõ/chính tả theo app.
- Import/export JSON có preview diff; reset mặc định từng nhóm; tìm app.
- Gõ thử bằng engine ngay trong UI; xem action trace.
- Theme sáng/tối, tray StatusNotifierItem (GNOME cần extension AppIndicator).
- Preedit mặc định; không chạy hook toàn cục.

## Trong gói

```text
vifastkey-settings          binary UI
vifastkey-settings.desktop  desktop entry
vifastkey.png               icon 192x192
vifastkey.svg               icon scalable
install.sh                  cài vào ~/.local
uninstall.sh                gỡ UI (giữ config)
README.md                   file này
DEVELOPER.md                tài liệu build/kỹ thuật
LICENSE
THIRD_PARTY_NOTICES.md
```

## Yêu cầu

- Linux x86_64, desktop X11 hoặc Wayland.
- Với IBus: engine ViFastKey đã cài (`linux/ibus/`) và đang chạy.
- Với Fcitx5: addon ViFastKey đã cài (`linux/fcitx5/`).
- Tray cần desktop có StatusNotifierWatcher. GNOME cần extension
  **AppIndicator and KStatusNotifierItem Support**; KDE/XFCE thường có sẵn.

Binary trong gói được build trên Ubuntu 26.04. Nếu distro cũ hơn báo lỗi glibc,
hãy build từ source (`cargo build --release -p vifastkey-settings`).

## Cài đặt

```bash
tar -xzf vifastkey-settings-linux-x86_64-0.1.0.tar.gz
cd vifastkey-settings-linux-x86_64-0.1.0
chmod +x install.sh
./install.sh
```

Script cài per-user, không cần root:

- binary → `~/.local/bin/vifastkey-settings`
- desktop entry → `~/.local/share/applications/`
- icon → `~/.local/share/icons/hicolor/...`

Sau đó mở **Cấu hình ViFastKey** trong app menu, hoặc chạy:

```bash
~/.local/bin/vifastkey-settings
```

Nếu `~/.local/bin` không có trong PATH, dùng đường dẫn đầy đủ hoặc thêm vào PATH.

## Sử dụng

- Tab **Tổng quan**: trạng thái file, kiểu gõ, số macro/app, gõ thử.
- Tab **Kiểu gõ**: phương pháp, dấu, chính tả, tuỳ chọn Telex, từ/câu.
- Tab **Macro**: bảng trigger → thay thế, copy JSON, validate trùng/rỗng.
- Tab **Ứng dụng**: tìm app, preset/override, loại trừ, output, delay/ACK, charset,
  trạng thái Việt/Anh do adapter ghi nhớ.
- Tab **Nâng cao**: Unicode, file/backup, import/export, reset nhóm, theme, tray.
- Tab **Chẩn đoán**: OS, đường dẫn, fingerprint, validate, gõ thử, copy diagnostics.

Tray: chuột phải mở menu (mở cấu hình/tải lại/thoát); click trái mở cửa sổ.
Đóng cửa sổ sẽ ẩn xuống tray nếu không có thay đổi chưa lưu; tắt trong Nâng cao.

## Preedit trên Linux

- **IBus**: engine dùng preedit; đây là chế độ mặc định.
- **Fcitx5**: preedit mặc định. `output_mode=replace` là opt-in theo app, chỉ dùng
  khi app cung cấp surrounding text; không phải giải pháp cho game.
- Không có global keyboard hook như Windows; không nên kỳ vọng Replace hoạt động
  trong game/fullscreen.

Nếu preedit không hiện trong một app, kiểm tra app/frontend có hỗ trợ IBus/Fcitx5
hay không; xem log adapter và thử app khác (GNOME Terminal, Chrome, LibreOffice).

## Cấu hình

- `$XDG_CONFIG_HOME/vifastkey/config.json` hoặc `~/.config/vifastkey/config.json`
- Override: `$VIFASTKEY_CONFIG`
- UI ghi atomic, tạo `config.json.bak`, giữ key lạ.
- `settings-ui.json` cùng thư mục: theme/tab/tray của UI, không thuộc engine.
- `app_modes` là runtime state do adapter ghi; UI chỉ xem/xoá.

## Gỡ cài đặt

```bash
./uninstall.sh
```

Gỡ binary, desktop entry và icon; **giữ** config và macro.

## Xử lý sự cố

| Hiện tượng | Cách xử lý |
|---|---|
| Không thấy app trong menu | `update-desktop-database ~/.local/share/applications`; kiểm tra file `.desktop` |
| Không thấy icon tray | Cài/bật extension AppIndicator (GNOME) hoặc kiểm tra StatusNotifierWatcher |
| Không gõ được tiếng Việt | Kiểm tra engine IBus/Fcitx5 đã cài, chọn đúng input source, xem config |
| UI báo không đọc được config | File hỏng được backup `config.bad.<epoch>.json`; dùng “Dùng mặc định” hoặc sửa JSON |
| Lỗi glibc khi chạy | Build lại từ source trên distro hiện tại |
| Theme tối/sáng không đổi | Chọn trong Nâng cao; theme lưu vào `settings-ui.json` |

## Giấy phép

Mã nguồn MIT. Xem `LICENSE` và `THIRD_PARTY_NOTICES.md`.
