# Roadmap

Trạng thái hiện tại: lõi Python + Rust, C ABI, WASM, CLI, playground, UI cấu hình desktop
(`vifastkey-settings`) và bộ conformance đã chạy được.
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
  - Windows: ✅ TSF preedit (`windows/tsf/`, C++ COM + C ABI) mặc định + hook Replace
    fallback (`rust/apps/vfk-windows`, binary `vifastkey-replace.exe`); UI quản lý Preedit/Replace, hook nền + Run key;
    ✅ cài x64/x86 + switch chế độ + close-to-tray trên Windows 11; còn test gõ thật
    trong game/app anti-cheat và signing.
  - Linux: ✅ IBus engine Python (`linux/ibus/`), ✅ Fcitx5 C++ addon (`linux/fcitx5/`),
    đã test runtime IBus + Fcitx5 trên Xvfb; còn test Wayland desktop và app thật.
  - Android: ✅ IME Java + JNI full keyboard, suggestions, settings, emoji; đã test emulator;
    ✅ cấu hình release signing qua env; còn Play Store listing/theme và test máy thật.
  - iOS: ✅ host + keyboard extension Swift, build simulator + engine test; còn test bật extension/device signing.
- **Fcitx5 production hardening** (bài học từ Areca, xem `docs/research/08-areca-ime.md`):
  ✅ field kind/password, commit trước reset, xác minh surrounding trước `Replace`, transaction
  barrier, AppProfile timing/ACK, config reload, status action, CTest adapter. Còn test Wayland
  desktop/app thật; thêm uinput/Libei fallback chỉ khi gặp app không nhận surrounding/forwardKey.
- **Adapter-core dùng chung**: ✅ `vfk-adapter-core` + `vfk_plan_action`; Fcitx5, macOS, iOS,
  Android, Windows dùng chung field policy/surrounding guard/action plan/timing. IBus dùng
  port Python `vfk_adapter_core.py` của cùng contract; chạy chung `adapter-conformance`.
  Còn mở rộng conformance runner cho Swift/Java và có thể chuyển IBus sang C ABI.
- **Tích hợp bảng mã vào đầu ra engine** (hiện là module/CLI/FFI riêng): cần định nghĩa
  đơn vị xoá byte/UTF-16 cho từng adapter và test trên app thật.
- **Sửa từ đã gõ trong PREEDIT**: ✅ engine trả `Replace` + adapter dùng surrounding text;
  còn kiểm thử trên nhiều app thật.
- **Macro nhiều từ/suffix, trigger tùy chọn, macro khi tắt tiếng Việt**: ✅ macro nhiều từ + off-mode;
  ✅ import/export toàn config; còn file import/export riêng cho bảng macro.
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
- **Bộ gõ tắt/macro nâng cao**: macro theo app, import/export file macro riêng.
  ✅ kiểm tra trùng/rỗng trong UI.
- **UI cấu hình desktop**: ✅ macOS preferences window; ✅ `vifastkey-settings` (egui) cho
  Windows/Linux: typing/macro/app rule, save atomic, validate, gõ thử, import preview diff,
  reset từng nhóm, theme sáng/tối, `settings-ui.json`, tab Windows backend; ✅ tray
  (Windows/macOS `tray-icon`, Linux StatusNotifierItem `ksni`) + ẩn xuống tray;
  ✅ `--screenshots` headless; ✅ watcher mtime macOS IMK + Windows hook + IBus;
  ✅ Windows `Ctrl+Shift+,` mở settings + Start Menu shortcut; ✅ test UI macOS,
  Ubuntu 26.04 X11/Xvfb + Wayland/Weston headless, tray qua D-Bus; ✅ Windows 11 render 6 tab
  + close-to-tray. Còn test tương tác menu tray trên Windows.
- **macOS smoothness**: ✅ input source prefs, secure input, Ctrl+Shift+F, clipboard tools.

## Hiệu năng / chất lượng

- `WordState.letters` ✅ `ArrayVec`; còn `Composer.states`/bitset/cache.
- Bitset cho `locked`, buffer `[char; 24]` cho `toneless_lower`, cache snapshot theo tiền tố raw.
- Fuzz hai bản Python/Rust bằng cùng seed, thêm property test cho mọi `InputMethodSpec`.
- Benchmark guard ✅ CI Ubuntu (`rust/tools/bench_guard.py`, ngưỡng rộng).

## Đóng gói

- Release binary CLI + `vifastkey-settings` cho macOS/Linux/Windows ✅ workflow `release.yml`
  (CLI + settings + WASM + header + lib); ✅ thêm macOS IMK app, Fcitx5 tarball, Android APK
  (debug + signed release nếu có secrets), iOS simulator app. Linux tray dùng D-Bus,
  không thêm dependency GTK/AppIndicator. npm package, `.deb`/`.rpm`/AUR chính thức còn lại.
- UniFFI cho Swift/Kotlin.
- GitHub Pages cho `web/` ✅ `pages.yml` (cần bật Pages); trang tài liệu API còn lại.
