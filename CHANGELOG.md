# Changelog

## Unreleased

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
- Sửa `đ` viết tắt (`ddc`, `vcdd`, `NDD`), undo tiếng Anh (`banana`, `coffee`, `cheese`),
  VNI literal number (`nam2024`, `ba10`), sticky restore (`text` + `s`), escape pass-key,
  open `uơ`/`ươ` theo âm đầu, nguyên âm có dấu đứng riêng (`aw`, `aws`).
- Auto-cap mở rộng cho `…`, ngoặc kép/đóng, newline; không cắt số thập phân.
- `AppProfile` ghi đè `input_method`, `tone_style`, `spell_check`; thêm gợi ý `charset`.
- `schema_version` trong config; `config_from_dict` bỏ qua field lạ.
- Module bảng mã 16 charset (TCVN3, VNI-Windows, VISCII, VIQR, CP1258, NCR, C-string...);
  FFI C/WASM/Python và CLI `vfk-charset`.
- Python FFI đầy đủ hơn: `process_char`, focus, surrounding text, field kind, toggle, reset,
  config JSON một phần, app profile, dictionary, phiên bản.
- Rust/WASM: `Config::apply_json_patch` dùng chung; C ABI giờ thật sự nhận JSON cấu hình một phần
  (trước đây JSON thiếu trường bị thay bằng mặc định).

### CLI

- `--text`, `--method`, `--tone`, `--mode`, `--nfd`, `--no-spell`, `--allow-coda-k`,
  `--macro-digits`, `--macro`, `--trace`, `--version`, `--help` cho cả bản Python và Rust.
- REPL thêm `:coda_k`, `:macro_digits`, `:spell`, `:help`; `|` = Enter.

### Web

- Playground có chế độ **Preedit** (mô phỏng TSF/IMK/Fcitx5) bên cạnh Replace.
- Cấu hình macro bằng textarea, chọn NFC/NFD, bật macro có số, âm cuối k, sửa từ vừa gõ.
- Ví dụ `Đắk Lắk` tự bật tuỳ chọn cần thiết.
- Công cụ chuyển bảng mã + hiển thị hex cho bảng một byte.

### macOS adapter

- Thêm IMK input method thật (`macos/`): Swift controller → Rust C ABI → ActionApplier.
- Preedit native, menu Telex/Simple Telex/VNI/VIQR, toggle Việt/Anh, config JSON atomic + backup.
- Build/install/doctor/self-test; app id normalization cho bundle id.
- Ad-hoc signing để test local; `package.sh` build universal2 + hook ký/notarize;
  cần Developer ID/chứng chỉ thật khi phát hành.

### Fcitx5 nâng cao

- Tách `VifastkeyState`/`ConfigStore`, status action Việt/Anh, config reload, surrounding guard,
  transaction barrier cho REPLACE và CTest; thêm C ABI `vfk_flush`, `vfk_display_text`,
  `vfk_set_output_mode`, `vfk_apply_config_json`, profile helpers.

### Chrome extension

- Thêm `chrome-extension/`: MV3, context menu chuẩn hoá/bỏ dấu vùng chọn, popup
  chuyển Telex/VNI/VIQR + bàn phím ảo; dùng WASM `normalize_text`.
- Playwright E2E: popup `tieengs` → `tiếng`, giữ `data`/`hello`, `Vieejt` → `Việt`.
- Release workflow đóng gói thêm `vifastkey-chrome-extension.zip`.

### Android / iOS

- Android: IME Java + JNI Rust, bàn phím QWERTY, build APK và test end-to-end
  trên emulator (`tieengs` → `tiếng`).
- iOS: host app + keyboard extension Swift, Rust staticlib, build simulator + unit test engine.

### Linux Fcitx5 adapter

- Thêm `linux/fcitx5/`: addon C++ + CMake dùng C ABI Rust; CI Ubuntu build.
- Sửa C++20 + multiarch; đã test runtime Xvfb/Fcitx5 5.1.19 (`tieengs` → `tiếng`).

### Linux IBus adapter

- Thêm `linux/ibus/`: engine IBus Python, core `adapters.ibus.IbusCore`, install script,
  component XML; test core trên macOS/CI.

### Windows prototype

- Thêm adapter `apps/vfk-windows`: low-level keyboard hook + SendInput Unicode,
  đọc config JSON, toggle Ctrl+Shift+F; cross-check target Windows MSVC.

### Hạ tầng

- `python/pyproject.toml`, cài được bằng `pip install -e ".[dev]"`, có entry point `vfk`.
- CI chạy Python (3.10–3.13), kiểm tra conformance không lệch, và Rust trên 3 hệ điều hành.
- Thêm `LICENSE` (MIT), `CONTRIBUTING.md`, `.editorconfig`; thống nhất giấy phép Rust về MIT.
- `rust/tools/sync_conformance.sh` nhận biến `PYTHON`, không còn phụ thuộc lệnh `python`,
  đồng bộ cả bảng mã.
- Rust stable 1.99.1, dependency trực tiếp và lockfile ở bản mới nhất; thêm
  `rust-toolchain.toml` và Dependabot.
- Thêm `docs/`, `THIRD_PARTY_NOTICES.md`, `apps/vfk-charset`.
