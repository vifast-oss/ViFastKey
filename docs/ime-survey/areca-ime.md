# Areca IME — sổ tay kỹ thuật

> Tài liệu này viết cho anh: đọc là hiểu Areca được xây dựng thế nào, vì sao nó khó,
> và nếu cần sửa/debug/mở rộng thì bắt đầu từ đâu. Không phải bài giới thiệu cho
> người dùng cuối — phần cài đặt chỉ chiếm một mục, còn lại là kiến trúc và mã nguồn.
>
> **Snapshot:** commit `952ddd8` (2026-11-01), tag `v8.3.0`.
> **Giấy phép:** MIT (kể cả `bamboo-core`). Có thể đọc, sửa, dùng thương mại.
> **Nền tảng:** Linux, Fcitx5 addon, tối ưu Wayland.

---

## 0. Mười phút nắm Areca

Areca **không viết lại engine tiếng Việt**. Nó lấy `bamboo-core` (MIT, cùng engine
với ibus-bamboo) qua một bridge Go, rồi dồn gần như toàn bộ công sức vào thứ khó
hơn: **làm cho việc sửa chữ đã hiện trên màn hình không bị sai, không bị mất, không
bị lặp, trên hàng trăm ứng dụng Linux khác nhau**.

Ba câu tóm tắt:

1. **Engine là phần dễ.** `bamboo-core` đã lo Telex/VNI/VIQR, bảng mã, kiểm tra
   chính tả, macro.
2. **Adapter là phần khó.** Fcitx5 giao phím đồng bộ, nhưng `deleteSurroundingText()`
   và `commitString()` là bất đồng bộ với app. App nào hỗ trợ gì, xử lý lệnh xóa
   nhanh hay chậm, có gửi ACK hay không — mỗi app một kiểu.
3. **Giá trị của Areca nằm ở 6 cơ chế:** FIFO + single-flight + transaction barrier;
   backend abstraction với fallback; trust probe cho surrounding text; timing/ACK
   thích ứng; nhận diện app/field; và vòng đời preedit/reset/click.

Nếu chỉ nhớ một ý: **Areca là một nghiên cứu điển hình về lập trình IME trên Linux,
không phải về thuật toán tiếng Việt.**

---

## 0.1 Đọc theo quỹ thời gian

| Anh có | Đọc gì |
|---|---|
| 10 phút | §0, §2, §3, §17 (mạnh/yếu) |
| 30 phút | §0–§9 + §16 (đọc source theo thứ tự) |
| 2 giờ | Toàn bộ, trừ §12 (config) và §13 (build) |
| Muốn sửa code | §4–§9, §15, §16 rồi mở `src/input_scheduler.cpp` |

## 0.2 Thử nhanh (5 phút)

```bash
# 1. Cài Fcitx5 (Ubuntu/Debian)
sudo apt install fcitx5 fcitx5-config-qt fcitx5-frontend-all

# 2. Cài Areca
curl -fsSL https://raw.githubusercontent.com/xhkzeroone/ArecaIME/main/install.sh | bash

# 3. Thêm input method
fcitx5-configtool    # tìm "Areca (Bamboo)" → Add

# 4. Khởi động lại và kiểm tra
fcitx5 -rd
pgrep -a fcitx5
fcitx5-diagnose | grep -i areca
```

Gõ thử `tieengs` trong một editor đơn giản (gedit/kate). Nếu ra `tiếng`, engine
và addon đã thông. Nếu không, xem §14.

## 0.3 Mục lục

1. Vì sao IME trên Linux khó
2. Kiến trúc tổng thể
3. Ba chế độ
4. Đường đi của một phím
5. Engine: bamboo-core và diff
6. Scheduler: FIFO, single-flight, barrier
7. Backend rewrite và chính sách chọn
8. Trust probe cho surrounding text
9. Timing, ACK và AdaptiveWait
10. Nhận diện app, field, context
11. Preedit, finalized word, reset, click
12. Cấu hình đầy đủ
13. Build, cài, đóng gói
14. Debug: playbook thực dụng
15. Test: 22 CTest và `TestInputContext`
16. Đọc source theo thứ tự
17. Điểm mạnh, điểm yếu, giới hạn
18. Nếu anh muốn đóng góp
19. Glossary
20. Nguồn
- Phụ lục A — Cheat sheet (một trang)

---

## 1. Vì sao IME trên Linux khó

### 1.1 Mô hình Fcitx5

Fcitx5 là một daemon input method. Ứng dụng (client) và Fcitx5 nói chuyện qua
frontend:

| Frontend | Cơ chế | Hệ quả |
|---|---|---|
| `wayland` (text-input-v3) | Giao thức Wayland | Chuẩn mới, nhưng nhiều app hỗ trợ chưa đủ |
| `dbus` | IBus DBus / portal | Phổ biến trên GNOME |
| `xim` | XIM (X11) | Cũ, độ trễ cao, hay mất key |
| `fcitx4` | Tương thích ngược | Hiếm |
| `gtk` / `qt` | Module IM nhúng | Nhanh, nhưng app tự quyết định |

Mỗi frontend trả về một tập **capability** khác nhau. Areca phải đọc capability
thay vì đoán theo tên app — nhưng capability cũng không đáng tin tuyệt đối
(xem §8).

### 1.2 Ba cách đưa chữ vào app

```text
Cách 1 — Preedit (composition):
  app hiển thị vùng soạn thảo có gạch chân, Fcitx5 sở hữu nó.
  Ưu: an toàn, không cần xóa chữ đã commit.
  Nhược: gạch chân, một số app/terminal không hỗ trợ, IME "nháy".

Cách 2 — Commit trực tiếp (direct/rewrite):
  IME gửi thẳng chuỗi vào app, không có vùng đệm.
  Ưu: giống UniKey trên Windows, không gạch chân.
  Nhược: muốn sửa chữ đã hiện thì phải xóa ngược (backspace) rồi chèn lại.

Cách 3 — Gõ phím vật lý / uinput / XTest:
  IME giả lập bàn phím ở tầng kernel hoặc X server.
  Ưu: app không thể "nuốt" key.
  Nhược: cần quyền, dễ tạo duplicate event, khó đồng bộ.
```

Areca mặc định **Cách 2 (Rewrite)**, và có **Cách 1 (Preedit)** cùng **Cách 3**
làm fallback. Đây là lựa chọn thiết kế trung tâm: Rewrite cho trải nghiệm giống
UniKey, nhưng buộc phải giải bài toán "xóa chữ đã commit" thật nghiêm túc.

### 1.3 Những thứ có thể hỏng khi rewrite

- App nhận `deleteSurroundingText(-N, N)` nhưng xóa thiếu/không xóa.
- App xóa xong nhưng chưa cập nhật `surroundingText()`; IME đọc snapshot cũ.
- Key tiếp theo đến trước khi app xử lý xong lệnh xóa → mất/lặp ký tự.
- App tự động thêm autocomplete/URL completion → xóa nhầm.
- Terminal/Electron nuốt `forwardKey(Backspace)`.
- Người dùng đang bôi đen text → lệnh xóa ăn vào selection.
- Click chuột giữa từ → state engine vẫn tưởng đang ở caret cũ.

