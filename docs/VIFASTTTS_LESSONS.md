# Học từ ViFastTts cho adapter macOS

ViFastTts là dự án TTS (code MIT hoặc Apache-2.0), không phải IME, nhưng có nhiều quyết định kỹ thuật
chuyển được sang ViFastKey. Báo cáo đầy đủ: [`research/07-vifasttts.md`](research/07-vifasttts.md).

## 1. Điều đã áp dụng ngay

| Bài học | Áp dụng trong `macos/` |
|---|---|
| Engine thuần, adapter mỏng | Swift chỉ gọi `VfkEngine` (C ABI), áp `Action`; không có logic tiếng Việt |
| Hợp đồng trước, code sau | `ActionApplier` + self-test headless; test trước khi test app thật |
| Config typed/versioned, atomic | `ConfigFile` đọc/ghi JSON, backup `.bak`, fail-safe default |
| IMK trước, event tap sau | Adapter dùng `IMKInputController` + preedit; không cần Accessibility |
| Doctor/self-test | `macos/build.sh`, `macos/doctor.sh`, `vifastkey-selftest` |
| Privacy mặc định | Không log nội dung từ; không gọi mạng; secure/password pass-through |
| Bundle id/TCC | Bundle id có `.inputmethod.`, connection name cố định, ký ad-hoc để test |
| App context | `normalize_app_id` xử lý bundle id/alias trước khi áp app profile |

## 2. Điều học được nhưng chưa làm

- **Developer ID + notarize + universal2**: cần cho bản phát hành; ad-hoc chỉ test local.
- **Event-tap fallback**: cho app không hỗ trợ IMK; cần Input Monitoring/Accessibility và
  cơ chế watchdog/marker/PID pin như GõViệt/ViFastTts gợi ý.
- **First-run/permission UI**: onboarding, kiểm tra input source, nút mở System Settings.
- **Preferences UI**: hiện menu mở file JSON; nên có SwiftUI/AppKit window.
- **Candidate/OSD**: chưa cần cho Telex/VNI nhưng hữu ích cho trạng thái Việt/Anh.
- **Stable signing** để TCC không reset sau mỗi build.
- **Release CI**: build universal, notarize, staple, smoke test trên app thật.
- **Adapter conformance fixtures**: hiện test engine + action applier; cần bộ ca cho
  `NSEvent` → Action → `IMKTextInput`.
- **Secure input detection** và password field policy; IMK thường pass nhưng cần kiểm tra.
- **Crash/recovery/watchdog** cho IMK process; macOS sẽ tự relaunch nhưng cần tránh loop.

## 3. Bài học kỹ thuật quan trọng

1. **Đừng trộn engine với UI**: TTS tách `text`/`audio`/`engine`/`native`; ViFastKey tách
   `vfk-core`/`vfk-engine`/`vfk-ffi`/Swift. Giữ nguyên.
2. **Test theo tầng**: engine conformance → action/applier fixtures → Swift fakes →
   app thật → packaged smoke. Không bỏ qua tầng nào.
3. **Config là API**: version, migration, backup, atomic write, lỗi rõ ràng. Không ghi
   file bằng `write` không atomic rồi bỏ qua lỗi.
4. **UTF-16/NSRange**: IMK dùng UTF-16; engine Action dùng code point/UTF-16 ở FFI. Test
   NFD, emoji, grapheme; đừng cắt theo scalar.
5. **App id**: bundle id dạng `com.google.Chrome` phải match preset `chrome`; đã sửa
   `normalize_app_id` ở engine + adapter.
6. **Escape**: commit raw nhưng không forward Esc; đã sửa engine và adapter.
7. **Signing**: ad-hoc làm TCC/input-source không ổn định; dùng Developer ID.
8. **Không copy asset/model**: ViFastTts model/data không thuộc giấy phép của code; chỉ học pattern.

## 4. Quyết định cho macOS adapter

- Dùng **IMK** làm mặc định, preedit native.
- Tạo config tại `~/Library/Application Support/ViFastKey/config.json`.
- Bundle id `app.vifastkey.inputmethod.ViFastKey` (có segment `.inputmethod.`).
- Menu đổi Telex/Simple Telex/VNI, toggle, reload, mở config.
- App id lấy từ frontmost app + normalize; không dùng private API.
- Ký ad-hoc trong repo; tài liệu ghi rõ bản phát hành cần Developer ID.

## 5. Việc cần làm tiếp

Xem `ROADMAP.md` và `macos/README.md` mục “Giới hạn hiện tại”.
