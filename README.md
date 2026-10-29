<p align="center">
  <img src="resources/brand/vifastkey-lockup-cream.png" alt="ViFastKey · Bộ gõ tiếng Việt" width="620">
</p>

# ViFastKey

Bộ gõ tiếng Việt gọn, thuần logic: chuỗi phím thô → chữ tiếng Việt (Telex, Simple Telex, VNI, VIQR), kèm tầng engine cho IME (preedit/replace, macro, khôi phục tiếng Anh, hồ sơ ứng dụng).

Tính năng nổi bật: `allow_coda_k` (Đắk Lắk), `macro_digits` (mp3, k8s), macro undo một phím,
`đ` viết tắt (`vcdd`), 16 bảng mã (TCVN3, VNI-Windows, VISCII, VIQR, CP1258, NCR... cùng Unicode),
app profile ghi đè kiểu gõ/kiểu dấu, FFI C/Python/WASM, playground Preedit + macro + chuyển bảng mã.

Đợt phát triển này dựa trên phân tích 20 dự án IME tiếng Việt công khai; xem
[`docs/IME_FEATURE_RESEARCH.md`](docs/IME_FEATURE_RESEARCH.md).

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
│   ├── apps/                   # CLI demo + REPL + vfk-charset
│   ├── tests/                  # 602 test, golden, corpus
│   ├── tools/                  # export conformance, bench
│   ├── conformance/            # core.jsonl + engine.jsonl (oracle cho Rust)
│   ├── vfk_core/data/          # bảng mã legacy (bamboo-core MIT)
│   ├── README.md               # đặc tả hành vi chi tiết
│   ├── PORTING.md
│   └── EDGE_CASES.md
├── rust/                       # bản port
│   ├── crates/
│   │   ├── vi-syllable/
│   │   ├── vfk-core/           # transform + charset
│   │   ├── vfk-engine/
│   │   ├── vfk-ffi/            # C ABI + include/vifastkey.h
│   │   └── vfk-wasm/           # wasm-bindgen cho web demo
│   ├── apps/vfk-cli/           # CLI demo + REPL
│   ├── apps/vfk-charset/       # chuyển bảng mã
│   ├── conformance/            # bản sao dữ liệu oracle
│   ├── testdata/               # golden + corpus
│   ├── tools/sync_conformance.sh
│   └── README.md
├── docs/                       # nghiên cứu, hợp đồng, adapter, config
├── macos/                      # adapter IMK thật (Swift + C ABI)
├── web/                        # landing page + playground
├── design/                     # token thiết kế
└── resources/                  # brand + font
```

Ngoài hai bản engine, repo còn có:

- `web/` — landing page + playground gõ thử chạy WASM trong trình duyệt.
- `resources/brand/` — logo, favicon, OG; `resources/fonts/` — Be Vietnam Pro (OFL).
- `design/tokens.json` — màu/font/spacing dùng chung cho desktop, web, extension, mobile.

Hai bản tự chứa dữ liệu `conformance/`, `tests/golden/`, `tests/data/` để chạy test độc lập. Khi sửa hành vi, sửa bản Python trước rồi đồng bộ sang Rust bằng `rust/tools/sync_conformance.sh`.

## Chạy bản Python

Yêu cầu Python 3.10+. Có thể cài như package (kèm `pytest`/`ruff`):

```bash
cd python
pip install -e ".[dev]"
python -m pytest -q               # 602 test
python -m tools.export_conformance
python -m tools.bench
python -m apps.vfk_cli --demo
python -m apps.vfk_cli            # REPL tương tác
python -m apps.vfk_cli --text "Tieengs Vieejt"
python -m apps.vfk_cli --method vni --tone new --text "Vie65t Nam"
python -m apps.vfk_charset --to tcvn3 --text "Tiếng Việt"
python -m apps.vfk_charset --from tcvn3 --file vi.tcvn --to unicode
```

## Build bản Rust

Yêu cầu Rust stable (đã kiểm tra 1.99.1); `rust/rust-toolchain.toml` ghim stable + wasm target.

```bash
cd rust
cargo build --workspace
cargo test --workspace
cargo run -p vfk-cli -- --demo
cargo run -p vfk-cli
cargo run -p vfk-cli -- --method vni --tone new --text "Vie65t Nam"
cargo run -p vfk-charset -- --to tcvn3 --text "Tiếng Việt"
```

## macOS input method

Adapter IMK thật, gọi engine Rust qua C ABI:

```bash
./macos/build.sh          # build app + self-test
./macos/install.sh        # cài vào ~/Library/Input Methods
./macos/doctor.sh         # chẩn đoán permission/input source
```

Chi tiết cài, menu, config và giới hạn: `macos/README.md`. Adapter dùng preedit, app profile,
menu Telex/VNI, macro undo; chưa ký Developer ID/notarize nên bản phát hành cần bổ sung.

## Web demo

Playground gõ thử chạy engine Rust qua WASM, không cần server API:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
./web/build.sh
python3 -m http.server 8080 -d web
# mở http://127.0.0.1:8080/
```

Playground mô phỏng cả `REPLACE` và `PREEDIT`, có macro, NFC/NFD và các tuỳ chọn mới. Chi tiết hành vi, giới hạn và cách deploy tĩnh: `web/README.md`.

## Kiểm tra nhanh

```bash
make test     # pytest + cargo test
make check    # thêm ruff, rustfmt, clippy, kiểm tra conformance không lệch
make web      # dựng lại WASM + asset
```

## Đồng bộ hai bản

```bash
cd rust
PYTHON=python3 tools/sync_conformance.sh ../python
cargo test --workspace
```

## Tài liệu

- `python/README.md` — đặc tả hành vi, mô hình tính toán, danh sách bất biến.
- `python/PORTING.md` — hướng dẫn port và mô tả bộ conformance.
- `python/EDGE_CASES.md` — tổng hợp edge case từ các dự án IME khác.
- `rust/README.md` — kiến trúc crate, khác biệt có chủ đích, C ABI.
- `docs/IME_FEATURE_RESEARCH.md` — tổng hợp 20 dự án IME, ma trận tính năng, quyết định.
- `docs/research/` — 7 báo cáo phân tích chi tiết (+ ViFastTts).
- `docs/VIFASTTTS_LESSONS.md` — bài học kỹ thuật từ ViFastTts cho adapter macOS.
- `docs/COMPATIBILITY.md` — hợp đồng hành vi (tone, undo, đ, VNI số, charset, Action).
- `docs/ADAPTER_GUIDE.md` — hợp đồng viết adapter macOS/Windows/Linux.
- `docs/CONFIG.md` — toàn bộ trường config và app profile.
- `docs/IME_TEST_VECTORS.md` — bộ ví dụ hành vi dùng để port/regression.
- `macos/README.md` — build/cài/test adapter macOS IMK.
- `THIRD_PARTY_NOTICES.md` — ghi công bảng mã bamboo-core MIT.
- `CONTRIBUTING.md` — quy trình sửa hành vi, chạy test, đồng bộ conformance.
- `CHANGELOG.md` — thay đổi theo phiên bản.
- `ROADMAP.md` — việc còn lại: adapter Windows/Linux, macro nhiều từ, preedit nâng cao...
