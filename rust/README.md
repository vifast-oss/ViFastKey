# ViFastKey (Rust)

Port nguyên văn bản tham chiếu Python v4. **Bản Python là oracle**: bản Rust đạt khi qua 100% `conformance/*.jsonl`.

> Lưu ý: code này được viết khi **chưa biên dịch thử**. Có thể còn lỗi biên dịch nhỏ (lifetime, kiểu dữ liệu, import), cần sửa ở lần `cargo build` đầu tiên. Xem mục 5 để biết thứ tự kiểm tra.

```bash
cargo build --workspace
cargo test --workspace                       # syllable, golden, conformance core + engine, FFI
cargo run -p vfk-cli -- --demo               # demo
cargo run -p vfk-cli                         # REPL (:telex :vni :old :new :replace :preedit :trace :q)
cargo run -p vfk-core --release --example bench
```

## 1. Cấu trúc

```
rust/
├── crates/
│   ├── vi-syllable/   tách âm tiết, chính tả 3 trạng thái, đặt thanh (dùng chung ViFastTts)
│   ├── vfk-core/      rules (Telex/Simple Telex/VNI), transform, Composer, to_raw/rebuild, diff
│   ├── vfk-engine/    ports (Config, AppProfile, 4 trait), session (ImeSession), testing (Harness)
│   └── vfk-ffi/       C ABI: lib "vifastkey" (cdylib + staticlib), include/vifastkey.h
├── apps/vfk-cli/      REPL + demo (bin "vfk")
├── conformance/       core.jsonl (13.252 ca), engine.jsonl (3.000 kịch bản) sinh từ Python
├── testdata/          golden/*.tsv, corpus_vi.txt (dùng chung với Python)
└── tools/sync_conformance.sh
```

| Python | Rust |
|---|---|
| `vi_syllable/__init__.py` | `crates/vi-syllable/src/lib.rs` |
| `vfk_core/rules.py` | `crates/vfk-core/src/rules.rs` |
| `vfk_core/transform.py`, `__init__.py` | `crates/vfk-core/src/transform.rs`, `lib.rs` |
| `vfk_engine/ports.py` | `crates/vfk-engine/src/ports.rs` |
| `vfk_engine/session.py` | `crates/vfk-engine/src/session.rs` |
| `adapters/__init__.py` | `crates/vfk-engine/src/testing.rs` |
| `vfk_ffi/__init__.py` | `crates/vfk-ffi/src/lib.rs` (mở rộng thành C ABI thật) |
| `apps/vfk_cli.py` | `apps/vfk-cli/src/main.rs` |
| `tests/test_*.py` | `crates/*/tests/*.rs` |

Phụ thuộc: chỉ có `unicode-normalization`, `serde`, `serde_json`. Không dùng `rand` (test dùng LCG tự viết), không dùng `phf` (bảng tra sinh một lần bằng `OnceLock`).

## 2. Khác biệt có chủ đích so với Python

| Chỗ | Python | Rust | Lý do |
|---|---|---|---|
| Phím | chuỗi `"Backspace"` | `enum Key` (+ `Key::parse` nhận đúng tên cũ) | an toàn kiểu |
| `final` | tham số | `is_final` | `final` là từ khoá dự phòng của Rust |
| `locked` | `set` | `Vec<char>` | rất ít phần tử |
| Chữ số VNI | `str.isdigit()` | `is_ascii_digit()` | tránh `²`, `٣` bị coi là phím |
| Kiểu gõ lạ trong config | KeyError | dùng Telex | không panic trong IME |
| Port | Protocol | `trait` + `Box<dyn>` trong `Ports` | |

## 3. C ABI (`include/vifastkey.h`)

```c
VfkSession *s = vfk_session_new("{\"input_method\":\"telex\"}", VFK_MODE_REPLACE);
vfk_focus_changed(s, "notepad.exe");
vfk_set_surrounding_text(s, text_before_cursor);      // nếu nền tảng có
VfkAction a = vfk_process_key(s, "s", false, false, false);
switch (a.kind) {
  case VFK_PASS:    /* trả phím cho OS */ break;
  case VFK_REPLACE: /* xoá a.delete_utf16 code unit, chèn a.text, nếu a.pass_key gửi tiếp phím */ break;
  case VFK_PREEDIT: /* đặt preedit = a.text, con trỏ a.cursor_utf16 */ break;
  case VFK_COMMIT:  /* commit a.text, nếu a.pass_key gửi tiếp phím */ break;
}
const char *hint = vfk_app_profile_json(s);           // độ trễ backspace, chờ ack, preedit cho terminal...
vfk_session_free(s);
```

- Mọi hàm đều bắt panic. Nếu có panic, hàm trả `PASS` và xoá từ đang gõ.
- Độ dài và vị trí con trỏ trả về theo UTF-16, được tính tại FFI từ chuỗi đầu ra (NFC hoặc NFD).
- Swift/Kotlin: có thể bọc header này trực tiếp, hoặc thêm UniFFI sau.

## 4. Đồng bộ với bản Python

Khi đổi hành vi: sửa bản Python, thêm golden test, rồi chạy `tools/sync_conformance.sh ../../python` và `cargo test`. Bản Rust sẽ báo đỏ đúng những ca bị đổi. Test conformance in ra 15 đến 30 ca sai đầu tiên, kèm vị trí phím lệch đầu tiên và Action của cả hai bản.

## 5. Thứ tự kiểm tra lần đầu

1. `cargo test -p vi-syllable`: bảng chính tả và đặt thanh.
2. `cargo test -p vfk-core --test golden`, sau đó `--test conformance`.
3. `cargo test -p vfk-engine --test engine`, sau đó `--test conformance`.
4. `cargo test -p vfk-ffi`.
5. `cargo clippy --workspace --all-targets`.

Nếu core conformance sai, các chỗ đáng nghi nhất (vì logic tinh tế nhất) là:
- `display()`: luật `ươ` và `uơ`, cờ `auto_horn`.
- Thứ tự ứng viên trong `candidates()`: cặp đứng trước, sau đó sort ổn định giảm dần theo vị trí.
- `check()`: nhánh `Partial` dùng `relaxed_any` và `relaxed_for_coda`.

## 6. Bước tiếp theo đề xuất

- Tối ưu: `ArrayVec<Letter, 24>` thay `Vec`, bitset cho `locked`, bỏ cấp phát `String` trong `toneless_lower` (dùng buffer `[char; 24]`). Chỉ làm sau khi conformance đã xanh.
- Adapter đầu tiên: Fcitx5 (C++ gọi `vifastkey.h`) hoặc macOS IMK (Swift).
- UniFFI cho Kotlin/Swift, wasm-bindgen cho web.
