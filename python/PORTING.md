# Hướng dẫn port sang Rust

Bản Python là **oracle**. Bản Rust đạt yêu cầu khi chạy qua 100% `conformance/core.jsonl` và `conformance/engine.jsonl`, khớp tới từng ký tự và từng Action.

## 1. Thứ tự port (mỗi bước chạy được test riêng)

| Bước | Crate | Nguồn Python | Kiểm tra bằng |
|---|---|---|---|
| 1 | `vi-syllable` | `vi_syllable/__init__.py` | Port `tests/test_syllable.py` (bảng VALID/INVALID/PARTIAL, `TONE_TABLE`) |
| 2 | `vfk-core` | `vfk_core/rules.py`, `transform.py`, `__init__.py` | `core.jsonl` (13.311 ca) + so `Composer` với `transform` |
| 3 | `vfk-engine` | `vfk_engine/*.py` | `engine.jsonl` (3.000 kịch bản, so từng Action) |
| 4 | `vfk-ffi` | `vfk_ffi/__init__.py` | Test UTF-16 (`test_ffi_*`) |
| 5 | `vfk-cli` | `apps/vfk_cli.py` | Chạy tay, so với `--demo` |

## 2. Ánh xạ kiểu dữ liệu

```rust
// vi-syllable
pub enum Status { Valid, Partial, Invalid }
pub enum ToneStyle { Old, New }
pub enum Mark { Circumflex, Breve, Horn, Stroke }       // ^ ( + -
pub struct Syllable<'a> { onset: &'a str, cluster: &'a str, coda: &'a str, cluster_start: usize }

// vfk-core
#[derive(Clone, Copy)]
pub struct Letter { base: u8, upper: bool, mark: Option<Mark>, from_standalone: bool, auto_horn: bool }
#[derive(Clone)]
pub struct WordState {
    letters: Vec<Letter>,        // bản hiện tại; ArrayVec<Letter, 24> là tối ưu sau
    tone: u8,
    undone: bool, latched: bool, transformed: bool,
    locked: u32,                 // bitset theo phím a-z (thay cho set Python)
}
pub struct Composer { raw: Vec<char>, states: Vec<WordState>, im: &'static InputMethodSpec, opt: TransformOptions }

// vfk-engine
pub enum Action {
    PassThrough,
    Preedit { text: String, cursor: usize },
    Replace { delete_before: usize, insert: String, pass_key: bool },
    Commit { text: String, pass_key: bool },
}
```

## 3. Các bẫy khi port

1. **Thứ tự duyệt phải giống hệt Python**:
   - Ứng viên dấu phụ: cặp `ươ` trước, rồi từ phải sang trái.
   - Luật dấu trong `marks["w"]` đi theo đúng thứ tự khai báo.
   - Khử trùng lặp nhưng giữ thứ tự.
2. **`_display` chuẩn hoá móc**: `ư`+`o` thành `ươ`. `ươ` ở cuối từ mà móc ở `u` có `auto_horn` thì hiển thị `uơ`. Đây là chỗ dễ sai nhất (`thuow` ra `thuơ`, `thuowng` ra `thương`).
3. **Bảng tra sinh lúc build**: `_RELAXED_ANY` và `_RELAXED_FOR_CODA` sinh trong `build.rs` (hoặc `const fn`), rồi dùng `phf::Set<&str>`.
4. **Đơn vị độ dài**:
   - Core và engine tính theo `char` (Unicode scalar) trên chuỗi **đầu ra** (NFC hoặc NFD).
   - Lớp FFI đổi sang UTF-16.
   - Không dùng `str::len()` (vì là byte).
5. **NFC/NFD**: dùng crate `unicode-normalization`. Bên trong luôn là NFC, chỉ chuẩn hoá ở `_out()`.
6. **Hoa/thường**: viết hoa `đ` ra `Đ`. Phím chức năng viết hoa (`S`, `W`) vẫn là phím chức năng, nhưng chữ sinh ra theo kiểu hoa của chữ gốc. Riêng `W` đứng riêng thì ra `Ư`.
7. **`MAX_RAW = 24`**: khi vượt giới hạn, `Composer` ngừng tính tiếp nhưng vẫn ghi raw.
8. **`Config` trong `engine.jsonl`**: không có `app_profiles`. Dùng `DEFAULT_APP_PROFILES`, và app trong mọi kịch bản là `"app"`. Các trường `allow_coda_k`/`macro_digits` có thể thiếu trong file cũ; mặc định `false`.
9. **Tuỳ chọn chính tả**: `TransformOptions.allow_coda_k` phải được truyền vào `check_with`/`tone_allowed_with`; bản Python dùng tham số thứ ba của `check`/`tone_allowed`.
9. **`FakeTextField`**: cần port sang Rust làm test harness, vì `engine.jsonl` giả định đúng ngữ nghĩa `apply()` (PassThrough thì OS gõ phím, Backspace xoá 1 code point).

## 4. Hiệu năng

| | Python (tham khảo) | Rust (tham khảo) |
|---|---|---|
| `transform` (tính lại từ đầu) | ~22 µs/phím | ~1.0 µs/phím |
| `Composer` (tăng dần) | ~10 µs/phím | ~0.5 µs/phím |

Số đo phụ thuộc máy; chạy `python -m tools.bench` và
`cargo run -p vfk-core --release --example bench` để đo lại.

`python -m tools.bench` để đo lại. Trong Rust, `Composer` hiện dùng `Vec`; thay bằng `ArrayVec` (24/25 phần tử) là bước tối ưu tiếp theo, không đổi hành vi.

## 5. Khi muốn đổi hành vi

1. Sửa bản Python và thêm golden test.
2. Chạy `python -m tools.export_conformance` để sinh lại 2 file `.jsonl`.
3. Commit cả code lẫn `.jsonl`. Bản Rust sẽ đỏ đúng những ca bị đổi.
