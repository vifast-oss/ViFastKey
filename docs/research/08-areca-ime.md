> **Báo cáo nghiên cứu nội bộ.** Tài liệu này chỉ ghi nhận hành vi, kiến trúc, UX và
> test fact của dự án được khảo sát. Không có mã nguồn, bảng dữ liệu, từ điển hay test
> corpus nào được sao chép vào ViFastKey. Areca phát hành MIT; báo cáo vẫn ưu tiên mô tả
> hành vi thay vì sao chép code để giữ kiến trúc ViFastKey độc lập. Xem
> `THIRD_PARTY_NOTICES.md` và `docs/IME_FEATURE_RESEARCH.md`.

# Areca IME: phân tích kiến trúc và bài học cho ViFastKey

**Snapshot phân tích**

| Mục | Giá trị |
|---|---|
| Repository | [`xhkzeroone/ArecaIME`](https://github.com/xhkzeroone/ArecaIME) |
| Bản phát hành | `v8.2.6` (2026-10-29) |
| Commit được đọc | `ac738bf2408d69fa1d143943b560ac10e6477639` |
| Giấy phép | MIT; `bamboo-core` là submodule MIT |
| Nền tảng | Linux, Fcitx5 addon, tối ưu Wayland |
| Ngôn ngữ | C++20, Go bridge, CMake; settings app SDL3 + Dear ImGui |
| Kích thước | ~18k dòng C++ trong `src/`, `tests/`, `settings/` (chưa kể ImGui) |
| Test | 22 CTest target, 23 file test |
| Đóng gói | DEB, RPM, Arch package, AUR, installer tự nhận distro |

---

## 0. TL;DR

- **Areca là tài liệu tham chiếu sát nhất cho adapter Fcitx5 của ViFastKey**, không phải
  tham chiếu cho engine tiếng Việt. Engine của ViFastKey đã mạnh hơn ở nhiều điểm;
  Areca dùng `bamboo-core` làm engine và dồn gần như toàn bộ giá trị kỹ thuật vào
  **lớp adapter production-grade**.
- Giá trị lớn nhất của Areca nằm ở 6 nhóm cơ chế:
  1. **FIFO + single-flight + transaction barrier** cho mọi thao tác sửa văn bản.
  2. **Backend abstraction** với fallback: surrounding text, forward Backspace,
     uinput Backspace, Shift+Left, native Libei/XTest.
  3. **Trust probe** cho surrounding text: coi snapshot của app là dữ liệu không đáng tin,
     xác minh bằng sự kiện `SurroundingTextUpdated`, fallback khi timeout.
  4. **Timing/ACK/AdaptiveWait**: delay theo frontend, chờ ACK, commit ở pha post của
     event loop, tự giãn khi event loop hoặc CPU bị lag.
  5. **App/field/context detection**: terminal, browser, address bar, password, app override,
     compatibility list cho IDE/Office/Electron.
  6. **Vòng đời preedit**: reset được bảo vệ, click chuột, finalized word, Backspace quay lại
     sửa từ vừa commit.
- **ViFastKey hiện tại**: engine/action contract và FFI khá đầy đủ, nhưng adapter Fcitx5
  (`linux/fcitx5/src/vifastkey.cpp`, ~300 dòng) vẫn là bản demo:
  - xử lý đồng bộ, không có queue/barrier;
  - luôn áp `Action::Replace` bằng `deleteSurroundingText()` mà không xác minh;
  - không gọi `vfk_set_field_kind()`, không gọi `vfk_cursor_moved()`;
  - không dùng `AppProfile.backspace_delay_ms`, `delay_per_char_ms`, `wait_ack`;
  - `reset()` gọi `vfk_reset()` rồi xóa preedit, có thể làm mất từ đang soạn;
  - không có fallback khi app không xử lý `deleteSurroundingText`.
- **5 việc nên làm trước** (chi tiết ở §4):
  1. Field kind + password/URL pass-through.
  2. Commit composition trước reset; bảo vệ reset trong transaction.
  3. Xác minh surrounding text trước `Replace`; fail-safe về preedit.
  4. Transaction barrier cho `Replace`/`Commit` bất đồng bộ.
  5. Test adapter bằng `InputContext` giả lập, không chỉ test engine.

- **Không nên bê nguyên**:
  - bảng compatibility hàng trăm tên app;
  - uinput mặc định;
  - nhận diện address bar bằng kích thước `cursorRect`;
  - adaptive state dùng chung toàn cục;
  - settings app SDL3/ImGui + 9 theme trước khi adapter ổn định.

---

## 1. Snapshot và kiến trúc

### 1.1 Thành phần

Areca là addon Fcitx5 viết bằng C++20. Nó không tự viết lại engine tiếng Việt mà gọi
`bamboo-core` qua một bridge Go (c-archive). Các phần chính:

| Thành phần | Trách nhiệm |
|---|---|
| `ArecaEngine` | Nhận event Fcitx5, dispatch theo mode, giữ cache verdict backend theo input context. |
| `RewriteModeHandler` | Chế độ sửa trực tiếp: nhận key, đẩy vào scheduler, áp kết quả rewrite. |
| `PreeditModeHandler` | Chế độ preedit: xử lý Bamboo đồng bộ, quản lý preedit và finalized word. |
| `RedirectModeHandler` | Chuyển phím nguyên bản (password/EN). |
| `KeyQueue` | FIFO giữ key gốc, codepoint, UTF-8, sequence và watch reference tới input context. |
| `InputScheduler` | Single-flight, chọn backend, transaction barrier, timer sau commit. |
| `BambooEngineAdapter` | Bridge C++/Go gọi `bamboo-core`, biến kết quả thành `BambooResult` (diff). |
| `ReliabilityChecker` / `SurroundingReliabilityState` | Probe surrounding text lần đầu, cache verdict, retry một lần ở từ kế tiếp. |
| `RewriteBackend` | Interface chung cho SurroundingText, ForwardBackspace, UinputBackspace, UinputShiftSelect, NativeXTest. |
| `AdaptiveWait` + `AdaptiveWaitMonitor` | Tự điều chỉnh delay theo timer drift, ACK, CPU pressure, load, nhiệt độ. |
| `InputTypeDetector`, `WindowFocusTracker` | Nhận diện terminal/browser/address bar; đọc AT-SPI khi `inputContext.program()` rỗng. |
| `MouseClickTracker` + `areca-mouse-monitor` | Process con libinput/udev phát hiện click để reset composition. |
| `areca-settings` | Settings app SDL3 + ImGui: bộ gõ, macro, ứng dụng, nâng cao, giao diện. |

### 1.2 Pipeline Rewrite

```text
Fcitx5 keyEvent
  │
  ├─ password? ──────────────────────────────► RedirectMode / pass-through
  ├─ Backspace do uinput replay? ────────────► forward, không đưa vào engine
  ├─ Shift+Left do uinput replay? ───────────► forward/filter theo ACK
  │
  ▼
mode handler (Rewrite / Preedit / Redirect)
  │
  ├─ Rewrite:
  │    key → filterAndAccept() → KeyQueue
  │         → InputScheduler (single-flight)
  │         → BambooEngineAdapter → BambooResult
  │         → deleteCount == 0 ? commitString
  │         → deleteCount > 0  ? chọn RewriteBackend
  │              → SurroundingText / ForwardBackspace / Uinput... 
  │              → commit sau settling delay
  │         → post-commit barrier (20 ms)
  │
  ├─ Preedit:
  │    key → Bamboo đồng bộ → preedit
  │         → boundary: commit + clear
  │         → Backspace: recovery / finalized word
  │
  └─ Redirect:
       key → forward nguyên bản
```

### 1.3 Ba mode

| Mode | Hành vi | Ghi chú |
|---|---|---|
| `Rewrite` | Gõ trực tiếp vào app; engine tính diff rồi xóa/chèn. | Mặc định trong code; cần backend và timing phức tạp. |
| `Preedit` | Hiển thị composition trong Fcitx5/app; commit khi hết từ. | An toàn hơn cho terminal/Electron; là mặc định của ViFastKey. |
| `Redirect` | Chuyển phím nguyên bản, không xử lý tiếng Việt. | Dùng cho password hoặc chế độ EN. |

Điểm quan trọng: **Areca chọn Rewrite làm mặc định**, nên nó buộc phải giải bài toán
"xóa chữ đã commit" rất nghiêm túc. ViFastKey chọn **Preedit làm mặc định**, nên không cần
bê toàn bộ cỗ máy rewrite ngay. Tuy nhiên, ViFastKey vẫn có nhánh `Action::Replace`
(khi sửa từ trước con trỏ, macro undo, hoặc app profile `replace`), và chính nhánh này
cần các bảo vệ tương tự Areca.

### 1.4 Engine contract và diff

`BambooEngineAdapter` giữ chuỗi `renderedText_` mà nó tin là đang hiển thị. Sau mỗi key:

```text
currentText  chuỗi trước key
newText      chuỗi mới sau khi Bamboo xử lý
deleteCount  số ký tự ở suffix cũ cần xóa
commitText   suffix mới cần chèn
```

Nó tính common prefix trên **codepoint UTF-8**, sau đó encode theo `OutputCharset` trước khi
tính diff. Vì vậy `deleteCount` và `commitText` luôn khớp chuỗi thực tế đã commit, kể cả
Unicode tổ hợp hoặc bảng mã legacy.

Bài học cho ViFastKey:

- Engine của ViFastKey trả `Action::Replace { delete_before, insert }` theo **code point**;
  FFI đổi sang UTF-16. Đây là contract tốt, nhưng adapter phải đảm bảo `delete_before`
  khớp đúng chuỗi đang hiển thị tại thời điểm xóa.
- Nếu sau này adapter commit bảng mã legacy, phải tính diff/độ dài trên chuỗi đã encode,
  không trộn code point Unicode với byte legacy. Xem `docs/COMPATIBILITY.md` và
  `docs/CONFIG.md`.

---

## 2. Những cơ chế đáng học

### 2.1 FIFO, single-flight và transaction barrier

**Vấn đề Areca giải quyết:** Fcitx5 gọi `keyEvent()` liên tục, trong khi
`deleteSurroundingText()` và `commitString()` là bất đồng bộ với app. Nếu xử lý key tiếp
theo khi backend chưa xóa/chèn xong, các lỗi điển hình là:

- mất ký tự do commit sau đè lên commit trước;
- nhân đôi từ do Backspace/commit chạy song song;
- xóa sai vị trí khi app chưa xử lý lệnh xóa trước;
- preedit và text app lệch trạng thái.

Areca áp bốn bất biến:

1. **FIFO**: key đi qua queue giữ đúng thứ tự.
2. **Single-flight**: chỉ một key được Bamboo xử lý tại một thời điểm.
3. **Transaction barrier**: khi backend đang xóa/chèn, key mới chỉ vào queue, không chen ngang.
4. **Không sleep trên main thread**: mọi delay dùng timer/post task của Fcitx event loop.

Chi tiết đáng chú ý:

- Key đầu tiên có thể được xử lý **inline** nếu scheduler đang rảnh; nếu không, nó vào queue.
- `filterAndAccept()` được gọi **trước** khi enqueue, để app không nhận lại key gốc.
- `KeyQueue` lưu `watch()` reference tới `InputContext`, tránh dùng con trỏ đã chết.
- `shouldRejectReset()` chặn reset khi đang xử lý hoặc trong **20 ms** sau commit gần nhất.
- Commit được đưa sang **post phase** của event loop, không commit trực tiếp trong timer
  callback, để các công việc đang nghẽn được xử lý trước.
- Với editor web cần nhận key đầu để mở chế độ soạn thảo (Google Sheets), Areca có nhánh
  `handleIdleKey()`: nếu Bamboo không đổi output, event **không** bị filter/accept để
  frontend nhận key; nếu output đổi, event bị chặn và commit kết quả. Trạng thái Bamboo
  vẫn được giữ để `a` + `s` vẫn thành `á`.

**Đối chiếu ViFastKey:**

Adapter hiện tại xử lý tuần tự trong `VifastkeyState::keyEvent()`:

```cpp
refreshSurrounding();
VfkAction action = vfk_process_key(session_, key.c_str(), ctrl, alt, meta);
apply(action, &event);
```

`apply()` gọi `deleteSurroundingText()` rồi `commitString()` ngay. Không có barrier, không
có queue. Với `PREEDIT`, phần lớn key chỉ cập nhật preedit nên ít rủi ro; nhưng khi engine
trả `VFK_REPLACE` (sửa từ trước con trỏ hoặc macro undo), thao tác xóa/chèn là bất đồng bộ
và key tiếp theo có thể đến trước khi app xử lý xong.

**Khuyến nghị cho ViFastKey:**

- Không cần dựng full FIFO ngay nếu vẫn giữ PREEDIT mặc định.
- Nhưng phải có **transaction lock** cho `VFK_REPLACE`: đánh dấu `busy`, giữ các key đến
  trong lúc chờ commit/xóa xong, rồi mới xử lý tiếp.
- Nếu sau này thêm chế độ direct-rewrite, nâng transaction lock thành FIFO + post-commit
  barrier giống Areca.
- Test bắt buộc: gõ nhanh 10–20 ký tự trong khi backend xóa/chèn; không mất, không lặp,
  không lệch dấu.

### 2.2 Backend abstraction và fallback chain

Areca định nghĩa interface:

```cpp
class RewriteBackend {
  virtual ApplyStatus apply(InputContext &, const RewritePlan &,
                            RewriteDone onDone) = 0;
};
```

Các backend:

| Backend | Cách sửa | Khi dùng |
|---|---|---|
| `SurroundingTextBackend` | `deleteSurroundingText()` + `commitString()` | App hỗ trợ surrounding text và đáng tin. |
| `SurroundingTextV2Backend` | Xóa từng ký tự, có delay | Editor không ổn định với xóa gộp (Firefox/Web editor). |
| `ForwardBackspaceBackend` | `forwardKey(Backspace)` × N + commit | Frontend nhận được forward key; app không hỗ trợ surrounding. |
| `UinputBackspaceBackend` | Gửi `KEY_BACKSPACE` × N qua `/dev/uinput` | App/frontend nuốt forward Backspace; cần quyền uinput. |
| `UinputShiftSelectBackend` | Shift+Left × N+1 rồi commit | Browser không hỗ trợ surrounding, cần thay selection. |
| `NativeXTestBackend` | Libei (Wayland) hoặc XTest (X11) | uinput không khả dụng hoặc người dùng chọn native. |

Thứ tự chọn backend (rút gọn):

```text
terminal / plasmashell / VS Code embedded terminal
    → ForwardBackspace hoặc hardware Backspace
selection đang mở hoặc browser autocomplete
    → Backspace (+1) để xóa an toàn
URL / address bar
    → Shift+Select nếu có, ngược lại Backspace
app override (Surrounding / ShiftSelect / Backspace / Native / Forward)
    → backend tương ứng
surrounding đáng tin
    → SurroundingText (hoặc V2)
browser + fallback bật
    → Shift+Select
còn lại
    → ForwardBackspace
```

Điểm tinh tế:

- `canForwardBackspaces()` kiểm tra frontend/capability:
  - Wayland, XIM, IBus: thường cho phép;
  - DBus: cần `KeyEventOrderFix` hoặc `SurroundingText`;
  - Fcitx4: cần `FormattedPreedit` hoặc `SurroundingText`;
  - SDL app: không cho phép, phải dùng hardware path.
- `UinputBackspaceBackend` phải **forward mọi event Backspace quay lại Fcitx**, kể cả event
  đến muộn; bộ đếm ACK chỉ là mốc tương đối, không dùng để lọc event.
- Timer phát phím và timer commit **độc lập**; commit không được chạy trước khi phát đủ N
  phím, dù ACK đến sớm.
- Nếu ACK không đủ trong timeout, backend vẫn chuyển sang settling wait rồi commit, không
  treo.
- `UinputShiftSelectBackend` gửi Shift down, Left × N+1, forward N Left đầu, filter Left
  xác nhận cuối, Shift up, chờ settling, rồi commit từng ký tự. Nếu có selection của người
  dùng, phải hủy Shift+Select và fallback Backspace +1.

**Đối chiếu ViFastKey:**

Adapter hiện tại chỉ có một đường: `deleteSurroundingText()` + `commitString()`. Không có
fallback. Nếu app khai báo `SurroundingText` nhưng không xử lý đúng, chữ sẽ bị xóa sai hoặc
mất. Engine đã cẩn thận khi **adopt** từ trước con trỏ (`adopt_previous_word` kiểm tra
`before_cursor` khớp output), nhưng adapter vẫn có thể xóa nhầm nếu app thay đổi text giữa
lúc đọc snapshot và lúc xóa.

**Khuyến nghị cho ViFastKey:**

- Trước mắt chỉ cần 2 backend:
  1. `SurroundingReplaceBackend`: xóa + commit, chỉ dùng khi đã xác minh.
  2. `PreeditFallbackBackend`: không xóa; đưa text vào preedit hoặc commit an toàn.
- Chỉ thêm `ForwardBackspaceBackend` khi có app thực tế không nhận surrounding nhưng vẫn
  nhận `forwardKey`.
- `uinput`/Libei/XTest để sau, sau khi có transaction barrier và test app thật. Đừng bật
  uinput mặc định: cần quyền `/dev/uinput`, có thể tạo duplicate event, và phức tạp ACK.
- Nếu app không hỗ trợ surrounding, **ưu tiên preedit** thay vì mạo hiểm xóa mù.

### 2.3 Trust probe cho surrounding text

Đây là cơ chế đáng học nhất của Areca.

`SurroundingReliabilityState`:

- `known`: đã đánh giá app chưa.
- `reliable`: snapshot có đáng tin không.
- `forceForwardBackspace`: app thuộc nhóm compatibility.
- `probePending`: đang chờ xác nhận.
- `probeAttempts`: số lần probe.
- `retryOnNextWord`: chỉ retry một lần ở từ kế tiếp.
- Timeout: **150 ms** cho lần đầu, **80 ms** cho retry.

Luồng probe:

```text
1. App khai báo CapabilityFlag::SurroundingText và snapshot valid.
2. Text ngay trước cursor phải kết thúc bằng currentText của engine.
3. Backend gửi deleteSurroundingText(-N, N).
4. Chờ event InputContextSurroundingTextUpdated.
5. Hàm surroundingProbeMatches() kiểm tra ngữ nghĩa:
   - cursor == anchor (không có selection);
   - text trước cursor khớp expected text sau khi xóa;
   - text cũ không còn;
   - không rơi vào trạng thái trung gian cursor di chuyển trước rồi mới xóa.
6. Khớp → đánh dấu reliable, hủy deadline, commit.
7. Timeout / không khớp → đánh dấu unreliable, fallback backend.
```

Điểm mạnh: Areca không tin `CapabilityFlag` và không tin snapshot ngay sau khi app báo.
Nó **chờ bằng chứng** rằng lệnh xóa đã thực sự có hiệu lực.

Ngoài ra còn có các guard cho browser:

- `isSelectionImmediatelyAfterText()`: selection phải bắt đầu ngay sau toàn bộ text engine
  đang hiển thị; nếu selection cắt vào giữa từ thì snapshot stale, không cộng thêm Backspace.
- `isBrowserAutocomplete()`: selection phải kéo từ sau composition tới hết dòng, không đi
  qua newline. Nếu đúng, cộng thêm **1 Backspace** để xóa phần autocomplete trước khi commit.
- `isCollapsedFirefoxAutocomplete()`: xử lý trường hợp Firefox thu gọn selection và suffix
  giống URL.
- Nếu `cursor != anchor` (đang có selection), Areca **không dùng Shift+Select**, mà fallback
  Backspace +1 để tránh phá selection của người dùng.

**Đối chiếu ViFastKey:**

- `refreshSurrounding()` chỉ lấy `before_cursor`, không lấy anchor/selection.
- Engine `adopt_previous_word()` kiểm tra `before_cursor` khớp output, nhưng adapter không
  kiểm tra lại sau khi engine trả `Replace`.
- Không có probe, không có timeout, không có fallback.

**Khuyến nghị cho ViFastKey:**

1. Trước mỗi `VFK_REPLACE` có `delete_utf16 > 0`:
   - đọc lại surrounding;
   - yêu cầu `cursor == anchor` (không có selection) — nếu có selection, fallback preedit;
   - yêu cầu text trước cursor kết thúc bằng chuỗi engine tin là đang hiển thị;
   - nếu không khớp, **không xóa**; chuyển sang preedit/commit an toàn.
2. Cần API để lấy chuỗi engine tin là đang hiển thị. C ABI hiện chưa có
   `vfk_current_text()`/`vfk_display_text()`; có thể:
   - thêm API này; hoặc
   - adapter tự mirror text đã commit/preedit.
3. Nếu vẫn muốn dùng `deleteSurroundingText()` cho app “có vẻ hỗ trợ”, thêm probe:
   - lần đầu xóa, chờ `InputContextSurroundingTextUpdated` tối đa ~80–150 ms;
   - nếu không có event khớp, đánh dấu app không đáng tin và từ sau dùng preedit.
4. Không probe trong password/URL/number/email.

### 2.4 Timing, ACK và AdaptiveWait

Areca cấu hình delay theo **frontend** và **loại thao tác**, không dùng một con số chung:

| Nhóm | Mặc định tiêu biểu |
|---|---|
| Backspace giữa các phím | 1 ms |
| Chờ sau Backspace cuối (Wayland) | 3 ms |
| Chờ sau Backspace cuối (DBus/IBus/XIM/Fcitx4) | 10 ms |
| Chờ sau xóa surrounding | 3 ms |
| Chờ sau Shift+Left (Wayland) | 5 ms |
| Chờ sau Shift+Left (frontend khác) | 10 ms |
| Post-commit barrier | 20 ms |
| uinput Backspace ACK timeout | 150 ms |
| Timer accuracy | 1 µs khi `PreciseTiming=True` |

`AdaptiveWait`:

- `extraWaitMs` bắt đầu 0, tăng theo bước **10 ms**, tối đa **50 ms**.
- Mỗi tín hiệu lag tăng ít nhất 10 ms; giảm 10 ms sau **5 transaction ổn định**.
- Tín hiệu:
  - timer drift ≥ 5 ms;
  - ACK round-trip ≥ 4 ms;
  - ACK timeout → đề nghị 30 ms;
  - CPU PSI, load average, nhiệt độ (80/85°C).
- Trước commit, backend đọc lại mức adaptive; nếu tăng trong lúc chờ, hẹn thêm rồi mới commit.
- `event loop post` được dùng để tránh commit trong timer callback khi hàng đợi hệ thống còn
  việc.

**Đối chiếu ViFastKey:**

- `AppProfile` đã có `backspace_delay_ms`, `delay_per_char_ms`, `wait_ack` — đúng hướng.
- Nhưng adapter Fcitx5 **không đọc** các field này; `vfk_app_profile_json()` không được dùng.
- Không có post-commit barrier; `commitString()` chạy ngay trong callback.

**Khuyến nghị cho ViFastKey:**

- Dùng `vfk_app_profile_json()` sau `vfk_focus_changed()` để lấy timing/ACK; parse JSON
  (hoặc thêm API typed) và áp cho nhánh Replace.
- Bắt đầu với:
  - delay giữa các Backspace: 2–5 ms;
  - chờ sau xóa: 5–10 ms;
  - post-commit: 10–20 ms;
  - `wait_ack` chỉ bật cho app thực sự cần.
- Không cần port PSI/nhiệt độ ngay. Chỉ cần timer drift/ACK nếu sau này dùng uinput.
- Đừng để delay làm người dùng cảm nhận rõ; 50 ms là ngưỡng khá lớn. Đo trước, tối ưu sau.

### 2.5 Preedit, finalized word và protected reset

Areca có `PreeditModeHandler` riêng, không dùng scheduler/backend rewrite. Các điểm đáng học:

- Khi composition còn, Backspace gọi `processBackspace()` nếu `BackspaceRecovery=True`,
  cho phép Bamboo dựng lại chuỗi spell-check (`nhanhsh → nhánh`).
- Sau khi commit một từ + boundary (space/dấu câu), adapter giữ **finalized snapshot** và
  đếm số boundary đã commit.
- Backspace đi ngược qua boundary; khi chạm từ, handler xóa `từ + boundary cuối` bằng
  `deleteSurroundingText()` rồi đưa từ trở lại preedit — nhưng chỉ khi app hỗ trợ
  SurroundingText và snapshot khớp chính xác. Nếu không, reset finalized state và forward
  Backspace gốc.
- `requestProtectedReset()` trì hoãn reset nếu đang có transaction hoặc trong cửa sổ bảo vệ;
  reset đến từ frontend ngay sau commit không được xóa snapshot cần cho Backspace.
- Click chuột: `prepareSurroundingRestoreAfterClick()` chỉ chuẩn bị đọc lại từ quanh caret,
  không xóa preedit đang hiển thị nếu frontend chưa kết thúc composition.
- `resetProtectedUntilUsec` bảo vệ state trong một khoảng ngắn sau commit.

**Đối chiếu ViFastKey:**

- Adapter `reset()`:

```cpp
void VifastkeyState::reset() {
    if (session_) vfk_reset(session_);
    clearPreedit();
}
```

  Nếu đang có composition, `vfk_reset()` xóa state và `clearPreedit()` xóa UI, **không commit**
  từ đang gõ. Đây là rủi ro mất chữ khi Fcitx5 gọi reset do đổi focus/app/IME.
- `VifastkeyState::focus()` gọi `focus_changed()` (engine flush → Commit) nên focus thường
  an toàn; nhưng `reset()` thì không.
- Không gọi `vfk_cursor_moved()` khi click/caret đổi; engine có `adopt_allowed=false` sau
  `cursor_moved()` nhưng adapter không dùng.
- Không có finalized-word recovery; engine có `restore_after_space` nhưng adapter chỉ xử lý
  `Action` đơn lẻ.

**Khuyến nghị cho ViFastKey:**

- `reset()` phải:
  1. lấy `Action` từ một API flush/commit composition (hoặc gọi `vfk_focus_changed` với
     app hiện tại);
  2. áp action trước khi `vfk_reset()`;
  3. chỉ clear preedit sau khi commit.
- Thêm protected reset: nếu đang `busy` hoặc trong 20 ms sau commit, hoãn reset.
- Gọi `vfk_cursor_moved()` khi phát hiện caret/click đổi (nếu Fcitx5 cung cấp event), và
  trước khi bắt đầu composition mới.
- Học behavior finalized word của Areca nếu muốn “Backspace quay lại sửa từ vừa commit”;
  nhưng đây là P1, không phải P0.

### 2.6 App, field và context detection

Areca phân biệt:

- **Program**: `inputContext.program()`, fallback `WindowFocusTracker` đọc AT-SPI.
- **Frontend**: `wayland`, `xim`, `ibus`, `dbus`, `fcitx4`.
- **Capabilities**: `SurroundingText`, `Password`, `Url`, `KeyEventOrderFix`,
  `FormattedPreedit`.
- **Field type**: terminal, browser, address bar, editor/IDE, office.
- **Selection/anchor**: phát hiện selection đang mở.
- **Browser autocomplete**: selection kéo tới cuối dòng.
- **Address bar**: `CapabilityFlag::Url`; nếu không, browser identity + AT-SPI browser UI;
  cuối cùng là heuristic `cursorRect` (rất dễ vỡ).

Compatibility list của Areca rất lớn: hàng trăm tên IDE, editor, terminal, browser, office,
chat. Nó dùng `normalizedProgramName()` (bỏ path, lowercase, bỏ `.desktop`) và
`matchesNameOrVariant()` (prefix + dấu `.`/`-`) để tránh match nhầm.

**Đối chiếu ViFastKey:**

- Engine có `AppProfile`, `normalize_app_id`, `app_aliases`, preset terminal/Chromium/
  Firefox/Electron/game.
- Adapter chỉ gọi `vfk_focus_changed()` trong `activate()` với `inputContext->program()`;
  không cập nhật khi focus đổi bên trong cùng input context.
- Không gọi `vfk_set_field_kind()`; password/number/email/URL không được tắt ở adapter.
- Không nhận diện browser/address bar/terminal; không có app override UI.

**Khuyến nghị cho ViFastKey:**

- **Capability-first, table sau**:
  - `CapabilityFlag::Password` → `VFK_FIELD_PASSWORD` + pass-through.
  - `CapabilityFlag::Url` → `VFK_FIELD_URL` + pass-through (hoặc preedit).
  - Terminal: `program` + frontend + AT-SPI role nếu cần.
  - Browser: chỉ cần khi thêm address bar/autocomplete; đừng đoán bằng cursorRect.
- Giữ danh sách preset nhỏ, dễ bảo trì; cho người dùng override theo app.
- Không copy bảng hàng trăm app của Areca. Nó là bằng chứng Linux phân mảnh, không phải
  thiết kế lý tưởng.

### 2.7 Macro, auto-cap và restore

Areca:

- Macro được expand tại word boundary, trước spell-check.
- `CapitalizeMacro`:
  - key lowercase → replacement lowercase;
  - key ALL CAPS → replacement ALL CAPS;
  - key Firstupper → Title Case từng từ;
  - mixed-case khác → giữ value cấu hình.
- `MacroBypassModifier`: giữ Shift/Ctrl/Alt/Super rồi nhấn Space để bỏ qua macro cho **đúng
  boundary đó**. Cờ `suppressMacro` nằm trên từng `QueuedKey`, không phải state one-shot,
  nên gõ nhanh không bị áp nhầm sang phím sau.
- `RevertMacroOnBackspace`: Backspace ngay sau macro khôi phục trigger + boundary.
- `AutoCapitalizeAfterPunctuation`: state machine theo input context cho `.`, `!`, `?` rồi
  space; chỉ đổi ASCII `a-z`; reset khi Enter/Backspace/nav/shortcut.
- Sau khi finalize, giữ composition và đếm boundary; Backspace quay lại sửa từ.

**Đối chiếu ViFastKey:**

- Đã có `macro_undo`, `macro_in_off_mode`, `auto_capitalize`, `smart_caps`,
  `restore_after_space`, macro nhiều từ.
- Chưa có macro bypass modifier; chưa có Title Case cho macro nhiều từ (chỉ viết hoa chữ
  đầu); macro undo chỉ chạy trong `OutputMode::Replace`.

**Khuyến nghị:**

- P2: thêm `macro_bypass_modifier` (Shift/Ctrl/Alt/Super) và test với gõ nhanh.
- P2: Title Case cho macro nhiều từ nếu người dùng cần (`Hcm → Hồ Chí Minh`).
- P1: đảm bảo macro undo hoạt động cả khi adapter đang ở PREEDIT; nếu engine chỉ hỗ trợ
  Replace, ghi rõ trong docs hoặc mở rộng.

### 2.8 Config, UI, packaging, debug và test

**Config:**

- Areca tách:
  - `areca.conf` — cấu hình chính;
  - `areca-advanced.conf` — timing/fallback;
  - `areca-macro-table.conf` — macro;
  - `areca-apps.conf` — app override.
- Field timing cũ trong `areca.conf` được giữ hidden để migrate; file advanced load sau và
  ưu tiên.
- Settings app đọc/ghi qua `fcitx::readAsIni`/`safeSaveAsIni`.

**UI:**

- `areca-settings` dùng SDL3 + Dear ImGui; 5 tab: Bộ gõ, Macro, Ứng dụng, Nâng cao, Giao diện.
- 9 theme; font size; shortcut capture; app override; “Best performance” preset;
  save/apply/cancel/restore defaults.
- Có action menu trong Fcitx5 status area: input method, charset, spellcheck, modern style,
  macro, auto-cap, settings.

**Packaging:**

- `install.sh` tự nhận distro/version, tải asset từ GitHub release, cài bằng apt/pacman/dnf/
  zypper, cấu hình uinput, restart Fcitx5.
- GitHub Actions build matrix: Ubuntu 24.04/25.10/26.04, Mint 22, Pop!_OS 24.04, Zorin 17,
  Fedora 42–44, Nobara, Bazzite, openSUSE Tumbleweed, Arch/CachyOS/Manjaro/EndeavourOS/Garuda.
- AUR `fcitx5-areca-bin` / `fcitx5-areca-git`.
- udev rules cho `/dev/uinput` và pointer; bundle SDL3 + `$ORIGIN` rpath cho Ubuntu 24.04.
- Docs debug: kiểm tra `/proc/<pid>/maps` xem addon cũ còn `(deleted)` không; phân biệt lỗi
  backend; log `journalctl --user`.

**Test:**

- 22 CTest target, dùng `assert` + `-UNDEBUG`, không framework.
- Có `TestInputContext` giả lập `commitStringImpl`, `deleteSurroundingTextImpl`,
  `forwardKeyImpl`, `surroundingText().setText()`, `setCapabilityFlags()`.
- Test các hành vi: adaptive wait, event-loop post, Bamboo adapter, word segment, browser
  autocomplete, sentence capitalizer, preedit logic, program compatibility, app mode routing,
  forward backspace probe, uinput backspace/shift-select, XTest, surrounding probe/restore,
  idle key forward, preedit surrounding restore, rewrite feature toggle, input type detector,
  mouse click tracker.
- Điểm mạnh: test **hành vi adapter**, không chỉ engine.

**Đối chiếu ViFastKey:**

- Config JSON tốt, có schema_version, app profile, macro, charset; nhưng adapter chỉ đọc
  config một lần khi tạo state, không reload khi `setConfig`/`reloadConfig`.
- Không có GUI; `linux/fcitx5/README.md` ghi rõ “chưa có preferences UI”.
- Chưa có packaging Linux; CI mới build addon.
- Test engine/conformance mạnh, nhưng không có test adapter Fcitx5.

**Khuyến nghị:**

- P0/P1: test adapter bằng `InputContext` giả lập trước khi làm GUI.
- P1: thêm `vfk-doctor`/script kiểm tra addon đã nạp, config, surrounding, capability,
  `fcitx5-diagnose`.
- P2: packaging `.deb`/`.rpm`/AUR; GUI settings sau.
- Ghi docs debug giống Areca: log theo backend, cách xác nhận addon mới đã nạp, cách đọc
  surrounding snapshot.

---

## 3. Đối chiếu nhanh với ViFastKey

| Năng lực | Areca | ViFastKey hiện tại | Khoảng trống / ưu tiên |
|---|---|---|---|
| Engine tiếng Việt | Bamboo core (MIT) | Engine riêng, conformance mạnh | Không cần thay |
| Preedit | Có, mode riêng | Mặc định, engine hỗ trợ | Nền tốt |
| Rewrite trực tiếp | Mặc định, full backend | Chỉ `Replace` cho sửa từ trước/macro | Không cần full nếu giữ preedit |
| FIFO/single-flight | Có | Không | **P0** cho `Replace` |
| Transaction barrier | Có | Không | **P0** |
| Post-commit delay | 20 ms | Không | **P0/P1** |
| Surrounding trust probe | Có | Không | **P0** |
| Fallback khi xóa lỗi | Có | Không | **P0** |
| Field kind/password | Có | Engine có, adapter không gọi | **P0** |
| Reset protection | Có | Không | **P0** |
| Cursor/click lifecycle | Có | Không | **P0/P1** |
| AppProfile timing/ACK | Có | Schema có, adapter bỏ qua | **P1** |
| App override | Có UI/config | Engine có, adapter chưa dùng đầy đủ | **P1** |
| Browser/address bar | Có | Không | **P1** |
| Preedit finalized word | Có | Engine có `restore_after_space`, adapter chưa | **P1** |
| Macro bypass | Có | Không | **P2** |
| Adaptive wait | Có | Không | **P2** |
| uinput/Libei/XTest | Có | Không | **P2** |
| Settings GUI | Có | Không | **P2** |
| Packaging Linux | Có | Chưa | **P1/P2** |
| Adapter tests | Có | Chưa | **P1** |

---

## 4. Ưu tiên áp dụng cho ViFastKey

### P0 — nên làm ngay

| # | Việc | Cụ thể | Tiêu chí hoàn thành |
|---|---|---|---|
| 1 | Field kind + password | Trong `keyEvent`, kiểm tra `CapabilityFlag::Password`, `Url`, input purpose; gọi `vfk_set_field_kind()` trước khi xử lý. Password/URL/number/email → pass-through. | Gõ trong ô password không tạo preedit, không commit, không lộ nội dung; test với `TestInputContext`. |
| 2 | Commit trước reset | `reset()` phải flush composition (commit action) trước `vfk_reset()`; không gọi `clearPreedit()` khi còn text. | Đổi focus/app khi đang gõ không mất từ; test reset giữa composition. |
| 3 | Xác minh surrounding | Trước `VFK_REPLACE` có `delete_utf16 > 0`, kiểm tra `cursor == anchor` và `before_cursor` kết thúc bằng text engine tin là đang hiển thị. Không khớp → fallback preedit, không xóa. | App trả snapshot cũ/selection không bị xóa sai; test với snapshot lệch. |
| 4 | Transaction barrier | Đánh dấu `busy` khi `deleteSurroundingText`/`commitString` của Replace đang chạy; key mới chờ; post-commit delay trước khi xử lý tiếp. | Gõ nhanh trong lúc Replace không mất/lặp ký tự; test 20 key liên tiếp. |
| 5 | Cursor/focus lifecycle | Gọi `vfk_cursor_moved()` khi caret/click đổi; gọi `vfk_focus_changed()` khi focus đổi; không adopt từ cũ sau click. | Click giữa từ rồi gõ tiếp không sửa nhầm từ cũ; test preedit + click. |

### P1 — nên làm sau khi P0 ổn định

| # | Việc | Cụ thể | Tiêu chí |
|---|---|---|---|
| 6 | AppProfile timing/ACK | Parse `vfk_app_profile_json()`, áp `backspace_delay_ms`, `delay_per_char_ms`, `wait_ack` cho Replace/backspace. | Chrome/Firefox/Electron không mất chữ; timing đo được. |
| 7 | Fallback backend | Nếu app không xử lý surrounding (probe timeout), chuyển preedit hoặc forward Backspace. | App “nói” có surrounding nhưng không xóa vẫn gõ được. |
| 8 | App override | Dùng `app_profiles`/`app_modes` để chọn preedit/replace, tắt tiếng Việt, kiểu gõ, tone style. | `minecraft` tắt, `code` VNI, `chrome` preedit… hoạt động. |
| 9 | Browser/URL | `CapabilityFlag::Url` + browser identity; xử lý address bar bằng preedit hoặc select-then-replace; không dùng cursorRect nếu tránh được. | Chrome address bar không nhân đôi autocomplete. |
| 10 | Preedit finalized word | Học Areca: Backspace sau space quay lại từ vừa commit nếu snapshot khớp; nếu không, forward Backspace. | `Chào ` + BS + `f → Chaof` (hoặc behavior đã chọn) ổn định. |
| 11 | Adapter tests | Dựng `TestInputContext`/fake backend; test queue, reset, surrounding mismatch, password, rapid typing. | CI chạy được test adapter không cần GUI. |
| 12 | Doctor/package | Script kiểm tra addon, config, capability; build `.deb`/`.rpm`/AUR. | Người dùng cài và chẩn đoán được. |

### P2 — để sau, khi có nhu cầu thực tế

| # | Việc | Lý do |
|---|---|---|
| 13 | Adaptive wait | Chỉ cần khi dùng uinput/ACK hoặc gặp lag thật; đừng tăng delay mặc định. |
| 14 | uinput/Libei/XTest | Cần quyền, ACK, replay filtering; chỉ bật opt-in. |
| 15 | Mouse click tracking | Hữu ích nhưng phức tạp (process con/libinput); có thể dùng focus/cursor event trước. |
| 16 | Settings GUI | Đắt; ưu tiên config JSON + CLI/doctor trước. |
| 17 | Macro bypass/title case | Tính năng tiện dụng, không chặn adoption. |

---

## 5. Thiết kế adapter Fcitx5 đề xuất

### 5.1 Module layout

Giữ `vifastkey.cpp` mỏng, tách phần adapter:

```text
linux/fcitx5/
├── src/
│   ├── vifastkey.cpp          # addon entry, map KeyEvent, lifecycle
│   ├── engine_bridge.{h,cpp}  # wrap C ABI, copy Action, config
│   ├── input_state.{h,cpp}    # session + mirror + busy/transaction
│   ├── input_scheduler.{h,cpp}# FIFO/transaction barrier (P0 cho Replace)
│   ├── rewrite_backend.{h,cpp}
│   ├── surrounding_backend.{h,cpp}
│   ├── preedit_fallback.{h,cpp}
│   ├── app_context.{h,cpp}    # program/field/capability
│   └── timing.{h,cpp}
├── CMakeLists.txt
└── README.md
```

Không cần tách hết ngay; nhưng `input_state` + `scheduler` + `backend` nên có ranh giới rõ để
test.

### 5.2 State machine

```text
Idle
  │ key event
  ▼
Processing ── Action::PassThrough ───────────────► Idle
  │
  ├─ Action::Preedit ────────────────────────────► Idle (preedit visible)
  ├─ Action::Commit ── commitString ─────────────► PostCommit ──► Idle
  └─ Action::Replace
        │
        ├─ delete_before == 0 ── commit ─────────► PostCommit ──► Idle
        └─ delete_before > 0
              │ verify surrounding
              ├─ OK ── deleteSurroundingText ── wait ── commit ──► PostCommit ──► Idle
              └─ FAIL ── fallback preedit/commit ──────────────► PostCommit ──► Idle

Reset khi Processing/PostCommit → DeferredReset → chạy sau khi an toàn
```

### 5.3 Pseudo-code key event

```cpp
void VifastkeyState::keyEvent(KeyEvent &event) {
    if (!session_ || event.isRelease()) return;

    // 1. Password/URL/field policy
    if (event.inputContext()->capabilityFlags().test(CapabilityFlag::Password)) {
        apply(vfk_set_field_kind(session_, VFK_FIELD_PASSWORD), &event);
        return;
    }
    if (isUrlField(event.inputContext())) {
        apply(vfk_set_field_kind(session_, VFK_FIELD_URL), &event);
        return;
    }

    // 2. Shortcut toggle
    if (isToggleShortcut(event)) { ... return; }

    // 3. Nếu đang busy, giữ key vào queue (hoặc bỏ qua an toàn)
    if (busy_) {
        queue_.push(copyKey(event));
        event.filterAndAccept(); // giữ thứ tự
        return;
    }

    // 4. Cập nhật surrounding (chỉ khi app hỗ trợ)
    refreshSurrounding(event.inputContext());

    // 5. Xử lý
    std::string key = keyName(event.rawKey());
    if (key.empty()) return;
    VfkAction action = vfk_process_key(session_, key.c_str(), ctrl, alt, meta);

    // 6. Áp action
    apply(action, &event);
}
```

`apply()` cho Replace:

```cpp
case VFK_REPLACE: {
    std::string text = action.text ? action.text : "";
    if (action.delete_utf16 == 0) {
        commit(text, action.pass_key, event);
        break;
    }
    if (!surroundingMatches(action.delete_utf16, text)) {
        // Fail safe: không xóa mù
        showPreedit(text, event); // hoặc commit an toàn nếu app không có preedit
        break;
    }
    busy_ = true;
    int n = static_cast<int>(action.delete_utf16);
    ic_->deleteSurroundingText(-n, n);
    schedulePostCommit([this, text, passKey = action.pass_key, event] {
        commit(text, passKey, event);
        busy_ = false;
        drainQueue();
    }, profile_.wait_ack ? profile_.backspace_delay_ms : 0);
    break;
}
```

Lưu ý: đây là pseudo-code; cần xử lý `event` lifetime (Fcitx5 `KeyEvent` chỉ valid trong
callback), nên copy dữ liệu cần thiết, không giữ con trỏ `event` qua timer. Areca dùng
`TrackableObjectReference<InputContext>` và commit sau timer; ViFastKey cũng phải làm vậy.

### 5.4 Hàm xác minh surrounding

```cpp
bool surroundingMatches(int deleteUtf16, const std::string &insert) {
    auto &ic = *ic_;
    if (!ic.capabilityFlags().test(CapabilityFlag::SurroundingText)) return false;
    auto &s = ic.surroundingText();
    if (!s.isValid()) return false;
    if (s.cursor() != s.anchor()) return false; // selection: fail safe

    // Text engine tin là đang hiển thị: mirror hoặc API mới vfk_display_text()
    std::string shown = currentShownText();
    std::string before = utf8Prefix(s.text(), s.cursor());
    if (!endsWith(before, shown)) return false;

    // Với Replace, cần chắc rằng phần cần xóa chính là suffix của shown
    if (deleteUtf16 > 0 && utf16Length(shown) < deleteUtf16) return false;
    return true;
}
```

Cần thêm một trong hai:

- C ABI `const char *vfk_display_text(VfkSession *s)` trả text engine tin là đang hiển thị;
- hoặc adapter mirror `text` sau mỗi `Preedit`/`Commit`/`Replace`.

API `vfk_set_surrounding_text()` hiện chỉ nhận `before_cursor`; nên bổ sung
`vfk_set_surrounding_text_ex(session, before, cursor, anchor)` hoặc `vfk_set_selection()` để
engine/adapter phát hiện selection.

### 5.5 Chọn backend tối thiểu

```text
if (fieldKind == Password/Url/Number/Email) -> pass-through
if (app profile output_mode == preedit) -> preedit
if (delete_before == 0) -> commit/preedit
if (!hasSurrounding || !surroundingMatches) -> preedit fallback
if (hasSurrounding && probe OK) -> SurroundingReplace
else if (app needs replace && forwardKey available) -> ForwardBackspace
else -> preedit fallback
```

Chỉ thêm `uinput` khi có app thực tế không nhận cả surrounding lẫn forwardKey, và người dùng
bật rõ ràng.

---

## 6. Những điều không nên bê nguyên

1. **Bảng compatibility hàng trăm app**: hữu ích cho Areca vì đã trưởng thành, nhưng ViFastKey
   nên bắt đầu bằng capability + preset nhỏ + user override. Danh sách dài tạo gánh nặng
   bảo trì và dễ sai.
2. **uinput mặc định**: cần quyền `/dev/uinput`, có thể tạo duplicate event, phức tạp ACK.
   Areca phải thêm hướng dẫn `modprobe`, udev, nhóm `input`, và vẫn có fallback. ViFastKey
   nên để opt-in.
3. **Address bar bằng `cursorRect`**: Areca dùng `width <= 2`, `height 18–24`, `top < 200`
   và cache 5 giây. Rất dễ vỡ khi đổi theme/scale/font. Ưu tiên `CapabilityFlag::Url` và
   browser identity.
4. **Adaptive state toàn cục**: Areca chia sẻ một `AdaptiveWait` cho mọi app/backend; một app
   chậm có thể làm mọi app chậm thêm. Nếu ViFastKey làm adaptive, scope theo app/backend.
5. **Config migration im lặng**: Areca giữ field timing cũ hidden; nếu file advanced chưa có,
   giá trị cũ có thể che default mới. ViFastKey nên dùng `schema_version` + migration rõ ràng
   (đã có schema_version, hãy giữ kỷ luật).
6. **Reset không commit**: Issue #40 của Areca (“mất chữ ở mode preedit khi chưa commit mà
   click chuột nơi khác”) là bài học trực tiếp. ViFastKey `reset()` hiện có cùng rủi ro.
