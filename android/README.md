# ViFastKey Android (IME)

Bàn phím Android tối giản dùng Rust engine qua JNI.

## Yêu cầu

- Android SDK + NDK, JDK 21.
- Rust target `aarch64-linux-android`, `x86_64-linux-android`.
- `cargo-ndk`, Gradle 9.x.

Môi trường dev hiện tại đã có SDK/NDK tại
`/opt/homebrew/share/android-commandlinetools`; build script tự dò NDK.

## Build

```bash
./android/build.sh
# APK: android/app/build/outputs/apk/debug/app-debug.apk
```

## Chạy trên emulator

```bash
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb shell ime enable app.vifastkey/.ViFastKeyService
adb shell ime set app.vifastkey/.ViFastKeyService
adb shell am start -n app.vifastkey/.TestActivity
```

Chạm ô nhập, gõ `tieengs` → `tiếng`. Nút **VI** toggle Việt/Anh.

## Trạng thái

- ✅ Build APK arm64-v8a + x86_64; đã test end-to-end trên emulator Android 36.
- ✅ QWERTY + số + space/backspace/enter; Telex/VNI/VIQR qua engine.
- 🟡 Bàn phím tối giản: chưa có dấu tiếng Việt riêng, suggestion bar, settings UI,
  theme, clipboard, keyboard height tùy biến.
- Chưa ký release/Play Store; chưa test trên nhiều hãng máy.