Toàn bộ phần còn lại của tài liệu là cách Areca xử lý từng mục trên.

---

## 2. Kiến trúc tổng thể

### 2.1 Thành phần

```text
┌──────────────────────────────────────────────────────────────────┐
│                         Fcitx5 daemon                            │
│                                                                  │
│  ArecaEngine (addon)                                             │
│   ├─ chọn PresentationMode: Rewrite / Preedit / Redirect          │
│   ├─ password? → forward nguyên bản                               │
│   ├─ mouse click? → protected reset                               │
│   ├─ hotkey? → menu chọn backend cho app                          │
│   └─ dispatch → InputModeHandler                                  │
│                                                                  │
│   RewriteModeHandler ──► KeyQueue ──► InputScheduler              │
│                                          │                        │
│                                          ├─ BambooEngineAdapter   │
│                                          │     └─ Go bridge ──────┼──► bamboo-core
│                                          │                        │
│                                          ├─ ReliabilityChecker    │
│                                          └─ RewriteBackend        │
│                                               ├─ SurroundingText  │
│                                               ├─ SurroundingV2    │
│                                               ├─ ForwardBackspace │
│                                               ├─ UinputBackspace  │
│                                               ├─ UinputShiftSelect│
│                                               └─ NativeXTest      │
│                                                                  │
│   PreeditModeHandler ──► Bamboo trực tiếp + preedit UI            │
│   RedirectModeHandler ─► forward                                  │
│                                                                  │
│   WindowFocusTracker (AT-SPI)   MouseClickTracker → helper process│
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 Quy mô mã nguồn

| Khu vực | File | Dòng | Ghi chú |
|---|---:|---:|---|
| `src/` (addon) | 73 | ~11.900 | C++20, gồm AT-SPI và notification |
| `src/areca_engine.cpp` | 1 | 2.066 | File lớn nhất; routing + policy |
| `tests/` | 23 | ~4.400 | 22 CTest target |
| `settings/` | — | ~2.670 | SDL3 + Dear ImGui, chưa kể ImGui vendored |
| `bamboo-bridge/main.go` | 1 | 475 | c-archive, export C ABI |
| Tổng đọc được (không tính ImGui) | — | ~19.000 | |

Một chi tiết dễ gây nhầm: `CMakeLists.txt` khai báo `project(... VERSION 0.1.0)`
trong khi release tag là `v8.3.0`. Đừng dùng số trong CMake để đánh giá độ chín;
hãy nhìn tag, số test và lịch sử commit.

### 2.3 Vì sao có Go bridge?

`bamboo-core` là thư viện Go. Areca cần gọi nó từ C++:

```text
C++ (BambooEngineAdapter)
   │  extern "C" ArecaBambooProcess(id, key, spellCheck, ...)
   ▼
Go bridge (bamboo-bridge/main.go, buildmode=c-archive)
   │  bamboo.NewEngine(method, flags)
   ▼
bamboo-core (Go)
```

Bridge giữ một `map[uint64]bamboo.IEngine`. Mỗi input context có một engine id
riêng, tránh state của app A lẫn sang app B. API export gồm:

`ArecaBambooCreate`, `Destroy`, `Process`, `Backspace`, `RecoverBackspace`,
`Reset`, `AddMacro`, `ExpandMacro`, `FinalizeWord`, `Restore`,
`InputMethodNames`, `CharsetNames`, `Encode`, `CanProcess`.

Điểm đáng học: bridge **không trả chuỗi cuối cùng**, nó trả đủ dữ liệu để C++
tính diff (xem §5.2). Nhờ vậy toàn bộ logic "xóa bao nhiêu, chèn gì" nằm ở một
chỗ, không phân tán.

---

## 3. Ba chế độ

| Mode | Hành vi | Khi nào dùng |
|---|---|---|
| `Rewrite` | Gõ trực tiếp; engine tính diff; IME xóa/chèn | Mặc định; giống UniKey |
| `Preedit` | Composition có gạch chân; commit khi hết từ | App/terminal không chịu được rewrite |
| `Redirect` | Forward phím gốc, không xử lý tiếng Việt | Chế độ EN toàn cục |

Chuyển mode:

- Global: `SwitchModeKey` — tài liệu kiến trúc nói `Alt+Space`, nhưng trong
  commit này default là rỗng và nhánh xử lý đang bị comment trong
  `ArecaEngine::keyEvent`; hạ tầng mode vẫn tồn tại. Đừng giả định hotkey có
  sẵn khi debug.
- Theo app: `AppOverrides` trong `areca-apps.conf` (Auto / Surrounding /
  Shift select / Backspace / Preedit / Redirect).
- Theo field: `CapabilityFlag::Password` luôn đi đường forward, bất kể mode.

**Điểm cần hiểu:** ba mode có state riêng. `RewriteInputState` và
`PreeditInputState` không dùng chung engine instance. Đổi mode phải reset cả hai.
Đây là lý do Areca không bị "di chuyển state" khi người dùng chuyển qua lại.

---

## 4. Đường đi của một phím (Rewrite)

### 4.1 Trình tự

```text
1. Fcitx5 gọi ArecaEngine::keyEvent()
2. Nếu Password → handler hiện tại forward, xóa state cũ, return
3. Nếu đang chọn backend (menu) → xử lý phím menu, return
4. Nếu là Backspace do uinput replay → Forward/Uinput backend "nuốt" trước
5. Nếu là Left do Shift+Select replay → ShiftSelect backend xử lý
6. Nếu là press:
   a. resolveProgram() (AT-SPI nếu cần)
   b. có mouse click pending? → reset có bảo vệ
   c. hotkey chọn backend? → mở menu
7. handlerFor(context).handleKeyEvent(event)
   └─ RewriteModeHandler:
      - release: bỏ qua (trừ Backspace recovery)
      - modifier-only: bỏ qua
      - Backspace khi rewrite pending: forward, không đổi state
      - Password: reset + forward
      - tính suppressMacro (nếu giữ modifier + Space)
      - nếu engine rảnh và là browser: thử handleIdleKey()
      - ngược lại: filterAndAccept() rồi enqueue(codepoint, utf8)
8. InputScheduler::processNext()
   - pop đúng một key
   - engine->process() hoặc processBackspace()
   - applyResult()
9. applyResult():
   - deleteCount == 0 → commitString + post-commit barrier
   - deleteCount > 0  → chọn backend → apply → chờ callback
10. rewriteDone() → post-commit timer (20 ms) → xử lý key kế
```

### 4.2 Vì sao `filterAndAccept()` trước khi enqueue?

Fcitx5 có hai lựa chọn cho mỗi event:

- **Không filter**: app vẫn nhận phím gốc.
- **`filterAndAccept()`**: app không nhận phím gốc; IME chịu trách nhiệm gửi
  kết quả.

Areca accept ngay khi đưa key vào queue, vì nó sẽ tự commit kết quả. Nếu không
accept, app sẽ nhận cả phím gốc lẫn chữ đã biến đổi → nhân đôi.

Ngoại lệ quan trọng: **`handleIdleKey()`** (§4.3) cố tình *không* accept khi
Bamboo không đổi output, để editor web nhận được phím đầu.

### 4.3 `handleIdleKey()` — chiêu cho Google Sheets/editor web

Một số web editor chỉ mở chế độ soạn thảo khi nhận được `keydown` thật. Nếu IME
nuốt phím đầu và chỉ gửi `commitString`, editor không vào chế độ edit.

Areca xử lý:

```cpp
// Chỉ khi: browser, engine rỗng, scheduler rảnh, không pending, không stalled.
result = engine->process(codepoint, text);

