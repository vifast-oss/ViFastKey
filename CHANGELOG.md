# Changelog

## Unreleased

### Core & engine

- Thêm `tô` vào từ điển tiếng Việt thông dụng: gõ Telex `too` + dấu cách
  ra `tô ` thay vì bị khôi phục thành từ tiếng Anh `too`.

### Desktop

- Thêm UI cấu hình `vifastkey-settings` (egui) cho Windows/Linux, kèm `vfk-config`
  đọc/merge/validate/atomic save, `--self-test`/`--check`/`--import`/`--export`.
- **Windows Preedit/Replace**: mặc định `tsf` (Preedit); UI chỉ còn Preedit/Replace.
  Chọn Replace chạy hook nền + Run key, chọn Preedit dừng hook + xoá Run key; bỏ
  lựa chọn `auto` khỏi UI (giá trị cũ đọc như Preedit).
- **Icon**: cân lại icon ViFastKey theo tỉ lệ ViFastTts (cùng sao/speed lines, bàn phím
  tách khỏi sao), regenerate toàn bộ PNG/ICO/ICNS; thêm icon cho Windows exe/window,
  macOS `.icns`, iOS AppIcon, Chrome extension, Linux window; Android dùng icon mới.
- **Crate Windows Replace** đổi tên `vfk-windows` → `vfk-windows-replace`, thư mục
  `rust/apps/vfk-windows-replace`; binary vẫn `vifastkey-replace.exe`.
- Đổi tên binary Replace từ `vfk-windows.exe` sang **`vifastkey-replace.exe`** cho dễ hiểu;
  installer/settings tự dọn process và file tên cũ khi nâng cấp.
- Gói phát hành Windows/Linux có `README.md` hướng dẫn cài đặt/giới thiệu tính năng,
  `DEVELOPER.md`, `LICENSE`, third-party notices; Linux thêm `install.sh`/`uninstall.sh`.
- **Tray icon**: Windows/macOS dùng `tray-icon`, Linux dùng StatusNotifierItem
  (`ksni`, không cần GTK/AppIndicator trong binary); menu mở cấu hình/tải lại/thoát,
  click trái mở cửa sổ, đóng cửa sổ ẩn xuống tray (tắt được trong tab Nâng cao).
- **Import preview diff**: so sánh giá trị hiệu lực trước/sau, hiện đường dẫn field,
  số lỗi/cảnh báo và chặn nhập khi có lỗi; giữ nguyên key lạ.
- **Khôi phục mặc định từng nhóm** (kiểu gõ, tùy chọn macro, ứng dụng, nâng cao);
  reset nhóm macro không xoá bảng macro, reset app giữ `app_modes`.
- Tìm app trong tab Ứng dụng; sửa app profile áp dụng vào bản nháp ngay, bỏ nút
  “Áp dụng” gây mất thay đổi khi chuyển app.
- Theme sáng/tối cố định theo lựa chọn (không bị theme hệ thống ghi đè); sửa màu
  chữ nút Lưu/Khôi phục và tab đang chọn; sửa bảng macro bị co cột.
- `--screenshots <dir>`/`--screenshot <png>` chụp framebuffer 6 tab (macOS, X11,
  Wayland headless, Windows) để kiểm thử giao diện; `--self-test` dựng đủ 6 tab.
- `vfk-config` nhận file JSON có UTF-8 BOM (PowerShell 5.1/editor Windows) thay vì báo hỏng.
- Đã kiểm thử UI trên macOS, Ubuntu 26.04 X11/Xvfb, Wayland/Weston headless và
  Windows 11 (render 6 tab qua scheduled task, close-to-tray); tray Linux test qua
  D-Bus (menu mở/tải lại/thoát), tray Windows xác nhận cửa sổ Shell_NotifyIcon.
  Còn test tương tác menu tray trên Windows.
- Release đóng gói `vifastkey-settings.exe` trong gói Windows TSF, thêm shortcut
  `ViFastKey Settings` và asset cấu hình desktop riêng cho Windows/Linux.
