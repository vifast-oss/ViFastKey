# Edge case tổng hợp từ các bộ gõ khác

Ký hiệu trạng thái: ✅ đã làm, có test · 🔌 thuộc adapter (ghi lại để làm khi viết adapter) · ⏸ hoãn

## A. Core (biến đổi phím)

| # | Edge case | Nguồn | Trạng thái | Test |
|---|---|---|---|---|
| A1 | Một `w` sau `uo` tạo `ươ` (`truowngf` ra `trường`, `nguoiwf` ra `người`) | OpenKey/VietTelex B1 | ✅ | golden |
| A2 | `uow` ở cuối từ ra `uơ` (`thuở`, `huơ`, `quở`). Gõ `w` lần 2 nâng lên `ươ` (`thuowwng` ra `thương`) | fcitx5-unikey testunikey | ✅ | golden |
| A3 | `uwoi` ra `ươi` (móc u gõ trước) | fcitx5-unikey | ✅ | golden |
| A4 | Âm cuối c/ch/p/t chỉ nhận sắc/nặng (`batf` giữ `batf`) | OpenKey/VietTelex B2 | ✅ | golden |
| A5 | `d` cuối từ biến `d` đầu thành `đ` (`duowngd`) | OpenKey C1 | ✅ | golden |
| A6 | `w` quét ngược qua âm cuối (`quatw` ra `quăt`) | OpenKey C2 | ✅ | golden |
| A7 | Thanh gõ sau thay thanh trước (`looixfsx` ra `lỗi`) | bamboo-core | ✅ | golden |
| A8 | Gõ cùng phím thanh 2 lần là hoàn tác (`ass` ra `as`) | bamboo-core, UniKey | ✅ | golden |
| A9 | `aaa` ra `aa`, `aaan` ra `aan` | fcitx5-unikey | ✅ | golden |
| A10 | `WW` ra `W` hoa, không ra `w` thường | vnkey issue #16 | ✅ | golden + engine |
| A11 | `gi`/`qu` là phụ âm đầu (`gìn`, `quét`, `quýt`, `quạp`) | VietTelex, fcitx5-lotus (từ điển thêm `quạp`) | ✅ | golden + syllable |
| A12 | Vần `uy` + âm cuối (`huỵch`, `tuýp`, `khuỷa`) | VietTelex | ✅ | golden |
| A13 | Kiểu đặt dấu mới `oà`, `uý` | OpenKey, vnkey, fcitx5-lotus | ✅ | golden + bảng thanh |
| A14 | Gõ dấu tự do ở mọi vị trí | vnkey, fcitx5-lotus | ✅ | corpus mọi thứ tự gõ |
| A15 | Từ sai chính tả thì khôi phục phím ASCII | vnkey, OpenKey, fcitx5-lotus | ✅ | golden |
| A16 | Quick Telex `cc=ch`, `gg=gi`, `nn=ng`... | OpenKey | ✅ | golden |
| A17 | Simple Telex (không có `w` đứng riêng) | vnkey, OpenKey | ✅ | golden |
| A18 | Gõ tắt phụ âm đầu `f`→`ph`, `j`→`gi`, `w`→`qu` và phụ âm cuối `g`→`ng`, `h`→`nh`, `k`→`ch` | OpenKey | ⏸ | Đụng với phím thanh. Cần thiết kế kiểu gõ riêng |
| A19 | Tiếng Tây Nguyên (Êđê...): `oow`→`ö`, `cw`→`č`... | vnkey v1.0.3 | ⏸ | Chỉ cần thêm `InputMethodSpec` và ký tự mới, không đổi thuật toán |
| A20 | Âm cuối `k` cho địa danh (`Đắk Lắk`) | | ⏸ | Tuỳ chọn tên riêng |

## B. Engine (session)

| # | Edge case | Nguồn | Trạng thái | Test |
|---|---|---|---|---|
| B1 | **Soft reset**: Backspace ngay sau dấu cách thì nạp lại trạng thái từ trước. `Chào` + Space + Backspace + `f` ra `Chaof` | vnkey `soft_reset`, fcitx5-unikey #50 | ✅ | `test_soft_reset_*` |
| B2 | Không soft reset nếu từ đã bị macro hoặc khôi phục làm đổi | (suy ra từ B1) | ✅ | `test_soft_reset_not_after_macro` |
| B3 | Nhấn riêng phím modifier (Shift/Option) không làm mất từ đang gõ | vnkey macOS "Option key over-reset" | ✅ | `test_modifier_alone_*` |
| B4 | Ô mật khẩu, number, email, URL thì tắt bộ gõ | fcitx5-lotus (cờ Password) | ✅ | `test_password_field_disabled` |
| B5 | Click chuột hoặc con trỏ đổi chỗ thì chốt preedit, không để kẹt | fcitx5-unikey #53, ibus-bamboo #607 | ✅ | `test_cursor_moved_commits_preedit` |
| B6 | Focus out phải chốt từ, không làm mất từ | fcitx5-unikey #54 | ✅ | `test_preedit_commit_on_focus_change` |
| B7 | Smart Caps: 2 chữ hoa đầu từ thì gõ thẳng (`JSON`, `HTTP`), trừ `DD` | VNKey (hoquangthaiholy) | ✅ tuỳ chọn | `test_smart_caps` |
| B8 | Macro giữ kiểu hoa/thường (`bc`, `Bc`, `BC`) | vnkey v1.0.3, fcitx5-lotus | ✅ | `test_macro_and_case` |
| B9 | Nhớ chế độ Việt/Anh theo từng app | UniKey 4.6, fcitx5-lotus | ✅ | `test_toggle_remembered_per_app` |
| B10 | Sửa từ đã gõ qua surrounding text; phím làm sai từ thì chỉ nối thêm | fcitx5-unikey (modifySurroundingText) | ✅ | `test_edit_previous_word_*` |
| B11 | Từ tiếng Anh kèm số trong chế độ macro (`mp3`, `k8s`) | fcitx5-lotus #480 | ⏸ | Hiện chữ số là ranh giới từ. Cần buffer token riêng cho macro |

