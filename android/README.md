# ViFastKey Android (IME)

Bàn phím Android dùng engine Rust qua JNI, không dùng mạng.

## Tính năng

- 3 layout theo Gboard: chữ, bàn phím số (+, -, *, /, %, 0, =) và trang ký tự
  (số + ký tự, phím `=\\` / `12/34`).
- Hàng qwerty có số nhỏ 1–0; long-press hiện số (và dấu với nguyên âm).
- Nút phẩy có emoji; giữ phím phẩy để mở emoji.
- Emoji 1.121 biểu tượng, cuộn dọc (không lật trang); chọn emoji xong tự về layout chữ.
- Hàng “Hay dùng” ở đầu bảng emoji, xếp theo tần suất/lần dùng gần nhất; nhãn trái,
  icon gợi ý bên phải. Tab emoji không còn hàng gợi ý trống phía trên.
- Lưới emoji luôn lộ một phần hàng kế tiếp kèm scrollbar để biết còn cuộn; chỉ rung
  khi thực sự chọn emoji, chạm/kéo để cuộn không rung.
- App ghi chú tự chốt composing text: IME chỉ `finishComposingText()` và không
  commit lại preedit, tránh nhân đôi từ (`NguyễnNguyễn`).
- Tab emoji cao bằng layout đang dùng (chữ/ký tự/số, kể cả VNI); hàng “Hay dùng”
  co lại, lưới tự bù chiều cao nên không giật khi chuyển tab.
- Settings có logo và số build (`0.1.0 · build N`). Tùy chọn “Tự về bàn phím chữ
  sau ký tự đặc biệt” nằm trong **Nâng cao**, mặc định tắt; bật nếu muốn quay về
  bàn phím chữ sau mỗi ký tự đặc biệt.
- Tùy chọn “Hiện hàng số 1–0 trên bàn phím chữ”: Telex/Simple Telex/VIQR mặc định
  tắt và bật được; VNI luôn bật và khóa tắt vì cần số để gõ dấu/âm.
- Chiều cao bàn phím 100/115/130/145% trong Settings → Bàn phím; áp dụng cho mọi
  layout, kể cả emoji. Nút 4 ô trên bàn phím mở cài đặt nhanh (chiều cao, hàng số,
  mở Settings đầy đủ).
- Khi build APK, dùng `./android/build.sh` để rebuild JNI từ core; `gradle` trực
  tiếp có thể dùng `.so` cũ trong `jniLibs`.
- Mặc định không gạch chân preedit (Replace/commit như Gboard). Bật
  **Nâng cao → Hiện gạch chân chữ đang gõ** nếu muốn thấy preedit.
- Backspace sau ký tự thừa: `thoảng` + `e` → xoá `e` về `thoảng`; xử lý trong core,
  không phải từng UI.
- Chế độ Replace gộp xoá + commit bằng batch edit và bỏ composing rỗng; giảm nháy
  khi gõ nhanh, suggestion/autocorrect cũng dùng batch.
- Vào app/field mới tự mở layout chữ (hoặc bàn phím số cho field số), không giữ
  nhầm tab ký tự đặc biệt; quay lại cùng field giữ layout hiện tại.
- Android 14+ dùng `InputConnection.replaceText()` để thay range trong một lệnh;
  Android cũ/field dài fallback batch delete + commit.
- Mỗi lần ẩn rồi mở lại bàn phím đều về layout chữ (hoặc bàn phím số), không giữ
  trang ký tự đặc biệt.
- Chọn dấu (long-press) xong cũng về layout chữ; chọn số thì giữ nguyên bàn phím số.
- Số gợi ý `1..0` nhỏ ở góc trên phải hàng qwerty; giữ phím ra số ngay.
- Nhớ panel đang dùng: từ chữ mở lại panel gần nhất (mặc định là ký tự đặc biệt);
  `12/34` và `!?#` chuyển giữa bàn phím số và ký tự.
- **Bảng nhớ tạm**: nút 📋 trên thanh trên; lưu 20 clip gần nhất (local), dán,
  chọn tất cả, sao chép, cắt và xoá lịch sử. Clipboard chỉ đọc khi mở menu.
  Menu có nút ✕, đóng khi chạm ra ngoài hoặc chạm lại nút 📋; menu nằm dưới thanh
  trên nên không che icon clipboard; phím phẩy giữ nguyên vị trí, chỉ nhích dấu phẩy
  bên trong lên; sau khi chọn số/dấu, phím không còn bị highlight.
- Long-press nguyên âm mở popup biến thể bo góc: mặc định là số (ví dụ `e` → `3`),
  kéo sang trái/phải để chọn `ê/é/...`; thả tay chọn mục đang highlight.