if (result.deleteCount == 0 &&
    result.commitText == text &&
    !result.macroExpanded) {
    event.forward();          // để frontend nhận phím gốc
    // Vẫn giữ state Bamboo: gõ 'a' rồi 's' vẫn thành 'á'
    finishKeyAfterDelay();    // vẫn giữ barrier 20 ms
} else {
    event.filterAndAccept();
    applyResult(result);      // áp kết quả đã tính, không xử lý lại
}
```

Điểm tinh tế: `event.forward()` trong Fcitx5 là **getter**, không phải hành động.
Việc "cho phím đi tiếp" là **không gọi `filterAndAccept()`**. Rất dễ đọc sai chỗ này.

Điều kiện để đi đường tắt rất chặt: `processing_ == false`, không rewrite pending,
queue rỗng, không stalled, và đã qua cửa sổ bảo vệ 20 ms. Nếu không, rơi về FIFO
bình thường.

---

## 5. Engine: bamboo-core và diff

### 5.1 Cấu hình engine

```go
method := bamboo.ParseInputMethod(bamboo.InputMethodDefinitions, name)
flags  := uint(bamboo.EstdFlags)
if modernStyle { flags &^= bamboo.EstdToneStyle }  // ngược tên có chủ ý
engine := bamboo.NewEngine(method, flags)
```

`ModernStyle=True` → `oà/uý`; `False` → `òa/úy`. Tên flag nội bộ `EstdToneStyle`
ngược với tên option, và điều này được ghi chú rõ trong code — một ví dụ tốt về
việc ghi lại "tại sao" thay vì chỉ "cái gì".

### 5.2 `BambooResult` và common-prefix diff

Adapter giữ `renderedText_` — chuỗi nó tin là đang hiển thị. Sau mỗi key:

```text
currentText  = chuỗi trước key
newText      = chuỗi Bamboo trả về sau key
deleteCount  = số code point ở suffix cũ cần xóa
commitText   = suffix mới cần chèn
```

Ví dụ:

```text
currentText = "a"   key = "w"   newText = "ă"
→ deleteCount = 1, commitText = "ă"

currentText = "nguoi"  key = "wf"  newText = "người"
→ deleteCount = 5, commitText = "người"
```

**Chi tiết quan trọng:** diff được tính trên chuỗi **đã encode theo `OutputCharset`**,
không phải trên Unicode nội bộ. Vì vậy với `Unicode tổ hợp`, VNI-Windows hay VIQR,
`deleteCount` vẫn khớp đúng số ký tự thực tế đã commit. Nếu tính diff trên Unicode
rồi encode sau, độ dài xóa sẽ sai với bảng mã nhiều codepoint.

Đây là bài học trực tiếp cho bất kỳ ai viết adapter: **độ dài xóa phải tính trên
representation mà app thực sự nhận.**

### 5.3 Spell check ba mức

| `SpellcheckMode` | Hành vi |
|---|---|
| `Off` | Không kiểm tra; commit nguyên trạng |
| `Basic` | Khi hết từ, gọi `IsValid(true)`; nếu từ có dấu nhưng cấu trúc sai → `RestoreLastWord(false)` về chuỗi phím Latin |
| `Realtime` | Như Basic, và kiểm tra từng ký tự trung gian (`spellCheck=1`) để rollback ngay |

Restore không dùng từ điển ngoài; nó dùng luật âm tiết của `bamboo-core`.

### 5.4 Macro

- Macro table là sub-config riêng (`Macro` key/value).
- Expand tại **word boundary**, trước spell check.
- Lookup không phân biệt hoa/thường.
- `CapitalizeMacro`: key thường → value thường; key HOA → value HOA;
  key Firstupper → Title Case từng từ; mixed-case khác → giữ value gốc.
- `MacroBypassModifier`: giữ Shift/Ctrl/Alt/Super + Space để bỏ macro cho **đúng
  boundary đó**. Cờ `suppressMacro` nằm trên từng `QueuedKey`, không phải state
  one-shot → gõ nhanh không bị lây sang phím sau.
- `RevertMacroOnBackspace`: Backspace ngay sau macro khôi phục trigger + boundary.

### 5.5 `RestoreSurroundingText`

Tính năng thử nghiệm: đọc từ ngay trước caret từ `surroundingText()` rồi nạp lại
vào engine để sửa tiếp từ đã commit.

Cách làm rất thận trọng:

1. Chỉ đọc tối đa 16 chữ cái Latin/tiếng Việt dựng sẵn.
2. Từ chối chuỗi UTF-8 lỗi, dấu tổ hợp, từ bị cắt.
3. Không đoán: tách chữ gốc + hiệu ứng dấu, thử một tập phím hữu hạn trên **engine
   tạm**, yêu cầu output khớp chính xác từ trên màn hình.
4. Chỉ khi xác minh thành công mới nạp chuỗi phím vào engine thật.

Đây là pattern "verify on a scratch engine before mutating the real one" — rất
đáng bắt chước.

---

## 6. Scheduler: FIFO, single-flight, barrier

### 6.1 Bốn bất biến

1. **FIFO** — key giữ đúng thứ tự.
2. **Single-flight** — chỉ một key được engine xử lý tại một thời điểm.
3. **Transaction barrier** — khi backend đang xóa/chèn, key mới chỉ vào queue.
4. **Không sleep trên main thread** — mọi delay dùng timer của event loop.

### 6.2 Code path

```cpp
void InputScheduler::enqueue(...) {
    key.sequence = nextSequence_++;
    key.inputContext = inputContext.watch();   // không giữ con trỏ thô
    queue_.push(std::move(key));
    scheduleNext();
}

void InputScheduler::scheduleNext() {
    if (processing_ || queue_.empty() || stalled_) return;
    processNext();
}

void InputScheduler::processNext() {
    auto key = queue_.pop();
    processing_ = true;
    ...
    applyResult(...);
}
```

`stalled_` là trạng thái "fail closed": nếu backend báo lỗi, Areca **dừng xử lý**
thay vì tiếp tục trong trạng thái không xác định. Người dùng có thể phải chuyển
app hoặc restart Fcitx5, nhưng không bị mất/lặp chữ hàng loạt. Đây là lựa chọn
đúng: thà treo còn hơn hỏng dữ liệu.

### 6.3 Vì sao key đầu được xử lý inline?

Nếu mọi key đều qua timer, độ trễ tăng và gõ nhanh sẽ cảm thấy lag. Areca chỉ
enqueue khi pipeline bận; key đầu tiên khi rảnh được xử lý ngay trong callback.
Queue chỉ dùng cho key đến trong lúc bận — thường là 0–2 key.

### 6.4 Post-commit barrier

Sau mỗi commit:

```cpp
lastRewriteCompletionTimeUsec_ = now();
postCommitTimer_ = eventLoop_.addTimeEvent(
    CLOCK_MONOTONIC, now() + postCommitDelayMs * 1000, timerAccuracyUsec,
    [this](...) { finishKey(); return false; });
