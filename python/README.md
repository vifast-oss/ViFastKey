# ViFastKey v4: bản tham chiếu Python (core + engine)

Bản này là **đặc tả chạy được**. Khi port sang Rust, chỉ cần dịch từng module và cho chạy lại đúng bộ test này.
Bản Rust đạt khi ra cùng kết quả với bản Python trên toàn bộ golden test và corpus.

```bash
python -m pytest -q               # 623 test
python -m tools.export_conformance  # sinh conformance/*.jsonl (oracle cho Rust)
python -m tools.bench             # đo µs/phím
python -m apps.vfk_cli --demo     # chạy ví dụ
python -m apps.vfk_cli            # REPL (:telex :vni :simple_telex :viqr :old :new :replace :preedit :toggle :trace :coda_k :macro_digits :spell :help)
python -m apps.vfk_cli --text "Tieengs Vieejt"
python -m apps.vfk_cli --method vni --tone new --text "Vie65t Nam"
python -m apps.vfk_charset --to tcvn3 --text "Tiếng Việt"
python -m apps.vfk_charset --list
python -m apps.vfk_charset --clipboard --to tcvn3
python -m apps.vfk_suggest nguoi
python -m apps.vfk_emoji "cười"
```

Chỉ dùng thư viện chuẩn khi chạy engine. Muốn chạy test:
`pip install -e ".[dev]"` (pytest + ruff) hoặc cài `pytest` riêng.
Cấu hình mẫu đầy đủ: `python/config.example.json`.

## 1. Cấu trúc và ánh xạ sang Rust

| Python | Crate Rust | Vai trò |
|---|---|---|
| `vi_syllable/` | `vi-syllable` | Tách âm tiết, kiểm tra chính tả 3 trạng thái, đặt thanh kiểu cũ/mới. Parser chính tả riêng; ViFastTts có frontend âm vị riêng |
| `vfk_core/rules.py` | `vfk-core::rules` | Telex, Simple Telex, VNI, VIQR, Quick Telex dạng dữ liệu |
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
| `d` ở bất kỳ đâu biến `d` đầu từ thành `đ`; `d` sau phụ âm tạo đ viết tắt | `duowngfd` ra `đường`, `vcdd` ra `vcđ`, `ddc` ra `đc` |
| Gõ lặp lại để hoàn tác | `aaa` ra `aa`, `ass` ra `as`, `ww` ra `w`, `ddd` ra `dd` |
| Hoàn tác giữa từ tiếng Anh thì khôi phục raw | `coffee`, `banana`, `cheese`, `daddy` giữ nguyên |
| Hoàn tác **thanh** thì phần còn lại của từ gõ thẳng. Hoàn tác **dấu phụ** chỉ khoá đúng phím đó | `xooongj` ra `xoọng` |
| Thanh bị từ chối nếu sai chính tả | `batf` ra `batf` |
| `z` xoá thanh. Nếu không có thanh thì `z` xoá dấu phụ | `tieengsz` ra `tiêng` |
| Thanh gõ ngay sau `qu` | `qusa` ra `quá` |
| Tuỳ chọn Quick Telex | `ccaof` ra `chào` |
| Tuỳ chọn phím `[` `]` | `m]a` ra `mưa` |

### 2.5 Khôi phục từ tiếng Anh (2 tầng)
1. **Core**: đang gõ mà từ trở thành INVALID thì hiện phím thô (`text`, `windows`). Khi kết thúc từ, PARTIAL có âm đầu cũng bị khôi phục (`law`, không thành `lă`), trừ nguyên âm có dấu đứng một mình (`aw` → `ă`) và họ `ươ/uơ` (`duow` → `dươ`). Từ đã chủ động hoàn tác ở cuối từ thì core giữ ý người gõ (`ass` ra `as`); hoàn tác nằm giữa từ tiếng Anh thì khôi phục raw (`coffee`).
2. **Engine** (port `Dictionary`): khôi phục khi phím thô là từ tiếng Anh **và** kết quả không phải từ Việt thông dụng. Ví dụ `data` (ra `dât`), `boss`, `server` được khôi phục. Còn `this` ra `thí`, `did` ra `đi` vẫn giữ tiếng Việt.

