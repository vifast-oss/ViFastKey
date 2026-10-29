# Trạng thái nền tảng

| Nền tảng | Adapter | Build/test | Trạng thái |
|---|---|---|---|
| macOS | `macos/` IMK + `package.sh` universal2 | macOS 12+, Swift/Xcode, đã build arm64/x86_64 + self-test | ✅ dùng được; cần logout/login để đăng ký input source; Developer ID/notarize cần chứng chỉ |
| Windows | `rust/apps/vfk-windows` low-level hook + SendInput | `cargo check --target x86_64-pc-windows-msvc` trên macOS; build native trên Windows | 🟡 prototype; chưa test runtime, chưa TSF/tray/signing |
| Linux (IBus) | `linux/ibus/` engine Python | core test trên macOS/CI; cần Linux + PyGObject/IBus để chạy | 🟡 adapter có mã + install; chưa test runtime |
| Linux (Fcitx5) | `linux/fcitx5/` C++ addon | CI Ubuntu cài Fcitx5 dev, build Rust staticlib + addon | 🟡 có mã/build; chưa test GUI Fcitx5 |
| Android | `android/` IME Java + JNI | SDK/NDK + Gradle, APK arm64/x86_64; đã test emulator | 🟡 bàn phím tối giản, chưa Play Store |
| iOS | `ios/` host + keyboard extension | Xcode 27.1, simulator; build + engine test | 🟡 extension build; chưa test bật bàn phím/device signing |

## Vì sao chưa có Fcitx5/Android/iOS?

- macOS hiện tại không có Fcitx5 headers, Android SDK/JNI, hay iOS keyboard extension toolchain.
- IBus đã có adapter Python vì không cần header C++ và core test được trên macOS.
- Có thể viết code “trông đúng” nhưng không compile/test được; điều đó vi phạm tiêu chí
  “chạy được và có test” của repo.
- Phần lõi đã sẵn sàng: C ABI (`vfk-ffi/include/vifastkey.h`), Action model, preedit/replace,
  surrounding text, config, charset. Adapter chỉ cần chuyển Action sang API nền tảng.

## Hợp đồng tối thiểu cho adapter mới

1. Đọc `KeyEvent` / `Action` qua C ABI.
2. Áp 4 Action (Pass/Preedit/Replace/Commit), tôn trọng `pass_key`.
3. Cung cấp `ConfigStore`, `AppContext`, `SurroundingText`, `Dictionary` (hoặc dùng file JSON).
4. Xử lý focus/cursor/field; password/secure input pass-through.
5. Chạy self-test headless trước, rồi test app thật (TextEdit/Chrome/terminal/Office).
6. Không log nội dung từ; ký/notarize bản phát hành.

Xem `ADAPTER_GUIDE.md` và `macos/README.md`.
