# Roadmap

Trạng thái hiện tại: lõi Python + Rust, C ABI, WASM, CLI, playground và bộ conformance đã chạy được.
Các mục dưới đây là phần còn thiếu để thành sản phẩm dùng hằng ngày.

## Đã hoàn thành trong đợt nghiên cứu

- `đ` viết tắt, undo tiếng Anh, VNI literal number, sticky restore, escape pass-key.
- Open `uơ`/`ươ`, nguyên âm có dấu đứng riêng, auto-cap mở rộng.
- `macro_digits`, `macro_undo`; AppProfile ghi đè kiểu gõ/kiểu dấu/chính tả.
- 16 bảng mã legacy + CLI/FFI/WASM; config `schema_version`; tài liệu nghiên cứu/hợp đồng.

## Ưu tiên cao

- **Adapter thật** cho từng nền tảng:
  - macOS: ✅ IMK (Swift) đã có trong `macos/`; còn Developer ID/notarize/universal2 và
    event-tap fallback.
  - Windows: TSF (C++) hoặc hook + replace.
  - Linux: Fcitx5/IBus (C++), ưu tiên preedit cho terminal.
  - Android/iOS: bàn phím riêng dùng C ABI/UniFFI, xử lý preedit và surrounding text.
- **Tích hợp bảng mã vào đầu ra engine** (hiện là module/CLI/FFI riêng): cần định nghĩa
  đơn vị xoá byte/UTF-16 cho từng adapter và test trên app thật.
- **Sửa từ đã gõ trong PREEDIT**: cần action `DeleteSurrounding`/replace cho vùng đã commit,
  sau đó adapter áp lên TSF/IMK/Fcitx5. Hiện chỉ REPLACE hỗ trợ.
- **Macro nhiều từ/suffix, trigger tùy chọn, macro khi tắt tiếng Việt.**
- **Từ điển lớn**: nạp ~7.000 âm tiết Việt và ~20.000 từ tiếng Anh; giữ danh sách nhỏ hiện tại
  làm fallback để test nhanh.

## Ưu tiên trung bình

- **VIQR** ✅ đã có (dấu câu là `extra_word_keys`); còn MS Vietnamese layout và các kiểu gõ khác.
- **Gợi ý/word prediction local**, user dictionary, emoji, clipboard GUI.
- **Gõ tắt phụ âm** (OpenKey): `f`→`ph`, `j`→`gi`, `w`→`qu`, coda `g`→`ng`... cần kiểu gõ riêng
  để không đụng phím thanh.
- **Tiếng Tây Nguyên** (Êđê...): thêm ký tự và `InputMethodSpec`, không đổi thuật toán.
- **Bộ gõ tắt/macro nâng cao**: nhiều macro theo app, import/export, kiểm tra trùng.
- **UI cấu hình desktop**: hiện chỉ có CLI, file JSON và playground.

## Hiệu năng / chất lượng

- Thay `Vec` bằng `ArrayVec` cố định 24/25 phần tử trong `WordState`/`Composer`.
- Bitset cho `locked`, buffer `[char; 24]` cho `toneless_lower`, cache snapshot theo tiền tố raw.
- Fuzz hai bản Python/Rust bằng cùng seed, thêm property test cho mọi `InputMethodSpec`.
- Đo benchmark tự động trong CI (ngưỡng µs/phím) và theo dõi hồi quy.

## Đóng gói

- Release binary CLI cho macOS/Linux/Windows, WASM npm package, C header + staticlib.
- UniFFI cho Swift/Kotlin.
- GitHub Pages cho `web/` và trang tài liệu API.
