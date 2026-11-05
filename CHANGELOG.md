# Changelog

## 0.1.2 — 2026-11-05

### Kiến trúc

- Gom policy thử lại `Replace` khi surrounding cũ vào `vfk-adapter-core`
  (`plan_action_with_fallback` / `vfk_plan_action_with_fallback`); IBus, Fcitx5,
  Wayland và macOS IMK không còn tự gọi planner lần hai. Fallback không bỏ qua
  selection hoặc caret giữa từ.
- App profile/alias chuyển sang nguồn chung `python/vfk_engine/app_profiles.json`;
  Python đọc lúc import, Rust `include_str!` lúc compile. Thêm conformance cho
  profile và terminal soft-reset.
- Thêm helper C ABI `vfk_char_to_byte_offset` / `vfk_utf16_to_byte_offset` để
  Fcitx5 không lặp thuật toán UTF-8/UTF-16.

### Core & engine

- **Bỏ cơ chế dictionary auto-restore tiếng Anh** (`auto_restore_english`): từ Việt hợp lệ
  luôn thắng (`mix` → `mĩ`, `too` → `tô`, `this` → `thí`); gõ tiếng Anh bằng escape Telex
  (`mixx` → `mix`, `thiss` → `this`, `lasst` → `last`) hoặc chuyển chế độ Anh. Config cũ
  còn khoá này vẫn đọc được, khoá đó bị bỏ qua.
- Kiểm tra cuối từ chặt hơn: âm cuối tắc (`c`, `ch`, `p`, `t`, `k`) phải có thanh sắc/nặng.
  `data`/`boot`/`meet` trả raw thay vì `dât`/`bôt`/`mêt`; `last` vẫn ra `lát`.
- Thêm `tô` vào danh sách từ Việt thông dụng (dùng cho backspace/gợi ý).
- Backspace sau khi gõ thừa ký tự vào từ Việt khôi phục lại từ đúng:
  `nhưng` + `l` → `nhuwngl`, Backspace → `nhưng` (trước đây ra `nhuwng`).
- Giữ trạng thái soft-reset qua từ mới: gõ sai một từ rồi xoá hết từ đó + dấu cách
  sẽ khôi phục từ trước để sửa tiếp (áp dụng cho cả Preedit và Replace).
- **Terminal Linux**: tắt soft-reset sau dấu cách cho xterm/ptyxis/gnome-terminal/
  konsole... vì terminal bỏ qua `delete_surrounding_text`; gõ sai rồi Backspace +
  phím mới ra `hoaf` thay vì append `hoaà`.

### Desktop

- **Windows tray**: menu “Mở cấu hình” giờ khôi phục cửa sổ Settings đúng khi
  cửa sổ đang thu nhỏ/ẩn. Trước đây đóng xuống tray dùng `Visible(false)` làm
  event loop có thể ngừng render, nên thao tác từ tray không được xử lý.
- **Windows Settings**: cửa sổ cấu hình không còn chiếm taskbar/Alt+Tab; mở từ
  tray vẫn hiện bình thường. Tab Ứng dụng hiển thị danh sách app đã cấu hình và
  picker **app đang chạy** (process + tiêu đề cửa sổ), chuẩn hoá `excluded_apps`
  để UI/engine khớp nhau. Sidebar có icon cho từng tab.
- **macOS Preferences**: thêm chọn app đang chạy để đưa vào `excluded_apps`,
  thay vì chỉ nhập tay.
- **Linux Settings**: tab Ứng dụng có picker tiến trình GUI (đối chiếu `.desktop`),
  cửa sổ dùng app_id `vifastkey-settings` để khớp icon dock với desktop entry.
  Bấm X ẩn hẳn cửa sổ xuống tray (không còn kẹt trên dock/taskbar Ubuntu); nếu
  tray không khả dụng thì app thoát bình thường.
- **Adapter Wayland standalone (`linux/wayland/`)**: `zwp_input_method_v2` +
  `zwp_virtual_keyboard_v1`, preedit/commit/delete surrounding, field policy
  (password/URL/email/number/terminal), pass-through phím qua virtual keyboard,
  config reload, `--probe`/`--check`, systemd user service. Test unit 8 case và
  E2E Sway headless + GTK4 + `wtype` trên Ubuntu 24.04 aarch64 và Ubuntu 26.04
  x86_64 (Sway 1.11): telex, dấu, Escape, toggle, replace từ trước, pass-through.
- **Fcitx5/IBus fail-safe giữ từ**: khi `Replace` không có surrounding hoặc app
  không nhận `delete_surrounding_text`, adapter commit chữ đang soạn thay vì
  reset làm mất từ; thêm test CTest và test `_apply` với IBus giả lập.