- Thêm gói cài **IBus Linux** (`vifastkey-ibus-linux.tar.gz`) gồm engine, component
  XML, wheel Python và `install.sh`; bổ sung vào release CI.
- Web playground mặc định **Preedit**; Windows TSF mặc định `backend: "tsf"` (preedit),
  `auto`/`hook` vẫn chọn được trong UI.
- Backspace trong từ literal thử khôi phục nhiều lần: `nhuwnggg` + 2 Backspace → `nhưng`.

## 0.1.0 — 2026-10-31

### Phát hành

- Bản phát hành chính thức đầu tiên `v0.1.0`, kèm GitHub Release đa nền tảng,
  `sha256sums.txt` và build Windows TSF x64/x86.
- Landing page có link tải Windows, release notes, mã kiểm tra và thống kê lượt bấm tải
  theo nền tảng (`POST /api/download`, `/api/stats`, `/admin`).
- CI release kiểm tra tag khớp version workspace trước khi publish.

### Web

- Thêm mục **Sản phẩm** lớn giới thiệu ViFastKey + ViFastTts.
- Chuyển **Góp ý** từ popup sang mục hiển thị trên landing.
- Thêm mục **Thống kê** cuối trang; tăng tương phản chữ phụ và chuẩn hoá nút **Tải về →**.
- Thêm link **Admin** ở mục thống kê và footer.

### Nghiên cứu & đối chiếu

- Phân tích 20 dự án IME tiếng Việt công khai (SKey, UniKey, OpenKey, VnKey, Vigo, GõViệt,
  VKey, Funput, Bamboo/Lotus, Telebit, AVIM, v7...). Tổng hợp tại
  `docs/IME_FEATURE_RESEARCH.md`, hợp đồng tại `docs/COMPATIBILITY.md`, adapter tại
  `docs/ADAPTER_GUIDE.md`, config tại `docs/CONFIG.md`.
- Không chép code/dữ liệu GPL; bảng mã dùng dữ liệu MIT của bamboo-core, ghi công tại
  `THIRD_PARTY_NOTICES.md`.

### Core & engine

- Thêm `allow_coda_k`: cho phép âm cuối `k` cho tên riêng/địa danh (`Đắk Lắk`), chỉ với thanh sắc/nặng.
  Bật/tắt bằng `Config.allow_coda_k` hoặc `TransformOptions.allow_coda_k`.
- Thêm `macro_digits`: giữ chữ số trong từ để macro có số hoạt động (`mp3`, `k8s`), không ảnh hưởng VNI.
- Thêm `macro_undo`: Backspace ngay sau macro khôi phục trigger + dấu cách.
- Macro nhiều từ/suffix câu trong REPLACE; `macro_in_off_mode`.
- PREEDIT sửa từ đã gõ bằng `Replace` + surrounding text.
- Gợi ý từ local 23k âm tiết + 20k tiếng Anh (`vfk-suggest`), emoji CLI (`vfk-emoji`),
  clipboard conversion (`vfk-charset --clipboard`).
- `WordState.letters` dùng `ArrayVec`; thêm Case conversion.
- Sửa `đ` viết tắt (`ddc`, `vcdd`, `NDD`), undo tiếng Anh (`banana`, `cheese`),
  VNI literal number (`nam2024`, `ba10`), sticky restore (`text` + `s`), escape pass-key,
  open `uơ`/`ươ` theo âm đầu, nguyên âm có dấu đứng riêng (`aw`, `aws`).
- Escape đôi phím thanh **liền kề** theo Telex chuẩn: `vissa` → `visa`,
  `serrver` → `server`, `ViFFastTts` → `ViFastTts`; xoá sau escape trả lại dấu
  (`phar` + `rr`, xoá 2 `r`, gõ `r` → `phả`). Hai `f` chỉ ra một `f` (`coffee` →
  `cofee`); gõ ba `f` mới ra hai `f` (`cofffee` → `coffee`). Bỏ heuristic từ điển/
  `FF` viết hoa cho escape.
