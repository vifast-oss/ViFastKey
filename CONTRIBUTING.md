# Đóng góp cho ViFastKey

Cảm ơn bạn đã quan tâm! Repo có hai bản engine và một landing page:

| Thư mục | Vai trò | Ngôn ngữ |
|---|---|---|
| `python/` | **Oracle**: đặc tả chạy được, mọi thay đổi hành vi bắt đầu ở đây | Python 3.10+ |
| `rust/` | Port hiệu năng cao, C ABI, WASM, CLI | Rust 1.75+ |
| `web/` | Landing page + playground chạy WASM | HTML/CSS/JS |
| `design/`, `resources/` | Token thiết kế, logo, font | — |

## Quy trình khi đổi hành vi

1. **Sửa bản Python trước.** Thêm test vào `python/tests/` (hoặc golden trong `python/tests/golden/*.tsv`).
2. Chạy toàn bộ test Python:
   ```bash
   cd python
   python -m pytest -q
   ```
3. Sinh lại oracle và đồng bộ sang Rust:
   ```bash
   python -m tools.export_conformance
   cd ../rust
   PYTHON=python3 tools/sync_conformance.sh ../python
   ```
4. Port thay đổi sang Rust (kể cả bảng mã trong `vfk_core/data/charsets.json`), rồi chạy:
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
5. Nếu đổi web demo, dựng lại WASM:
   ```bash
   ./web/build.sh
   ```

## Môi trường phát triển

```bash
# Python
cd python
python3 -m venv .venv
. .venv/bin/activate
pip install -e ".[dev]"
python -m pytest -q
ruff check .

# Rust
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cd rust && cargo test --workspace
```

## Quy ước code

- Bản Python chỉ dùng thư viện chuẩn (test dùng `pytest`). Không thêm dependency runtime.
- Bản Rust ưu tiên không cấp phát trong đường xử lý phím; `WordState` dùng `Vec` nhưng mọi thứ bị chặn bởi `MAX_RAW = 24`.
- Mọi hàm trong `vfk_core` phải thuần (không I/O, không trạng thái toàn cục) và giữ nguyên thứ tự duyệt của bản Python.
- Chuỗi nội bộ dùng NFC; chỉ chuẩn hoá NFC/NFD ở đầu ra. FFI đổi độ dài/xoá sang UTF-16.
- Comment và tài liệu viết bằng tiếng Việt, code/định danh bằng tiếng Anh.
- Bảng mã legacy dùng dữ liệu MIT của `bamboo-core`; không chép bảng từ dự án GPL.
  Khi sửa, cập nhật `THIRD_PARTY_NOTICES.md` và test round-trip.

## Thêm kiểu gõ mới

Thêm một `InputMethodSpec` trong `python/vfk_core/rules.py` (và `rules.rs`), khai báo:
`tones`, `remove_tone`, `marks`, `standalone`, `insert_keys`, `digit_keys`. Không sửa thuật toán
`transform` trừ khi kiểu gõ cần loại phím mới (ví dụ phím là dấu câu như VIQR).

Sau đó thêm golden test, chạy `export_conformance`, port sang Rust và cập nhật tài liệu.

## Commit / PR

- Commit nhỏ, thông điệp dạng `core: ...`, `engine: ...`, `web: ...`, `docs: ...`.
- PR cần mô tả hành vi trước/sau và test đi kèm.
- Không commit `__pycache__`, `target/`, file cấu hình cá nhân.

## Giấy phép

Code là MIT hoặc Apache-2.0, tuỳ chọn (`LICENSE-MIT` / `LICENSE-APACHE`). Logo/wordmark trong
`resources/brand/` không thuộc phạm vi hai giấy phép đó.
