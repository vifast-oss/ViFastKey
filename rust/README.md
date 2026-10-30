# ViFastKey (Rust)

Port nguyên văn bản tham chiếu Python v4. **Bản Python là oracle**: bản Rust đạt khi qua 100% `conformance/*.jsonl`.

> Trạng thái: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --workspace` và demo CLI đều đã xanh. Bản Rust qua 100% conformance sinh từ Python.

```bash
cargo build --workspace
cargo test --workspace                       # syllable, golden, conformance core + engine, FFI
cargo run -p vfk-cli -- --demo               # demo
cargo run -p vfk-cli                         # REPL (:telex :vni :old :new :replace :preedit :trace :coda_k :macro_digits :spell :help :q)
cargo run -p vfk-cli -- --method vni --tone new --text "Vie65t Nam"
cargo run -p vfk-charset -- --to tcvn3 --text "Tiếng Việt"
cargo run -p vfk-core --release --example bench
cargo run -p vifastkey-settings                # UI cấu hình desktop
```

Linux cần thêm thư viện GUI để build `vifastkey-settings`:

```bash
sudo apt-get install -y pkg-config libxkbcommon-dev libwayland-dev \
  libx11-dev libxi-dev libgl1-mesa-dev libegl1-mesa-dev
```

## 1. Cấu trúc

```
rust/
├── crates/
│   ├── vi-syllable/   tách âm tiết, chính tả 3 trạng thái, đặt thanh (thuần; parser riêng, không phải frontend âm vị của ViFastTts)
│   ├── vfk-core/      rules, transform, Composer, to_raw/rebuild, diff, charset, emoji
│   ├── vfk-suggest/   lexicon gợi ý local
│   ├── vfk-engine/    ports (Config, AppProfile, 4 trait), session (ImeSession), testing (Harness)
│   ├── vfk-config/    load/merge/validate/atomic save config dùng chung cho UI/adapter
│   ├── vfk-adapter-core/ field policy, surrounding guard, action plan, timing dùng chung
│   ├── vfk-ffi/       C ABI: lib "vifastkey" (cdylib + staticlib), include/vifastkey.h
│   └── vfk-wasm/      wasm-bindgen: Session + Action JSON cho web demo
├── apps/vfk-cli/      REPL + demo (bin "vfk")
├── apps/vfk-charset/  chuyển bảng mã (bin "vfk-charset")
├── apps/vfk-suggest/  gợi ý từ (bin "vfk-suggest")
├── apps/vfk-settings/ UI cấu hình desktop egui cho Windows/Linux (bin "vifastkey-settings")
├── apps/vfk-windows/  adapter Windows hook Replace fallback (WH_KEYBOARD_LL)
├── conformance/       core.jsonl (13.337 ca), engine.jsonl (3.000 kịch bản) sinh từ Python
├── testdata/          golden/*.tsv, corpus_vi.txt (dùng chung với Python)
└── tools/sync_conformance.sh
```

| Python | Rust |
|---|---|
| `vi_syllable/__init__.py` | `crates/vi-syllable/src/lib.rs` |
| `vfk_core/rules.py` | `crates/vfk-core/src/rules.rs` |
| `vfk_core/transform.py`, `__init__.py` | `crates/vfk-core/src/transform.rs`, `lib.rs` |
| `vfk_core/charset.py`, `data/charsets.json` | `crates/vfk-core/src/charset.rs`, `data/charsets.json` |
| `vfk_engine/ports.py` | `crates/vfk-engine/src/ports.rs` |
| `vfk_engine/session.py` | `crates/vfk-engine/src/session.rs` |
| `adapters/__init__.py` | `crates/vfk-engine/src/testing.rs` |
| `vfk_ffi/__init__.py` | `crates/vfk-ffi/src/lib.rs` (mở rộng thành C ABI thật) |
| — | `crates/vfk-wasm/src/lib.rs` (wasm-bindgen cho web) |
| `apps/vfk_cli.py` | `apps/vfk-cli/src/main.rs` |
| `tests/test_*.py` | `crates/*/tests/*.rs` |

Phụ thuộc: `unicode-normalization` 0.1.26, `serde` 1.0.229, `serde_json` 1.0.154,
`wasm-bindgen` 0.2.130, `arrayvec` 0.7.8; adapter Windows dùng `windows-sys` 0.61.2;
UI settings dùng `eframe`/`egui` 0.32.3. Không dùng `rand` (test dùng LCG tự viết), không dùng `phf`
(bảng tra sinh một lần bằng `OnceLock`). Toolchain: stable 1.99.1 (xem `rust-toolchain.toml`);
`cargo update` không còn dependency nào lệch bản mới nhất tại thời điểm cập nhật.

## 2. Tính năng engine

- `allow_coda_k`: âm cuối `k` cho tên riêng (`Đắk Lắk`), chỉ sắc/nặng.
- `macro_digits`, `macro_undo`: macro có số (`mp3`, `k8s`) và Backspace hoàn tác macro.
- `đ` viết tắt (`ddc`, `vcdd`, `NDD`), undo tiếng Anh, VNI literal number, sticky restore.
- Open `uơ`/`ươ` theo âm đầu; giữ `aw`/`aws`/`aa` đứng riêng.
- Auto-cap `…`, ngoặc kép, newline; Esc không forward.
- `AppProfile` ghi đè kiểu gõ/kiểu dấu/chính tả.
- Charset 16 bảng mã; FFI C (`vfk_charset_*`), WASM, CLI `vfk-charset`.
- PREEDIT sửa từ đã commit bằng `Replace` + surrounding text.
- Gợi ý từ/emoji local; CLI `vfk-suggest`; ArrayVec cho `WordState`.
- FFI C: JSON cấu hình một phần được trộn lên cấu hình hiện tại (không reset trường thiếu).
- `vfk-adapter-core`: field policy, surrounding guard, `plan_action` và timing dùng chung
  cho Fcitx5/macOS/iOS/Android/Windows; C ABI `vfk_plan_action`; scenario
  `adapter-conformance/*.jsonl`.
- WASM: `Config::apply_json_patch`, preedit/replace, dictionary, surrounding text, charset.

## 3. Khác biệt có chủ đích so với Python

| Chỗ | Python | Rust | Lý do |
|---|---|---|---|
| Phím | chuỗi `"Backspace"` | `enum Key` (+ `Key::parse` nhận đúng tên cũ) | an toàn kiểu |
| `final` | tham số | `is_final` | `final` là từ khoá dự phòng của Rust |
| `locked` | `set` | `Vec<char>` | rất ít phần tử |
| Chữ số | ASCII (`0`–`9`) | ASCII (`0`–`9`) | tránh `²`, `٣` bị coi là phím chức năng |
| Kiểu gõ lạ trong config | dùng Telex (fallback) | dùng Telex | không panic trong IME |
| Port | Protocol | `trait` + `Box<dyn>` trong `Ports` | |

## 4. C ABI (`include/vifastkey.h`)

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
- Bảng mã: `vfk_charset_encode/decode` trả chuỗi Unicode của codepoint legacy;
  `vfk_charset_encode_bytes` trả bytes thô cho bảng một byte.
- Swift/Kotlin: có thể bọc header này trực tiếp, hoặc thêm UniFFI sau.

## 5. Đồng bộ với bản Python

Khi đổi hành vi: sửa bản Python, thêm golden test, rồi chạy `PYTHON=python3 tools/sync_conformance.sh ../python` và `cargo test`. Bản Rust sẽ báo đỏ đúng những ca bị đổi. Test conformance in ra 15 đến 30 ca sai đầu tiên, kèm vị trí phím lệch đầu tiên và Action của cả hai bản.

## 6. Kiểm tra

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p vfk-cli -- --demo
```

Nếu core conformance sai, các chỗ đáng nghi nhất (vì logic tinh tế nhất) là:
- `display()`: luật `ươ` và `uơ`, cờ `auto_horn`.
- Thứ tự ứng viên trong `candidates()`: cặp đứng trước, sau đó sort ổn định giảm dần theo vị trí.
- `check()`: nhánh `Partial` dùng `relaxed_any` và `relaxed_for_coda`.

## 7. Bước tiếp theo đề xuất

- Tối ưu: `ArrayVec<Letter, 24>` thay `Vec`, bitset cho `locked`, bỏ cấp phát `String` trong `toneless_lower` (dùng buffer `[char; 24]`). Chỉ làm sau khi conformance đã xanh.
- Adapter đầu tiên: Fcitx5 (C++ gọi `vifastkey.h`) hoặc macOS IMK (Swift).
- Macro nhiều từ/suffix, MS Vietnamese layout, preedit edit-previous-word.
- Tối ưu charset decode (longest-match) và thêm cache nếu cần.
- UniFFI cho Kotlin/Swift. Web đã có `vfk-wasm` + demo trong `web/`.