- Backspace sau khi gõ nhầm phím lặp ở cuối từ khôi phục lại từ đã biến đổi:
  `nhuwngg` + Backspace → `nhưng`; `text` + Backspace vẫn giữ `tex`.
- Mặc định `w` đứng riêng là chữ `w`; bật `w_as_u` mới đổi thành `ư`.
  Khi bật, `ww` là escape literal `w` và các phím sau viết thẳng: `wwe` → `we`,
  `wweb` → `web`. Mặc định `web` gõ trực tiếp vẫn ra `web`.
- Auto-cap mở rộng cho `…`, ngoặc kép/đóng, newline; không cắt số thập phân.
- `AppProfile` ghi đè `input_method`, `tone_style`, `spell_check`; thêm gợi ý `charset`.
- `schema_version` trong config; `config_from_dict` bỏ qua field lạ.
- Module bảng mã 16 charset (TCVN3, VNI-Windows, VISCII, VIQR, CP1258, NCR, C-string...);
  FFI C/WASM/Python và CLI `vfk-charset`.
- Python FFI đầy đủ hơn: `process_char`, focus, surrounding text, field kind, toggle, reset,
  config JSON một phần, app profile, dictionary, phiên bản.
- Rust/WASM: `Config::apply_json_patch` dùng chung; C ABI giờ thật sự nhận JSON cấu hình một phần
  (trước đây JSON thiếu trường bị thay bằng mặc định).

### Desktop UI

- Thêm `vfk-config`: đọc/merge/validate/ghi atomic config, giữ key lạ, backup và phát hiện conflict.
- Thêm `vifastkey-settings` (egui 0.32.3): tab Tổng quan/Kiểu gõ/Macro/Ứng dụng/Nâng cao/Chẩn đoán,
  gõ thử bằng engine, `--check` và `--self-test`.
- UI chỉ ghi field đã sửa; không materialize preset `app_profiles`; `app_modes` là runtime state.
- Engine merge `app_profiles` người dùng lên preset theo từng app; preset khác không bị mất.
- macOS preferences thêm Unicode NFC/NFD, app loại trừ, auto-restore English, macro enabled,
  default Việt/Anh, restore sau dấu cách; nội dung cuộn được.
- Import/export JSON qua file dialog + CLI `--import`/`--export`; lưu theme/tab vào
  `settings-ui.json`; tab Windows backend (`windows.json`).
- macOS `persistEngineConfig` chỉ merge `app_modes`, không ghi đè toàn bộ config.
- macOS IMK, Windows hook và IBus poll mtime để reload config không cần restart.
- Windows hook: `Ctrl+Shift+,` mở settings; installer tạo Start Menu shortcut.
- Thêm `docs/DESKTOP_UI.md`, desktop entry Linux và build settings trong CI/release.
- Workspace MSRV 1.85 → 1.89 do dependency transitive của eframe/egui 0.32.3.

### CLI

- `--text`, `--method`, `--tone`, `--mode`, `--nfd`, `--no-spell`, `--allow-coda-k`,
  `--macro-digits`, `--macro`, `--trace`, `--version`, `--help` cho cả bản Python và Rust.
- REPL thêm `:coda_k`, `:macro_digits`, `:spell`, `:help`; `|` = Enter.

### Web

- Playground có chế độ **Preedit** (mô phỏng TSF/IMK/Fcitx5) bên cạnh Replace.
- Cấu hình macro bằng textarea, chọn NFC/NFD, bật macro có số, âm cuối k, sửa từ vừa gõ.
- Ví dụ `Đắk Lắk` tự bật tuỳ chọn cần thiết.
- Công cụ chuyển bảng mã + hiển thị hex cho bảng một byte.
- Trang production `https://vifastkey.hoctuthien.com/`: thêm API Python
  (`/api/type`, `/api/charset`), thống kê ẩn danh, góp ý/báo lỗi, `/admin`
  mặc định mở công khai (Basic Auth tuỳ chọn) và healthcheck Docker.
