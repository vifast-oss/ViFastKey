# Host composition & nested preedit

Trạng thái: **thiết kế, chưa triển khai đầy đủ**. Tài liệu này chốt hợp đồng để
các adapter có thể làm theo mà không copy logic của nhau.

Phạm vi: `vfk-adapter-core`, C ABI, WASM và adapter nền tảng. **Không** đưa logic
này vào `vfk-core`/`vfk-engine`; engine vẫn chỉ nhận `KeyEvent` và trả `Action`.

## 1. Vấn đề

Một text path có thể có nhiều tầng composition:

```text
bàn phím / OS IME / Gboard
        └─ host composition
              └─ app / remote desktop / webtop
                    └─ ViFastKey engine + preedit nội bộ
```

Nếu hai tầng cùng preedit, các lỗi thường gặp:

- nhân đôi ký tự hoặc nhân đôi dấu;
- mất ký tự khi một bên commit/backspace;
- preedit của tầng ngoài bị tầng trong xoá hoặc commit sai thời điểm;
- gõ tiếng Việt xong nhưng engine vẫn giữ raw cũ, phím sau bị ghép sai;
- remote desktop/webtop gửi text đã commit nhưng adapter vẫn cố biến đổi lại.

Web hiện tại đã xử lý một phần trường hợp Gboard/Chrome Android trong
`web/app.js`, nhưng đó là **prototype**, chưa dùng chung và chưa có conformance.

## 2. Thuật ngữ

| Thuật ngữ | Nghĩa |
|---|---|
| **Inner** | ViFastKey engine + preedit của adapter hiện tại |
| **Host** | IME/OS/app/remote client đang composition phía ngoài inner |
| **Host commit** | Text host đã chốt và đưa vào ô nhập; đây là text cuối, không phải raw key |
| **Nested preedit** | Host và inner cùng có preedit trên một text path |
| **Resync** | Bỏ trạng thái inner, nhận text/cursor thật từ host |
| **Ownership** | Tầng nào được phép biến đổi phím/text trên text path |

## 3. Nguyên tắc

1. **Một text path chỉ có một owner tại một thời điểm.** Không để hai IME cùng
   biến đổi một chuỗi phím.
2. **Host commit là text cuối.** Chữ có dấu đã commit (NFC/NFD) không được đưa
   lại qua `transform()` như chuỗi phím thô.
3. **Reset inner phải im lặng.** Không gọi `flush`/`commit` để xoá preedit nội bộ;
   dùng `reset`/cancel. Nếu không sẽ chèn thêm một lần.
4. **Resync từ text thật của ô nhập**, không chỉ `event.data`: host có thể commit
   một phần, thay selection, autocomplete hoặc đổi cả vùng text.
5. **Không chắc owner → pass/fail-safe**, không đoán rồi xoá.
6. **Ownership thắng character class.** Không chỉ dựa vào “có dấu hay không” để
   quyết định text là raw hay final.
7. **Host composition đang active → không phát inner preedit.**

## 4. State machine

```text
Idle
 ├─ inner_key ─────────────────► InnerOwned
 │                                └─ host_start → Suspend (clear preedit)
 ├─ host_start ────────────────► HostOwned
 │                                ├─ host_update → Pass
 │                                ├─ host_commit → Resync
 │                                └─ host_cancel → Idle
 └─ focus/field change ────────► FailSafe / Reset
```

### Quyết định

```rust
pub enum InputOwner {
    Inner,
    Host,
    Unknown,
}

pub enum GuardDecision {
    /// Engine xử lý bình thường.
    Own,
    /// Nhường host; xoá preedit nội bộ, không commit.
    Suspend,
    /// Forward phím/text, không biến đổi.
    Pass,
    /// Host vừa commit: nhận text/cursor thật, reset inner.
    Resync { text: String, cursor_utf16: Option<u32> },
    /// Không đủ tin cậy: reset và để phím đi tiếp.
    FailSafe,
}
```

### Bảng sự kiện

