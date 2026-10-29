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
- Terminal/Electron: ưu tiên preedit; nếu app không hỗ trợ, commit từng từ và dùng
  backspace có kiểm soát.
- Escape: engine trả `Commit(pass_key=false)` hoặc `Replace(pass_key=false)`; **không**
  chuyển tiếp Esc, tránh đóng dialog.

## 6. Replace / backspace

- `delete_before` của engine tính theo **code point** của chuỗi đầu ra (NFC/NFD).
  FFI C/WASM đổi sang UTF-16; nền tảng khác phải đổi theo đơn vị của mình.
- Gửi backspace theo `AppProfile`: `backspace_delay_ms`, `delay_per_char_ms`, `wait_ack`.
- Không gửi quá nhanh cho Electron/Firefox/Notion; chờ app xử lý (ACK) nếu có.
- Nếu app nuốt backspace hoặc chèn lặp, dừng, đọc lại surrounding, rồi fallback preedit.
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
- IMK nếu muốn preedit chuẩn; CGEventTap nếu mô phỏng UniKey.
- Dùng marker cho synthetic event, ghim PID đích, serial queue cho backspace chậm.
- Watchdog event tap; secure input → tắt IME.
- Accessibility permission cần ký code ổn định.

### Windows
- TSF cho app hiện đại; WH_KEYBOARD_LL + SendInput cho app không hỗ trợ TSF.
- Kiểm tra layout CJK/IME đang bật; đừng nuốt phím khi app đang composition.
- Console dùng `WriteConsoleInputW`; một số app cần `VK_BACK` thay vì unicode backspace.
- Game/anti-cheat: tôn trọng exclusion, không hook khi không cần.

### Linux
- Fcitx5/IBus preedit là mặc định; surrounding text nếu frontend hỗ trợ.
- Wayland: dùng text-input-v3/preedit; uinput/libei chỉ khi người dùng bật.
- Terminal: preedit; Chromium address bar: preedit hoặc select-then-replace.
- GNOME/KDE: xử lý focus/DBus cẩn thận; tránh reset từ khi popup mở.