- **Windows hook-only hoàn thiện**: một binary UI + hook; HKL guard EN-US, VI/JP pass-through;
  icon Logo/E/mờ, toast, click tray toggle, double-click mở UI, single-instance UI, watchdog hook;
  X ẩn xuống tray, Quit dừng hook; installer gỡ TSF legacy. Đã rebuild package Windows/Linux/macOS,
  Android, iOS simulator, IBus, Fcitx5, WASM, Chrome extension và CLI; cập nhật website/INSTALL.
- **Windows chuyển sang hook-only**: TSF không còn build/đóng gói mặc định; hook chạy
  trong cùng `vifastkey-settings.exe` (`--hook`), chỉ xử lý khi input source foreground
  là English US (HKL 0x0409). Vietnamese/Japanese/ngôn ngữ khác pass-through để tránh
  gõ đôi. Tray đổi Logo/E/logo mờ; toast khi đổi chế độ hoặc nhường IME Windows;
  `Alt+,` chỉ hoạt động khi hook sở hữu English US. Thêm `docs/INSTALL.md` cho mọi nền tảng.

- **Hotkey Việt/Anh mặc định `Alt+,`** trên Windows TSF/Replace, macOS IMK,
  Fcitx5 và IBus; tránh `Ctrl+Shift+F` (tìm kiếm toàn project trong IDE) và
  `Ctrl+Space` (chuyển input method của hệ điều hành).
- **macOS IMK**: không còn gọi `TISRegisterInputSource`/`TISEnableInputSource` ở mỗi lần
  app khởi động — một nguyên nhân hộp thoại “Allow ViFastKey to enable ViFastKey?”
  hiện lặp lại. `install.sh` chỉ đăng ký một lần, không ký ad-hoc lại sau khi copy và
  kill process cũ; `build.sh` tự dùng Apple Development/Developer ID nếu có; thêm
  `--status` và kiểm tra chữ ký/crash report trong `doctor.sh`. Buffer preedit
  chuyển sang `NSAttributedString` để Notes/NSTextView rich text hiện chữ.
  Mặc định **Replace** (không gạch chân) giống Android/Windows; **Preedit**
  (gạch chân) là opt-in trong Cấu hình → “Chữ đang gõ”/`output_mode` cho app
  xử lý replacement range không ổn định; terminal/Electron/Firefox luôn Preedit.
  ZIP phát hành giờ tự chứa `install.sh`/`uninstall.sh`/README; sửa hướng dẫn
  web/docs (đường dẫn `~/Library/Input Methods`, hộp thoại Allow, không cài lại
  nhiều lần).
- **Sửa lặp từ Preedit khi sửa từ đã commit**: commit bằng dấu cách, xoá dấu cách
  và ký tự cuối, gõ lại rồi xoá tiếp từng ra `ViFastKeyViFastKe`; giờ giữ đúng
  `ViFastKe` và vẫn dùng Replace khi chỉnh từ đã commit.
- **Title bar Settings** dùng ASCII `ViFastKey Settings` để không bị ô vuông khi
  font titlebar thiếu glyph `ấ`; chữ tiếng Việt trong app vẫn dùng Be Vietnam Pro.
- **IBus/Xwayland**: installer đồng bộ `IBUS_ADDRESS` X11 cũ sang daemon Wayland
  đang chạy, tránh app X11 đọc address của daemon đã chết và mất gõ tiếng Việt.
- **Windows Terminal**: TSF per-user không được Windows Terminal nạp; hook Replace
  tự bật khi foreground là terminal/console, còn app thường vẫn dùng Preedit/TSF.
  Hook giữ đúng Shift/Caps Lock: `OS`/`Os` không bị thành `os`, `DD` ra `Đ`.
- **Settings**: nút X ẩn xuống tray mặc định (có thể tắt trong Nâng cao để X thoát);
  title bar dùng ASCII `ViFastKey Settings` để tránh lỗi font `ấ` trên một số desktop Linux.
- **Settings Windows**: ẩn cửa sổ console của `tasklist`/`taskkill`/`reg` khi khởi
  động, không còn nháy console trước khi giao diện hiện lên.
- **Webtop/RDP**: tài liệu hoá quy tắc chỉ bật IME ở một đầu; Windows IME đang bật
  sẽ nuốt composition trước khi browser gửi phím sang Linux.
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

### UEFI

- **Bản demo UEFI x86_64 đầu tiên**: app `no_std` chạy trước OS, dùng lại
  `vfk-core`/`vi-syllable`; editor Telex/VNI, English escape, preedit, GOP +
  font tiếng Việt nhúng; logo + URL trang chủ ở header, QR trong `F1`;
  `F2` Việt/Anh, `F3` Telex/VNI, `F10` thoát.
- Đóng gói `vifastkey-uefi-x86_64.zip` (giải nén vào USB FAT32), thêm vào release
  CI, self-test 15 case trong QEMU/OVMF và UI smoke test.