| Sự kiện | Quyết định | Ghi chú |
|---|---|---|
| `host_start` khi inner đang preedit | `Suspend` | Xoá preedit nội bộ, **không commit** |
| `host_update` | `Pass` | Không đụng engine |
| `host_commit(text)` | `Resync` | Nhận text/cursor thật từ app |
| `host_cancel` | `Idle`/`Pass` | Trả về trạng thái trước composition |
| focus/field đổi khi host active | `FailSafe`/`Reset` | Không giữ preedit |
| host commit đến muộn sau đổi field | `Ignore` | Dùng `epoch`/sequence |
| không quan sát được host | `Unknown` → `Pass` | App profile quyết định ownership |

### Resync

`Resync` là **adopt text của host**, không phải chạy engine:

```text
text, cursor = host_value, host_cursor
engine.reset()                 // im lặng
clear_inner_preedit()          // không commit
adopt_text(text, cursor)
update_surrounding_cache(text, cursor)
skip_adopt_once = true         // phím sau bắt đầu từ mới
```

Nếu host commit trùng `engine.display()`: chỉ reset, không chèn lại.
Nếu khác: nhận text host, reset, không cố sửa.
Nếu host commit rỗng/cancel: khôi phục trạng thái trước composition.

## 5. Phân loại host commit

Không phải mọi text host đưa vào đều giống nhau:

| Loại | Ví dụ | Xử lý |
|---|---|---|
| **HostCommittedText** | Gboard tiếng Việt commit `tiếng` | Giữ nguyên, reset, không transform |
| **RawKeyStream** | Keyboard layout/Telex gửi `t i e e n g s` | Có thể đưa qua engine nếu inner owns |
| **Mixed / NFD** | `tiếng` dạng tổ hợp, emoji, autocomplete | Resync; normalize về dạng nội bộ |
| **Stale commit** | Commit của composition cũ sau focus change | Ignore theo epoch |

Quy tắc thực tế:

- Nếu host owns composition: **mọi** host commit là literal (kể cả ASCII).
- Nếu inner owns và host chỉ là lớp chuyển phím: ASCII có thể là raw; khi đó
  adapter phải biết chắc, không đoán.
- Text có dấu/precomposed/emoji: luôn literal.
- NFD: normalize về NFC cho state nội bộ, nhưng giữ dạng hiển thị theo config.

## 6. API đề xuất

### Rust adapter-core

```rust
pub struct HostCompositionGuard {
    owner: InputOwner,
    epoch: u64,
    had_inner_preedit: bool,
    skip_adopt_once: bool,
}

pub enum HostEvent {
    Start,
    Update { text: String },
    Commit { text: String, cursor_utf16: Option<u32> },
    Cancel,
}

impl HostCompositionGuard {
    pub fn on_host_event(&mut self, event: HostEvent, inner_display: &str)
        -> GuardDecision;
    pub fn should_skip_adopt(&mut self) -> bool;
    pub fn invalidate(&mut self); // focus/field change, tăng epoch
}
```

Không đưa `GuardDecision` vào `PlanKind` hiện tại. `plan_action` vẫn thuần
Action + surrounding; adapter gọi guard trước/sau tùy sự kiện.

### AppProfile

Thêm một trường thay vì nhiều cờ rời:

```json
{
  "input_ownership": "auto",
  "host_composition": "observe"
}
```

- `input_ownership`: `auto | inner | host | disabled`
  - `auto`: quan sát host, nhường khi host active.
  - `inner`: ViFastKey chủ động xử lý; dùng khi biết chắc inner owns.
  - `host`: pass-through cho host/remote; dùng cho webtop/RDP/noVNC.
  - `disabled`: tắt hẳn IME trong app/field.
- `host_composition`: `observe | none`
  - `none` cho backend không có API composition (Windows hook, một số remote client).

### C ABI / WASM

Thêm hàm mỏng, không nhét state machine vào JS riêng:

```c
VfkGuardDecision vfk_guard_host_event(
    VfkSession *session,
    VfkHostEvent event,
    const char *text,
    int32_t cursor_utf16,
    const char *inner_display
);
```

WASM export tương đương để web/extension dùng chung contract.

## 7. Adapter nền tảng

