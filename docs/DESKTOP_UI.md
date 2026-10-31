# UI cấu hình desktop

Tài liệu này chốt kiến trúc và hợp đồng của UI cấu hình desktop. UI dùng egui cho
Windows/Linux; macOS giữ preferences AppKit hiện có.

## 1. Quyết định

| Hạng mục | Quyết định |
|---|---|
| Tiến trình | App riêng `vifastkey-settings`, không nhúng vào hook/IMK/Fcitx5 |
| UI toolkit | `eframe`/`egui` 0.32.3 (workspace MSRV 1.89 do dependency transitive) |
| Nền tảng | Windows + Linux; macOS giữ AppKit |
| Nguồn sự thật | File JSON `config.json`; không IPC bắt buộc |
| Ghi file | Patch các field đã sửa, atomic write + backup |
| Key lạ | Giữ nguyên |
| `app_profiles` | UI chỉ ghi entry người dùng sửa; preset không bị materialize |
| Runtime state | `app_modes` do adapter sở hữu; UI chỉ xem/xoá |
| UI-only state | `settings-ui.json` (theme, tab cuối); không thêm vào schema engine |
| Ngôn ngữ UI | Tiếng Việt; **không làm i18n vi/en** |

Lý do không nhúng egui vào IME:

- AppKit/IMK và winit/egui đều muốn sở hữu event loop.
- Crash UI không được làm chết bộ gõ.
- Cùng một UI cấu hình dùng được cho nhiều adapter.
- macOS đã có UI native; thay bằng egui sẽ mất native look/accessibility.

## 2. Kiến trúc

```
config.json ◄── atomic patch ── vifastkey-settings (egui)
     ▲                                │
     │ mtime/hash                     │ vfk-engine (preview)
     ├── macOS IMK ──────────┐        │
     ├── Windows hook/TSF ───┼──► vfk-engine
     └── Fcitx5/IBus ────────┘
```

- `vfk-config` là thư viện lõi, không phụ thuộc egui.
- `vifastkey-settings` chỉ render và gọi `vfk-config`.
- Adapter tự reload khi file đổi: macOS IMK, Windows hook/TSF, Fcitx5 và IBus đều đã có
  watcher/reload (mtime hoặc focus).
- UI không giữ runtime state (Việt/Anh theo app); adapter là chủ sở hữu.

### Crate

```
rust/crates/vfk-config/
  src/paths.rs      # path theo nền tảng + VIFASTKEY_CONFIG
  src/raw.rs        # JSON gốc, fingerprint, backup file hỏng
  src/draft.rs      # bản nháp + dirty fields
  src/document.rs   # load/save/patch/merge/conflict + import preview
  src/diff.rs       # diff JSON cho preview import
  src/store.rs      # atomic write: temp -> fsync -> backup -> rename
  src/validate.rs   # lỗi/cảnh báo cross-field
  src/meta.rs       # FieldId

rust/apps/vfk-settings/
  src/main.rs       # eframe; CLI check/self-test/import/export/screenshot
  src/app.rs        # state + tabs + dialogs + tray actions
  src/tray.rs       # tray Windows/macOS (tray-icon), Linux (ksni)
  src/screenshot.rs # chụp PNG qua ViewportCommand::Screenshot
  src/theme.rs      # design/tokens.json -> egui, sáng/tối
  src/fonts.rs      # Be Vietnam Pro
  src/preview.rs    # gõ thử bằng engine
  src/platform.rs   # mở thư mục/file + windows.json
  src/ui_prefs.rs   # settings-ui.json (theme, tab, tray, close-to-tray)
```

## 3. Đường dẫn config

Giữ đúng path hiện có của adapter:

| Nền tảng | Path |
|---|---|
| macOS | `~/Library/Application Support/ViFastKey/config.json` |
| Windows | `%APPDATA%\ViFastKey\config.json` |
| Linux | `$XDG_CONFIG_HOME/vifastkey/config.json` hoặc `~/.config/vifastkey/config.json` |
| Override | `$VIFASTKEY_CONFIG` |

Không đổi chữ hoa/thường của path Linux (`vifastkey`) nếu chưa có migration.

## 4. Hợp đồng ghi file

### 4.1. Chỉ ghi field đã sửa

UI giữ baseline + draft. Khi lưu:

1. Đọc lại file hiện tại.
2. So fingerprint; nếu file đổi, merge 3 chiều.
3. Áp các field dirty:
   - scalar: ghi đè;
   - `macros`, `app_profiles`, `app_modes`: merge từng key;
   - `excluded_apps`: thay toàn bộ array.
4. Validate toàn bộ JSON bằng `vfk_engine::Config`.
5. Ghi atomic.

Không serialize `Config::default()` ra file: làm vậy sẽ ghi cứng preset
`app_profiles` và mọi default khác.

### 4.2. Key lạ

Key không thuộc schema được giữ nguyên. Điều này cần cho forward compatibility:
file do bản mới hơn tạo vẫn mở/sửa được bằng bản cũ mà không mất dữ liệu.

