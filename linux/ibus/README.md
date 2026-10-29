# ViFastKey IBus (Linux)

Adapter IBus dùng engine Python ViFastKey. Phù hợp desktop GNOME/IBus; không cần
biên dịch C++.

## Phụ thuộc

```bash
# Ubuntu/Debian
sudo apt install ibus python3-gi gir1.2-ibus-1.0 python3-pip
cd python && python3 -m pip install --user .
```

## Cài

```bash
./linux/ibus/install.sh
# hoặc PREFIX=/usr/local sudo -E ./linux/ibus/install.sh
```

Mở **IBus Preferences → Input Method → Add → Vietnamese → ViFastKey**, chọn engine
`ViFastKey`, rồi gõ thử. Nếu chưa thấy, chạy `ibus restart` hoặc logout/login.

## Hành vi

- Engine mặc định **PREEDIT** để terminal/Electron/Chromium ổn định hơn.
- Telex/VNI/VIQR, macro, auto-restore tiếng Anh, config
  `~/.config/vifastkey/config.json`.
- `~/.config/vifastkey/user_words.txt`: thêm từ tiếng Việt cho auto-restore.
- Surrounding text được lấy từ IBus khi app hỗ trợ; edit previous word dùng `Replace`.
- `Ctrl+F` (không Shift/Ctrl khác) tạm toggle Việt/Anh. `Ctrl+Shift+F` cũng dùng được
  qua engine nếu IBus không nuốt.

## Kiểm thử

Phần core chạy được trên macOS/CI:

```bash
cd python && python3 -m pytest -q tests/test_ibus_core.py
python3 -m py_compile linux/ibus/ibus_engine.py
```

## Giới hạn

- Chưa test runtime trên phiên bản IBus/app cụ thể trong môi trường macOS hiện tại.
- Chưa có tray/preferences UI riêng; dùng file JSON.
- Với app không hỗ trợ surrounding/delete, một số thao tác sửa từ cũ bị bỏ qua.
- Fcitx5 là adapter tiếp theo; dùng chung C ABI/engine.