### 2.6 Engine
- **Backspace xoá 1 ký tự hiển thị** và giữ dấu của phần còn lại (`tiếng` thành `tiến`), giống UniKey/OpenKey. Nếu từ còn lại không tái tạo được từ phím (vd `tex` sau khi khôi phục `text`) thì chuyển sang chế độ thô.
- Esc trả về đúng phím đã gõ. Các phím Ctrl/Alt/Cmd và phím điều hướng kết thúc từ.
- Có macro (giữ kiểu hoa/thường), viết hoa đầu câu, chế độ Việt/Anh theo từng app, loại trừ app.
- `macro_digits`: chữ số thuộc từ khi đang gõ, nhờ đó macro có số hoạt động (`mp3`, `k8s`). VNI không bị ảnh hưởng vì chữ số là phím chức năng.
- `macro_undo`: Backspace ngay sau macro khôi phục trigger + dấu cách (`ko ` → `không ` → `ko `).
- `allow_coda_k`: cho phép âm cuối `k` cho tên riêng/địa danh (`Đắk Lắk`). `k` là âm cuối tắc nên chỉ đi với sắc/nặng.
- Esc trả raw nhưng **không** forward phím Esc (`pass_key=False`).
- PREEDIT sửa từ đã gõ nếu adapter cung cấp surrounding text; engine trả `Replace` thay vì Preedit.
- Gợi ý từ/emoji local (`vfk_suggest`/`vfk_core.emoji`), không mạng.
- Auto-cap xử lý `…`, ngoặc kép/đóng, newline; `1.5` không phải ranh giới câu.
- `AppProfile` ghi đè được `input_method`, `tone_style`, `spell_check` và gợi ý `charset`.
- VNI: chuỗi số ở cuối có ≥2 phím thanh là số thật (`nam2024`, `ba10`); `viet65` vẫn chuyển.
- Sửa lại từ đã gõ (chế độ REPLACE): `tiêng` + `s` thành `tiếng`. Nếu phím gõ thêm làm từ sai thì chỉ nối phím vào (`Việt` + `f` thành `Việtf`).
- Bên trong luôn dùng NFC. Chỉ chuẩn hoá NFC/NFD ở đầu ra. FFI đổi số ký tự cần xoá sang UTF-16.

### 2.7 Edge case học từ các dự án khác
Xem `EDGE_CASES.md`: bảng tổng hợp từ vnkey, fcitx5-unikey, fcitx5-lotus, ibus-bamboo, OpenKey và UniKey, có trạng thái và test tương ứng. Engine có thêm:
- `restore_after_space` (soft reset)
- `smart_caps`
- `set_field_kind()` (password, terminal...)
- `cursor_moved()`
- Phím modifier nhấn riêng không làm mất từ đang gõ

### 2.8 Mới trong bản hiện tại
- `Composer`: bộ biến đổi tăng dần, nhanh gấp khoảng 2 lần, luôn bằng `transform()` (có property test).
- Bộ **conformance** trung lập ngôn ngữ: 13.337 ca core và 3.000 kịch bản engine (so từng Action). Xem `PORTING.md`.
- `AppProfile` (`DEFAULT_APP_PROFILES`): terminal dùng preedit, Chromium chờ ack và trễ theo độ dài, Firefox tắt sửa từ cũ, Minecraft tắt bộ gõ.
- Sửa lỗi: sửa từ cũ khi màn hình là NFC nhưng đầu ra là NFD thì trước đây xoá sai số ký tự.
- Thêm golden cho họ `ươ`/`uơ`: `thuowng`, `huowu`, `ruowuj`, `truowcs`...
- FFI Python đầy đủ như bản C: `vfk_process_char`, focus, surrounding, field kind, toggle, config JSON một phần, app profile, dictionary.
- Tuỳ chọn mới: `allow_coda_k`, `macro_digits`, `macro_undo`; CLI có `--text`, `--method`, `--tone`, `--mode`, `--nfd`, `--macro`.
- Module bảng mã 16 charset + CLI `vfk-charset`; FFI C/WASM/Python có hàm charset.
- `schema_version` trong config; `config_from_dict` bỏ qua field lạ.