- Emoji dùng GridView lazy (chỉ render ô nhìn thấy), hiện 3,5 hàng để thấy còn cuộn.
- Gõ xong ký tự đặc biệt (`@ # đ ... + % ...`) tự quay về layout chữ; số vẫn ở lại.
- Icon vector riêng cho Shift, Backspace, Enter (không dùng glyph font).
- Kích thước phím, khoảng cách và hàng home-row theo Gboard: phím chữ ~35dp rộng,
  ~47dp cao, gap ngang ~5,3dp, gap dọc ~11,8dp; chữ 24sp.
- Touch target phủ kín phần gap: chạm giữa hai phím sẽ chọn phím gần nhất, gõ nhanh đỡ trượt.
- Backspace xoá được vùng bôi đen; giữ Backspace tăng tốc dần (65ms → 22ms);
  xoá ký tự cuối không còn bị khựng/commit lại chữ đang compose.
- Tự bật shift ở đầu câu hoặc sau `.` `!` `?` `…` + khoảng trắng; shift/backspace có rung và âm thanh như phím chữ.
- Phản hồi rung + âm thanh ngay khi chạm xuống; không đợi nhả phím.
- Telex / Simple Telex / VNI / VIQR; tone style cũ/mới. Android mặc định `w → ư`
  (tắt trong Settings nếu muốn `w` là chữ `w`). Khi chọn **VNI**, layout chữ tự có
  thêm hàng số `1–0` để gõ dấu/âm VNI ngay, không cần mở `?123`.
- **Gợi ý từ** từ lexicon local (23k âm tiết Việt + 20k từ Anh), chạm để chèn.
- **Sửa lỗi gần đúng**: gõ nhầm phím cạnh bên / thiếu / thừa ký tự sẽ gợi ý từ gần nhất (ví dụ `tieengd` → `tiếng`).
- **Tự sửa lỗi gõ nhầm phím** (mặc định bật, tắt trong Settings): chỉ commit khi từ
  hiện tại không hợp lệ và có đúng một ứng viên tiếng Việt thông dụng; ví dụ
  `tieengd` + space → `tiếng `.
- **Smart space**: chọn gợi ý tự thêm dấu cách; gõ `.` `,` `!` `?` `:` `;` `)` `]` `}` ngay sau đó tự xoá dấu cách thừa.
- Không commit lại preedit khi app gửi tin nhắn/xoá field (tránh sót chữ như `:v`).
- **Long-press** `a e i o u y d` để chọn trực tiếp dấu: à á ả ã ạ ă â, đ...
- Backspace lặp khi giữ; space/enter; phím space hiển thị `ViFastKey`; chuyển bàn phím (globe).
- Settings: kiểu gõ, đặt dấu, chính tả, macro, quick Telex, âm cuối k, auto-cap,
  tự sửa lỗi gõ nhầm phím... Nút **Lưu cấu hình** cố định ở đáy màn hình.
- Password/number/email/URL: pass-through hoặc bàn phím số.
- Hỗ trợ bàn phím vật lý qua `onKeyDown`; cursor/selection guard.
- Haptic feedback; app icon; suggestion bar giữ chiều cao cố định (không nhảy layout).
- APK arm64-v8a + x86_64; debug signing.
- Release build có thể ký bằng keystore qua biến môi trường (xem dưới).

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

Smoke test tự động trên emulator/máy thật đang kết nối adb:

```bash
./android/runtime_test.sh
# Android IME output: tiếng
# Android runtime test passed
```

## Test đã chạy

- APK build arm64-v8a + x86_64.
- Cài trên emulator Android 36, đặt ViFastKey làm IME.
- Keyboard hiển thị full layout; engine preedit `tieengs` → `tiếng` qua các phím UI.
- Suggestion bar hiển thị và JNI `suggest` trả dữ liệu.
- SettingsActivity hiển thị đầy đủ tùy chọn; lưu vào SharedPreferences và engine reload.

## Release / Play Store

Tạo keystore và đặt biến môi trường:

```bash
keytool -genkeypair -v -keystore vifastkey.jks -alias vifastkey \
  -keyalg RSA -keysize 2048 -validity 10000
export VIFASTKEY_KEYSTORE="$PWD/vifastkey.jks"
export VIFASTKEY_KEYSTORE_PASSWORD=...
export VIFASTKEY_KEY_ALIAS=vifastkey
export VIFASTKEY_KEY_PASSWORD=...
./android/build.sh release
# -> app/build/outputs/apk/release/app-release.apk
```

Không commit keystore/password. Nếu chưa có keystore, dùng debug APK để test.

## Còn lại

- Chưa có theme/size tùy biến, clipboard history, swipe typing.
- Chưa phát hành Play Store; cần signing key, store listing và test trên máy thật.
- Chưa test trên nhiều hãng/máy thật.