- Trang tải có tab hướng dẫn cài chi tiết cho từng nền tảng, gồm UEFI; mirror
  `downloads/0.1.0/` và mapping thống kê lượt tải UEFI.

## 0.1.0 — 2026-10-31

### Phát hành

- Bản phát hành chính thức đầu tiên `v0.1.0`, kèm GitHub Release đa nền tảng,
  `sha256sums.txt` và build Windows TSF x64/x86.
- Landing page có link tải Windows, release notes, mã kiểm tra và thống kê lượt bấm tải
  theo nền tảng (`POST /api/download`, `/api/stats`, `/admin`).
- CI release kiểm tra tag khớp version workspace trước khi publish.

### Web

- Thêm mục **Cấu hình theo app** với ảnh minh hoạ Windows (tab Ứng dụng),
  macOS (chọn app đang chạy để loại trừ) và Linux (tab Ứng dụng); card Build 27
  dẫn tới mục này.
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

- QWERTY đầy đủ, shift/caps, symbols, emoji, long-press dấu tiếng Việt; hàng số nằm
  trong trang `?123` thay vì chiếm một hàng riêng.
- Kích thước bàn phím theo Gboard: phím chữ ~35dp x 47dp, gap ngang ~5,3dp,
  gap dọc ~11,8dp, home-row căn giữa, chữ 24sp; phím space hiển thị `ViFastKey`.
- 3 layout theo Gboard: chữ; bàn phím số (+, -, *, /, %, 0, =) và trang ký tự
  (số + ký tự, phím `=\\` / `12/34`, Backspace). Hàng qwerty có số nhỏ
  `1..0` ở góc trên phải; giữ phím ra số ngay.
- Nhớ panel gần nhất: chữ → `?123` mở lại panel vừa dùng (mặc định ký tự đặc biệt);
  `12/34` / `!?#` chuyển giữa bàn phím số và ký tự.
- **Bảng nhớ tạm**: lịch sử 20 clip local, menu bo góc; dán, chọn tất cả,
  sao chép, cắt, xoá lịch sử. Chỉ đọc clipboard khi mở menu. Thêm nút ✕,
  đóng khi chạm ra ngoài hoặc chạm lại nút 📋.
- Phím phẩy giữ cỡ chữ 20sp (emoji nhỏ phía trên) và nhích lên trên; menu clipboard
  nằm dưới thanh trên để không che icon, đóng bằng ✕/chạm ngoài/chạm lại 📋.
- Emoji chuyển sang GridView lazy, chỉ render ô nhìn thấy và hiện 3,5 hàng
  (lộ nửa hàng kế tiếp) để biết còn cuộn.
- Long-press nguyên âm mở popup biến thể bo góc: mặc định là số (`e` → `3`),
  kéo ngang để chọn dấu (`ê/é/...`), thả tay lấy mục highlight.
- Gõ xong ký tự đặc biệt tự quay về layout chữ; số vẫn giữ bàn phím số.
- **VNI**: layout chữ tự thêm hàng số `1–0` khi chọn VNI để gõ dấu/âm VNI ngay.
- **Settings**: thanh Lưu cấu hình cố định ở đáy màn hình, nút cam luôn thấy,
  kèm ghi chú cấu hình áp dụng ngay sau khi lưu.
- **Emoji hay dùng**: hàng “Hay dùng” ở đầu bảng emoji, xếp theo tần suất và lần
  dùng gần nhất, lưu local; nhãn bên trái, các icon gợi ý bên phải.
- **Tab emoji**: bỏ hàng gợi ý trống, hàng “Hay dùng” sát thanh trên; lưới luôn
  lộ một phần hàng kế tiếp kèm scrollbar.
- **Emoji**: chỉ rung/âm thanh khi thực sự chọn emoji; chạm/kéo để cuộn không rung.
- **Emoji**: mở rộng lên 1.121 biểu tượng (mặt, người, động vật, thiên nhiên,
  đồ ăn, du lịch, hoạt động, đồ vật, biểu tượng, cờ).
- **App ghi chú tự chốt composing text**: IME chỉ `finishComposingText()` và
  không commit lại preedit; sửa lỗi `Nguyễn` bị nhân đôi thành `NguyễnNguyễn`.
- **Độ cao tab emoji**: khớp chiều cao layout chữ/ký tự/số đang dùng, kể cả VNI;
  hàng “Hay dùng” co lại và lưới tự bù, không còn giật khi chuyển tab.
- **Settings Android**: thêm logo ViFastKey và dòng “Phiên bản 0.1.0 · build”;
  tùy chọn nâng cao “Tự về bàn phím chữ sau ký tự đặc biệt” mặc định tắt, ai quen
  quay về chữ có thể bật.