| Nền tảng | Tín hiệu host | Ghi chú |
|---|---|---|
| Web/PWA | `compositionstart/update/end`, `beforeinput`, `isComposing`, `input` | Prototype hiện tại ở `web/app.js` |
| Android | `InputConnection.getComposingText()`, `setComposingText()` | Không rewrite textarea trong lúc host compose |
| iOS | `UITextInput.markedTextRange`, `setMarkedText` | Resync sau commit |
| macOS IMK | `NSTextInputClient.hasMarkedText`, marked range | Không commit preedit nội bộ khi host commit |
| Windows TSF | `ITfContext` + `ITfCompositionView` đang active | Hook backend không thấy composition |
| Linux Fcitx5/IBus | Preedit của inner; surrounding text | Remote/webtop dùng profile |
| Chrome extension | `beforeinput`/composition trong content script | Popup không có host composition |

### Windows webtop → Linux

Đây là **bài toán ownership**, không chỉ dedup:

- Outer Windows ViFastKey và inner Linux IME không chia sẻ composition state.
- Mặc định: **inner owns** (Linux nhận phím thô; outer pass-through).
- App profile cho `webtop`, `guacamole`, `novnc`, `rustdesk`, `parsec`, `rdp`,
  `vmware horizon` nên đặt `input_ownership: "host"`.
- Nếu outer owns thì phải tắt inner IME, nếu không sẽ double.
- Nếu cả hai đầu là ViFastKey và kiểm soát được cả hai, có thể thêm handshake
  (named pipe/DBus/shared marker) để báo inner active; generic webtop không có.
- Dedup sau commit chỉ là safety net:
  - host commit trùng inner display → reset;
  - khác → resync;
  - không transform text host.

## 8. Conformance

Thêm `adapter-conformance/host-composition/*.jsonl` với schema mở rộng:

```json
{
  "name": "host-commit-while-inner-preedit",
  "owner": "inner",
  "inner_display": "tiếng",
  "events": [
    { "kind": "host_start" },
    { "kind": "host_commit", "text": "tiếng", "cursor_utf16": 5 }
  ],
  "expected": {
    "kind": "resync",
    "text": "tiếng",
    "clear_preedit": true,
    "skip_adopt": true
  }
}
```

Scenario tối thiểu:

1. host start khi inner preedit → suspend, clear preedit, không commit.
2. host commit trùng display → resync, không duplicate.
3. host commit khác display → adopt host text, reset.
4. host commit rỗng/cancel → về trạng thái trước composition.
5. host update → pass, không inner preedit.
6. focus/field change khi host active → fail-safe/reset.
7. host commit NFD → normalize, cursor đúng.
8. remote `input_ownership=host` → outer pass-through.
9. stale host commit sau epoch → ignore.
10. selection/emoji/astral → cursor/UTF-16 đúng.

## 9. Thứ tự triển khai

1. Chốt spec + scenario conformance.
2. Tách guard dùng chung trong `vfk-adapter-core`, C ABI, Python port.
3. Web: tách `web/app.js` thành `web/host-composition.js`, dùng scenario chung.
4. Android/iOS (quan sát composition tốt).
5. macOS IMK / Windows TSF.
6. Fcitx5/IBus + app profile remote.
7. Dọn logic đặc thù khỏi adapter; chỉ giữ binding mỏng.

## 10. Việc chưa làm / rủi ro

- Chưa có guard dùng chung; web đang tự xử lý.
- Web chưa normalize text host commit; mới dựa vào `state.text = pad.value`.
- Web chưa có epoch cho commit đến muộn.
- Web chưa đưa logic `MAX_RAW`/literal recovery vào engine; đây là workaround
  riêng của web, cần chuyển thành test/engine rule nếu muốn parity.
- Remote desktop không chia sẻ composition; không hứa “khử 2 lần” tự động.
- Không dùng text diff đơn thuần để dedup; có thể mất phím.
- Không log nội dung text/preedit.

## 11. Liên quan

- `docs/ADAPTER_GUIDE.md` — hợp đồng adapter và preedit hiện tại.
- `adapter-conformance/README.md` — schema và cách chạy conformance.
- `web/app.js` — prototype host composition cho Gboard/Chrome Android.
- `docs/VIFASTTTS_LESSONS.md` — bài học timing/ACK/preedit từ ViFastTts.
