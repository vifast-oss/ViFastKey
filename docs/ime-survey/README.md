# Khảo sát bộ gõ tiếng Việt — bộ tài liệu

Bộ tài liệu này phân tích **31 repository bộ gõ tiếng Việt** trong
[`hthienloc/vietnamese-ime-tracker`](https://hthienloc.github.io/vietnamese-ime-tracker),
đối chiếu với **ViFastKey**, và có một tài liệu kỹ thuật riêng cho **Areca IME**.

> **Snapshot dữ liệu:** 2026-11-01.
> Metadata (stars, ngôn ngữ, license, ngày tạo, commit cuối) lấy từ GitHub API và
> `bo-go-tieng-viet.csv` của tracker. Tính năng lấy từ README và mã nguồn tại cùng
> thời điểm. Không phải dự án nào cũng được build/chạy thử; chỗ nào suy luận từ cấu
> trúc sẽ được ghi rõ.

## Link công khai

Đã deploy tại <https://key.vifast.org/docs/>:

| Trang | URL |
|---|---|
| Mục lục | <https://key.vifast.org/docs/> |
| Sổ tay Areca | <https://key.vifast.org/docs/areca-ime.html> |
| Khảo sát 31 IME | <https://key.vifast.org/docs/survey-all-imes.html> |
| So sánh ViFastKey | <https://key.vifast.org/docs/comparison-vifastkey.html> |

Bản HTML sinh từ Markdown trong `web/docs/`. Sau khi sửa `.md`, chạy:

```bash
python3 docs/ime-survey/build_site.py   # cần node/npx + marked@15
./deploy/vifastkey_web/deploy-jp.sh     # hoặc chỉ rebuild static nếu chỉ đổi docs
```

`survey-all-imes.html` và `comparison-vifastkey.html` có `meta robots=noindex`
để không bị search index; `areca-ime.html` được index bình thường.

## Đọc gì trước?

| Tài liệu | Dành cho | Nội dung |
|---|---|---|
| [`areca-ime.md`](areca-ime.md) | Kỹ sư muốn hiểu/sửa/mở rộng Areca | Sổ tay kỹ thuật ~1.200 dòng: kiến trúc, scheduler, 6 backend, trust probe, timing, config, build, debug, test, cheat sheet |
| [`survey-all-imes.md`](survey-all-imes.md) | Người cần bức tranh toàn cảnh | Phân tích từng repo theo nhóm nền tảng, ma trận tính năng, bài học chung |
| [`comparison-vifastkey.md`](comparison-vifastkey.md) | Người chọn bộ gõ hoặc định vị sản phẩm | So sánh 31 IME với ViFastKey theo 8 trục, scorecard, khuyến nghị |
| [`data/imes.csv`](data/imes.csv) | Ai cần dữ liệu máy đọc | Bảng tổng hợp một dòng/repo |

## Bức tranh trong một bảng

| Chỉ số | Giá trị |
|---|---|
| Repository | 31 (36 dòng vì nhiều repo đa nền tảng) |
| Còn hoạt động | 24 repo |
| Archived | 3 repo (UniLume tombstone, Unikey-Wayland-Pro, LotusVibe) |
| Inactive / Dead | 1 / 3 repo (theo commit cuối; 3 repo đã bị xóa khỏi GitHub) |
| Tổng stars (unique repo) | ~4.700 |
| Nền tảng tích hợp | Fcitx5: 16 · IBus: 5 · Standalone Wayland: 5 · Fcitx5+IBus: 4 · macOS native: 3 · Windows native: 3 |
| Ngôn ngữ | C++ 12 · Rust 12 · Go 3 · Makefile 3 · Swift 3 · Kotlin 1 · C# 1 · HTML 1 |
| Giấy phép | GPL-3.0: 18 · MIT: 9 · BSD-3: 3 · không rõ: 3 · 0BSD: 1 · MPL-2.0: 1 · GPL-2.0: 1 |
| Ra đời năm 2026 | 27/36 dòng — một đợt bùng nổ sau khi Fcitx5 + Wayland phổ biến |

### Ba quan sát lớn

1. **Hệ sinh thái Linux phân mảnh theo framework, không theo engine.** Cùng một
   `bamboo-core` được dùng bởi ibus-bamboo, fcitx5-bamboo, Areca, NepKey; cùng một
   họ UniKey được dùng bởi fcitx5-unikey, Unikey-Wayland, UniLume legacy. Giá trị
   cạnh tranh nằm ở **adapter**, không ở thuật toán gõ.
2. **Rust đã ngang C++ về số repo** (12/12), nhưng phần lớn repo Rust mới (2026) và
   nhỏ; C++ vẫn giữ các dự án lâu năm và nhiều tính năng nhất.
3. **Chỉ một số ít có test/CI thực sự.** Đa số dựa vào báo lỗi người dùng. Đây là
   khoảng trống lớn nhất của cả hệ sinh thái, và là chỗ ViFastKey có lợi thế rõ.

## Cách phân nhóm

```text
Linux
├── Fcitx5 addon
│   ├── Dùng/port bamboo-core: fcitx5-bamboo, Areca, Lotus (C++ port),
│   │   VMK, lilypad, Ngó Sen (Rust port), (NepKey là IBus)
│   ├── Engine riêng: skey, telebit, Clak, CanType, pinakey
│   └── Kế thừa UniKey: fcitx5-unikey
├── IBus engine
│   ├── ibus-bamboo (Go, bamboo-core)
│   ├── ibus-lotus (đã chết)
│   ├── TypeVN, NepKey
│   └── (pinakey từng là IBus, nay chuyển sang Fcitx5 — tracker còn ghi IBus)
├── Standalone Wayland (uinput / virtual keyboard / libei)
│   ├── Unikey-Wayland, Unikey-Wayland-Pro
│   ├── vi-ime, vietc, codekeyvn (đã chết)
└── Khác: UniLume (legacy), TextVN/Funput/VieLotus (đa nền tảng)
macOS / Windows
├── OpenKey (C++, lâu năm)
├── Gõ Nhanh (Rust, 3 nền tảng, sản phẩm hoá)
├── VieLotus (Swift + core nhị phân)
└── Funput, TextVN, BambooMintKey (cross-platform mới)
```

## Ghi chú về độ chính xác

- **Có**: số stars, ngôn ngữ chính, license, ngày tạo, commit cuối, mô tả repo.
- **Có**: danh sách tính năng nếu README/source nói rõ.
- **Suy luận**: chất lượng test, độ chín của adapter, mức độ bảo trì — dựa trên
  số test, CI, packaging, tần suất commit, không dựa trên tuyên bố marketing.
- **Không kiểm chứng**: hành vi thực tế trên từng bản distro/app. Muốn chắc, phải
  cài và đo; tài liệu này chỉ khoanh vùng.

## Quy ước

- `✅` có, `◐` một phần, `—` không có, `🔌` thuộc adapter/OS.
- "Active" ở đây là repo còn phát triển. Tracker có thể gắn "Active" cho repo
  đã archive nếu còn commit trong 3 tháng; bộ dữ liệu này phân loại lại.
- Repo đã xóa (404) được giữ trong khảo sát để không mất dấu lịch sử, nhưng không
  có metadata hiện tại.
