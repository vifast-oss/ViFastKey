# Hướng dẫn viết adapter cho ViFastKey

Engine là thư viện thuần: nhận `KeyEvent`, trả một `Action`. Adapter là phần duy nhất được
phép nói chuyện với OS/app. Tài liệu này là hợp đồng tối thiểu để adapter không gây mất chữ,
nhân đôi ký tự hoặc làm treo preedit — các lỗi phổ biến nhất của IME tiếng Việt.

## 1. Kiến trúc

```
OS key event ──► Adapter ──► ImeSession::process_key ──► Action ──► Adapter áp lên app
                    │                                              │
                    ├─ app/window/field context                    ├─ preedit
                    ├─ surrounding text                            ├─ replace/backspace
                    ├─ delay/ACK                                   └─ commit
                    └─ config/app profile
```

Không đưa logic tiếng Việt vào adapter. Không dịch lại raw/Unicode ở adapter; engine đã xử lý.

### 1.1 Adapter-core dùng chung

Các quyết định dễ gây mất chữ được gom vào crate `vfk-adapter-core` và C ABI
`vfk_plan_action`:

- field policy: password/URL/number/email/terminal;
- surrounding guard: selection, caret giữa từ, snapshot lệch, emoji/NFD;
- action plan: `pass/preedit/commit/replace/fail_safe`;
- `delete_chars` cho Fcitx5/IBus và `delete_utf16` cho macOS/Android/Windows;
- timing `wait_ms` từ AppProfile.

Adapter chỉ cần:

1. lấy `before_display = vfk_display_text()`;
2. lấy surrounding đầy đủ + cursor/anchor UTF-16 (nếu có);
3. gọi `vfk_plan_action(...)` sau khi engine trả `VfkAction`;
4. thi hành `VfkPlan`.

```c
VfkPlan plan = {0};
if (!vfk_plan_action(session, &action, field_kind,
                     before_display, surrounding_text,
                     cursor_utf16, anchor_utf16,
                     /*allow_unverified_replace=*/false, &plan)) {
    /* fail-safe: reset engine, để phím gốc đi tiếp */
    return;
}
switch (plan.kind) {
case VFK_PLAN_PREEDIT:  set_preedit(plan.text, plan.cursor_utf16); break;
case VFK_PLAN_COMMIT:   commit(plan.text); break;
case VFK_PLAN_REPLACE:  delete_codepoints(plan.delete_chars);
                        commit(plan.text); break;
case VFK_PLAN_FAIL_SAFE: reset_engine(); break;
case VFK_PLAN_PASS:     break;
}
```

`plan.wait_ms` là gợi ý chờ trước commit. `plan.delete_chars` dùng cho API
codepoint (Fcitx5/IBus); `plan.delete_utf16` dùng cho API UTF-16
(macOS/Android/Windows).

Scenario dùng chung nằm ở [`adapter-conformance/`](../adapter-conformance/README.md).
Port không nên tự quyết định xoá bao nhiêu ký tự hoặc có được xoá không.

Fcitx5, macOS IMK, iOS, Android và Windows đã dùng planner này. IBus Python dùng
`python/vfk_adapter_core.py` — bản port cùng contract — và chạy chung
`adapter-conformance`; có thể chuyển sang C ABI khi đóng gói extension nhị phân.

Host composition / preedit lồng (Gboard, TSF/IMK, webtop → Linux) dùng một guard
riêng trong adapter-core; xem [`HOST_COMPOSITION.md`](HOST_COMPOSITION.md).

## 2. Chọn output mode

| Mode | Dùng khi | Cách áp |
|---|---|---|
| `PREEDIT` | TSF/IMK/Fcitx5/IBus, terminal, trình duyệt có composition | Đặt vùng preedit = `Action::Preedit.text`, con trỏ `cursor`; commit khi `Commit` |
| `REPLACE` | Hook bàn phím, app không có composition, macOS event tap | Xoá `delete_before` code unit (UTF-16 ở FFI) rồi chèn `insert`; nếu `pass_key` gửi tiếp phím gốc |
| `AUTO` (adapter) | Không chắc | Ưu tiên preedit; fallback surrounding/replace; thử lại preedit nếu app nuốt backspace |

`AppProfile.output_mode` chỉ là gợi ý; adapter quyết định cuối cùng theo app/field.

## 3. Vòng đời một từ