- Playground hỗ trợ bàn phím ảo Android/Gboard qua `beforeinput` + `compositionend`;
  giữ nguyên chữ có dấu do Gboard commit, và thoát chế độ thô sau khi xoá từ dài
  quá `MAX_RAW` để gõ tiếng Việt lại được.
- Thêm `deploy/vifastkey_web/` (FastAPI + Docker + Traefik labels) để deploy
  cạnh `vifasttts` trong LXD/Coolify.

### macOS adapter

- Thêm IMK input method thật (`macos/`): Swift controller → Rust C ABI → ActionApplier.
- Preedit native, menu Telex/Simple Telex/VNI/VIQR, toggle Việt/Anh, config JSON atomic + backup.
- Build/install/doctor/self-test; app id normalization cho bundle id.
- Ad-hoc signing để test local; `package.sh` build universal2 + hook ký/notarize;
  cần Developer ID/chứng chỉ thật khi phát hành.

### macOS

- Installer ghi `AppleEnabledInputSources` để input source còn sau logout/login.
- Secure input pass-through, hotkey Ctrl+Shift+F, menu Clipboard (bỏ dấu/NFC/VIQR/TCVN3).
- Sửa `LSBackgroundOnly` đúng kiểu boolean; doctor kiểm tra enabled lẫn selected.

### Fcitx5 nâng cao

- Tách `VifastkeyState`/`ConfigStore`, status action Việt/Anh, config reload, surrounding guard,
  transaction barrier cho REPLACE và CTest; thêm C ABI `vfk_flush`, `vfk_display_text`,
  `vfk_set_output_mode`, `vfk_apply_config_json`, profile helpers.
- Theo dõi program change, drop composition khi field chuyển sang password/URL/number, cache
  text sau commit, giữ đúng thứ tự pass-key sau Replace, fallback config lỗi về mặc định.
- CTest adapter: preedit/commit, reset/focus-out, password, surrounding lệch/selection, edit
  previous word, replace mode, pass-key, emoji/UTF-16 và config lỗi.
- Thêm crate `vfk-adapter-core` dùng chung cho mọi port: field policy, surrounding guard,
  action planner, timing; C ABI `vfk_plan_action`, `vfk_char_to_utf16_offset`,
  `vfk_is_word_codepoint`. Fcitx5/Windows/macOS/iOS dùng planner; Android JNI có
  `NativeEngine.plan`; bộ `adapter-conformance/*.jsonl` giữ hợp đồng hành vi.
- IBus dùng `python/vfk_adapter_core.py` (cùng contract) và conformance Python.
- Fcitx5 nạp `user_words.txt`; release workflow thêm macOS app, Fcitx5 tarball,
  Android APK và iOS simulator app; Android có cấu hình release signing qua env.

### Chrome extension

- Thêm `chrome-extension/`: MV3, context menu chuẩn hoá/bỏ dấu vùng chọn, popup
  chuyển Telex/VNI/VIQR + bàn phím ảo; dùng WASM `normalize_text`.
- Playwright E2E: popup `tieengs` → `tiếng`, giữ `data`/`hello`, `Vieejt` → `Việt`.
- Release workflow đóng gói thêm `vifastkey-chrome-extension.zip`.

### Android full keyboard

- QWERTY đầy đủ, số, shift/caps, symbols, emoji, long-press dấu tiếng Việt.
- Suggestion bar local qua JNI `vfk-suggest`; settings đầy đủ + SharedPreferences → engine JSON.
- Hardware keyboard, cursor guard, numeric/password handling, app icon.

### Android / iOS

- Android: IME Java + JNI Rust, bàn phím QWERTY, build APK và test end-to-end
  trên emulator (`tieengs` → `tiếng`); thêm `android/runtime_test.sh` để chạy lại
  smoke test qua adb.
- iOS: host app + keyboard extension Swift, Rust staticlib, build simulator + unit test engine.

