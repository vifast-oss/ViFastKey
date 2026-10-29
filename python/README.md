# ViFastKey v4: bản tham chiếu Python (core + engine)

Bản này là **đặc tả chạy được**. Khi port sang Rust, chỉ cần dịch từng module và cho chạy lại đúng bộ test này.
Bản Rust đạt khi ra cùng kết quả với bản Python trên toàn bộ golden test và corpus.

```bash
python -m pytest -q               # 388 test
python -m tools.export_conformance  # sinh conformance/*.jsonl (oracle cho Rust)
python -m tools.bench             # đo µs/phím
python -m apps.vfk_cli --demo     # chạy ví dụ
python -m apps.vfk_cli            # gõ tương tác (:telex :vni :simple_telex :old :new :replace :preedit :toggle :trace)
```

Chỉ dùng thư viện chuẩn. Muốn chạy test thì cần thêm pytest.

## 1. Cấu trúc và ánh xạ sang Rust

| Python | Crate Rust | Vai trò |
|---|---|---|
| `vi_syllable/` | `vi-syllable` | Tách âm tiết, kiểm tra chính tả 3 trạng thái, đặt thanh kiểu cũ/mới. Dùng chung với ViFastTts |
| `vfk_core/rules.py` | `vfk-core::rules` | Telex, Simple Telex, VNI, Quick Telex dạng dữ liệu |
| `vfk_core/transform.py` | `vfk-core` | `transform(raw, final)`, `to_raw`, `rebuild`, `diff` (hàm thuần) |
| `vfk_engine/ports.py` | `vfk-engine::ports` | `Config` + 4 port ra (trait) |
| `vfk_engine/session.py` | `vfk-engine` | `ImeSession.process_key -> Action` |
| `vfk_ffi/` | `vfk-ffi` | API phẳng, đổi đơn vị xoá sang UTF-16 |
| `adapters/` | `adapters/*` | Port mẫu, từ điển, ô nhập giả lập |
| `tests/golden/*.tsv` | dùng lại nguyên file | `tuỳ chọn<TAB>phím<TAB>kết quả` |
| `tests/data/corpus_vi.txt` | dùng lại nguyên file | Khoảng 525 từ, mỗi từ thử mọi thứ tự gõ |

## 2. Đặc tả hành vi

### 2.1 Mô hình tính toán
- Session chỉ giữ **chuỗi phím thô** của từ hiện tại. Mỗi phím gọi lại `transform(raw)` từ đầu.
- Thanh lưu riêng và chỉ đặt lúc render, nên luôn đúng vị trí dù gõ thanh trước hay sau (`tieesng`, `chuyenje`, `looixfsx` đều đúng).
- `MAX_RAW = 24`: dài hơn thì coi là chuỗi thô (URL, token), không biến đổi.

### 2.2 Chính tả 3 trạng thái (`vi_syllable.check`)
| Trạng thái | Ý nghĩa | Ví dụ |
|---|---|---|
| VALID | Âm tiết hoàn chỉnh | `tiếng`, `quýt`, `khuỷu`, `thuở`, `xoong` |
| PARTIAL | Còn hoàn thiện được bằng cách **gõ thêm chữ hoặc thêm dấu phụ** | `chuyen` (chờ `e`), `ngh`, `ă`, `qu` |
| INVALID | Không thể thành tiếng Việt | `kan`, `ce`, `quu`, `bàt` (âm cuối tắc + huyền) |

PARTIAL có tính đến việc thêm dấu sau, nhờ vậy gõ dấu tự do không bị chặn nhầm.

### 2.3 Đặt thanh
Đặt theo quy tắc đặt dấu thanh của chữ Quốc ngữ, kiểm tra bằng bảng `TONE_TABLE`:
- Ưu tiên `ơ` (cả trong `ươ`, `uơ`), sau đó đến nguyên âm có dấu phụ (`ê`, `â`, `ư`...).
- Có âm cuối thì đặt ở nguyên âm cuối cùng của cụm (`toàn`, `huỳnh`).
- Cụm 3 nguyên âm thì đặt ở nguyên âm giữa (`ngoái`, `khuỷu`).
- `oa`, `oe`, `uy` không có âm cuối: kiểu cũ là `hòa`, kiểu mới là `hoà`.
- `gi`, `qu` được coi là phụ âm đầu (`già`, `quạ`, `quýt`).

### 2.4 Phím chức năng (Telex)
| Hành vi | Ví dụ |
|---|---|
| Dấu phụ quét ngược qua âm cuối | `quatw` ra `quăt`, `tiengse` ra `tiếng` |
| Một `w` sau `uo` tạo `ươ`. Nếu `ươ` đứng cuối từ và móc ở u do tự động thì hiển thị `uơ` | `nguoiwf` ra `người`, `thuowr` ra `thuở` |
| Chọn ứng viên đầu tiên không sai chính tả | `muaw` ra `mưa` |
| `d` ở bất kỳ đâu biến `d` đầu từ thành `đ` | `duowngfd` ra `đường` |
| Gõ lặp lại để hoàn tác | `aaa` ra `aa`, `ass` ra `as`, `ww` ra `w`, `ddd` ra `dd` |
| Hoàn tác **thanh** thì phần còn lại của từ gõ thẳng. Hoàn tác **dấu phụ** chỉ khoá đúng phím đó | `xooongj` ra `xoọng` |
| Thanh bị từ chối nếu sai chính tả | `batf` ra `batf` |
| `z` xoá thanh. Nếu không có thanh thì `z` xoá dấu phụ | `tieengsz` ra `tiêng` |
| Thanh gõ ngay sau `qu` | `qusa` ra `quá` |
| Tuỳ chọn Quick Telex | `ccaof` ra `chào` |
| Tuỳ chọn phím `[` `]` | `m]a` ra `mưa` |

