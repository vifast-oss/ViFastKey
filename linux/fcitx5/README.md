# ViFastKey Fcitx5 (Linux)

Addon C++20 dùng C ABI Rust. Logic tiếng Việt nằm trong `rust/crates/vfk-ffi`;
addon chịu trách nhiệm lifecycle, preedit, surrounding text, timing và policy
theo app/field.

## Build (Ubuntu/Debian)

```bash
sudo apt install cmake pkg-config libfcitx5core-dev libfcitx5utils-dev
./linux/fcitx5/build.sh
sudo cmake --install linux/fcitx5/build --prefix /usr
fcitx5 -r
```

Thêm **ViFastKey** trong Fcitx5 Configuration → Input Method.

`build.sh` build Rust staticlib, addon và chạy test adapter bằng
`InputContext` giả lập (không cần X/Wayland).

Release GitHub có artifact `vifastkey-fcitx5-linux-x86_64.tar.gz` (prefix `/usr`):

```bash
tar -tzf vifastkey-fcitx5-linux-x86_64.tar.gz
sudo tar -xzf vifastkey-fcitx5-linux-x86_64.tar.gz -C /
fcitx5 -r
```

Đã test runtime trên Ubuntu 26.04 + Fcitx5 5.1.19 trong Xvfb/Openbox:
addon load `vifastkey`, gõ `tieengs` → `tiếng` qua GTK entry. CI dùng Fcitx5
5.1.7; addon build + CTest pass trên cả hai phiên bản.

## Hành vi

- **PREEDIT mặc định**, phù hợp terminal/Chromium/Electron và an toàn hơn
  direct-rewrite.
- Dùng `vfk_plan_action` từ `vfk-adapter-core`: field policy, surrounding guard,
  delete_chars/delete_utf16 và timing nằm ở logic dùng chung, không lặp trong C++.
- **Password/URL/number/email** được phát hiện từ `CapabilityFlag`; composition
  bị huỷ (không commit) khi field chuyển sang nhạy cảm.
- **Sửa từ đã commit / macro undo** dùng `Replace`: adapter kiểm tra
  `cursor == anchor`, suffix surrounding khớp text engine tin là đang hiển thị,
  rồi mới gọi `deleteSurroundingText`. Snapshot lệch → fail-safe, không xoá mù.
- **Transaction barrier**: trong lúc Replace đang xoá/chờ commit, key mới được
  giữ trong queue và xử lý tiếp sau khi commit xong. Không có hai thao tác sửa
  văn bản chạy song song.
- **Reset/focus-out**: composition được chốt trước reset. Với client preedit,
  Fcitx core đã commit khi focus-out; adapter không commit lần hai để tránh lặp từ.
- **AppProfile**: dùng `backspace_delay_ms`, `delay_per_char_ms`, `wait_ack` cho
  Replace; `output_mode=replace` chỉ được bật khi app có surrounding text.
- **Config reload**: `~/.config/vifastkey/config.json` được theo dõi mtime và
  nạp lại khi không có composition. Có thể override bằng `VIFASTKEY_CONFIG`.
- **Alt+,** hoặc click status action để bật/tắt tiếng Việt.
- Config `~/.config/vifastkey/config.json` dùng chung schema với macOS/IBus.

## Kiểm thử

```bash
./linux/fcitx5/build.sh
# hoặc
cmake -S linux/fcitx5 -B linux/fcitx5/build -DBUILD_TESTING=ON \
  -DVIFASTKEY_LIB="$PWD/rust/target/release/libvifastkey.a"
cmake --build linux/fcitx5/build -j2
ctest --test-dir linux/fcitx5/build --output-on-failure
```

CI Ubuntu cài Fcitx5 dev, build Rust staticlib, addon và chạy CTest. Test dùng
`TestInputContext` để kiểm tra preedit/commit, reset, focus-out, password,
surrounding guard, Replace transaction và emoji/UTF-16.

Test runtime trên desktop Wayland thật vẫn cần thiết; Xvfb/X11 không đại diện
cho mọi frontend Wayland.

Smoke test X11/GTK (tự tạo config tạm, không sửa config cá nhân):

```bash
sudo apt install fcitx5 fcitx5-frontend-gtk3 xvfb openbox xdotool zenity dbus-x11
./linux/fcitx5/runtime_test.sh
# Fcitx5 output: tiếng
# Fcitx5 runtime test passed
```

### Checklist app thật

| App | Việc cần thử |
|---|---|
| GNOME Terminal / Kitty / WezTerm | `tieengs` → `tiếng`; Backspace giữ dấu; Enter/Tab không mất chữ |
| Chrome/Chromium address bar | Gõ tiếng Việt, autocomplete không nhân đôi; chuyển tab qua lại |
| Google Sheets | Gõ vào cell chưa enter edit; từ đầu tiên không bị nuốt |
| Firefox | Address bar + trang web; selection/autocomplete không bị xoá nhầm |
| LibreOffice Writer | Preedit/commit, sửa từ trước con trỏ |
| VS Code / Cursor | Terminal embedded, editor, command palette |
| Discord / Signal / Slack | Gõ nhanh, macro, Backspace; không lặp từ |
| Password field | Không preedit, không commit nội dung cũ |
| URL/number/email field | Không tạo preedit; phím đi thẳng app |

Khi lỗi, bật `FCITX_LOG_LEVEL=debug` (hoặc xem journal) và ghi lại app, frontend,
field, action gần nhất; log adapter không chứa nội dung từ.

## Giới hạn

- Addon chưa có tray icon riêng; UI cấu hình `vifastkey-settings` có tray StatusNotifierItem, sửa JSON hoặc status action của Fcitx5.
- Chưa dùng `uinput`/Libei/XTest. Nếu app không nhận surrounding text và không
  nhận `forwardKey`, adapter fallback về preedit/pass-through thay vì inject phím.
- `~/.config/vifastkey/user_words.txt` được nạp thêm vào dictionary tiếng Việt
  (có thể override bằng `VIFASTKEY_USER_WORDS`).
- Chưa áp `AppProfile.charset` khi commit; Fcitx5 nhận UTF-8. Dùng `vfk-charset`
  cho chuyển đổi clipboard/file hoặc app legacy có adapter riêng.
- `output_mode=replace` là opt-in theo app và cần surrounding text; mặc định vẫn
  là PREEDIT.
- Fcitx5 có thể cache addon; sau khi cài chạy `fcitx5 -r` hoặc logout/login.