```

`PostCommitDelayMs` mặc định **20 ms**. Trong cửa sổ này:

- `shouldRejectReset()` trả `true` → reset từ app bị hoãn.
- Key mới vẫn được nhận nhưng chờ timer.
- Mouse click chưa được áp.

Mục đích: cho app kịp xử lý commit trước khi IME đọc lại surrounding hoặc xóa
tiếp. 20 ms là con số thực nghiệm, không phải chuẩn.

---

## 7. Backend rewrite và chính sách chọn

### 7.1 Sáu backend

| Backend | Cách sửa | Dùng khi |
|---|---|---|
| `SurroundingTextBackend` | `deleteSurroundingText(-N,N)` + `commitString` | App hỗ trợ surrounding và đáng tin |
| `SurroundingTextV2Backend` | Xóa từng ký tự, mỗi nhịp 1 ms | Firefox/Monaco/CodeMirror xử lý xóa gộp kém |
| `ForwardBackspaceBackend` | `forwardKey(Backspace)` × N | App nhận forward key, không có surrounding |
| `UinputBackspaceBackend` | `KEY_BACKSPACE` × N qua `/dev/uinput` | App/frontend nuốt forward Backspace |
| `UinputShiftSelectBackend` | Shift+Left × N+1 rồi commit | Browser cần thay selection |
| `NativeXTestBackend` | Libei (Wayland) / XTest (X11) | uinput không khả dụng, hoặc user chọn |

### 7.2 Thứ tự chọn (rút gọn theo `selectRewriteBackend`)

```text
terminal (kể cả VS Code embedded)
    → backspace backend ưu tiên hardware:
      Native (nếu bật) → uinput → Native → ForwardBackspace
browser autocomplete / Firefox autocomplete thu gọn / đang có selection
    ├─ nếu là URL và có shift-select → UinputShiftSelect
    └─ ngược lại → backspace backend (+1 nếu selection khớp text hiện tại)
URL không có surrounding + browser
    → UinputShiftSelect (nếu có), else Backspace
UseBackspaceForUrl bật
    → UinputShiftSelect (nếu có), else Backspace
app override != Auto
    → backend được chỉ định (kèm fallback)
decision.useSurrounding
    → V2 / ShiftSelect (nếu bật) / SurroundingText
còn lại
    → uinput/native nếu bật, else ForwardBackspace
```

Lưu ý: "terminal" ở đây là kết quả của `InputTypeDetector::isTerminal()`, không chỉ
tên app. Một app có thể là terminal ở cửa sổ chính nhưng là editor ở panel phụ;
Areca chọn theo context, không theo process.

### 7.3 Vì sao `plasmashell` bị đặc cách?

`plasmashell`/krunner công bố surrounding hợp lệ nhưng áp dụng update trễ và
không phát ACK khớp. Nếu dùng `deleteSurroundingText`, chữ bị xóa sai. Areca
đẩy thẳng vào nhóm compatibility, dùng ForwardBackspace. Đây là ví dụ của một
nguyên tắc: **capability chỉ là gợi ý; hành vi thực tế mới là sự thật.**

### 7.4 Selection guard

Nếu `cursor != anchor` (đang bôi đen) hoặc phát hiện browser autocomplete:

- **Không** dùng Shift+Select (sẽ phá selection người dùng).
- Fallback ForwardBackspace, cộng thêm **1** Backspace để xóa vùng autocomplete.

Điều kiện autocomplete (`browser_autocomplete.cpp`) khá chặt: selection phải bắt
đầu ngay sau composition, kéo tới hết dòng, không đi qua newline.

### 7.5 uinput: những cạm bẫy đã xử lý

`UinputBackspaceBackend` không chỉ là "gửi N phím Backspace":

- Mọi event Backspace quay lại Fcitx đều phải **forward**, kể cả event đến muộn.
- Bộ đếm ACK chỉ là mốc tương đối, **không dùng để lọc event**.
- Timer phát phím và timer commit **độc lập**; commit không được chạy trước khi
  phát đủ N phím, dù ACK đến sớm.
- Hết timeout ACK (150 ms) vẫn chuyển sang settling wait rồi commit, không treo.

`UinputShiftSelectBackend` còn khó hơn: gửi Shift down, Left × N+1, forward N Left
đầu, lọc Left xác nhận cuối, Shift up, chờ settling, rồi commit từng ký tự.

**Bài học:** nếu định làm uinput, hãy coi nó là một state machine có ACK, không
phải một lời gọi hàm.

---

## 8. Trust probe cho surrounding text

Đây là cơ chế đáng học nhất của Areca.

### 8.1 Vấn đề

App khai báo `CapabilityFlag::SurroundingText`. Nhưng:

- Snapshot có thể cũ.
- App có thể bỏ qua `deleteSurroundingText`.
- App có thể xóa nhưng không phát `SurroundingTextUpdated`.
- App có thể di chuyển caret trước rồi mới xóa.

Nếu tin ngay, IME sẽ xóa nhầm hoặc chèn đè.

### 8.2 Cách probe

```text
1. App khai báo SurroundingText, snapshot valid.
2. Text ngay trước cursor phải kết thúc bằng currentText của engine.
3. Gửi deleteSurroundingText(-N, N).
4. Chờ event SurroundingTextUpdated, tối đa 150 ms (lần đầu) / 80 ms (retry).
5. Kiểm tra ngữ nghĩa bằng surroundingProbeMatches():
   - cursor == anchor (không selection)
   - text trước cursor khớp expected sau khi xóa
   - text cũ không còn
   - loại trừ trạng thái trung gian "cursor di chuyển trước rồi xóa"
6. Khớp → đánh dấu reliable, hủy deadline, commit.
7. Timeout/không khớp → đánh dấu unreliable, fallback backend.
```

Verdict được cache theo `InputContext` UUID, và bị xóa khi lifecycle thay đổi.
Backspace, Ctrl+A và phím di chuyển bảo vệ verdict trong **1 giây** — vì những
thao tác đó có thể làm snapshot cũ đi nhưng không đủ để kết luận app hỏng.

### 8.3 Vì sao không tin capability?

Vì capability là **lời khai**, không phải **bằng chứng**. Areca chờ bằng chứng
rằng lệnh xóa đã thực sự có hiệu lực. Đây là khác biệt giữa "hy vọng app đúng"
và "biết app đúng".

### 8.4 Chi phí

Probe chỉ chạy ở lần rewrite đầu tiên cần xóa, không chạy mỗi phím. Timeout
150/80 ms chỉ xảy ra một lần cho mỗi context. Vì vậy chi phí gần như bằng 0
trong steady state.

---

## 9. Timing, ACK và AdaptiveWait

### 9.1 Delay theo frontend

Areca không dùng một con số chung. Cùng một thao tác, mỗi frontend một mức:

| Nhóm | Wayland | DBus/IBus/XIM/Fcitx4 |
|---|---:|---:|
| Giữa các Backspace | 1 ms | 1 ms |
| Sau Backspace cuối | 3 ms | 10 ms |
| Sau xóa surrounding | 3 ms | 3 ms |
| Sau Shift+Left | 5 ms | 10 ms |
| Sau commit | 20 ms | 20 ms |
| uinput ACK timeout | 150 ms | 150 ms |

### 9.2 AdaptiveWait

```text
extraWaitMs = 0
mỗi tín hiệu lag: +10 ms (tối đa 50 ms)
sau 5 transaction ổn định: -10 ms
tín hiệu:
  - timer drift ≥ 5 ms
  - ACK round-trip ≥ 4 ms
  - ACK timeout → đề nghị 30 ms
  - CPU PSI / load average / nhiệt độ (80/85°C)
