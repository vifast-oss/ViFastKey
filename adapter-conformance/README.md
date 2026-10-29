# Adapter conformance

Bộ scenario dùng chung cho mọi port (Fcitx5, macOS IMK, iOS, Android, Windows,
IBus). Mục tiêu: cùng một Action + cùng một surrounding snapshot phải cho ra
cùng một quyết định, bất kể port viết bằng C++, Swift, Kotlin, Java, Rust hay
Python.

## Vì sao có bộ này

Các port từng xử lý `Replace` khác nhau:

- Fcitx5 kiểm tra snapshot/selection/mid-word;
- macOS/iOS gọi `replaceText` trực tiếp;
- Android gọi `deleteSurroundingText` trực tiếp;
- Windows gửi Backspace trực tiếp;
- IBus gọi `delete_surrounding_text` trực tiếp.

Cùng một snapshot lệch có thể làm mất chữ ở port này nhưng không ở port khác.
`vfk-adapter-core` gom logic; bộ JSON này giữ hợp đồng hành vi.

## Schema

Mỗi dòng là một JSON object:

```json
{
  "name": "safe-suffix",
  "field": "text",
  "before": "tiếng",
  "surrounding": { "text": "tiếng", "cursor": 5, "anchor": 5 },
  "allow_unverified_replace": false,
  "action": {
    "kind": "replace",
    "delete_utf16": 3,
    "insert": "ềng",
    "pass_key": false
  },
  "expected": {
    "kind": "replace",
    "delete_utf16": 3,
    "delete_chars": 3,
    "text": "ềng",
    "pass_key": false
  }
}
```

- `field`: `text | password | number | email | url | terminal`.
- `before`: text engine tin là đang hiển thị trước phím.
- `surrounding`: snapshot đầy đủ; `cursor`/`anchor` tính bằng **UTF-16 code
  unit**. `null` nếu app không hỗ trợ.
- `allow_unverified_replace`: chỉ `true` cho port tự quản lý toàn bộ vùng text
  (Windows hook).
- `action.kind`: `pass | preedit | commit | replace`.
- `action.delete_utf16`: số UTF-16 code unit cần xoá (với `replace`).
- `expected.kind`: `pass | preedit | commit | replace | fail_safe`.

## Quy ước quan trọng

- `delete_utf16` dùng cho API UTF-16: macOS `NSRange`, Android
  `deleteSurroundingText`, Windows text model.
- `delete_chars` dùng cho API codepoint: Fcitx5/IBus `deleteSurroundingText`
  (UCS4).
- `fail_safe` nghĩa là **không xoá gì**; adapter reset engine và để phím gốc đi
  tiếp, hoặc forward key nếu key đã bị chặn.
- Selection (`cursor != anchor`) và caret giữa từ phải fail-safe.
- Emoji/astral phải xoá đúng 1 codepoint nhưng 2 UTF-16 code unit.

## Cách dùng

- Rust: crate `vfk-adapter-core`, test
  `rust/crates/vfk-adapter-core/tests/conformance.rs`.
- C/C++/Swift/Kotlin/Java: gọi C ABI `vfk_plan_action`, rồi so sánh với
  `expected`.
- Python: có thể gọi C ABI qua `ctypes` hoặc mirror đúng bảng scenario này.

Khi thêm một best practice mới (ví dụ selection guard của Android), thêm
scenario vào đây trước, sau đó cập nhật planner và các port.
