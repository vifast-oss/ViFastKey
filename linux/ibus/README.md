# ViFastKey IBus (Linux)

Adapter IBus dùng engine Python ViFastKey. Phù hợp desktop GNOME/IBus; không cần
biên dịch C++.

## Phụ thuộc

```bash
# Ubuntu/Debian
sudo apt install ibus ibus-gtk3 ibus-gtk4 python3-gi gir1.2-ibus-1.0 python3-pip
cd python && python3 -m pip install --user .
```

## Cài

```bash
./linux/ibus/install.sh
# hoặc PREFIX=/usr/local sudo -E ./linux/ibus/install.sh
```

IBus 1.5.29 chỉ quét `/usr/share/ibus/component`, nên script cài component XML
system-wide (có thể dùng `sudo`); mã engine vẫn nằm trong `PREFIX` của user. Có
thể override bằng `IBUS_COMPONENT_DIR` nếu bản IBus của bạn hỗ trợ đường dẫn khác.

Mở **IBus Preferences → Input Method → Add → Vietnamese → ViFastKey**, chọn engine
`ViFastKey`, rồi gõ thử. Nếu chưa thấy, chạy `ibus restart` hoặc logout/login.

## Hành vi

- Engine mặc định **PREEDIT** để terminal/Electron/Chromium ổn định hơn.
- Telex/VNI/VIQR, macro, auto-restore tiếng Anh, config
  `~/.config/vifastkey/config.json`.
- `~/.config/vifastkey/user_words.txt`: thêm từ tiếng Việt cho auto-restore.
- Surrounding text đầy đủ (cursor/anchor) được lấy từ IBus khi app hỗ trợ; edit previous
  word dùng `Replace` qua `vfk_adapter_core` bản Python.
- Field policy: password/URL/email/number pass-through nếu IBus cung cấp input purpose.
- `Alt+,` toggle Việt/Anh trong engine; không dùng `Ctrl+Space` (IBus dành cho
  chuyển input method).
- Installer đặt `GTK_IM_MODULE`/`QT_IM_MODULE`/`XMODIFIERS` sang IBus trong
  `~/.config/environment.d/`. Nếu terminal/editor/Firefox vẫn dùng fcitx sau khi
  cài, đăng xuất/đăng nhập một lần để GNOME Shell nhận environment mới.

## Kiểm thử

Phần core chạy được trên macOS/CI:

```bash
cd python && python3 -m pytest -q tests/test_ibus_core.py
python3 -m py_compile linux/ibus/ibus_engine.py
```

Runtime smoke test IBus + GTK trên X11/Xvfb (đã chạy pass trên Ubuntu 24.04):

```bash
sudo apt install ibus ibus-gtk3 xvfb openbox xdotool zenity dbus-x11
./linux/ibus/install.sh
./linux/ibus/runtime_test.sh
# IBus output: tiếng
# IBus runtime test passed
```

## Giới hạn

- Đã test runtime với IBus 1.5.29 + GTK3 trên X11/Xvfb; chưa test Wayland desktop
  và app cụ thể (Chromium/Electron/terminal).
- Engine chưa có tray icon riêng; UI cấu hình `vifastkey-settings` có tray StatusNotifierItem hoặc sửa file JSON.
- `IbusCore` poll mtime `~/.config/vifastkey/config.json` và reload khi rảnh.
- Với app không hỗ trợ surrounding/delete, một số thao tác sửa từ cũ bị bỏ qua.
- Dùng `python/vfk_adapter_core.py` (port của adapter-core) thay vì C ABI; giữ cùng
  `adapter-conformance`. Có thể chuyển sang C ABI khi đóng gói extension nhị phân.