1. **Bắt đầu**: key đầu tiên sau separator. Nếu app hỗ trợ surrounding text, gọi
   `set_surrounding(before_cursor)` trước mỗi key (hoặc cache + cập nhật sau mỗi action).
2. **Đang gõ**: áp `Preedit`/`Replace`. Kiểm tra bất biến: chữ trên màn hình kết thúc bằng
   `session.display()`.
3. **Kết thúc**: separator/Enter/Tab/focus/cursor move. Áp `Commit`/`Replace(pass_key)`.
4. **Reset**: engine tự reset sau commit; adapter xoá preedit và cập nhật surrounding cache.

**Không** giữ preedit khi focus đổi, click chuột, chuyển tab, hoặc mở menu. Gọi
`cursor_moved()`/`focus_changed()` để engine chốt từ.

## 4. Surrounding text

- Chỉ sửa từ cũ khi `before_cursor` **kết thúc đúng** bằng text đã commit và đúng dạng
  NFC/NFD cấu hình. Nếu không, bỏ qua edit-previous-word (fail safe).
- Trước khi xoá, đọc lại surrounding; app có thể đã thay đổi (autocomplete, Firefox U+202F).
- Với selection: xoá selection trước, reset session, rồi xử lý key mới.
- Không dùng surrounding text trong password/secure input.

## 5. Preedit

- Preedit phải là **một vùng nguyên tử**; không gửi từng ký tự vào app.
- Khi `Action::Commit` có `pass_key=true`, commit rồi gửi phím gốc (Enter).
- Khi `Action::Preedit` có text rỗng, xoá vùng preedit nhưng vẫn giữ session.
- Khi engine trả `Replace` trong PREEDIT (sửa từ đã commit), adapter phải xoá `delete_before`
  UTF-16 trước con trỏ rồi chèn text; nếu app không hỗ trợ, bỏ qua surrounding text để engine
  không sinh nhánh này.
- Trước focus-out/reset/đổi mode, gọi `vfk_flush` để chốt composition; dùng
  `vfk_display_text` để kiểm tra text engine tin là đang hiển thị.
- `vfk_set_output_mode` đổi PREEDIT/REPLACE sau khi áp Action trả về; `vfk_apply_config_json`
  trả Action cần áp trước khi reload cấu hình. AppProfile có helper delay/ack/output mode.
- Terminal/Electron: ưu tiên preedit; nếu app không hỗ trợ, commit từng từ và dùng
  backspace có kiểm soát.
- Escape: engine trả `Commit(pass_key=false)` hoặc `Replace(pass_key=false)`; **không**
  chuyển tiếp Esc, tránh đóng dialog.

## 6. Replace / backspace

- `delete_before` của engine tính theo **code point** của chuỗi đầu ra (NFC/NFD).
  FFI C/WASM đổi sang UTF-16; nền tảng khác phải đổi theo đơn vị của mình.
- Dùng `vfk_plan_action`: plan trả `fail_safe` nếu snapshot lệch/selection/mid-word.
  Không tự gọi `deleteSurroundingText` khi chưa có plan `replace`.
- `VfkPlan.delete_chars` dùng cho Fcitx5/IBus; `delete_utf16` dùng cho macOS/
  Android/Windows. Không trộn hai đơn vị.
- Gửi backspace theo `AppProfile`: `backspace_delay_ms`, `delay_per_char_ms`, `wait_ack`
  (đã được planner tính vào `wait_ms`).
- Không gửi quá nhanh cho Electron/Firefox/Notion; chờ app xử lý (ACK) nếu có.
- Nếu plan `fail_safe`, reset engine và để phím gốc đi tiếp; không xoá mù.
- Không dùng `Delete` thay Backspace cho vùng trước con trỏ.

## 7. App/field context

Thứ tự ưu tiên đề xuất (từ cao xuống thấp):

1. Field policy: password/email/URL/number/OTP → tắt IME; terminal → preedit.
2. App profile do người dùng đặt (`app_profiles`).
3. Built-in preset (terminal, Chromium, Firefox, Electron, game...).
4. Global config.
5. Default.

- Chuẩn hoá app id: lowercase, bỏ đường dẫn, `.exe`, `.app`; so khớp không phân biệt hoa thường.
- Với trình duyệt, có thể dùng domain/thêm rule cho thanh địa chỉ; mặc định preedit cho
  address bar để tránh autocomplete nhân đôi.
- Game/Minecraft: `vietnamese=false` hoặc `excluded_apps`.
- Gọi `focus_changed` khi đổi foreground window; adapter phải tự chống focus bounce
  (dùng timer/debounce, không reset từ ngay lập tức).