### 2.5 Khôi phục từ tiếng Anh (2 tầng)
1. **Core**: đang gõ mà từ trở thành INVALID thì hiện phím thô (`text`, `windows`). Khi kết thúc từ, PARTIAL có nguyên âm cũng bị khôi phục (`law`, không thành `lă`). Riêng từ người dùng đã chủ động hoàn tác thì core giữ nguyên ý người gõ (`ass` ra `as`).
2. **Engine** (port `Dictionary`): khôi phục khi phím thô là từ tiếng Anh **và** kết quả không phải từ Việt thông dụng. Ví dụ `data` (ra `dât`), `boss`, `server` được khôi phục. Còn `this` ra `thí`, `did` ra `đi` vẫn giữ tiếng Việt.

### 2.6 Engine
- **Backspace xoá 1 ký tự hiển thị** và giữ dấu của phần còn lại (`tiếng` thành `tiến`), giống UniKey/OpenKey. Nếu từ còn lại không tái tạo được từ phím (vd `tex` sau khi khôi phục `text`) thì chuyển sang chế độ thô.
- Esc trả về đúng phím đã gõ. Các phím Ctrl/Alt/Cmd và phím điều hướng kết thúc từ.
- Có macro (giữ kiểu hoa/thường), viết hoa đầu câu, chế độ Việt/Anh theo từng app, loại trừ app.
- Sửa lại từ đã gõ (chế độ REPLACE): `tiêng` + `s` thành `tiếng`. Nếu phím gõ thêm làm từ sai thì chỉ nối phím vào (`Việt` + `f` thành `Việtf`).
- Bên trong luôn dùng NFC. Chỉ chuẩn hoá NFC/NFD ở đầu ra. FFI đổi số ký tự cần xoá sang UTF-16.

### 2.7 Edge case học từ các dự án khác
Xem `EDGE_CASES.md`: bảng tổng hợp từ vnkey, fcitx5-unikey, fcitx5-lotus, ibus-bamboo, OpenKey và UniKey, có trạng thái và test tương ứng. Engine có thêm:
- `restore_after_space` (soft reset)
- `smart_caps`
- `set_field_kind()` (password, terminal...)
- `cursor_moved()`
- Phím modifier nhấn riêng không làm mất từ đang gõ

### 2.8 Mới trong v4
- `Composer`: bộ biến đổi tăng dần, nhanh gấp khoảng 2 lần, luôn bằng `transform()` (có property test).
- Bộ **conformance** trung lập ngôn ngữ: 13.252 ca core và 3.000 kịch bản engine (so từng Action). Xem `PORTING.md`.
- `AppProfile` (`DEFAULT_APP_PROFILES`): terminal dùng preedit, Chromium chờ ack và trễ theo độ dài, Firefox tắt sửa từ cũ, Minecraft tắt bộ gõ.
- Sửa lỗi: sửa từ cũ khi màn hình là NFC nhưng đầu ra là NFD thì trước đây xoá sai số ký tự.
- Thêm golden cho họ `ươ`/`uơ`: `thuowng`, `huowu`, `ruowuj`, `truowcs`...

## 3. Các bất biến đã có property test
- Sau mỗi phím, chữ trên ô nhập luôn khớp với `session.display` ở cả 2 mô hình output.
- PREEDIT và REPLACE cho cùng văn bản cuối với mọi chuỗi phím ngẫu nhiên (có Backspace, Esc, Enter, Left).
- Áp `diff(old, new)` lên `old` luôn ra đúng `new`.
- Gõ một từ rồi Backspace số lần bằng số ký tự hiển thị thì ô nhập về rỗng.
- Gõ toàn bộ bằng chữ hoa thì kết quả đúng bằng bản chữ thường viết hoa.
- Mọi từ trong corpus, gõ theo mọi thứ tự dấu (Telex và VNI), đều ra đúng từ.

## 4. Ghi chú khi port sang Rust
- `Letter` là struct `Copy`. `WordState` dùng `ArrayVec<Letter, 24>`, nên không cần cấp phát heap.
- Các bảng `OPEN_CLUSTERS`, `_RELAXED_ANY`, `_RELAXED_FOR_CODA` sinh lúc build (`build.rs`) thành `phf::Set`.
- `transform` hiện là O(n²) mỗi phím. Có thể tối ưu bằng cache snapshot theo từng tiền tố raw, nhưng chỉ làm sau khi golden test đã qua.
- `Action` là `enum`. Cả `Replace.delete_before` lẫn `Preedit.cursor` đều đổi sang UTF-16 ở lớp FFI.
- Từ điển: bản Python chỉ có danh sách nhỏ trong `adapters/data`. Bản thật nên nạp khoảng 7.000 âm tiết tiếng Việt và khoảng 20.000 từ tiếng Anh.

## 5. Giới hạn đã biết (có chủ đích)
- Âm cuối `k` (`Đắk Lắk`) bị coi là sai chính tả nên sẽ khôi phục phím thô. Muốn gõ được thì cần thêm tuỳ chọn cho tên riêng.
- Gõ `xoong`/`boong` cần gõ `ooo`, giống UniKey.
- Từ vừa là tiếng Anh vừa là từ Việt thông dụng (`this` ra `thí`, `as` ra `á`) luôn ưu tiên tiếng Việt.
- Chưa có bảng mã cũ (TCVN3, VNI-Windows, CP1258).
- Chế độ PREEDIT chưa hỗ trợ sửa lại từ đã gõ, vì cần thêm action `DeleteSurrounding`.