### 4.3. Backup và file hỏng

- Trước mỗi lần ghi: `config.json` → `config.json.bak`.
- JSON hỏng: sao lưu `config.bad.<epoch>.json`, không ghi đè; UI mở chế độ chỉ đọc.
- Ghi atomic: temp cùng thư mục, `fsync`, rename, `fsync` thư mục trên Unix.

### 4.4. Conflict

Nếu field UI sửa cũng bị người khác sửa:

- Không ghi.
- Trả `Error::Conflict { fields }`.
- UI hiện banner; người dùng chọn tải lại hoặc sửa lại.

Field không liên quan được merge tự động. Ví dụ adapter ghi `app_modes` trong khi
người dùng sửa `input_method` thì cả hai đều được giữ.

### 4.5. `app_profiles`

`Config::default()` có preset (`chrome`, `firefox`, `kitty`...). UI:

- Hiển thị union preset + user override.
- Engine merge preset + entry user theo từng app; preset không bị mất khi file chỉ có
  một vài app ghi đè.
- Chỉ ghi entry người dùng sửa.
- “Kế thừa preset” xoá entry khỏi file để quay lại preset.
- Một entry user hiện thay thế toàn bộ preset của app đó (giới hạn của schema 1).
  Merge từng field cần schema 2 và `AppProfile` dạng `Option`.
- Field lạ bên trong một `AppProfile` không được giữ khi profile đó bị sửa; field lạ
  ở top-level vẫn được giữ.

### 4.6. Import / export

- Export ghi **config hiệu lực** (preset + user) ra JSON; dùng để backup/chia sẻ.
  UI có mục “Xem JSON hiệu lực” trước khi xuất.
- Import thay thế toàn bộ bản nháp, giữ key lạ của file import; chưa ghi file cho đến
  khi người dùng bấm Lưu. File cũ được backup ở bước ghi.
- Trước khi nạp, UI hiện **preview diff** theo giá trị hiệu lực (đường dẫn field,
  trước → sau), số lỗi/cảnh báo; lỗi validate chặn nút Nhập.
- Có cả CLI headless:
  `--export <path>`, `--import <path>`.
- “Khôi phục mặc định” từng nhóm: kiểu gõ, tuỳ chọn macro, ứng dụng, nâng cao.
  Reset nhóm macro không xoá bảng macro; reset nhóm ứng dụng xoá `app_profiles` +
  `excluded_apps` nhưng giữ `app_modes` (runtime state).

### 4.7. `app_modes`

`app_modes` là runtime state do adapter ghi. UI chỉ:

- hiển thị trạng thái đã nhớ;
- cho “Quên” (xoá key).

Không cho sửa trực tiếp để tránh tranh chấp với adapter.

## 5. Information architecture

| Tab | Nội dung |
|---|---|
| Tổng quan | Trạng thái file, kiểu gõ hiệu lực, số macro/app, gõ thử, lỗi |
| Kiểu gõ | Method, tone, chính tả, Telex nâng cao, từ/câu, ngữ cảnh |
| Macro | Bật/tắt, macro số/undo/off-mode, bảng key-value, copy JSON |
| Ứng dụng | Tìm app, preset/user rule, loại trừ, output, delay/ACK, charset, app_modes |
| Nâng cao | Auto restore English, NFC/NFD, path, import/export + xem JSON, reload, reset nhóm, theme, tray, Windows backend |
| Chẩn đoán | OS/version/path/fingerprint, validate, gõ thử, copy diagnostics |

### Quy tắc disable

- `quick_telex`, `w_as_u`, `bracket_keys` chỉ bật cho Telex/Simple Telex.
- `macro_digits`, `macro_undo`, `macro_in_off_mode` chỉ bật khi `macro_enabled`.
- Không tự đổi giá trị khi disable; chỉ hiển thị cảnh báo.

### Validation

Lỗi chặn lưu:

- `input_method` không hợp lệ.
- macro key rỗng/trùng sau lowercase.
- charset không có trong `vfk_core::CHARSETS`.
- delay ngoài range.

Cảnh báo không chặn lưu:

- `macro_digits` khi macro tắt.
- `quick_telex` với VNI/VIQR.
- schema mới hơn bản hiện tại.
- app vừa bị loại trừ vừa ép bật tiếng Việt.
- app id chưa chuẩn hoá.

## 6. Gõ thử

Không bắt key trực tiếp: bộ gõ đang bật sẽ compose trước khi egui nhận event.
Thay vào đó dùng chuỗi raw key:

- `<` Backspace
- `~` Escape
- `|` Enter

`preview.rs` chạy `vfk_engine::testing::Harness` với config hiệu lực, hiển thị output
và action trace. Preview chỉ kiểm tra engine, không kiểm tra adapter/app thật.

## 7. Tích hợp nền tảng

### macOS