## 8. Phím tắt và chế độ

- Toggle Việt/Anh: gọi `toggle_vietnamese()`; engine lưu theo app nếu adapter cung cấp app.
- Phím tắt mở menu mode (backtick, Ctrl+Space...) do adapter quản lý; không đưa vào engine.
- Macro/UniKey import: đọc file ở adapter, nạp vào `Config.macros`; không để engine đọc file.
- Chuyển bảng mã clipboard/file: dùng module charset/CLI, không đổi config giữa từ đang gõ.
- OSD/notification: hiển thị trạng thái Việt/Anh và lỗi; không chặn key path.

## 9. An toàn dữ liệu

- Không log nội dung từ/mật khẩu; chỉ log app id, mode, action kind.
- Không gửi nội dung ra mạng. Nếu thêm gợi ý/AI, phải chạy local hoặc opt-in rõ ràng.
- Luôn kiểm tra `Action::Replace.delete_before` không vượt quá vùng đã biết; nếu vượt,
  fallback `PassThrough`/preedit thay vì xoá mù.
- Bọc mọi xử lý engine trong try/catch (C ABI đã bắt panic).
- Khi engine trả `PassThrough`, không tự ý biến đổi text.

## 10. Checklist adapter tối thiểu

- [ ] Áp đúng 4 Action; `pass_key` đúng cho separator/Enter/Escape.
- [ ] Preedit commit khi focus/cursor/field đổi; không mất từ.
- [ ] Backspace xoá đúng 1 ký tự hiển thị, giữ dấu.
- [ ] Surrounding text kiểm tra trước khi sửa; fail safe nếu lệch.
- [ ] Password/email/URL/number tắt IME.
- [ ] App profile delay/ACK/preedit được dùng.
- [ ] Escape không bị forward.
- [ ] NFD/NFC và UTF-16 đúng.
- [ ] Không log nội dung nhạy cảm.
- [ ] Test trên terminal, Chrome address bar, Firefox, Electron, LibreOffice, game.

## 11. Ghi chú nền tảng

### macOS
- **Đã có adapter IMK**: [`../macos/README.md`](../macos/README.md). IMK là mặc định vì
  preedit native, không cần Accessibility; CGEventTap để sau như chế độ tương thích.
- Controller nhận `NSEvent`, gọi `VfkEngine` (C ABI), áp `Preedit/Commit/Pass`.
- `handleEvent` đã map Backspace/Enter/Tab/Esc/arrows; Escape trả raw và không forward.
- App id lấy từ `NSWorkspace.frontmostApplication`; `normalize_app_id` xử lý bundle id
  (`com.google.Chrome` → `chrome`) và alias (`vscode` → `code`).
- Config atomic + backup; menu đổi Telex/VNI, toggle, reload.
- Bản phát hành cần Developer ID + notarize + universal2; ad-hoc chỉ để test local.
- Nếu app không hỗ trợ IMK: fallback CGEventTap (chưa triển khai), cần Input Monitoring/
  Accessibility và có nguy cơ race backspace.
- Secure input: IMK thường tự pass; adapter cần kiểm tra `IsSecureEventInputEnabled` nếu
  thêm event tap; không log nội dung.

### Windows
- TSF cho app hiện đại (`windows/tsf/`); WH_KEYBOARD_LL + SendInput cho app không hỗ trợ TSF.
- Hai backend không xử lý cùng context: TSF ghi `Local\ViFastKeyTsfState`, hook pass-through
  theo foreground window/PID; `windows.json` chọn `auto`/`tsf`/`hook`.
- TSF lấy surrounding từ `ITfRange`, flush composition khi focus/caret, map `ITfInputScope`
  sang password/PIN/URL/email/number.
- Kiểm tra layout CJK/IME đang bật; đừng nuốt phím khi app đang composition.
- Console dùng `WriteConsoleInputW`; một số app cần `VK_BACK` thay vì unicode backspace.
- Game/anti-cheat: tôn trọng exclusion, không hook khi không cần.

### Linux
- Fcitx5/IBus preedit là mặc định; surrounding text nếu frontend hỗ trợ.
- Wayland: dùng text-input-v3/preedit; uinput/libei chỉ khi người dùng bật.
- Terminal: preedit; Chromium address bar: preedit hoặc select-then-replace.
- GNOME/KDE: xử lý focus/DBus cẩn thận; tránh reset từ khi popup mở.
