# Roadmap

Trạng thái hiện tại: lõi Python + Rust, C ABI, WASM, CLI, playground và bộ conformance đã chạy được.
Các mục dưới đây là phần còn thiếu để thành sản phẩm dùng hằng ngày.

## Đã hoàn thành trong đợt nghiên cứu

- `đ` viết tắt, undo tiếng Anh, VNI literal number, sticky restore, escape pass-key.
- Open `uơ`/`ươ`, nguyên âm có dấu đứng riêng, auto-cap mở rộng.
- `macro_digits`, `macro_undo`; AppProfile ghi đè kiểu gõ/kiểu dấu/chính tả.
- 16 bảng mã legacy + CLI/FFI/WASM; config `schema_version`; tài liệu nghiên cứu/hợp đồng.

## Ưu tiên cao

- **Adapter thật** cho từng nền tảng (xem `docs/PLATFORMS.md`):
  - macOS: ✅ IMK (Swift), ✅ universal2 (`package.sh`); còn Developer ID/notarize
    (script đã có, cần chứng chỉ) và event-tap fallback.
  - Windows: ✅ prototype low-level hook + Replace (`rust/apps/vfk-windows`); còn TSF/tray/signing.
  - Linux: ✅ IBus engine Python (`linux/ibus/`), ✅ Fcitx5 C++ addon (`linux/fcitx5/`),
    đã test runtime Fcitx5/Xvfb; còn test Wayland desktop và IBus runtime.
  - Android: ✅ IME Java + JNI, build APK + test emulator (`tieengs` → `tiếng`); còn polish/Play Store.
  - iOS: ✅ host + keyboard extension Swift, build simulator + engine test; còn test bật extension/device signing.
- **Fcitx5 production hardening** (bài học từ Areca, xem `docs/research/08-areca-ime.md`):
  field kind/password, commit trước reset, xác minh surrounding trước `Replace`, transaction
  barrier cho rewrite, AppProfile timing/ACK, test adapter bằng `InputContext` giả lập.
- **Tích hợp bảng mã vào đầu ra engine** (hiện là module/CLI/FFI riêng): cần định nghĩa
  đơn vị xoá byte/UTF-16 cho từng adapter và test trên app thật.
- **Sửa từ đã gõ trong PREEDIT**: ✅ engine trả `Replace` + adapter dùng surrounding text;
  còn kiểm thử trên nhiều app thật.
- **Macro nhiều từ/suffix, trigger tùy chọn, macro khi tắt tiếng Việt**: ✅ macro nhiều từ + off-mode;
  còn import/export.
- **Từ điển lớn**: ✅ lexicon gợi ý 23k âm tiết + 20k từ tiếng Anh; dictionary auto-restore
  vẫn giữ danh sách nhỏ để tránh false positive.

## Ưu tiên trung bình

- **VIQR** ✅ đã có (dấu câu là `extra_word_keys`); còn MS Vietnamese layout và các kiểu gõ khác.
- **Gợi ý/word prediction local** ✅ prefix completion (`vfk-suggest`); user dictionary ✅ API;
  emoji ✅ CLI/module; clipboard ✅ `vfk-charset --clipboard` (macOS).
- **Chrome extension** ✅ MVP chuẩn hoá vùng chọn + popup/bàn phím ảo (WASM); chưa Web Store.
- **Gõ tắt phụ âm** (OpenKey): `f`→`ph`, `j`→`gi`, `w`→`qu`, coda `g`→`ng`... cần kiểu gõ riêng
  để không đụng phím thanh.
- **Tiếng Tây Nguyên** (Êđê...): thêm ký tự và `InputMethodSpec`, không đổi thuật toán.
- **Bộ gõ tắt/macro nâng cao**: nhiều macro theo app, import/export, kiểm tra trùng.
- **UI cấu hình desktop**: ✅ macOS preferences window; Windows/Linux GUI còn thiếu.

## Hiệu năng / chất lượng

- `WordState.letters` ✅ `ArrayVec`; còn `Composer.states`/bitset/cache.
- Bitset cho `locked`, buffer `[char; 24]` cho `toneless_lower`, cache snapshot theo tiền tố raw.
- Fuzz hai bản Python/Rust bằng cùng seed, thêm property test cho mọi `InputMethodSpec`.
- Benchmark guard ✅ CI Ubuntu (`rust/tools/bench_guard.py`, ngưỡng rộng).

## Đóng gói

- Release binary CLI cho macOS/Linux/Windows ✅ workflow `release.yml` (CLI + WASM + header + lib);
  npm package chính thức còn lại.
- UniFFI cho Swift/Kotlin.
- GitHub Pages cho `web/` ✅ `pages.yml` (cần bật Pages); trang tài liệu API còn lại.