7. **Shift+Select không guard**: có thể phá selection người dùng. Luôn kiểm tra
   `cursor != anchor`, autocomplete, và fallback Backspace +1.
8. **Test bằng `assert`**: Areca dùng `assert` + `-UNDEBUG`, đơn giản nhưng dừng ở lỗi đầu
   tiên. ViFastKey có test infra tốt hơn; nên viết test adapter có tên scenario và log rõ.
9. **GUI nặng trước correctness**: SDL3 + ImGui + 9 theme là đầu tư lớn. Với ViFastKey, P0/P1
   là adapter đúng, không phải theme.
10. **Copy code vì MIT**: Areca MIT nên có thể tái sử dụng có ghi công, nhưng ViFastKey nên
    giữ clean-room behavior để tránh kéo kiến trúc C++/Go/Bamboo vào. Nếu copy, cập nhật
    `THIRD_PARTY_NOTICES.md`.

---

## 7. Kế hoạch test adapter tối thiểu

### 7.1 Test harness

Dựng `TestInputContext` giống Areca:

- `commitStringImpl()` ghi lại text;
- `deleteSurroundingTextImpl()` ghi lại offset/count và có thể mô phỏng app xóa thật;
- `forwardKeyImpl()` ghi lại press/release;
- `updatePreeditImpl()`;
- `surroundingText().setText(text, cursor, anchor)`;
- `setCapabilityFlags(...)`.

