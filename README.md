# ViFastKey

Bộ gõ tiếng Việt gọn, thuần logic: chuỗi phím thô → chữ tiếng Việt (Telex, Simple Telex, VNI), kèm tầng engine cho IME (preedit/replace, macro, khôi phục tiếng Anh, hồ sơ ứng dụng).

Repository gồm hai bản:

| Thư mục | Vai trò |
|---|---|
| `python/` | **Bản chính, chạy được.** Là đặc tả tham chiếu (oracle) cho toàn bộ hành vi. Chỉ dùng thư viện chuẩn. |
| `rust/` | **Bản port sang Rust.** Đạt khi vượt qua 100% bộ `conformance/*.jsonl` sinh từ bản Python. |

## Cấu trúc

```
ViFastKey/
├── python/                     # bản tham chiếu
│   ├── vi_syllable/            # tách âm tiết, chính tả 3 trạng thái, đặt thanh
│   ├── vfk_core/               # rules + transform (hàm thuần)
│   ├── vfk_engine/             # ImeSession, ports, Config
│   ├── vfk_ffi/                # API phẳng kiểu FFI
│   ├── adapters/               # port mẫu, từ điển, ô nhập giả lập
│   ├── apps/                   # CLI demo + REPL
│   ├── tests/                  # 388 test, golden, corpus
│   ├── tools/                  # export conformance, bench
│   ├── conformance/            # core.jsonl + engine.jsonl (oracle cho Rust)
│   ├── README.md               # đặc tả hành vi chi tiết
│   ├── PORTING.md
│   └── EDGE_CASES.md
└── rust/                       # bản port
    ├── crates/
    │   ├── vi-syllable/
    │   ├── vfk-core/
    │   ├── vfk-engine/
    │   └── vfk-ffi/            # C ABI + include/vifastkey.h
    ├── apps/vfk-cli/           # CLI demo + REPL
    ├── conformance/            # bản sao dữ liệu oracle
    ├── testdata/               # golden + corpus
    ├── tools/sync_conformance.sh
    └── README.md
```

Hai bản tự chứa dữ liệu `conformance/`, `tests/golden/`, `tests/data/` để chạy test độc lập. Khi sửa hành vi, sửa bản Python trước rồi đồng bộ sang Rust bằng `rust/tools/sync_conformance.sh`.

## Chạy bản Python

Yêu cầu Python 3.10+ (đã kiểm tra với Python 3.14) và `pytest` nếu muốn chạy test.

```bash
cd python
python -m pytest -q               # 388 test
python -m tools.export_conformance
python -m tools.bench
python -m apps.vfk_cli --demo
python -m apps.vfk_cli            # REPL tương tác
```

## Build bản Rust

Yêu cầu Rust 1.75+.

```bash
cd rust
cargo build --workspace
cargo test --workspace
cargo run -p vfk-cli -- --demo
cargo run -p vfk-cli
```

## Đồng bộ hai bản

```bash
cd rust
tools/sync_conformance.sh ../../python
cargo test --workspace
```

## Tài liệu

- `python/README.md` — đặc tả hành vi, mô hình tính toán, danh sách bất biến.
- `python/PORTING.md` — hướng dẫn port và mô tả bộ conformance.
- `python/EDGE_CASES.md` — tổng hợp edge case từ các dự án IME khác.
- `rust/README.md` — kiến trúc crate, khác biệt có chủ đích, C ABI.
