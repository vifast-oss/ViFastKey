# Bộ test vector tiếng Việt (tổng hợp từ nghiên cứu)

Các ca dưới đây là **hợp đồng hành vi** của ViFastKey. Chúng được chạy trong
`python/tests/test_research_regressions.py`, golden `tests/golden/*.tsv`, và conformance
`core.jsonl`/`engine.jsonl`; bản Rust phải khớp 100%.

Quy ước: `raw` là chuỗi phím; kết quả là văn bản cuối từ (final) trừ khi ghi khác.
Các ca được restate từ hành vi của SKey/UniKey/Telebit/VnKey/GõViệt/OpenKey/VKey/Bamboo,
không chép code hay dữ liệu GPL.

## 1. Telex cơ bản

| Raw | Kết quả |
|---|---|
| `tieengs` | tiếng |
| `Vieejt` | Việt |
| `nguwowif` | người |
| `thuowng` | thương |
| `thuowwng` | thương |
| `quyeets` | quyết |
| `khuỷu` (đã có dấu) | khuỷu |
| `dduwowngf` | đường |
| `ddi` | đi |
| `qusa` | quá |
| `hoaf` | hòa (old) / hoà (new) |

## 2. Undo / khôi phục tiếng Anh

| Raw | Kết quả | Ghi chú |
|---|---|---|
| `aaa` | aa | escape |
| `aaan` | aan | escape + coda |
| `ass` | as | undo thanh ở cuối |
| `chuss` | chus | undo thanh ở cuối |
| `DDD` | DD | escape hoa |
| `banana` | banana | không mất chữ |
| `Canada` | Canada | |
| `coffee` | coffee | |
| `cheese` | cheese | |
| `daddy` | daddy | |
| `airport` | airport | |
| `mission` | mission | |
| `necessary` | necessary | |
| `soffa` | soffa | khác Telebit (`sofa`) — ưu tiên an toàn raw |
| `text` | text | |
| `texts` | texts | sticky restore ở engine |
| `boss` | boss | engine dictionary (core: bos) |

## 3. `đ` / d

| Raw | Kết quả |
|---|---|
| `ddc` | đc |
| `vcdd` | vcđ |
| `nddm` | nđm |
| `kdd` | kđ |
| `cmdd` | cmđ |
| `avcdd` | avcđ |
| `NDD` | NĐ |
| `NDd` | NĐ |
| `ddi` | đi |
| `duowngd` | đương |
| `duowngfd` | đường |
| `add` | add |
| `address` | address |
| `daddy` | daddy |

## 4. Nguyên âm có dấu đứng riêng

| Raw | Kết quả |
|---|---|
| `aw` | ă |
| `aws` | ắ |
| `aa` | â |
| `aas` | ấ |
| `ee` | ê |
| `ees` | ế |
| `oo` | ô |
| `oos` | ố |
| `uw` | ư |
| `uws` | ứ |
| `ow` | ơ |
| `ows` | ớ |
| `w` | ư |
| `ws` | ứ |

## 5. Họ `uơ` / `ươ`

| Raw | Kết quả | Âm đầu |
|---|---|---|
| `uow` | uơ | — |
| `huow` | huơ | h |
| `thuow` | thuơ | th |
| `thuowr` | thuở | th |
| `khuow` | khuơ | kh |
| `quow` | quơ | qu |
| `duow` | dươ | d (không cho uơ mở) |
| `duowf` | dườ | d |
| `tuowf` | tườ | t |
| `luowj` | lượ | l |
| `nguow` | ngươ | ng |
| `phuow` | phươ | ph |
| `duowng` | dương | có coda |
| `thuowng` | thương | có coda |

## 6. Tone / chính tả

| Raw | Kết quả |
|---|---|
| `batf` | batf (coda t không nhận huyền) |
| `looixfsx` | lỗi |
| `xooongj` | xoọng |
| `quatw` | quăt |
| `thuowj` | thuợ (partial) |
| `toanf` | toàn |
| `huynhf` | huỳnh |
| `khuyur` | khuỷu |
| `ngoeof` | ngoèo |
| `giuwx` | giữ |
| `gif` | gì |

## 7. VNI

| Raw | Kết quả |
|---|---|
| `Vie65t` | Việt |
| `nguo7i2` | người |
| `d9i` | đi |
| `a8n` | ăn |
| `tu7o7ng` | tương |
| `toa10n` | toan |
| `viet65` | việt |
| `nguye64n` | nguyễn |
| `a11` | a1 (escape) |
| `nam2024` | nam2024 (số) |
| `ba10` | ba10 (số) |
| `top10` | top10 (số) |
| `2024` | 2024 |
| `box1` | box1 |

## 8. Auto-cap / câu

| Chuỗi | Kết quả |
|---|---|
| `xin chaof. ddi` | xin chào. Đi |
| `xin chaof… ddi` | xin chào… Đi |
| `xin chaof.” ddi` | xin chào.” Đi |
| `xin chaof\n\nddi` | xin chào\n\nĐi |
| `1.5 met` | 1.5 met (không cap) |
| `v.v.` | chưa xử lý viết tắt (giới hạn) |

## 9. Macro

| Cấu hình | Chuỗi | Kết quả |
|---|---|---|
| `ko=không` | `ko ` | không |
| `ko=không` | `Ko KO` | Không KHÔNG |
| `mp3=máy nghe nhạc`, `macro_digits` | `mp3 ` | máy nghe nhạc |
| `mp3=...`, không `macro_digits` | `mp3 ` | mp3 |
| `ko=không`, `macro_undo` | `ko ` + BS | ko |
| `ko=không`, `macro_undo=false` | `ko ` + BS | không (OS xoá space) |

## 10. Bảng mã

| Unicode | Charset | Kết quả/bytes |
|---|---|---|
| `tiếng Việt` | VIQR | `tie^'ng Vie^.t` |
| `mưa.` | VIQR | `mu+a\.` |
| `Tiếng Việt` | TCVN3 bytes | `54 69 d5 6e 67 20 56 69 d6 74` |
| `Việt` | NCR decimal | `Vi&#7879;t` |
| `Việt` | NCR hex | `Vi&#x1EC7;t` |
| `Việt` | CP1258 | chuỗi nhiều codepoint; decode lại `Việt` |
| `Tiếng Việt` | mọi charset | encode → decode = `Tiếng Việt` (trừ mã dùng chung hoa/thường) |

## 11. Engine / context

| Tình huống | Kỳ vọng |
|---|---|
| Esc sau `vieejt` | trả `vieejt`, `pass_key=false` |
| Enter sau `vieejt` | commit `việt`, `pass_key=true` |
| Backspace `tieengs` | `tiến` |
| Backspace sau dấu cách | soft reset (`Chào ` + BS + `f` → `Chaof`) |
| Focus đổi khi preedit | commit từ, không mất chữ |
| Password/email/URL/number | pass through |
| App profile `kitty` | preedit, edit-previous-word=false |
| App profile `firefox` | edit-previous-word=false, delay 4ms |
| App profile `minecraft` | tiếng Việt tắt |
| App profile `code` + VNI | gõ VNI trong app đó |