Có thể chạy CTest riêng, không cần Fcitx5 GUI.

### 7.2 Scenario bắt buộc

| Scenario | Kỳ vọng |
|---|---|
| Password field | Không preedit, không commit, key pass-through. |
| Preedit rồi reset | Text được commit hoặc giữ, không mất. |
| Replace với surrounding khớp | `deleteSurroundingText` đúng N, commit đúng text. |
| Replace với surrounding lệch | Không gọi delete; fallback preedit/commit an toàn. |
| Selection đang mở | Không Shift+Select; fallback Backspace +1 hoặc preedit. |
| Gõ nhanh 20 key | Không mất/lặp; thứ tự đúng. |
| Reset trong transaction | Reset bị hoãn, không cắt transaction. |
| App không gửi `SurroundingTextUpdated` | Probe timeout → fallback, không treo. |
| Macro undo | Backspace ngay sau macro khôi phục trigger; auto-repeat không phá. |
| Backspace auto-repeat | Chỉ xử lý một lần press; repeat/release forward đúng. |
| Click giữa từ | `cursor_moved` tắt adopt; không sửa nhầm từ cũ. |
| URL/address bar | Preedit hoặc select-replace an toàn; không nhân đôi autocomplete. |

### 7.3 App thật cần thử

- Terminal: GNOME Terminal, Kitty, Alacritty, WezTerm.
- Browser: Chrome/Chromium (address bar + Google Sheets), Firefox, Brave.
- Office: LibreOffice Writer, OnlyOffice.
- Electron/chat: VS Code, Discord, Signal, Slack, Telegram.
- IDE: JetBrains, Zed, Sublime Text.

