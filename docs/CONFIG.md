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
| `tone_style` | `"old" \| "new"` | `"old"` | `hòa` / `hoà` |
| `spell_check` | bool | `true` | Chặn biến đổi sai chính tả, khôi phục raw |
| `w_as_u` | bool | `true` | Telex `w` đứng riêng → `ư` |
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
| `smart_caps` | bool | `false` | 2 chữ hoa đầu từ thì gõ thẳng (`JSON`, trừ `DD`) |

### Đầu ra

| Trường | Kiểu | Mặc định | Ý nghĩa |
|---|---|---|---|
| `unicode_form` | `"NFC" \| "NFD"` | `"NFC"` | Dạng Unicode đầu ra |
| `schema_version` | int | `1` | Phiên bản tài liệu cấu hình |

## 2. `app_profiles`

Mỗi app là một object; khoá được chuẩn hoá lowercase, bỏ đường dẫn/`.exe`/`.app`.

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

## 6. Migration

- File cũ thiếu `schema_version`/`macro_digits`/`macro_undo`/`allow_coda_k`: nhận mặc định.
- File mới hơn (schema_version > 1): vẫn đọc các field đã biết, bỏ qua field lạ.
- Khi đổi hành vi có chủ đích: thêm golden, chạy `python -m tools.export_conformance`,
  sync sang Rust, cập nhật `COMPATIBILITY.md` và `CHANGELOG.md`.

## 7. Windows adapter

Windows dùng thêm `%APPDATA%\ViFastKey\windows.json` (không thuộc engine config):

```json
{ "backend": "auto" }
```

| Giá trị | Ý nghĩa |
|---|---|
| `auto` | TSF xử lý khi profile active; hook pass-through khi TSF báo đang giữ foreground window/PID |
| `tsf` | Hook luôn pass-through; chỉ TSF preedit |
| `hook` | TSF không ăn phím; chỉ hook Replace |

Chi tiết build/cài: `windows/tsf/README.md`.