- **Hàng số Android**: thêm tùy chọn “Hiện hàng số 1–0 trên bàn phím chữ”.
  Telex/Simple Telex/VIQR mặc định tắt và có thể bật; VNI luôn bật, khóa tắt để
  không mất hàng số khi gõ dấu/âm VNI.
- **Chiều cao bàn phím Android**: thêm mức 100/115/130/145% trong Settings →
  Bàn phím; áp dụng cho chữ, ký tự, số, emoji và hàng số.
- **Cài đặt nhanh trong bàn phím**: nút 4 ô mở panel đổi nhanh chiều cao, bật/tắt
  hàng số và mở Cài đặt đầy đủ.
- **Build Android**: `android/build.sh` build lại `vfk-jni` từ core trước khi đóng
  APK, tránh dùng `.so` cũ trong `jniLibs` sau khi sửa engine/core.
- **Core Backspace**: gõ `thoảng`, thêm `e`, xoá `e` quay lại `thoảng`; khôi phục
  theo âm tiết Việt hợp lệ thay vì chỉ từ điển common. Áp dụng mọi adapter.
- **Android gạch chân**: mặc định dùng Replace/commit (không gạch chân); tùy chọn
  “Hiện gạch chân chữ đang gõ” trong Nâng cao để quay lại preedit.
- **Android Replace**: gộp xoá + commit trong `beginBatchEdit/endBatchEdit`, bỏ
  composing rỗng ở chế độ không gạch chân; suggestion/autocorrect cũng batch để
  giảm nháy khi gõ nhanh.
- **Android layout theo field**: vào app/field mới tự reset về bàn phím chữ
  (hoặc bàn phím số nếu field số), không giữ nhầm tab ký tự đặc biệt như trước;
  quay lại cùng field vẫn giữ layout đang dùng.
- **Android Replace atomic**: API 34+ dùng `InputConnection.replaceText()` để thay
  cả range trong một lệnh; fallback batch delete+commit cho API cũ hoặc field dài
  hơn 4096 ký tự.
- **Android mở lại bàn phím**: mỗi lần input view hiện lại đều về layout chữ
  (hoặc bàn phím số cho field số), không giữ trang ký tự đặc biệt của phiên trước.
- Phím phẩy giữ nguyên vị trí nút, chỉ nhích dấu phẩy bên trong lên; xoá highlight
  của phím sau khi chọn số/dấu từ popup.
- Nút phẩy có emoji; giữ phím phẩy mở emoji. Bỏ nút VI ở hàng dưới, dùng globe
  giống Gboard (VI vẫn ở thanh trên).
- Emoji tăng lên 240, cuộn dọc thay vì lật trang; chọn emoji xong về layout chữ.
- Chọn dấu long-press xong cũng về layout chữ; chọn số giữ nguyên bàn phím số.
- Vẽ lại icon vector cho Shift, Backspace, Enter; icon đổi màu theo theme và trạng thái shift/caps.
- Touch target phủ kín gap giữa các phím (nền vẽ inset): gõ nhanh/chạm giữa hai phím
  chọn phím gần nhất thay vì mất phím; vẫn là tap-only, chưa có glide/swipe typing.
- Backspace xoá được vùng text bôi đen; giữ Backspace lặp nhanh dần 65ms → 22ms.
- Sửa Backspace ký tự cuối bị "khựng": preedit rỗng phải xoá composing region
  (`setComposingText("")`) thay vì `finishComposingText()` commit lại chữ vừa xoá.
- Shift và Backspace có rung + âm thanh như phím chữ; giữ Backspace vẫn lặp xoá.
- Rung + âm thanh phát ngay khi chạm phím; cập nhật shift/labels và suggestion bar
  theo kiểu tái sử dụng view để giảm giật khi gõ nhanh.
- Tự bật shift ở đầu field/câu hoặc sau `.` `!` `?` `…` + khoảng trắng.
- Suggestion bar local qua JNI `vfk-suggest`; settings đầy đủ + SharedPreferences → engine JSON;
  thêm sửa lỗi gần đúng (edit distance 1–2 trên từ thông dụng) khi không có gợi ý theo tiền tố.
- Tự sửa lỗi gõ nhầm phím (mặc định bật, tắt trong Settings): chỉ commit khi từ không
  hợp lệ và có đúng một ứng viên tiếng Việt thông dụng, giữ chữ hoa đầu từ;
  ví dụ `tieengd` + space → `tiếng `.
- Chọn gợi ý tự thêm dấu cách; gõ dấu câu sát nghĩa (`. , ! ? : ; … ) ] }`) tự xoá
  dấu cách vừa thêm.
- Android mặc định `w` → `ư`, khớp trạng thái checkbox trong Settings.
- Khi app gửi tin nhắn/xoá field lúc còn preedit, IME xoá composition thay vì commit
  lại thành chữ sót (ví dụ `:v`).
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