Ghi lại app nào cần preedit, app nào cần replace, app nào cần delay/ACK. Đừng suy đoán từ
tên app; đo bằng log.

---

## 8. Kết luận

Areca không phải là “engine tiếng Việt hay hơn” để ViFastKey sao chép. Nó là một **playbook
adapter Fcitx5** cho những vấn đề mà engine thuần không giải quyết:

- thứ tự key và transaction khi frontend bất đồng bộ;
- surrounding text không đáng tin;
- app/frontend/field khác nhau;
- preedit/reset/click/backspace lifecycle;
- timing/ACK và fallback;
- đóng gói, debug, test trên app thật.

ViFastKey đã có lợi thế lớn: engine riêng, contract `Action` sạch, C ABI đầy đủ, preedit
mặc định an toàn, app profile schema tốt. Việc cần làm không phải viết lại engine hay bê
toàn bộ Areca, mà là **làm adapter Fcitx5 đủ tin cậy**:

1. field kind/password;
2. commit trước reset;
3. xác minh surrounding trước Replace;
4. transaction barrier + post-commit;
5. test adapter bằng InputContext giả lập.

Sau đó mới cân nhắc backend fallback, timing/ACK, browser/address bar, finalized word,
adaptive wait, uinput/Libei và GUI. Làm đúng thứ tự này sẽ giữ được lợi thế preedit-first
của ViFastKey mà vẫn học được phần khó nhất từ Areca.