```

Trước commit, backend đọc lại mức adaptive; nếu vừa tăng, hẹn thêm rồi mới commit.

### 9.3 Event loop post

Commit không chạy trực tiếp trong timer callback. Nó được đẩy sang **post phase**
của event loop để các công việc đang nghẽn được xử lý trước. Đây là chi tiết nhỏ
nhưng quan trọng: commit trong timer callback có thể vượt mặt thao tác xóa mà app
chưa xử lý.

### 9.4 `PreciseTiming`

`True` → timer accuracy 1 µs. `False` → accuracy 0, cho phép event loop coalesce
timer (tiết kiệm CPU, nhưng delay kém chính xác). Mặc định `True`.

---

## 10. Nhận diện app, field, context

### 10.1 Các tầng nhận diện

| Tầng | Nguồn | Dùng cho |
|---|---|---|
| Program | `inputContext.program()` | App override, compatibility list |
| Fallback program | AT-SPI (`WindowFocusTracker`) | Khi frontend trả rỗng |
| Frontend | `wayland`, `xim`, `ibus`, `dbus`, `fcitx4` | Chọn timing |
| Capability | `SurroundingText`, `Password`, `Url`, `KeyEventOrderFix`, `FormattedPreedit` | Field policy |
| Field | terminal, browser, address bar, editor/IDE, office | Backend policy |
| Selection | `cursor != anchor` | Selection guard |
| Browser autocomplete | selection kéo tới cuối dòng | +1 Backspace |
| Address bar | `CapabilityFlag::Url`, browser + AT-SPI, cuối cùng là `cursorRect` | Shift-Select/Backspace |

### 10.2 Compatibility list

`program_compatibility.cpp` chứa hàng trăm tên: JetBrains, VS Code, Zed, Sublime,
terminal (GNOME, Kitty, Alacritty, WezTerm...), browser, Office, chat.

Hai hàm quan trọng:

- `normalizedProgramName()`: bỏ path, lowercase, bỏ `.desktop`.
- `matchesNameOrVariant()`: prefix + kiểm tra ký tự kế tiếp là `.` hoặc `-`.

Nhờ vậy `code` không match `codeblocks`, `code-insiders`, `codelite`.

### 10.3 Address bar

Thứ tự ưu tiên:

1. `CapabilityFlag::Url` — đáng tin nhất.
2. Browser identity + AT-SPI browser UI.
3. Heuristic `cursorRect`: width ≤ 2, height 18–24, top < 200, cache 5 giây.

Heuristic cuối rất dễ vỡ khi đổi theme/scale/font. Areca dùng nó như phương án
cuối, không phải phương án chính. Nếu anh viết IME, đừng bắt đầu từ đây.

---

## 11. Preedit, finalized word, reset, click

### 11.1 Preedit mode

- Dùng `BambooEngineAdapter` riêng, xử lý đồng bộ.
- Hiển thị qua client preedit nếu app có `CapabilityFlag::Preedit`; nếu không
  dùng server-side preedit của Fcitx.
- `BackspaceRecovery=True`: Backspace trong composition gọi `processBackspace()`
  để Bamboo dựng lại chuỗi (`nhanhsh → nhánh`).
- Space/dấu câu: macro + spell check rồi commit ngay.

### 11.2 Finalized word recovery

Sau khi commit một từ + boundary (space/dấu câu), adapter giữ snapshot của từ và
đếm số boundary đã commit.

```text
Người dùng gõ:  "Chào "     (từ "Chào" đã commit, 1 boundary)
Backspace:      xóa boundary, đưa "Chào" trở lại preedit
Gõ tiếp "f":    "Chàof" → engine xử lý như đang soạn
```

Điều kiện: app hỗ trợ `SurroundingText` **và** snapshot khớp chính xác. Nếu không,
reset state và forward Backspace gốc. Không đoán.

Đây là tính năng rất khó, và Areca làm đúng cách: chỉ bật khi có bằng chứng.

### 11.3 Reset barrier

`ArecaEngine::reset()` không reset ngay:

```cpp
if (scheduler_.shouldRejectReset()) return;   // đang xử lý hoặc <20ms sau commit
handlerFor(context).requestProtectedReset(context);
```

`RewriteModeHandler::requestProtectedReset` còn từ chối nếu đang trong 20 ms sau
Backspace. `PreeditModeHandler` từ chối nếu đang `Erasing` (finalized word
recovery) hoặc trong 20 ms sau commit.

Mục đích: reset từ frontend (đổi focus, app gửi lifecycle giả) không được xóa
state mà IME cần để hoàn tất thao tác đang dở.

### 11.4 Mouse click tracker

```text
libinput/udev → areca-mouse-monitor (process con)
             → pipe stdout (R/C)
             → MouseClickTracker trên event loop
             → pending click
             → keyEvent kế tiếp → reset có bảo vệ
```

- `R` = context libinput đã khởi tạo.
- `C` = có nút chuột được nhấn (không phân biệt trái/phải/giữa).
- Pipe nonblocking; nhiều click gộp thành một cờ.
- Callback chỉ đánh dấu; trước phím kế, addon đọc thêm pipe để lấy click đến muộn.
- Nếu scheduler đang bảo vệ rewrite, cờ được giữ cho phím sau.

Đây là cách duy nhất phát hiện click trên Wayland mà không cần quyền root, nhưng
cần udev rule `70-areca-pointer.rules` cấp `uaccess` cho chuột/touchpad.

---

## 12. Cấu hình đầy đủ

### 12.1 File

| File | Nội dung |
|---|---|
| `areca.conf` | Cấu hình chính (kiểu gõ, charset, macro bật/tắt, app override...) |
| `areca-advanced.conf` | Timing, fallback, mouse tracking, debug |
| `areca-macro-table.conf` | Bảng macro |
| `areca-apps.conf` | Override backend theo app |

Field timing cũ trong `areca.conf` được giữ dạng `HiddenOption` để migrate.
File advanced load sau và **ưu tiên**. Đây là lý do nếu sửa tay cả hai file, kết
quả có thể khác dự đoán — hãy sửa qua UI hoặc chỉ sửa file advanced.

### 12.2 Nhóm chính

| Key | Default | Ý nghĩa |
|---|---|---|
| `BambooInputMethod` | `Telex 2` | Kiểu gõ (lấy từ bamboo-core) |
| `OutputCharset` | `Unicode` | Bảng mã đầu ra |
| `SpellcheckMode` | `Realtime` | Off / Basic / Realtime |
| `ModernStyle` | `true` | `oà/uý` thay `òa/úy` |
| `AutoCapitalizeAfterPunctuation` | `false` | Viết hoa sau `. ! ?` |
| `EnableMacro` | `true` | Bật macro |
| `CapitalizeMacro` | `true` | Giữ kiểu hoa/thường |
| `RevertMacroOnBackspace` | `true` | Backspace hoàn tác macro |
| `MacroBypassModifier` | `Shift` | Giữ modifier + Space để bỏ macro |
| `BackspaceRecovery` | `true` | Backspace dựng lại từ trong preedit |
| `RestoreSurroundingText` | `false` | Sửa từ đã commit (thử nghiệm) |
| `ShiftSelectFallbackForBrowser` | `false` | Shift+Left khi browser thiếu surrounding |
| `UseBackspaceForUrl` | `true` | Xử lý riêng ô URL |
| `SelectBackendKey` | `grave` (`` ` ``) | Mở menu chọn backend cho app |
| `SwitchModeKey` | rỗng | Hotkey đổi mode |
| `Debug` | `false` | Log chi tiết |

