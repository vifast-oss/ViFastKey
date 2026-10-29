# Báo cáo phân tích chi tiết

Đây là 8 báo cáo do các sub agent lập trong đợt đối chiếu 21 dự án IME tiếng Việt.
Bản tổng hợp và quyết định triển khai nằm ở [`../IME_FEATURE_RESEARCH.md`](../IME_FEATURE_RESEARCH.md).

| File | Nội dung |
|---|---|
| `01-skey-unikey.md` | SKey, SKey Engine, bamboo-core, UniKey 3.62, x-unikey: engine, charset, undo, macro, adapter |
| `02-rust-engines.md` | VnKey, Vigo, GõViệt, UVie, Telebit: kiến trúc engine, smart tier, test/benchmark |
| `03-desktop.md` | OpenKey, XKey, VKey, Funput, GõViệt: hook/event tap, app profile, settings UX |
| `04-linux.md` | ibus-bamboo, fcitx5-lotus, Bamboo Viet, ibus-unikey, fcitx5-unikey, Telebit: output mode, Wayland, ACK |
| `05-web-mobile-ai.md` | AVIM, v7, Funput, VKey, Bamboo Viet: browser, mobile, AI, config |
| `06-vnkey-deep.md` | Phân tích riêng VnKey: engine, 4 adapter, charset, config, lỗi và bài học |
| `07-vifasttts.md` | ViFastTts: engineering, desktop/macOS, test/release, bài học cho IMK |
| `08-areca-ime.md` | Areca IME (Fcitx5, MIT): FIFO/barrier, backend/fallback, trust probe, adaptive delay, app compatibility, bài học cho adapter Linux của ViFastKey |

## Lưu ý giấy phép

Các dự án được khảo sát có giấy phép GPL-2.0/3.0, AGPL-3.0, MIT hoặc Apache-2.0.
Các báo cáo chỉ chứa **hành vi, mô tả kiến trúc, test fact**; không chứa code, bảng mã,
từ điển hay corpus được sao chép. ViFastKey là MIT; bảng mã legacy dùng dữ liệu MIT của
`bamboo-core` và được ghi công trong `THIRD_PARTY_NOTICES.md`.

Khi dùng một test vector từ báo cáo, phải tự viết lại test và ghi nguồn hành vi, không copy
nguyên văn dữ liệu của dự án gốc.