---

## 9. Nguồn tham chiếu

### Areca

- `README.md`, `docs/ARCHITECTURE.md`, `docs/ARECA_ARCHITECTURE_SPECIFICATION.md`
- `docs/ADAPTIVE_DELAYS.md`, `docs/DEBUGGING.md`
- `src/areca_engine.{h,cpp}`, `src/input_scheduler.{h,cpp}`, `src/key_queue.h`
- `src/bamboo_engine_adapter.{h,cpp}`
- `src/surrounding_text_backend.{h,cpp}`, `src/surrounding_text_v2_backend.{h,cpp}`,
  `src/surrounding_reliability.h`, `src/surrounding_text_restore.{h,cpp}`,
  `src/surrounding_text_cache.{h,cpp}`
- `src/forward_backspace_backend.{h,cpp}`, `src/uinput_backspace_backend.{h,cpp}`,
  `src/uinput_shift_select_backend.{h,cpp}`, `src/native_device.{h,cpp}`,
  `src/xtest_*`
- `src/adaptive_wait.h`, `src/adaptive_wait_monitor.{h,cpp}`
- `src/rewrite_mode.{h,cpp}`, `src/preedit_mode.{h,cpp}`, `src/preedit_logic.{h,cpp}`
- `src/input_type_detector.{h,cpp}`, `src/program_compatibility.{h,cpp}`,
  `src/browser_autocomplete.{h,cpp}`, `src/window_focus_tracker.{h,cpp}`
- `src/sentence_capitalizer.{h,cpp}`, `src/word_segment.{h,cpp}`
- `settings/`, `install.sh`, `packaging/`, `.github/workflows/package-linux.yml`
- Issue #40 (mất chữ preedit khi click), #31 (lặp từ Discord/Signal), #22 (Google Sheets
  không tự edit cell), #28 (LibreOffice rewrite), #41 (Backspace auto-repeat).

### ViFastKey

- `linux/fcitx5/src/vifastkey.cpp`, `linux/fcitx5/README.md`
- `rust/crates/vfk-ffi/include/vifastkey.h`, `rust/crates/vfk-ffi/src/lib.rs`
- `rust/crates/vfk-engine/src/session.rs`, `rust/crates/vfk-engine/src/ports.rs`
- `docs/ADAPTER_GUIDE.md`, `docs/CONFIG.md`, `docs/COMPATIBILITY.md`,
  `docs/PLATFORMS.md`, `docs/IME_TEST_VECTORS.md`
- `docs/research/04-linux.md` (bối cảnh Fcitx5/Linux đã khảo sát trước đó)