### 12.3 Timing (advanced)

| Key | Default | Ghi chú |
|---|---:|---|
| `BackspaceDelayMs` | 1 | Giữa các Backspace |
| `AfterBackspaceWaitMs` | 10 | Sau Backspace cuối |
| `WaylandAfterBackspaceWaitMs` | 3 | Riêng Wayland |
| `XimAfterBackspaceWaitMs` | 10 | |
| `Fcitx4AfterBackspaceWaitMs` | 10 | |
| `DbusAfterBackspaceWaitMs` | 10 | |
| `IBusAfterBackspaceWaitMs` | 10 | |
| `UinputShiftSelectDelayMs` | 1 | Giữa các Shift+Left |
| `UinputShiftSelectAckTimeoutMs` | 150 | Hidden |
| `AfterUinputShiftSelectWaitMs` | 10 | |
| `WaylandAfterUinputShiftSelectWaitMs` | 5 | |
| `SurroundingWaitMs` | 3 | Sau xóa surrounding |
| `SurroundingProbeTimeoutMs` | 80 | Hidden; lần đầu dùng 150 |
| `SurroundingDeleteDelayMs` | 10 | V2, giữa các lệnh xóa 1 ký tự |
| `WaylandSurroundingDeleteDelayMs` | 0 | V2 trên Wayland |
| `AfterSurroundingDeleteWaitMs` | 1 | |
| `XTestBackspaceDelayMs` | 1 | Native |
| `PostCommitDelayMs` | 20 | Barrier sau commit |
| `PreciseTiming` | `true` | Timer 1 µs |
| `UseSurroundingV2InsteadOfSurroundingV1` | `false` | |
| `UseUinputInsteadOfForwardKey` | `true` | |
| `UseXTestInsteadOfUinput` | `false` | |
| `EnableMouseTracking` | `true` | |
| `ForwardFirstCharacter` | `true` | |

### 12.4 Preset "Best performance"

`applyBestPerformancePreset()` hạ các wait rewrite xuống 3–5 ms và surrounding
xuống 1 ms. Chỉ nên dùng khi máy nhanh và app hợp tác; nếu gõ nhanh bị mất chữ,
tăng lại `AfterBackspaceWaitMs` trước.

---

## 13. Build, cài, đóng gói

### 13.1 Dependency

```bash
# Ubuntu/Debian
sudo apt install build-essential cmake ninja-build extra-cmake-modules golang \
  libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev libfcitx5config-dev \
  libdbus-1-dev libinput-dev libudev-dev libx11-dev libxtst-dev \
  libsdl3-dev libfontconfig1-dev
```

CMake bắt buộc: `Fcitx5Core`, `Fcitx5Config`, `Fcitx5Utils`, `dbus-1`, `libinput`,
`libudev`, `x11`, `xtst`. Tùy chọn: `libei-1.0 >= 1.0` và `libportal >= 0.7.1`
(bật native Wayland backend nếu Fcitx5 >= 5.1.11), `SDL3` (settings app),
`fontconfig`.

### 13.2 Build + test

```bash
git clone --recurse-submodules https://github.com/xhkzeroone/ArecaIME.git
cd ArecaIME
cmake -S . -B build -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX=/usr
cmake --build build --parallel
ctest --test-dir build --output-on-failure
sudo cmake --install build
sudo udevadm control --reload-rules
sudo udevadm trigger
fcitx5 -rd
```

### 13.3 uinput

```bash
sudo modprobe uinput
echo uinput | sudo tee /etc/modules-load.d/areca-uinput.conf

sudo udevadm control --reload-rules
sudo udevadm trigger

sudo usermod -aG input "$USER"   # đăng xuất/đăng nhập lại
test -w /dev/uinput && echo OK
```

Nếu không muốn cấp quyền `input`, bỏ qua — Areca tự fallback.

### 13.4 Đóng gói

- `install.sh` tự nhận distro/version và tải asset từ GitHub release.
- Hỗ trợ Ubuntu 24.04/25.10/26.04, Mint 22, Pop!_OS 24.04, Zorin 17,
  Fedora 42–44, Nobara, Bazzite, openSUSE Tumbleweed, Arch/CachyOS/Manjaro/
  EndeavourOS/Garuda.
- AUR: `fcitx5-areca-bin`, `fcitx5-areca-git`.
- CI build matrix + `.deb`/`.rpm`/pacman; bundle SDL3 với `$ORIGIN` rpath cho
  Ubuntu 24.04.
- udev rules: `99-uinput-areca.rules`, `70-areca-pointer.rules`.

---

## 14. Debug: playbook thực dụng

### 14.1 Bật log

```ini
# areca-advanced.conf
Debug=True
```

```bash
fcitx5 -rd
journalctl --user -f | grep --line-buffered -i areca
```

### 14.2 Kiểm tra addon đã nạp

```bash
pgrep -a fcitx5
fcitx5-diagnose
# addon cũ còn sót?
grep -i areca /proc/$(pgrep -x fcitx5)/maps
```

Nếu thấy đường dẫn có `(deleted)`, Fcitx5 vẫn đang giữ binary cũ. Restart hẳn
(`fcitx5 -rd`) hoặc logout.

### 14.3 Phân biệt lỗi theo log

| Log | Nghĩa | Hướng xử lý |
|---|---|---|
| `rewrite select backend=...` | Backend được chọn | Kiểm tra app có đúng nhóm không |
| `surrounding probe timeout` | App không ACK | Fallback; thêm app vào override |
| `reset rejected` | Reset đến trong transaction/20 ms | Bình thường nếu không mất chữ |
| `rewrite backend failed` | Backend lỗi | `stalled_=true`; restart Fcitx5 |
| `uinput` + `failed` | Không mở được `/dev/uinput` | Kiểm tra module/quyền |
| `mouse reset skipped` | Click đến khi đang composition | Có thể là bảo vệ đúng |

### 14.4 Cấu hình cũ che default mới

Nếu vừa update mà hành vi không đổi: kiểm tra `areca.conf` có field timing cũ
không. File advanced load sau và ưu tiên; field cũ trong `areca.conf` chỉ để
migrate. Xóa field cũ hoặc sửa qua UI.

### 14.5 Kiểm tra nhanh bằng tay