### 2.9 FFI Python (`vfk_ffi`)
`vfk_ffi` mô phỏng đúng bề mặt C ABI để viết adapter/thử nghiệm mà không cần Rust:
`vfk_session_new/free`, `vfk_process_key/process_char`, `vfk_focus_changed`,
`vfk_set_surrounding_text`, `vfk_cursor_moved`, `vfk_set_field_kind`,
`vfk_toggle_vietnamese`, `vfk_reset`, `vfk_set_config_json` (JSON một phần),
`vfk_config_json`, `vfk_config_version`, `vfk_app_profile_json`, `vfk_load_dictionary`.
Mọi độ dài xoá/con trỏ trong JSON trả về đều là UTF-16. Handle sai/đã free an toàn như session rỗng.

Tài liệu đối chiếu 19 dự án IME: `../docs/IME_FEATURE_RESEARCH.md`; hợp đồng hành vi:
`../docs/COMPATIBILITY.md`; test vector: `../docs/IME_TEST_VECTORS.md`.

## 3. Các bất biến đã có property test
- Sau mỗi phím, chữ trên ô nhập luôn khớp với `session.display` ở cả 2 mô hình output.
- PREEDIT và REPLACE cho cùng văn bản cuối với mọi chuỗi phím ngẫu nhiên (có Backspace, Esc, Enter, Left).
- Áp `diff(old, new)` lên `old` luôn ra đúng `new`.
- Gõ một từ rồi Backspace số lần bằng số ký tự hiển thị thì ô nhập về rỗng.
- Gõ toàn bộ bằng chữ hoa thì kết quả đúng bằng bản chữ thường viết hoa.
- Mọi từ trong corpus, gõ theo mọi thứ tự dấu (Telex và VNI), đều ra đúng từ.

## 4. Ghi chú khi port sang Rust
- `Letter` là struct `Copy`. `WordState`/`Composer` hiện dùng `Vec`, mọi thứ bị chặn bởi `MAX_RAW`; thay bằng `ArrayVec` là tối ưu sau khi conformance đã xanh.
- Các bảng `OPEN_CLUSTERS`, `_RELAXED_ANY`, `_RELAXED_FOR_CODA` được dựng một lần bằng `OnceLock` (không dùng `phf`).
- `transform` hiện là O(n²) mỗi phím. Có thể tối ưu bằng cache snapshot theo từng tiền tố raw, nhưng chỉ làm sau khi golden test đã qua.
- `Action` là `enum`. Cả `Replace.delete_before` lẫn `Preedit.cursor` đều đổi sang UTF-16 ở lớp FFI.
- Từ điển: bản Python chỉ có danh sách nhỏ trong `adapters/data`. Bản thật nên nạp khoảng 7.000 âm tiết tiếng Việt và khoảng 20.000 từ tiếng Anh.

## 5. Giới hạn đã biết (có chủ đích)
- Âm cuối `k` mặc định vẫn bị coi là sai chính tả; bật `allow_coda_k` để gõ tên riêng (`Đắk Lắk`).
- Macro chưa hỗ trợ nhiều từ/suffix; macro undo chỉ một phím trong REPLACE.
- Gõ `xoong`/`boong` cần gõ `ooo`, giống UniKey.
- Từ vừa là tiếng Anh vừa là từ Việt thông dụng (`this` ra `thí`, `as` ra `á`) luôn ưu tiên tiếng Việt.
- Bảng mã legacy có module/CLI/FFI riêng, chưa nhúng vào `ImeSession` (adapter chuyển ở đầu ra).
- PREEDIT sửa từ đã gõ cần surrounding text đáng tin cậy; adapter không có thì bỏ qua.
