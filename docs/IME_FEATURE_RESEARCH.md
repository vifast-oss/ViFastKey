# Nghiên cứu tính năng các bộ gõ tiếng Việt

Tài liệu này tổng hợp phân tích 20 dự án IME tiếng Việt công khai (cùng các engine đi kèm),
đối chiếu với ViFastKey, và ghi rõ tính năng nào đã được triển khai, tính năng nào thuộc
adapter nền tảng, tính năng nào còn để lại trong roadmap.

Mục tiêu: **không sao chép code/dữ liệu GPL**, chỉ học hành vi, kiến trúc, UX và test case;
mọi phần triển khai trong repo này là MIT và được viết lại từ đặc tả.

## 1. Nguồn đã phân tích

| Nguồn | Vai trò | Giấy phép | Commit/phiên bản |
|---|---|---|---|
| [collyn/skey](https://github.com/collyn/skey) | Fcitx5 addon, output mode, app profile, GUI | GPL-3.0 | `2628606` |
| [collyn/skey-engine](https://github.com/collyn/skey-engine) | Rust engine, charset, spelling | GPL-3.0 | `4481412` |
| [BambooEngine/ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo) | IBus, nhiều chế độ gõ, charset, emoji | GPL-3.0 | `49e03d7` |
| [fcitx/fcitx5-unikey](https://github.com/fcitx/fcitx5-unikey) | Fcitx5 adapter kinh điển | GPL-2.0+ | `ed9170c` |
| [LotusInputMethod/fcitx5-lotus](https://github.com/LotusInputMethod/fcitx5-lotus) | Output mode, uinput/ACK, app rules | GPL-3.0 | `ce8b350` |
| [marixdev/vnkey](https://github.com/marixdev/vnkey) | Rust engine + Windows/macOS/Fcitx5/IBus | GPL-3.0 | `b75cdc0` |
| [haidinhtuan/vigo](https://github.com/haidinhtuan/vigo) | Rust engine tiers, undo history, smart tier | MIT/Apache (khai báo) | `06862b9` |
| [Funput/Funput](https://github.com/Funput/Funput) | Đa nền tảng, mobile, macro/config | MIT | `e63ac55` |
| [ngkhhuy/bamboo-viet](https://github.com/ngkhhuy/bamboo-viet) | GUI/CLI config, app presets | GPL-3.0 | `7164155` |
| [tuyenvm/OpenKey](https://github.com/tuyenvm/OpenKey) | macOS/Windows/Linux, charset, macro | GPL-3.0 | `89c2fd3` |
| [xmannv/xkey](https://github.com/xmannv/xkey) | macOS UX, context rules, macro | GPL/không rõ | `398e78e` |
| [phatMT97/VKey](https://github.com/phatMT97/VKey) | Windows hook/TSF, app lock, macro undo | AGPL-3.0 | `4399b89` |
| [nguyenrot/goviet](https://github.com/nguyenrot/goviet) | Rust engine + macOS event tap | MIT | `0da0b89` |
| [uvie-project/uvie-mac](https://github.com/uvie-project/uvie-mac) | uvie-rs engine, FFI | Không thấy LICENSE | `e0bd2c7` |
| [1ec5/avim](https://github.com/1ec5/avim) | Firefox/Thunderbird extension | MIT | `18572ba` |
| [ducngg/v7](https://github.com/ducngg/v7) | AI-assisted IME | Apache-2.0 | `4404b70` |
| [bamboo-vi/bamboo-core](https://github.com/bamboo-vi/bamboo-core) | Rust engine, 17 charset | **MIT** | `06d4ff7` |
| [sonnam0904/telebit](https://github.com/sonnam0904/telebit) | C++ Telex/VNI, spell/escape, fcitx5 | **MIT** | `969efe2` |
| [UniKey / x-unikey](https://www.unikey.org/source.html) | Engine gốc, charset, macro, config | GPL/LGPL + điều khoản riêng | UniKey 3.62 / x-unikey 1.0.4 |
| [vn-input/ibus-unikey](https://github.com/vn-input/ibus-unikey) | IBus adapter | GPL | `ede78c3` |

Các báo cáo phân tích chi tiết nằm trong [`docs/research/`](research/README.md) (6 báo cáo,
~5.800 dòng). Tài liệu này là bản tổng hợp. Không có mã nguồn, bảng dữ liệu, từ điển hay test
corpus GPL nào được sao chép.

## 2. Ma trận tính năng

Ký hiệu: ✅ có; ◐ một phần; — không có; 🔌 thuộc adapter/OS.

| Tính năng | SKey | UniKey | OpenKey | VnKey | Vigo | GõViệt | VKey | Funput | Bamboo/Lotus | **ViFastKey** |
|---|---|---|---|---|---|---|---|---|---|---|
| Telex / Simple Telex | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| VNI | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| VIQR / MS layout | VIQR | VIQR | VIQR | ◐ (VIQR lỗi) | — | — | ✅ | ✅ | ✅ | roadmap |
| Chính tả 3 trạng thái | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Khôi phục tiếng Anh | ✅ | ✅ | ✅ | ◐ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (dictionary) |
| Undo gõ lặp | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `đ` viết tắt (`vcdd`, `NDD`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ mới |
| Open `uơ`/`ươ` theo âm đầu | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ mới |
| Macro | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (+ undo, digits) |
| Macro nhiều từ / suffix | ◐ | ✅ | ◐ | — | — | — | ◐ | ✅ | ◐ | roadmap |
| Bảng mã legacy | 14 | 17/18 | 15 | 19 | — | — | 15 | ✅ | 17 | **14 legacy + 2 Unicode** |
| Chuyển bảng mã clipboard/file | — | ✅ | ✅ | ✅ | — | — | ✅ | ✅ | ✅ | CLI mới |
| NFC/NFD | ✅ | ✅ | ✅ | ◐ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Preedit / Replace | ✅ | 🔌 | 🔌 | 🔌 | 🔌 | 🔌 | 🔌 | 🔌 | ✅ | ✅ engine |
| Sửa từ vừa gõ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ REPLACE |
| App profile / exclusion | ✅ | ✅ | ✅ | ✅ | ◐ | ✅ | ✅ | ✅ | ✅ | ✅ + method/tone/spell |
| EMOJI | — | ✅ | — | — | — | — | ✅ | ✅ | ✅ | roadmap |
| Gợi ý / AI | — | — | — | — | ✅ | — | — | ◐ | — | roadmap |
| Chuyển chế độ theo app | ✅ | ✅ | ✅ | ✅ | ◐ | ✅ | ✅ | ✅ | ✅ | 🔌 profile hints |
| Wayland / uinput / libei | ✅ | — | — | ◐ | — | — | — | — | ✅ | roadmap (adapter) |
| GUI cấu hình | ✅ | ✅ | ✅ | ✅ | — | menu bar | ✅ | ✅ | ✅ | CLI/web hiện có |
| Auto-update / signing | — | ✅ | ✅ | ✅ | — | ✅ | ✅ | ✅ | ✅ | roadmap |

## 3. Phát hiện chính

### 3.1 ViFastKey mạnh ở đâu

- Mô hình âm tiết CVC 3 trạng thái và đặt thanh đã rất sát chuẩn; khi chạy 6.631 âm tiết
  tiếng Việt curated của Telebit, chỉ còn 16 khác biệt thuộc nhóm `oo`/viết tắt.
- Có conformance 13k+ ca core và 3k kịch bản engine, mạnh hơn phần lớn dự án khảo sát.
- Có sẵn preedit/replace, app profile, soft reset, smart caps, NFC/NFD, C ABI/WASM.

**Không thay** mô hình âm tiết hiện tại bằng engine khác; chỉ vá các lỗi cụ thể.

### 3.2 Lỗi/thiếu sót quan trọng đã tìm thấy

| Vấn đề | Ví dụ | Nguồn đối chiếu | Xử lý |
|---|---|---|---|
| `đ` chỉ áp cho d đầu từ | `ddc`, `vcdd`, `NDD` bị trả raw | SKey, VnKey | ✅ sửa |
| Undo quá rộng làm mất chữ tiếng Anh | `banana → banna`, `coffee → cofee` | Telebit, GõViệt | ✅ sửa |
| VNI coi số là phím dấu | `nam2024 → nãm`, `ba10 → ba` | Telebit | ✅ sửa |
| Auto-restore không dính | `text` + `s → tét` | GõViệt | ✅ sửa |
| Nguyên âm có dấu đứng một mình bị trả raw | `aw → aw`, `aws → aws` | SKey/Telebit | ✅ sửa |
| Open `uơ` sai theo âm đầu | `duow → duơ` (mong đợi `dươ`) | SKey | ✅ sửa |
| Escape bị forward | adapter đóng dialog khi Esc | Linux analysis | ✅ sửa |
| Không có bảng mã legacy | TCVN3/VNI-Win/VISCII... | tất cả | ✅ thêm module + CLI |
| Macro không undo được | xoá từng ký tự | Funput/VKey | ✅ thêm macro undo |
| Auto-cap bỏ sót `…`, ngoặc kép, newline | `. ”word`, xuống dòng | Funput/VKey | ✅ sửa |

### 3.3 Bài học kiến trúc

- **Tách engine thuần khỏi adapter**: engine chỉ trả `Action`; adapter quyết định preedit,
  replace, uinput, XTest/libei, delay/ACK, app profile.
- **Cấu hình theo app** nên chọn cả kiểu gõ/kiểu thanh/chính tả, không chỉ bật/tắt.
- **Bảng mã là module riêng**, không nhúng vào `transform`; tránh làm hỏng độ dài xoá.
- **Undo/restore cần bằng chứng**: chỉ coi phím lặp là undo khi phím trước đó thực sự đã
  biến đổi; nếu không, khôi phục raw.
- **Macro cần undo một phím** và trigger rõ ràng (Space/Enter/Tab/punctuation).
- **Adapter phải kiểm tra surrounding text** trước khi xoá; nếu không khớp thì fail safe.
- **UI cấu hình/CLI** quan trọng ngang engine: người dùng không muốn sửa JSON tay.

## 4. Đã triển khai trong đợt này

### Engine / thư viện

- `allow_coda_k` (Đắk Lắk), `macro_digits` (mp3/k8s), `macro_undo`.
- Sửa `đ` viết tắt, undo tiếng Anh, VNI literal number, sticky restore, escape pass-key.
- Open `uơ`/`ươ` theo âm đầu; giữ nguyên âm có dấu đứng riêng.
- Auto-cap mở rộng: `…`, ngoặc kép/đóng, newline, số thập phân.
- `AppProfile` ghi đè `input_method`, `tone_style`, `spell_check`, gợi ý `charset`.
- `schema_version` cho config; `config_from_dict` bỏ qua field lạ.
- Module bảng mã 16 charset: Unicode/NFD, TCVN3, VNI-Windows, CP1258, VIQR, VISCII, VPS,
  BKHCM1/2, Vietware X/Full, NCR Dec/Hex, C-string Hex/Dec.
- FFI C/WASM/Python cho charset; `vfk-charset` CLI (Python + Rust).
- Macro undo một Backspace; auto-cap scanner; charset detect.

### Test / tài liệu

- Golden mới cho các ca trên; conformance tăng từ 13.252 lên 13.311 ca.
- `tests/test_research_regressions.py` chứa ca hồi quy lấy từ nghiên cứu.
- Tài liệu: `COMPATIBILITY.md`, `ADAPTER_GUIDE.md`, `CONFIG.md`, `THIRD_PARTY_NOTICES.md`.

## 5. Việc thuộc adapter (không thể làm trong thư viện thuần)

Các mục này cần mã nền tảng, quyền OS và test trên app thật. ViFastKey cung cấp engine/API,
không tự gửi phím hay vẽ GUI:

- macOS IMK/CGEventTap, Windows TSF/WH_KEYBOARD_LL, Linux Fcitx5/IBus.
- Preedit native, surrounding text, selection/anchor, secure input, accessibility.
- uinput/libei/XTest, delay/ACK, focus bounce, app/domain detection.
- Clipboard/RTF/HTML converter, tray/menu bar, auto-update, signing/notarization.
- Emoji picker, gợi ý từ, AI code-switching.

Xem `ADAPTER_GUIDE.md` để biết hợp đồng từng bước.

## 6. Việc còn lại (roadmap)

- VIQR như **kiểu gõ**, MS Vietnamese layout.
- Macro nhiều từ/suffix, trigger tùy chọn, macro khi tắt tiếng Việt.
- Preedit sửa từ đã commit (`DeleteSurrounding`).
- Từ điển lớn 7k tiếng Việt + 20k tiếng Anh, user dictionary.
- Gợi ý/word prediction chạy local, không gửi mạng.
- Emoji, text tools, clipboard converter GUI.
- Adapter thật + GUI cấu hình.

## 7. Quy tắc giấy phép

- **Được dùng**: hành vi, tên tính năng, test vector tự viết, bảng mã MIT của `bamboo-core`
  (có ghi công), ý tưởng kiến trúc.
- **Không được dùng**: code/dữ liệu/từ điển/test corpus của GPL/AGPL (SKey, UniKey, OpenKey,
  VKey, VnKey, Bamboo/Lotus, ibus-unikey...), kể cả khi chỉ là bảng dữ liệu lớn.
- Nếu muốn dùng bảng mã từ dự án GPL, phải sinh lại từ tiêu chuẩn độc lập hoặc dùng nguồn MIT
  (`bamboo-core`) và giữ ghi công.
- Logo/wordmark không thuộc MIT; xem `resources/brand/README.md`.
