# ViFastKey Android (IME)

Bàn phím Android dùng engine Rust qua JNI, không dùng mạng.

## Tính năng

- QWERTY đầy đủ + hàng số, shift/caps, trang ký tự, emoji.
- Telex / Simple Telex / VNI / VIQR; tone style cũ/mới.
- **Gợi ý từ** từ lexicon local (23k âm tiết Việt + 20k từ Anh), chạm để chèn.
- **Long-press** `a e i o u y d` để chọn trực tiếp dấu: à á ả ã ạ ă â, đ...
- Backspace lặp khi giữ; space/enter; chuyển bàn phím (globe).
- Settings: kiểu gõ, đặt dấu, chính tả, macro, quick Telex, âm cuối k, auto-cap...
- Password/number/email/URL: pass-through hoặc bàn phím số.
- Hỗ trợ bàn phím vật lý qua `onKeyDown`; cursor/selection guard.
- APK arm64-v8a + x86_64; debug signing.

## Build

```bash
./android/build.sh
# APK: android/app/build/outputs/apk/debug/app-debug.apk
```

Yêu cầu: Android SDK 36, NDK 28.2, JDK 21, `cargo-ndk`, Gradle 9.x.
Build script tự dò `ANDROID_HOME` và NDK; trên macOS tự chọn JDK 21.

## Cài + test

```bash
ADB=/opt/homebrew/share/android-commandlinetools/platform-tools/adb
$ADB install -r android/app/build/outputs/apk/debug/app-debug.apk
$ADB shell ime enable app.vifastkey/.ViFastKeyService
$ADB shell ime set app.vifastkey/.ViFastKeyService
$ADB shell am start -n app.vifastkey/.SettingsActivity
```

Mở **Mở ô gõ thử**, gõ `tieengs` → `tiếng`; thử shift, `?123`, emoji, long-press `a`,
và chạm gợi ý.

## Test đã chạy

- APK build arm64-v8a + x86_64.
- Cài trên emulator Android 36, đặt ViFastKey làm IME.
- Keyboard hiển thị full layout; engine preedit `tieengs` → `tiếng` qua các phím UI.
- Suggestion bar hiển thị và JNI `suggest` trả dữ liệu.
- SettingsActivity hiển thị đầy đủ tùy chọn; lưu vào SharedPreferences và engine reload.

## Còn lại

- Chưa có theme/size tùy biến, clipboard history, swipe typing.
- Chưa ký release/Play Store; dùng debug APK để test.
- Chưa test trên nhiều hãng/máy thật.
