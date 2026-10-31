# Cấu hình ViFastKey

Cấu hình là JSON, dùng chung cho Python, Rust, C ABI và WASM. File mẫu:
[`python/config.example.json`](../python/config.example.json).

- `schema_version` hiện là **1**. File thiếu trường được hiểu là 1.
- Field lạ bị bỏ qua (forward-compatible); JSON một phần được merge khi cập nhật qua
  `vfk_set_config_json`/`WasmSession.set_config`.
- File hỏng/không đọc được → dùng `Config()` mặc định (không làm hỏng IME).

## 1. Bảng trường

### Kiểu gõ

| Trường | Kiểu | Mặc định | Ý nghĩa |
|---|---|---|---|
| `input_method` | `"telex" \| "simple_telex" \| "vni" \| "viqr"` | `"telex"` | Kiểu gõ; giá trị lạ fallback Telex |
| `tone_style` | `"old" \| "new"` | `"new"` | `hoà` (mặc định) / `hòa` |
| `spell_check` | bool | `true` | Chặn biến đổi sai chính tả, khôi phục raw |
| `w_as_u` | bool | `false` | Telex `w` đứng riêng → `ư`; mặc định giữ `w` là chữ thường |
| `bracket_keys` | bool | `false` | `[` → `ơ`, `]` → `ư` |
| `quick_telex` | bool | `false` | `cc→ch`, `gg→gi`, `nn→ng`... |
| `allow_coda_k` | bool | `false` | Âm cuối `k` cho tên riêng (`Đắk Lắk`) |

### Sau khi kết thúc từ

| Trường | Kiểu | Mặc định | Ý nghĩa |
|---|---|---|---|
| `auto_restore_english` | bool | `true` | Dùng `Dictionary` để trả raw từ tiếng Anh |
| `macro_enabled` | bool | `true` | Bật macro |
| `macro_digits` | bool | `false` | Giữ chữ số trong từ (`mp3`, `k8s`) |
| `macro_undo` | bool | `true` | Backspace ngay sau macro khôi phục trigger + dấu cách |
| `macro_in_off_mode` | bool | `false` | Vẫn mở macro khi đang tắt tiếng Việt |
| `macros` | object | `{}` | `{"ko":"không","vn":"Việt Nam"}` |
| `auto_capitalize` | bool | `false` | Viết hoa đầu câu |

### Ngữ cảnh

| Trường | Kiểu | Mặc định | Ý nghĩa |
|---|---|---|---|
| `excluded_apps` | string[] | `[]` | App luôn tắt tiếng Việt |
| `app_modes` | object | `{}` | Nhớ Việt/Anh theo app (`{"chrome":false}`) |
| `default_vietnamese` | bool | `true` | Chế độ mặc định |
| `edit_previous_word` | bool | `true` | Sửa từ vừa gõ bằng surrounding text (REPLACE) |
| `app_profiles` | object | preset | Gợi ý/hành vi theo app, xem dưới |
| `restore_after_space` | bool | `true` | Backspace sau dấu cách nạp lại từ trước |
| `smart_caps` | bool | `false` | 2 chữ hoa đầu từ thì gõ thẳng (`JSON`, trừ các cặp tạo chữ Việt: `DD`, `AA`, `AW`, `EE`, `OO`, `OW`, `UW`) |

### Đầu ra

| Trường | Kiểu | Mặc định | Ý nghĩa |
|---|---|---|---|
| `unicode_form` | `"NFC" \| "NFD"` | `"NFC"` | Dạng Unicode đầu ra |
| `schema_version` | int | `1` | Phiên bản tài liệu cấu hình |

Ghi chú URL/omnibox: field URL vẫn cho gõ tiếng Việt (ô địa chỉ trình duyệt
đồng thời là ô tìm kiếm), nhưng engine bật chế độ URL-safe: tắt macro,
`w_as_u`, `quick_telex`, `bracket_keys`. Từ đứng ngay sau `.`, `@`, `/`, `:`,
`-`, `_` được giữ nguyên để không phá domain/email/path (`vietnam.vn`,
`foo.bar`, `nguyen.van@example.com`, `src/main.rs`).

## 2. `app_profiles`

Mỗi app là một object; khoá được chuẩn hoá lowercase, bỏ đường dẫn/`.exe`/`.app`.
File người dùng chỉ cần chứa app ghi đè; preset còn lại vẫn được giữ. Một entry user
hiện thay thế toàn bộ entry preset cùng app (schema 1).

| Trường | Kiểu | Ý nghĩa |
|---|---|---|
| `output_mode` | `"preedit" \| "replace" \| null` | Gợi ý cho adapter |
| `backspace_delay_ms` | int | Trễ cơ sở giữa các backspace giả |
| `delay_per_char_ms` | int | Trễ cộng thêm theo số ký tự xoá |
| `wait_ack` | bool | Chờ app xử lý backspace rồi mới chèn |
| `edit_previous_word` | bool \| null | Ghi đè quyền sửa từ cũ |
| `vietnamese` | bool \| null | Ép bật/tắt tiếng Việt |
| `input_method` | string \| null | Ghi đè kiểu gõ theo app |
| `tone_style` | `"old" \| "new" \| null` | Ghi đè kiểu đặt dấu |
| `spell_check` | bool \| null | Ghi đè kiểm tra chính tả |
| `charset` | string \| null | Gợi ý bảng mã cho adapter |

Ví dụ:

```json
{
  "app_profiles": {
    "kitty":         { "output_mode": "preedit", "edit_previous_word": false },
    "chrome":        { "wait_ack": true, "delay_per_char_ms": 2 },
    "firefox":       { "backspace_delay_ms": 4, "edit_previous_word": false },
    "code":          { "input_method": "vni", "tone_style": "new" },
    "minecraft":     { "vietnamese": false }
  }
}
```

Thứ tự ưu tiên khi resolve: field policy (password...) → user app profile → preset có sẵn →
global config → default. Xem `ADAPTER_GUIDE.md`.

## 3. Macro

- Key được lowercase khi tra; giá trị giữ nguyên.
- Macro chạy ở mọi dấu kết thúc từ (space, Enter, Tab, dấu câu).
- `macro_digits=true` cần `macro_enabled=true`; VNI không bị ảnh hưởng.
- Macro undo chỉ có hiệu lực ngay sau macro + dấu cách; Backspace khôi phục trigger + dấu cách.
- Macro nhiều từ dùng lịch sử câu (key có thể chứa dấu cách), REPLACE.
- `macro_in_off_mode` cho phép macro khi đang Tắt tiếng Việt.
- Chưa hỗ trợ key có dấu câu bên trong hoặc suffix trong một từ.

## 4. Bảng mã

`charset` trong AppProfile chỉ là gợi ý cho adapter. Engine luôn trả Unicode; adapter chuyển
ở đầu ra bằng `vfk_core.charset` / `vfk_core::charset` / `vfk-charset` CLI.

Các key: `unicode`, `unicode_nfd`, `tcvn3`, `vni_windows`, `cp1258`, `viqr`, `viscii`, `vps`,
`bkhcm1`, `bkhcm2`, `vietware_x`, `vietware_full`, `ncr_decimal`, `ncr_hex`,
`cstring_hex`, `cstring_decimal`.

## 5. FFI / API cấu hình

```c
bool vfk_set_config_json(VfkSession *s, const char *json);   // merge một phần
const char *vfk_config_json(VfkSession *s);
uint64_t vfk_config_version(VfkSession *s);                   // tăng khi engine tự lưu
const char *vfk_app_profile_json(VfkSession *s);
```

Python tương đương: `vfk_set_config_json`, `vfk_config_json`, `vfk_config_version`,
`vfk_app_profile_json`.

Rust UI/adapter dùng `vfk-config` (`rust/crates/vfk-config`) để load/merge/validate/ghi
atomic; crate này dùng `vfk_engine::Config` làm schema, không định nghĩa lại field.

## 6. Migration

- File UTF-8 có BOM (PowerShell 5.1/editor Windows) được chấp nhận; khi UI ghi lại sẽ không còn BOM.

- File cũ thiếu `schema_version`/`macro_digits`/`macro_undo`/`allow_coda_k`: nhận mặc định.
- File mới hơn (schema_version > 1): vẫn đọc các field đã biết, bỏ qua field lạ.
- Khi đổi hành vi có chủ đích: thêm golden, chạy `python -m tools.export_conformance`,
  sync sang Rust, cập nhật `COMPATIBILITY.md` và `CHANGELOG.md`.

## 7. Windows adapter

Windows dùng thêm `%APPDATA%\ViFastKey\windows.json` (không thuộc engine config):

```json
{ "backend": "tsf" }
```

| Giá trị | Ý nghĩa |
|---|---|
| `tsf` (mặc định) | Chỉ TSF preedit; hook pass-through |
| `auto` | TSF xử lý khi profile active; hook pass-through khi TSF báo đang giữ foreground window/PID |
| `hook` | TSF không ăn phím; chỉ hook Replace |

Chi tiết build/cài: `windows/tsf/README.md`.

## 8. UI desktop và ghi config

UI cấu hình desktop dùng chung file JSON với mọi adapter. Hợp đồng ghi:

- UI chỉ ghi field người dùng sửa, không serialize toàn bộ `Config`.
- `macros`, `app_profiles`, `app_modes` merge từng key; `excluded_apps` thay toàn bộ array.
- Key lạ được giữ nguyên (forward-compatible).
- Ghi atomic, tạo `config.json.bak` trước khi thay; file hỏng được sao lưu
  `config.bad.<epoch>.json` và không bị ghi đè.
- Nếu field UI sửa cũng bị adapter sửa, UI báo conflict thay vì ghi đè.
- Adapter chỉ được ghi runtime key (`app_modes`), không ghi toàn bộ config.

`app_profiles` trong file người dùng chỉ nên chứa app đã ghi đè. Preset mặc định nằm
trong engine; UI merge preset khi hiển thị/validate, không materialize vào file.

Import thay thế toàn bộ document và giữ key lạ của file import; chỉ ghi khi bấm Lưu.
Trước khi nạp, UI hiện preview diff theo giá trị hiệu lực và chặn nếu có lỗi validate.
“Khôi phục mặc định” có thể áp dụng cho từng nhóm (kiểu gõ, tùy chọn macro, ứng dụng,
nâng cao); nhóm macro không xoá bảng macro, nhóm ứng dụng giữ `app_modes`.
Theme/tab/tray/bật-ẩn của UI nằm trong `settings-ui.json` cùng thư mục, không thuộc
schema engine.

Chi tiết UI, tab, đường dẫn và platform: [`DESKTOP_UI.md`](DESKTOP_UI.md).
