# Trạng thái nền tảng

| Nền tảng | Adapter | Build/test | Trạng thái |
|---|---|---|---|
| macOS | `macos/` IMK + `package.sh` universal2 | macOS 12+, Swift/Xcode, đã build arm64/x86_64 + self-test | ✅ dùng được; cần logout/login để đăng ký input source; Developer ID/notarize cần chứng chỉ |
| Windows | `rust/apps/vfk-windows` low-level hook + SendInput | cross-check MSVC + CI Windows build | 🟡 prototype; chưa test runtime, chưa TSF/tray/signing |
| Linux (IBus) | `linux/ibus/` engine Python | core test trên macOS/CI; component đăng ký được trong Xvfb | 🟡 runtime key event chưa xác nhận; Fcitx5 là đường đã test |
| Linux (Fcitx5) | `linux/fcitx5/` C++ addon | CI build ✅; runtime test Ubuntu 26.04/Fcitx5 5.1.19 trong Xvfb: `tieengs` → `tiếng` | ✅ chạy được trên X11; 🟡 cần test Wayland desktop thật |
| Android | `android/` IME Java + JNI | SDK/NDK + Gradle, APK arm64/x86_64; CI build + đã test emulator | ✅ gõ được; 🟡 bàn phím tối giản, chưa Play Store |
| iOS | `ios/` host + keyboard extension | Xcode 27.1, simulator; CI build + engine unit test | ✅ build/test; 🟡 chưa test bật extension/device signing |

Bài học adapter Fcitx5 từ Areca (queue/barrier, trust probe surrounding text, timing/ACK,
field kind, test bằng `InputContext` giả lập): `docs/research/08-areca-ime.md`.

## Môi trường build hiện tại

- macOS: Xcode 27.1 + iOS 27 simulator; Swift + Rust iOS targets đã cài.
- Android: SDK 36, build-tools 36, NDK 28.2, `cargo-ndk`, Gradle 9.8 + JDK 21.
- Linux: Fcitx5 dev chỉ có trên Linux; CI Ubuntu cài và build addon. IBus engine test core trên macOS.
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