### Linux Fcitx5 adapter

- Thêm `linux/fcitx5/`: addon C++ + CMake dùng C ABI Rust; CI Ubuntu build.
- Sửa C++20 + multiarch; đã test runtime Xvfb/Fcitx5 5.1.19 (`tieengs` → `tiếng`).
- Thêm `linux/fcitx5/runtime_test.sh` để chạy lại smoke test X11/GTK trên Ubuntu
  24.04/Fcitx5 5.1.7 mà không sửa config cá nhân.

### Linux IBus adapter

- Thêm `linux/ibus/`: engine IBus Python, core `adapters.ibus.IbusCore`, install script,
  component XML; test core trên macOS/CI.
- Sửa khởi tạo engine theo `IBus.Factory`: `__init__()` không tham số và
  `GObject.__init__()`, tránh engine thiếu `core`; sửa `keyval_to_unicode` trả `str`
  (trước đây gọi `chr(str)` làm mọi key bị pass-through).
- Cài component vào `/usr/share/ibus/component` (IBus 1.5.29 không quét user component),
  thêm `IBUS_COMPONENT_DIR`; `vfk_adapter_core` được khai báo `py-modules`.
- Thêm `linux/ibus/runtime_test.sh`: IBus 1.5.29 + GTK3 X11/Xvfb,
  `tieengs` → `tiếng`.

### Windows prototype / TSF

- Thêm adapter `apps/vfk-windows`: low-level keyboard hook + SendInput Unicode,
  đọc config JSON, toggle Ctrl+Shift+F; cross-check target Windows MSVC.
- Thêm `windows/tsf/`: TSF text service C++ gọi C ABI (`vfk_process_key`,
  `vfk_plan_action`, `vfk_flush`, `vfk_set_field_kind`, `vfk_set_output_mode`).
  Preedit composition + display attribute gạch chân, surrounding text, input scope
  (password/PIN/URL/email/number), flush khi focus/caret, toggle Ctrl+Shift+F.
- Backend Windows `windows.json`: `auto`/`tsf`/`hook`; TSF ghi named mapping
  `Local\ViFastKeyTsfState`, hook pass-through theo foreground window/PID để không double input.
- Hook bỏ qua phím injected (`LLKHF_INJECTED`), không fake preedit.
- Installer per-user `windows/installer/`; CI build TSF x64/x86 bằng MSVC.
- TSF: đặt caret về cuối text sau commit (Notepad/RichEdit không còn đảo chữ);
  `ActivateEx` idempotent và tự lấy lại context khi đổi input method
  (ViFastKey ↔ Vietnamese Telex ↔ English).

### Hạ tầng

- `python/pyproject.toml`, cài được bằng `pip install -e ".[dev]"`, có entry point `vfk`.
- CI chạy Python (3.10–3.13), kiểm tra conformance không lệch, và Rust trên 3 hệ điều hành.
- Nâng GitHub Actions lên runtime Node 24/ESM: `checkout@v7`, `setup-python@v7`,
  `upload-artifact@v7`, `download-artifact@v8`, `setup-node@v7`, `setup-java@v6`,
  `configure-pages@v6`, `upload-pages-artifact@v5`, `deploy-pages@v5`,
  `action-gh-release@v3`. Giữ `gradle/actions/setup-gradle@v5` vì v6 chuyển caching
  sang component thương mại riêng.
- Thêm `LICENSE` (MIT), `CONTRIBUTING.md`, `.editorconfig`; thống nhất giấy phép Rust về MIT.
- `rust/tools/sync_conformance.sh` nhận biến `PYTHON`, không còn phụ thuộc lệnh `python`,
  đồng bộ cả bảng mã.
- Rust stable 1.99.1, dependency trực tiếp và lockfile ở bản mới nhất; thêm
  `rust-toolchain.toml` và Dependabot.
- Thêm `docs/`, `THIRD_PARTY_NOTICES.md`, `apps/vfk-charset`.
