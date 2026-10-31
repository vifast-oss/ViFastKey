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
- Xcode Command Line Tools (`swiftc`), Rust 1.89+.
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
3. Copy `Info.plist`, icon `.icns` + TIFF/PNG vào bundle (`make_icns.sh`).
4. Chạy self-test.

## Đóng gói universal2 / phát hành

```bash
./macos/package.sh
# -> macos/build/ViFastKey-macos-universal.zip
```

Build cả arm64 + x86_64, lipo app, ký ad-hoc. Ký/notarize bản phát hành:

```bash
SIGN_ID="Developer ID Application: ..." \
NOTARY_PROFILE="vifastkey-notary" \
./macos/package.sh
```

## Cài

```bash
./macos/install.sh
```

Script copy app vào `~/Library/Input Methods/ViFastKey.app`, mở app, đăng ký input source
bằng `TISRegisterInputSource`, rồi ghi entry vào `AppleEnabledInputSources` của HIToolbox.

Sau đó:

1. **Đăng xuất/đăng nhập một lần** để macOS làm mới danh sách input source.
2. **System Settings → Keyboard → Input Sources → chọn ViFastKey**.
3. Nếu chưa thấy, chạy lại:
   ```bash
   "$HOME/Library/Input Methods/ViFastKey.app/Contents/MacOS/ViFastKeyIM" --repair-input-source
   killall TextInputMenuAgent
   ```
4. Mở Preferences trực tiếp (kể cả khi chưa chọn input source):
   ```bash
   "$HOME/Library/Input Methods/ViFastKey.app/Contents/MacOS/ViFastKeyIM" --preferences
   ```

Kiểm tra:

```bash
./macos/doctor.sh
```

## Dùng

- Gõ Telex/Simple Telex/VNI như engine.
- **Alt+,**: bật/tắt tiếng Việt (lưu theo app).
- Menu input method:
  - Bật/tắt tiếng Việt.
  - Chọn Telex / Simple Telex / VNI / VIQR.
  - Mở Preferences, file JSON, nạp lại cấu hình.
  - Clipboard: bỏ dấu, NFC, VIQR ↔ Unicode, TCVN3 → Unicode.
  - Mở Input Sources.
- Escape trả raw và **không** forward Esc.
- Backspace xoá ký tự hiển thị, giữ dấu.
- Preedit được IMK vẽ gạch chân; Enter/dấu cách/click/focus commit đúng.
- Secure input/password: reset preedit và pass-through, không commit chữ vào field mật khẩu.

## Cấu hình

`~/Library/Application Support/ViFastKey/config.json`

File mẫu được tạo lần chạy đầu. Các trường giống
[`../docs/CONFIG.md`](../docs/CONFIG.md), gồm `input_method`, `tone_style`, `spell_check`,
`allow_coda_k`, `macro_digits`, `macro_undo`, `macros`, `unicode_form`, `app_profiles`...

IMK poll mtime 1 giây và tự nạp lại khi file đổi; **Nạp lại cấu hình** trong menu
vẫn dùng được để ép reload. Không cần khởi động lại app.

## Test

### Headless

```bash
./macos/build/vifastkey-selftest
```

Các ca hiện có:

```
telex, d-stroke (vcdd/ddc/NDD), english (coffee/banana),
open-uo (duow/thuow), shaped-vowel (aw/aws), vni (Vie65t/nam2024),
macro, escape pass-key, backspace, key mapping, app profile chrome,
charset VIQR encode/decode, replace committed text
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
- Chưa có candidate window/suggestion bar (không cần cho Telex/VNI cơ bản).
- App id lấy từ `NSWorkspace.frontmostApplication`; một số app đặc biệt có thể trả id
  không ổn định — dùng `app_profiles` với bundle id/alias.
- Chưa có auto-update; preferences AppKit có kiểu gõ/dấu, Unicode NFC/NFD, chính tả,
  macro, auto-restore English, app loại trừ, default Việt/Anh, restore sau dấu cách.
  App profile/charset nâng cao vẫn dùng file JSON; Windows/Linux có `vifastkey-settings`
  đầy đủ hơn.
- Chưa ký Developer ID/notarize; xem mục trên.
- Lần đầu cần logout/login để macOS refresh input source.

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