```bash
# 1. Addon có trong fcitx5?
fcitx5-remote -o   # bật
# 2. Gõ thử trong editor đơn giản
gedit &            # hoặc kate, mousepad
# 3. Nếu gedit OK mà Chrome không OK → lỗi frontend/backend, không phải engine
```

Quy tắc: **cô lập theo tầng**. Engine sai thì mọi app đều sai. Nếu chỉ một app
sai, đó là adapter/backend.

---

## 15. Test: 22 CTest và `TestInputContext`

### 15.1 Triết lý

Areca test **hành vi adapter**, không chỉ engine. Đây là điều nhiều dự án IME
thiếu: họ test `transform("tieengs") == "tiếng"` rồi tưởng đã xong.

### 15.2 Fake InputContext

`TestInputContext` ghi lại:

- `commitStringImpl()`
- `deleteSurroundingTextImpl()`
- `forwardKeyImpl()`
- `updatePreeditImpl()`
- `surroundingText().setText(text, cursor, anchor)`
- `setCapabilityFlags(...)`

Nhờ vậy có thể chạy CTest headless, không cần GUI, không cần app thật.

### 15.3 Các nhóm test

| Test | Kiểm tra |
|---|---|
| `adaptive_wait_test` | Tăng/giảm delay, ngưỡng lag |
| `app_mode_routing_test` | Chọn mode theo app |
| `bamboo_engine_adapter_test` | Diff, charset, macro |
| `browser_autocomplete_test` | Nhận diện autocomplete |
| `event_loop_post_test` | Commit ở post phase |
| `forward_backspace_probe_test` | Fallback khi forward |
| `idle_key_forward_test` | Phím đầu đi tiếp |
| `input_type_detector_test` | Terminal/browser/URL |
| `mouse_click_tracker_test` | Pipe protocol, reset |
| `preedit_logic_test` | Composition, backspace |
| `preedit_surrounding_restore_test` | Khôi phục từ đã commit |
| `program_compatibility_test` | Match tên app |
| `rewrite_feature_toggle_test` | Bật/tắt tính năng |
| `sentence_capitalizer_test` | Auto-cap |
| `surrounding_text_probe_test` | Probe/ACK/timeout |
| `surrounding_text_restore_test` | Restore state |
| `surrounding_text_v2_backend_test` | Xóa từng ký tự |
| `uinput_backspace_backend_test` | ACK, forward, timeout |
| `uinput_shift_select_backend_test` | Shift+Select |
| `word_segment_test` | Tách từ |
| `xtest_backspace_backend_test` | XTest |
| `xtest_shift_select_backend_test` | XTest selection |

### 15.4 Chạy một test

```bash
ctest --test-dir build -R surrounding_text_probe --output-on-failure
```

### 15.5 Điểm yếu của test hiện tại

- Dùng `assert` + `-UNDEBUG`, không framework → dừng ở lỗi đầu tiên, không có
  report đẹp.
- Chưa có test end-to-end với app thật trong CI (chỉ unit/integration).
- Chưa có fuzzing cho parser/charset.

Nếu anh muốn đóng góp có giá trị cao, thêm một harness ghi lại **scenario app
thật** (serialize InputContext callbacks) sẽ hữu ích hơn thêm test unit.

---

## 16. Đọc source theo thứ tự

Nếu mở repo lần đầu, đừng đọc `areca_engine.cpp` (2.066 dòng). Thứ tự gợi ý:

| Bước | File | Vì sao |
|---:|---|---|
| 1 | `src/types.h` | `BambooResult`, `RewritePlan`, enum |
| 2 | `src/rewrite_backend.h` | Hợp đồng backend |
| 3 | `src/key_queue.h` | FIFO, watch reference |
| 4 | `src/input_scheduler.{h,cpp}` | Trái tim của adapter |
| 5 | `src/bamboo_engine_adapter.cpp` | Diff + bridge |
| 6 | `src/surrounding_text_backend.cpp` | Backend phổ biến nhất |
| 7 | `src/surrounding_probe_match.h` | Logic trust probe |
| 8 | `src/rewrite_mode.cpp` | Policy key/Backspace/macro |
| 9 | `src/preedit_mode.cpp` | Preedit + finalized word |
| 10 | `src/areca_engine.cpp` | Routing tổng |
| 11 | `src/program_compatibility.cpp` | Danh sách app |
| 12 | `settings/` | UI (không cần cho hiểu engine) |

### 16.1 Cảnh báo: tài liệu và code lệch nhau

Đây là dự án đang tiến hoá nhanh; đừng tin tài liệu tuyệt đối. Ba chỗ tôi đã
kiểm tra và thấy lệch tại snapshot:

| Tài liệu nói | Code thực tế |
|---|---|
| `SwitchModeKey` mặc định `Alt+Space` | Default rỗng; nhánh xử lý bị comment trong `keyEvent` |
| `UinputShiftSelectAckTimeoutMs` 20 ms (spec) | Default **150 ms** trong `SchedulerTiming`/`types.h` |
| `project(... VERSION 0.1.0)` | Release tag `v8.3.0` |

Quy tắc: khi debugging, **đọc code trước, docs sau**. Khi viết PR, sửa docs cùng
lúc với code — đây là nợ kỹ thuật dễ trả nhất của Areca.

---

## 17. Điểm mạnh, điểm yếu, giới hạn

### 17.1 Điểm mạnh

1. **Adapter production-grade** — thứ khó nhất của IME Linux được làm tới nơi.
2. **Fail-closed** — backend lỗi thì dừng, không hỏng dữ liệu.
3. **Trust probe** — không tin capability, chờ bằng chứng.
4. **Fallback chain đầy đủ** — hầu như app nào cũng gõ được bằng cách này hay cách khác.
5. **Test adapter** — 22 target, có fake InputContext.
6. **Đóng gói nghiêm túc** — đa distro, AUR, udev, CI matrix.
7. **Tài liệu kiến trúc tốt** — ARCHITECTURE.md + spec + debugging.

### 17.2 Điểm yếu / rủi ro

1. **Phụ thuộc bamboo-core** — engine không phải của Areca; bug engine phải chờ upstream.
2. **Go bridge** — thêm một runtime, một lớp FFI, khó debug hơn C++ thuần.
3. **Phức tạp cao** — 6 backend, nhiều timing, nhiều state; người mới khó tiếp cận.
4. **Compatibility list dài** — bảo trì nặng, dễ sai khi app đổi tên.
5. **Heuristic address bar bằng `cursorRect`** — rất dễ vỡ.
6. **AdaptiveWait toàn cục** — một app chậm có thể làm mọi app chậm thêm.
7. **uinput mặc định bật** — cần quyền, có thể tạo duplicate event trên máy lạ.
8. **Chưa có test app thật trong CI** — nhiều hành vi chỉ biết qua báo lỗi.
9. **Một maintainer** — bus factor thấp.
10. **Chưa có fuzzing/benchmark công khai** — chất lượng dựa nhiều vào test hồi quy
    viết tay; chưa có số đo latency/CPU chính thức.

### 17.3 Giới hạn cần biết trước khi dùng

- Chỉ Linux + Fcitx5. Không IBus, không macOS/Windows.
- Rewrite mặc định có thể gây lỗi trên app lạ; chuyển Preedit cho app đó.
- uinput cần quyền; nếu không có, một số app vẫn có thể nuốt chữ.
- Một số app (Discord/Signal cũ, LibreOffice, Google Sheets) cần override tay.

