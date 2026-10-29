# ViFastKey iOS (keyboard extension)

Host app + keyboard extension, dùng Rust staticlib qua C ABI.

## Yêu cầu

- Xcode 27.1 + iOS 27 simulator (môi trường hiện tại).
- Rust target `aarch64-apple-ios-sim` (simulator) và `aarch64-apple-ios` (device).
- `xcodegen`.

## Build + test (simulator)

```bash
./ios/build.sh
xcodebuild -project ios/ViFastKey.xcodeproj -scheme ViFastKeyHost \
  -sdk iphonesimulator -destination 'platform=iOS Simulator,name=iPhone 17' \
  -derivedDataPath ios/build ARCHS=arm64 ONLY_ACTIVE_ARCH=YES \
  CODE_SIGNING_ALLOWED=NO test
```

App: `ios/build/Build/Products/Debug-iphonesimulator/ViFastKeyHost.app`.

## Chạy thử

1. Mở app **ViFastKey** → Settings → General → Keyboard → Keyboards → Add New Keyboard.
2. Chọn **ViFastKey**, rồi trong app chạm ô nhập và nút 🌐 để chọn bàn phím.
3. Gõ `tieengs` → `tiếng`.

## Trạng thái

- ✅ Build host + keyboard extension; Rust engine test trên simulator pass.
- ✅ Keyboard UI QWERTY + số + VI toggle/space/backspace/enter.
- 🟡 Chưa test thao tác bật keyboard extension bằng UI automation; cần bật thủ công.
- Chưa có App Group/config UI/theme/suggestion bar; chưa ký device/TestFlight.
- Link staticlib simulator hiện tại arm64; máy Intel cần build thêm target
  `x86_64-apple-ios-sim` và chỉnh `LIBRARY_SEARCH_PATHS`.
