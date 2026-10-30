# Hợp đồng tương thích ViFastKey

Tài liệu này mô tả hành vi **có chủ đích** của core/engine sau đợt đối chiếu SKey, UniKey,
OpenKey, VnKey, Vigo, GõViệt, VKey, Funput, Bamboo/Lotus và Telebit. Khi port sang nền tảng
khác, phải giữ đúng các ví dụ ở đây; các khác biệt phải được ghi vào `CHANGELOG.md` trước.

## 1. Kiểu gõ

- Telex: `s f r x j` = sắc, huyền, hỏi, ngã, nặng; `z` xoá thanh/dấu.
- Simple Telex: như Telex nhưng không có `w` đứng riêng, không `[ ]`.
- VNI: `1 2 3 4 5` thanh; `6` mũ; `7` móc; `8` trăng; `9` đ; `0` xoá.
- VIQR: `^` mũ, `(` trăng, `+` móc, `d` → đ; thanh `'` `?` `~` `.`; `0` xoá dấu.
  Dấu câu thuộc từ khi raw khác rỗng; gõ dấu cách trước dấu câu cuối câu.
- Bracket: `[` → ơ, `]` → ư (khi bật `bracket_keys`).
- Quick Telex: `cc ch`, `gg gi`, `kk kh`, `nn ng`, `qq qu`, `pp ph`, `tt th` ở đầu từ.
- `w` đứng riêng → `ư` khi bật `w_as_u`; mặc định `w` là chữ `w`.
- Chữ hoa/thường được giữ; `đ` → `Đ`.

## 2. Đặt thanh

- Ưu tiên `ơ` (trong `ươ`, `uơ`), sau đó nguyên âm có dấu phụ (`ê`, `â`, `ă`...).
- Có âm cuối: đặt ở nguyên âm cuối của cụm (`toàn`, `huỳnh`).
- Cụm 3 nguyên âm: giữa (`ngoái`, `khuỷu`).
- `oa`, `oe`, `uy` không có âm cuối: cũ `hòa`, mới `hoà`.
- `gi`/`qu` là âm đầu (`già`, `quýt`, `quạ`).
- `allow_coda_k`: chỉ `k`, chỉ sắc/nặng (`Đắk Lắk`, `lắk`); không áp cho từ thường.

## 3. Chính tả 3 trạng thái

| Trạng thái | Nghĩa | Ví dụ |
|---|---|---|
| VALID | âm tiết hoàn chỉnh | `tiếng`, `quýt`, `xoong` |
| PARTIAL | còn gõ thêm chữ/dấu được | `chuyen`, `ngh`, `ă`, `uơ` |
| INVALID | không thể thành tiếng Việt | `kan`, `ce`, `bàt` |

Khi `spell_check=true`, biến đổi làm INVALID bị chặn và từ có thể được khôi phục raw.

## 4. Undo, khôi phục, backspace

- Gõ lặp cùng phím thanh/dấu liền kề là hoàn tác: `ass → as`, `aaa → aa`, `DDD → DD`.
- Hai phím thanh liền kề luôn là escape chuẩn Telex: bỏ dấu và giữ một ký tự literal;
  ba phím mới giữ hai ký tự: `coffee → cofee`, `cofffee → coffee`,
  `mission → mision`, `misssion → mission`, `serrver → server`.
- `ass → as` và `chuss → chus` được giữ (không khôi phục `ass`/`chuss`) vì người dùng
  chủ động hoàn tác ở cuối từ.
- Khôi phục tiếng Anh: core trả raw khi INVALID; engine còn dùng `Dictionary` khi kết thúc từ.
- **PREEDIT sửa từ đã gõ**: nếu adapter cung cấp surrounding text, engine nạp lại từ trước con trỏ
  và trả `Replace` (delete/insert) thay vì `Preedit`; click/focus làm tắt adopt cho key đầu.
- **Sticky restore**: sau khi một từ đã bị khôi phục raw, các phím tiếp theo vẫn ở chế độ raw
  cho tới khi kết thúc từ: `text` + `s → texts`, không thành `tét`.
- **Backspace** xoá một ký tự hiển thị, giữ dấu phần còn lại: `tiếng → tiến`.
- Soft reset: Backspace ngay sau dấu cách nạp lại từ trước (`Chào ` + BS + `f → Chaof`),
  trừ khi từ đã bị macro/khôi phục đổi.
- **Macro undo**: ngay sau macro + dấu cách, một Backspace khôi phục trigger + dấu cách
  (`ko ` → `không ` → `ko `); Backspace tiếp theo xoá dấu cách (`ko`).
- `macro_undo=false` tắt hành vi này.

## 5. `đ` và họ `uơ` / `ươ`

- `dd` → `đ`; `d` sau phụ âm/đầu từ cũng tạo đ viết tắt: `ddc → đc`, `vcdd → vcđ`,
  `nddm → nđm`, `kdd → kđ`, `NDD → NĐ`.
- `add`, `address`, `daddy`, `sudden`, `riddle`, `ladder` giữ nguyên (không thành `ađ...`).
- `d` cuối từ đánh dấu `d` đầu: `duowngd → đương`, `duowngfd → đường`.
- `aaa`/`DDD` là escape, không phải `đ`.
- `aw/aws/aa/...` một mình giữ nguyên âm có dấu: `ă`, `ắ`, `â`...
- Open `uơ` chỉ sau âm đầu `""`, `c`, `h`, `k`, `kh`, `qu`, `th`:
  - `thuow → thuơ`, `huow → huơ`, `quow → quơ`, `uow → uơ`
  - `duow → dươ`, `tuowf → tườ`, `luowj → lượ`, `nguow → ngươ`, `phuow → phươ`