---

## 18. Nếu anh muốn đóng góp

### 18.1 Quy trình

1. Đọc `CONTRIBUTING.md`, `AGENTS.md` (nếu có), `docs/ARCHITECTURE.md`.
2. Chọn issue nhỏ; đừng bắt đầu bằng việc thêm backend mới.
3. Build + chạy full `ctest` trước khi sửa.
4. Sửa + thêm test **hành vi** (không chỉ test hàm).
5. Chạy `clang-format`/`clang-tidy` theo `settings/.clang-format`.
6. PR mô tả: app nào, frontend nào, trước/sau, log.

### 18.2 Việc đáng làm

- Thêm test cho app thật (serialize callback).
- Giảm phụ thuộc `cursorRect`.
- Scope AdaptiveWait theo app/backend.
- Thêm backend fallback cho app mới.
- Cải thiện docs cho người dùng cuối.

### 18.3 Việc không nên làm

- Thêm app vào compatibility list mà không có log.
- Tăng delay mặc định để "che" lỗi.
- Copy code từ dự án GPL (Areca MIT, nhưng đừng kéo GPL vào).
- Refactor lớn trước khi có test.

---

## 19. Glossary

| Thuật ngữ | Nghĩa |
|---|---|
| **Rewrite** | Gõ trực tiếp, sửa bằng xóa/chèn |
| **Preedit** | Vùng soạn thảo có gạch chân |
| **Redirect** | Forward phím gốc |
| **Surrounding text** | API cho IME đọc text quanh caret |
| **Transaction** | Một lượt xóa + chèn của backend |
| **Single-flight** | Chỉ một transaction tại một thời điểm |
| **Barrier** | Khoảng chờ sau commit trước khi xử lý key tiếp |
| **Trust probe** | Thử xóa và chờ ACK để đánh giá app |
| **ACK** | Tín hiệu app đã xử lý lệnh |
| **AdaptiveWait** | Tự tăng/giảm delay theo lag |
| **Finalized word** | Từ đã commit nhưng còn khả năng sửa |
| **Protected reset** | Reset bị hoãn để không mất state |
| **Idle key** | Phím đầu khi engine rỗng, có thể forward |

---

## 20. Nguồn

### Trong repo Areca

- `README.md`, `docs/ARCHITECTURE.md`, `docs/ARECA_ARCHITECTURE_SPECIFICATION.md`
- `docs/ADAPTIVE_DELAYS.md`, `docs/DEBUGGING.md`
- `src/areca_engine.cpp`, `src/input_scheduler.cpp`, `src/key_queue.h`
- `src/bamboo_engine_adapter.cpp`, `bamboo-bridge/main.go`
- `src/surrounding_text_backend.cpp`, `src/surrounding_text_v2_backend.cpp`,
  `src/surrounding_probe_match.h`, `src/surrounding_text_cache.cpp`
- `src/forward_backspace_backend.cpp`, `src/uinput_backspace_backend.cpp`,
  `src/uinput_shift_select_backend.cpp`, `src/native_device.cpp`, `src/xtest_*`
- `src/adaptive_wait.h`, `src/adaptive_wait_monitor.cpp`
- `src/rewrite_mode.cpp`, `src/preedit_mode.cpp`, `src/preedit_logic.cpp`
- `src/input_type_detector.cpp`, `src/program_compatibility.cpp`,
  `src/browser_autocomplete.cpp`, `src/window_focus_tracker.cpp`
- `src/sentence_capitalizer.cpp`, `src/word_segment.cpp`
- `settings/`, `install.sh`, `packaging/`, `.github/workflows/package-linux.yml`

### Ngoài repo

- [bamboo-core](https://github.com/BambooEngine/bamboo-core) — engine MIT
- [Fcitx5 documentation](https://fcitx-im.org/wiki/Develop)
- [Using Fcitx5 on Wayland](https://fcitx-im.org/wiki/Using_Fcitx_5_on_Wayland)
- text-input-v3 protocol, libei/libportal, AT-SPI

### Ghi chú về số liệu

Số dòng/file/test lấy từ commit `952ddd8`. Stars/issue thay đổi theo thời gian;
xem `docs/ime-survey/README.md` cho snapshot khảo sát.

---

## Phụ lục A — Cheat sheet (một trang)

### Bất biến

1. FIFO — key giữ thứ tự.
2. Single-flight — một key xử lý tại một thời điểm.
3. Transaction barrier — backend đang sửa thì key mới chờ.
4. Không sleep trên main thread — chỉ timer event loop.
5. Fail closed — backend lỗi thì dừng, không đoán.
6. Commit trước reset — không xóa state khi còn composition.

### Sáu backend

| Backend | Một dòng |
|---|---|
| `SurroundingTextBackend` | `deleteSurroundingText(-N,N)` + commit |
| `SurroundingTextV2Backend` | Xóa từng ký tự, chậm mà chắc |
| `ForwardBackspaceBackend` | `forwardKey(Backspace)` × N |
| `UinputBackspaceBackend` | `KEY_BACKSPACE` qua `/dev/uinput` + ACK |
| `UinputShiftSelectBackend` | Shift+Left × N+1 rồi commit |
| `NativeXTestBackend` | Libei (Wayland) / XTest (X11) |

### Timing mặc định

| Thông số | Giá trị |
|---|---:|
| Backspace giữa các phím | 1 ms |
| Sau Backspace cuối (Wayland) | 3 ms |
| Sau Backspace cuối (khác) | 10 ms |
| Sau xóa surrounding | 3 ms |
| Sau commit | 20 ms |
| Probe surrounding (lần đầu / retry) | 150 / 80 ms |
| uinput ACK timeout | 150 ms |

### Lệnh

```bash
# build + test
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel
ctest --test-dir build --output-on-failure

# cài + reload
sudo cmake --install build
sudo udevadm control --reload-rules && sudo udevadm trigger
fcitx5 -rd

# log
journalctl --user -f | grep --line-buffered -i areca

# kiểm tra addon
grep -i areca /proc/$(pgrep -x fcitx5)/maps
```

### Sáu file nên đọc

1. `src/types.h`
2. `src/input_scheduler.cpp`
3. `src/bamboo_engine_adapter.cpp`
4. `src/surrounding_text_backend.cpp`
5. `src/rewrite_mode.cpp`
6. `src/preedit_mode.cpp`

### Năm lỗi thường gặp

| Triệu chứng | Nguyên nhân thường gặp | Xử lý |
|---|---|---|
| Mất chữ khi gõ nhanh | Barrier/timing quá ngắn | Tăng `PostCommitDelayMs`, kiểm tra `stalled_` |
| Chữ lặp | Commit hai lần / forward key | Kiểm tra backend + `filterAndAccept` |
| Xóa sai vị trí | Snapshot surrounding cũ | Probe/verify trước khi xóa |
| Không gõ được trong terminal | Preedit/forward không tương thích | Chuyển app override sang Backspace/Preedit |
| Addon không nạp | Binary cũ `(deleted)` | Restart Fcitx5, kiểm tra `/proc/<pid>/maps` |