- `persistEngineConfig()` chỉ merge `app_modes`, không ghi toàn bộ `engine.configJSON`.
- IMK poll mtime 1 giây (`Timer` + `ConfigFile.modificationDate()`); reload engine/menu
  khi UI ghi file. Không cần NotificationCenter từ process khác.
- Giữ preferences AppKit cho macOS; đã bổ sung Unicode NFC/NFD, app loại trừ,
  auto-restore English, macro enabled, default Việt/Anh, restore sau dấu cách.
  App profile/charset nâng cao vẫn qua file JSON.

### Windows

- TSF reload config khi focus đổi.
- Hook poll mtime config + `windows.json` mỗi 500 ms; không reload khi đang có preedit.
- Tab “Windows Backend” ghi `windows.json`: `tsf` = Preedit (mặc định), `hook` = Replace;
  chọn Replace chạy hook nền + Run key, chọn Preedit dừng hook + xoá Run key.
- `Ctrl+Shift+,` trong hook mở `vifastkey-settings.exe --tab apps`; installer tạo
  Start Menu shortcut.
- Tray `tray-icon` (Win32): chuột phải mở menu (cấu hình, tải lại, thoát); click trái mở cửa sổ;
  đóng cửa sổ ẩn xuống tray nếu không có thay đổi chưa lưu.

### Linux

- Fcitx5 poll mtime 500 ms.
- IBus `IbusCore` poll mtime, reload khi không có preedit.
- Desktop entry: `linux/vifastkey-settings.desktop`.
- Wayland: nhận diện app có thể không chính xác; hiển thị cảnh báo.
- Tray dùng StatusNotifierItem qua `ksni` (thuần D-Bus, không cần GTK/AppIndicator
  trong binary). Desktop cần StatusNotifierWatcher; GNOME cần extension AppIndicator.
- Đã test UI X11 (Xvfb) và Wayland (Weston headless); tray test qua D-Bus menu
  (mở/tải lại/thoát) trên Ubuntu 26.04.

## 8. Build và chạy

```bash
cd rust
cargo run -p vifastkey-settings
cargo run -p vifastkey-settings -- --config /tmp/config.json
cargo run -p vifastkey-settings -- --tab apps
cargo run -p vifastkey-settings -- --check --config /tmp/config.json
cargo run -p vifastkey-settings -- --self-test --config /tmp/config.json
cargo run -p vifastkey-settings -- --export /tmp/vifastkey-config.json
cargo run -p vifastkey-settings -- --import /tmp/vifastkey-config.json
cargo run -p vifastkey-settings -- --screenshot /tmp/settings.png --tab advanced
cargo run -p vifastkey-settings -- --screenshots /tmp/settings-shots
cargo run -p vifastkey-settings -- --tray     # macOS mặc định tắt tray
```

Linux cần:

```bash
sudo apt-get install -y pkg-config libxkbcommon-dev libwayland-dev \
  libx11-dev libxi-dev libgl1-mesa-dev libegl1-mesa-dev
```

## 9. Test

- `cargo test -p vfk-config`: path, atomic write, backup, unknown key, macro merge,
  app profile preset, conflict, import/export, preview diff, reset từng nhóm.
- `cargo test -p vifastkey-settings`: render headless mọi tab/dialog, theme sáng/tối,
  screenshot PNG, đọc/ghi `settings-ui.json`.
- `--check`: CI/support, exit 1 nếu có lỗi.
- `--self-test`: load config + dựng UI lần lượt 6 tab, không cần cửa sổ.
- `--import` / `--export`: test headless, không cần dialog.
- `--screenshots <dir>`: chụp framebuffer 6 tab; đã chạy trên macOS, X11/Xvfb,
  Wayland (Weston headless) và Windows 11 (scheduled task trong session desktop).
- Manual: Windows DPI 150%, gõ tiếng Việt vào ô macro, file sửa bên ngoài khi UI mở,
  file read-only, file hỏng, tray menu trên GNOME/KDE.

## 10. Giai đoạn tiếp theo

| Ưu tiên | Việc |
|---|---|
| P1 | ✅ Tray icon; ✅ test UI Linux (X11 + Wayland headless); ✅ Windows 11 render/close-to-tray; còn tương tác menu tray Windows |
| P1 | ✅ Import preview diff; ✅ reset từng nhóm |
| P2 | `app_profiles` merge từng field (schema 2) |
| P2 | Theme theo hệ thống; accessibility audit; diff chi tiết trong export |
| P3 | Windows native shell (Win32/WinUI) dùng chung `vfk-config` nếu cần |
| P3 | macOS SwiftUI preferences dùng chung validation qua C ABI |

## 11. Không làm

- Không nhúng egui vào tiến trình IME.
- Không ghi toàn bộ `Config` từ adapter/UI.
- Không thêm field UI vào `config.json`.
- Không tự động xoá macro/dictionary khi reset settings.
- Không log nội dung người dùng gõ hoặc giá trị macro.
- Không làm i18n vi/en; UI và tài liệu dùng tiếng Việt.
