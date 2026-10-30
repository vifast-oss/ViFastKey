# Trạng thái nền tảng

| Nền tảng | Adapter | Build/test | Trạng thái |
|---|---|---|---|
| macOS | `macos/` IMK + `package.sh` universal2 | macOS 12+, Swift/Xcode, đã build arm64/x86_64 + self-test; installer ghi AppleEnabledInputSources | ✅ dùng được; cần logout/login một lần; Developer ID/notarize cần chứng chỉ |
| Windows | `windows/tsf/` TSF preedit + `rust/apps/vfk-windows` hook Replace | CI Windows build MSVC (TSF) + cross-check hook; **chưa test runtime** | 🟡 code đủ backend; cần smoke test Windows thật, tray/signing |
| Linux (IBus) | `linux/ibus/` engine Python | core test + runtime smoke test Ubuntu 24.04/IBus 1.5.29 + GTK3 Xvfb: `tieengs` → `tiếng` | ✅ X11 preedit; 🟡 cần test Wayland desktop và app thật |
| Linux (Fcitx5) | `linux/fcitx5/` C++ addon | CI build + CTest adapter ✅; runtime test Ubuntu 24.04/Fcitx5 5.1.7 và Ubuntu 26.04/Fcitx5 5.1.19 trong Xvfb: `tieengs` → `tiếng` | ✅ X11 + preedit/surrounding/guard/barrier; 🟡 cần test Wayland desktop và app thật |
| Android | `android/` IME Java + JNI | SDK/NDK + Gradle, APK arm64/x86_64; CI build + test emulator; release signing qua env | ✅ QWERTY full, suggestions, settings, emoji, long-press; 🟡 chưa Play Store/device matrix |
| iOS | `ios/` host + keyboard extension | Xcode 27.1, simulator; CI build + engine unit test | ✅ build/test; 🟡 chưa test bật extension/device signing |
| Chrome/Edge | `chrome-extension/` MV3 + WASM | build + Playwright E2E (popup normalize) | ✅ MVP; 🟡 context menu/selected text cần test thêm; chưa Web Store |

Bài học adapter Fcitx5 từ Areca (queue/barrier, trust probe surrounding text, timing/ACK,
field kind, test bằng `InputContext` giả lập): `docs/research/08-areca-ime.md`.

## Adapter-core dùng chung

`rust/crates/vfk-adapter-core` + C ABI `vfk_plan_action` gom logic dễ lặp:

- field policy (password/URL/number/email/terminal);
- surrounding guard (selection, caret giữa từ, snapshot lệch, emoji/NFD);
- action plan (`pass/preedit/commit/replace/fail_safe`);
- `delete_chars` cho Fcitx5/IBus và `delete_utf16` cho macOS/Android/Windows;
- timing `wait_ms` từ AppProfile.

Đã dùng ở Fcitx5, macOS IMK, iOS keyboard, Android JNI và Windows hook. IBus Python
dùng bản port `python/vfk_adapter_core.py` của cùng logic (field policy, guard, plan),
và chạy chung bộ scenario [`adapter-conformance/`](../adapter-conformance/README.md).

## Môi trường build hiện tại

- macOS: Xcode 27.1 + iOS 27 simulator; Swift + Rust iOS targets đã cài.
- Android: SDK 36, build-tools 36, NDK 28.2, `cargo-ndk`, Gradle 9.8 + JDK 21.
- Linux: Fcitx5 dev chỉ có trên Linux; CI Ubuntu cài và build addon. Runtime test IBus/Fcitx5
  chạy headless bằng Xvfb; đã chạy trên Ubuntu 24.04 VM (Lima) và Android emulator arm64.
- Windows: không dùng Windows container trên Docker/macOS được (container Windows cần
  Windows kernel). Hook/TSF cần Windows thật hoặc CI Windows; runtime hook chưa xác nhận.
- Phần lõi đã sẵn sàng: C ABI (`vfk-ffi/include/vifastkey.h`), Action model, preedit/replace,
  surrounding text, config, charset. Adapter chỉ cần chuyển Action sang API nền tảng.

## Test runtime tái lập

```bash
# Android emulator/máy thật đang kết nối adb
./android/runtime_test.sh

# Linux X11/Xvfb: Fcitx5
./linux/fcitx5/runtime_test.sh

# Linux X11/Xvfb: IBus
./linux/ibus/install.sh
./linux/ibus/runtime_test.sh

# Windows: build TSF trước (xem windows/tsf/README.md), rồi cài per-user
powershell -ExecutionPolicy Bypass -File windows/installer/install.ps1 -BuildDir build/tsf
```

Các script Linux cần gói `xvfb openbox xdotool zenity dbus-x11` (và `fcitx5-frontend-gtk3`/`gtk4`
hoặc `ibus-gtk3`/`ibus-gtk4`). iOS dùng simulator + `xcodebuild test`; Windows cần môi trường Windows thật.

## Hợp đồng tối thiểu cho adapter mới

1. Đọc `KeyEvent` / `Action` qua C ABI.
2. Áp 4 Action (Pass/Preedit/Replace/Commit), tôn trọng `pass_key`.
3. Cung cấp `ConfigStore`, `AppContext`, `SurroundingText`, `Dictionary` (hoặc dùng file JSON).
4. Xử lý focus/cursor/field; password/secure input pass-through.
5. Chạy self-test headless trước, rồi test app thật (TextEdit/Chrome/terminal/Office).
6. Không log nội dung từ; ký/notarize bản phát hành.

Xem `ADAPTER_GUIDE.md` và `macos/README.md`.