## C. Adapter (ghi lại để khi viết adapter khỏi vấp)

| # | Vấn đề | Nguồn | Cách xử lý |
|---|---|---|---|
| C1 | Terminal (vim, Claude Code) lỗi khi xoá rồi commit tách rời | vnkey v1.0.3 | 🔌 `FieldKind.TERMINAL`: adapter chuyển sang PREEDIT để commit nguyên tử |
| C2 | Electron/Firefox/Notion nuốt backspace giả khi gửi quá nhanh | vnkey (macOS, Windows) | 🔌 Thêm độ trễ backspace theo app. fcitx5-lotus cho độ trễ tăng theo độ dài từ bị xoá |
| C3 | Firefox chèn U+202F, surrounding text sai | vnkey, fcitx5-unikey #50 | 🔌 Chuẩn hoá text trước khi gọi `before_cursor`. Khi nghi ngờ thì tắt sửa từ cũ cho app đó |
| C4 | Lặp chữ trên Chrome do autocomplete thanh địa chỉ | ibus-bamboo #582, #589 | 🔌 Mặc định PREEDIT trên thanh địa chỉ, hoặc kiểu select-then-replace như fcitx5-lotus |
| C5 | GNOME Wayland bỏ qua backspace do IME gửi | Ngó Sen | 🔌 Dùng preedit, hoặc forward key qua framework |
| C6 | Game và Minecraft | fcitx5-lotus (mode Minecraft) | 🔌 Mặc định loại trừ app |
| C7 | Phím số ở bàn phím phụ bị loạn trên Chrome | ibus-bamboo #561 | 🔌 Phím numpad luôn PassThrough |
| C8 | `Win+D` hoặc chuyển cửa sổ làm treo trạng thái | vnkey v1.0.1 | 🔌 Gọi `focus_changed` khi đổi foreground window |
| C9 | Đơn vị xoá UTF-16, chữ NFD | (tất cả) | ✅ Đã xử lý ở `vfk_ffi` |
| C10 | Bảng mã cũ TCVN3, VNI-Windows, VISCII, CP1258 (vnkey có 15 bảng mã) | vnkey, OpenKey | ⏸ Làm ở bước chuyển đổi đầu ra, sau `_out()` |

## D. Lỗi tìm ra trong quá trình làm v4

| # | Lỗi | Nguyên nhân | Sửa |
|---|---|---|---|
| D1 | Sửa từ cũ khi ô nhập đang chứa NFC nhưng cấu hình xuất NFD thì xoá thiếu hoặc thừa ký tự | So độ dài trên 2 dạng chuẩn hoá khác nhau | Chỉ nạp lại khi chữ trên màn hình đã đúng dạng đầu ra. Tìm ra nhờ conformance ngẫu nhiên |
| D2 | Ranh giới từ bị cắt giữa chữ NFD (dấu tổ hợp không phải `isalpha`) | Duyệt ngược chỉ theo `isalpha` | Duyệt ngược cả dấu tổ hợp (`unicodedata.combining`) |

## E. Họ ươ / uơ (trả lời câu hỏi `thuowng`)

| Gõ | Ra | Ghi chú |
|---|---|---|
| `thuowng` | thương | Một `w` là đủ. `uo` + `w` tạo cặp `ươ` |
| `thuowwng` | thương | Kiểu UniKey: `w` thứ hai nâng `uơ` lên `ươ`, cũng ra đúng |
| `thuow` | thuơ | Đang gõ dở. Gõ thêm `ng` thì thành `ương`, gõ `r` thì thành `thuở` |
| `uowi`, `huowu` | ươi, hươu | `ươ` + bán âm cuối |
| `giowf`, `gioowf` | giờ | `w` sau `ô` đổi `ô` thành `ơ` |

Quy tắc chính tả làm nền: `ươ`, `uô`, `iê` chỉ đứng trước âm cuối (gồm cả bán âm i/u), còn `uơ` chỉ có trong âm tiết mở (`thuở`, `huơ`, `quở`).
