# ViFastKey cho macOS (IMK)

Adapter macOS **thật** cho ViFastKey: một Input Method Kit app chạy trong
`~/Library/Input Methods`, nhận phím qua `IMKInputController`, gọi engine Rust qua C ABI và
áp `Action` lên text client.

```
NSEvent ──► ViFastKeyInputController ──► VfkEngine (C ABI) ──► ActionApplier ──► NSTextInputClient
                    │                                    │
                    ├─ app id / focus                     ├─ marked text (preedit)
                    ├─ menu Telex/VNI/toggle              └─ insert/commit/backspace
                    └─ config JSON
```

Không có logic tiếng Việt trong Swift. Swift chỉ chuyển key, gọi engine và áp `Action`.

## Yêu cầu

- macOS 12+ (đã build/test trên macOS 27, Apple Silicon).
- Xcode Command Line Tools (`swiftc`), Rust 1.75+.
- Build mặc định theo kiến trúc hiện tại (`arm64` hoặc `x86_64`).

## Build

```bash
# từ repo root
./macos/build.sh
```

Kết quả:

- `macos/build/ViFastKey.app` — IMK app đã ký ad-hoc.
- `macos/build/vifastkey-selftest` — self-test headless.

Build gồm:

1. `cargo build --release -p vfk-ffi` → `libvifastkey.a`.
2. `swiftc` compile `Sources/*.swift`, link staticlib + `Cocoa` + `InputMethodKit`.
3. Copy `Info.plist`, icon TIFF/PNG vào bundle.
4. Chạy self-test.

## Cài

```bash
./macos/install.sh
```

Script copy app vào `~/Library/Input Methods/ViFastKey.app`, mở app, đăng ký input source
bằng `TISRegisterInputSource`, rồi mở System Settings.

Sau đó:

1. **System Settings → Keyboard → Input Sources → Edit… → +**
2. Chọn **Vietnamese → ViFastKey** (hoặc tìm “ViFastKey”).
3. Nếu chưa thấy: đăng xuất/đăng nhập lại (macOS cache input source theo login session),
   hoặc chạy:
   ```bash
   "$HOME/Library/Input Methods/ViFastKey.app/Contents/MacOS/ViFastKeyIM" --register-input-source
   killall TextInputMenuAgent
   ```
4. Chọn input source **ViFastKey** trong menu bàn phím.

Kiểm tra:

```bash
./macos/doctor.sh
```

## Dùng

- Gõ Telex/Simple Telex/VNI như engine.
- Menu input method:
  - Bật/tắt tiếng Việt (lưu theo app).
  - Chọn Telex / Simple Telex / VNI / VIQR.
  - Mở cấu hình JSON.
  - Nạp lại cấu hình.
- Escape trả raw và **không** forward Esc.
- Backspace xoá ký tự hiển thị, giữ dấu.
- Preedit được IMK vẽ gạch chân; Enter/dấu cách/click/focus commit đúng.

## Cấu hình

`~/Library/Application Support/ViFastKey/config.json`

File mẫu được tạo lần chạy đầu. Các trường giống
[`../docs/CONFIG.md`](../docs/CONFIG.md), gồm `input_method`, `tone_style`, `spell_check`,
`allow_coda_k`, `macro_digits`, `macro_undo`, `macros`, `unicode_form`, `app_profiles`...

Sau khi sửa file, chọn **Nạp lại cấu hình** trong menu; không cần khởi động lại app.

## Test

### Headless

```bash
./macos/build/vifastkey-selftest
```

Các ca hiện có:

```
telex, d-stroke (vcdd/ddc/NDD), english (coffee/banana),
open-uo (duow/thuow), shaped-vowel (aw/aws), vni (Vie65t/nam2024),
macro, escape pass-key, backspace, key mapping, app profile chrome
```

### Thủ công trên app thật

| App | Việc cần thử |
|---|---|
| TextEdit | `Tieengs Vieejt`, Backspace, dấu cách, Enter, Esc |
| Chrome/Edge | ô địa chỉ + contenteditable; kiểm tra không lặp từ |
| Terminal | gõ lệnh, Enter, Ctrl+C; preedit không làm mất ký tự |
| VS Code/Slack/Discord | gõ nhanh, backspace, chuyển focus |
| Password field | phải pass-through hoàn toàn |
| Minecraft/game | thêm vào `excluded_apps` hoặc profile `vietnamese=false` |
| NFD config | `unicode_form=NFD`, kiểm tra dấu và backspace |

### Log

```bash
log stream --predicate 'process == "ViFastKeyIM"' --level debug
```

## Đóng gói / phát hành

- Bundle ID: `app.vifastkey.inputmethod.ViFastKey`.
- Input source ID: `app.vifastkey.inputmethod.ViFastKey`.
- Connection: `ViFastKey_Connection`.
- Controller: `ViFastKeyInputController`.
- Hiện ký **ad-hoc** để chạy local. Muốn phát hành:
  1. Ký `Developer ID Application` + hardened runtime.
  2. Notarize bằng `notarytool`, staple.
  3. Build universal2 (cần thêm target Rust `x86_64-apple-darwin`).
  4. Đóng gói `.pkg` hoặc `.dmg`, hướng dẫn cài vào `~/Library/Input Methods`.
- Ad-hoc signature có thể làm macOS không lưu trạng thái enable sau khi logout; dùng
  Developer ID để TCC/input-source ổn định.

## Giới hạn hiện tại

- Chưa có event-tap fallback cho app không hỗ trợ IMK; ưu tiên IMK vì đây là API chuẩn.
- Chưa có candidate window (không cần cho Telex/VNI).
- Chưa sửa từ đã commit trong preedit.
- App id lấy từ `NSWorkspace.frontmostApplication`; một số app đặc biệt có thể trả id
  không ổn định — dùng `app_profiles` với bundle id/alias.
- Chưa có auto-update, preferences UI đầy đủ, secure-input indicator.
- Chưa ký Developer ID/notarize; xem mục trên.

## Gỡ

```bash
./macos/uninstall.sh
```

Cấu hình vẫn giữ tại `~/Library/Application Support/ViFastKey/`; xoá thủ công nếu muốn.

## Học từ ViFastTts

- Tách engine thuần, adapter mỏng; test headless trước khi test app thật.
- Config typed/versioned, ghi atomic, có backup; lỗi config fallback default.
- Ưu tiên IMK trước event tap; không đánh đổi quyền Accessibility nếu không cần.
- Stable signing/bundle id quan trọng với TCC; không dùng ad-hoc cho bản phát hành.
- Doctor/self-test và hướng dẫn cài rõ ràng; không log nội dung từ.
