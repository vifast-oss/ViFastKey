# ViFastKey Fcitx5 (Linux)

Addon C++20 dùng C ABI Rust. Logic tiếng Việt nằm trong `rust/crates/vfk-ffi`;
addon chịu trách nhiệm lifecycle, preedit, surrounding text, timing và policy
theo app/field.

## Build (Ubuntu/Debian)

```bash
sudo apt install cmake pkg-config libfcitx5core-dev libfcitx5utils-dev
./linux/fcitx5/build.sh
sudo cmake --install linux/fcitx5/build
fcitx5 -r
```

Thêm **ViFastKey** trong Fcitx5 Configuration → Input Method.

`build.sh` build Rust staticlib, addon và chạy test adapter bằng
`InputContext` giả lập (không cần X/Wayland).

Đã test runtime trên Ubuntu 26.04 + Fcitx5 5.1.19 trong Xvfb/Openbox:
addon load `vifastkey`, gõ `tieengs` → `tiếng` qua GTK entry. CI dùng Fcitx5
5.1.7; addon build + CTest pass trên cả hai phiên bản.

## Hành vi

- **PREEDIT mặc định**, phù hợp terminal/Chromium/Electron và an toàn hơn
  direct-rewrite.
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
- **Ctrl+Shift+F** hoặc click status action để bật/tắt tiếng Việt.
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

## Giới hạn

- Chưa có preferences GUI/tray riêng; sửa JSON hoặc dùng status action.
- Chưa dùng `uinput`/Libei/XTest. Nếu app không nhận surrounding text và không
  nhận `forwardKey`, adapter fallback về preedit/pass-through thay vì inject phím.
- Chưa nạp `user_words.txt`/dictionary file; engine dùng dictionary mặc định.
- `output_mode=replace` là opt-in theo app và cần surrounding text; mặc định vẫn
  là PREEDIT.
- Fcitx5 có thể cache addon; sau khi cài chạy `fcitx5 -r` hoặc logout/login.