- Có âm cuối thì luôn `ươ`: `duowng → dương`, `thuowng → thương`.

## 6. VNI và số

- VNI literal number: chuỗi số ở cuối có **từ 2 phím thanh (0–5)** trở lên là số thật:
  `nam2024`, `ba10`, `top10`, `a10` giữ nguyên.
- Tổ hợp dấu + thanh vẫn chuyển: `viet65 → việt`, `nguye64n → nguyễn`, `toa10n → toan`.
- Cặp số lặp là escape: `a11 → a1`; `viet655 → viet65`.
- Chữ số không bắt đầu một từ (giống VNI/UniKey): gõ `2024` ở đầu đi thẳng ra app.

## 7. Auto-cap

- Bật `auto_capitalize`: viết hoa chữ đầu câu sau `.`, `!`, `?`, `…` và dấu cách.
- Ngoặc kép/đóng `" ) ] » ” ’ ›` sau dấu câu không làm mất trạng thái câu.
- Xuống dòng luôn là ranh giới câu (kể cả không có dấu chấm).
- `1.5` không phải ranh giới câu (số thập phân).
- Enter luôn bật cap cho dòng mới; field password/email/URL/number tắt IME hoàn toàn.
- Không viết hoa sau chữ viết tắt kiểu `v.v.`, `U.S.` (giới hạn đã biết; cần từ điển
  viết tắt nếu muốn xử lý).

## 8. Macro

- Khớp chính xác từ hiện tại hoặc raw, không phân biệt hoa/thường.
- Giữ kiểu hoa: `ko → không`, `Ko → Không`, `KO → KHÔNG`.
- `macro_digits` giữ chữ số trong từ (`mp3`, `k8s`); cần `macro_enabled`.
- Macro chạy ở mọi dấu kết thúc (space, Enter, Tab, dấu câu).
- Macro nhiều từ: key có thể chứa dấu cách (`k biet`), khớp đuôi câu trong lịch sử
  REPLACE; ưu tiên key dài nhất, kiểm tra biên từ.
- `macro_in_off_mode`: macro vẫn chạy khi Tắt tiếng Việt (mặc định tắt).
- Chưa hỗ trợ key có dấu câu bên trong/suffix trong một từ; xem roadmap.

## 9. Bảng mã

Module `vfk_core.charset` (Python) / `vfk_core::charset` (Rust) hỗ trợ:

| Key | Tên | Kiểu |
|---|---|---|
| `unicode` | Unicode NFC | chuỗi |
| `unicode_nfd` | Unicode NFD | chuỗi |
| `tcvn3` | TCVN3 (ABC) | 1 byte |
| `vni_windows` | VNI Windows | 1 byte (một số chữ 2 codepoint) |
| `cp1258` | Windows 1258 | chuỗi nhiều codepoint |
| `viqr` | VIQR | ASCII + escape |
| `viscii` | VISCII | 1 byte |
| `vps` | VPS | 1 byte |
| `bkhcm1`, `bkhcm2` | BKHCM | 1 byte |
| `vietware_x`, `vietware_full` | Vietware | 1 byte |
| `ncr_decimal`, `ncr_hex` | NCR | ASCII |
| `cstring_hex`, `cstring_decimal` | C string | ASCII |

- `encode()` trả chuỗi codepoint legacy (giống bamboo-core). `encode_bytes()` chỉ dùng cho
  bảng một byte. `decode()` nhận chuỗi hoặc bytes và trả NFC.
- TCVN3/Vietware Full dùng chung mã cho một số chữ hoa/thường; decode chọn chữ thường nếu
  hai ký tự chỉ khác case, bỏ qua nếu khác hẳn. Đây là giới hạn của bảng mã, không phải lỗi.
- VIQR escape `\.` khi dấu câu sẽ dính vào mã trước; decode hiểu `\` + dấu câu là literal.
- Không dùng charset trong `ImeSession` trực tiếp: engine trả Unicode; adapter chuyển ở
  đầu ra (hoặc dùng CLI/FFI) để độ dài xoá và UTF-16 luôn đúng.

## 10. Hành vi `Action`

| Action | Trường | Ghi chú |
|---|---|---|
| `PassThrough` | — | OS xử lý phím gốc |
| `Preedit` | text, cursor | cursor code point trong engine; UTF-16 ở FFI |
| `Replace` | delete_before, insert, pass_key | delete_before code point; FFI đổi UTF-16 |
| `Commit` | text, pass_key | commit preedit |

- **Escape**: `pass_key=false` — adapter không được gửi tiếp phím Esc.
- Enter: `pass_key=true` khi cần xuống dòng.
- Dấu cách/kết thúc từ: `pass_key=true` để OS chèn dấu cách.
- PREEDIT: dấu cách/Enter được gộp vào `Commit`; REPLACE: dùng `Replace(pass_key=true)`.
- NFD: mọi độ dài trong engine theo code point của chuỗi NFD; FFI đổi sang UTF-16.

## 11. Config

- `schema_version` hiện là 1; file thiếu trường được hiểu là 1; engine bỏ qua field lạ.
- UI desktop (`vfk-config`) giữ field lạ khi ghi; adapter chỉ ghi runtime key (`app_modes`).
- `config_from_dict`/`Config::apply_json_patch` merge JSON một phần, không reset field thiếu.
- AppProfile có thể ghi đè `input_method`, `tone_style`, `spell_check`, gợi ý `charset`.
- Xem `CONFIG.md` để biết toàn bộ trường.
