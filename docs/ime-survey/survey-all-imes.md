# Khảo sát 31 bộ gõ tiếng Việt

> Snapshot 2026-11-01. Nguồn: `hthienloc/vietnamese-ime-tracker` (repos.yml +
> bo-go-tieng-viet.csv), GitHub API, README và mã nguồn từng repo.
> Metadata (stars, license, ngày) lấy tự động; tính năng lấy từ README/source.
> Chỗ nào chỉ suy ra từ cấu trúc sẽ ghi "có vẻ"/"theo cấu trúc".
>
> Quy ước: ✅ có · ◐ một phần · — không · 🔌 thuộc adapter/OS.

---

## 1. Phương pháp và giới hạn

**Đã làm:**

- Đối chiếu 36 dòng dữ liệu tracker → 31 repository unique.
- Lấy metadata GitHub API cho 28 repo còn tồn tại (3 repo đã bị xóa: 404).
- Đọc toàn bộ README của 28 repo.
- Đọc mã nguồn/kiến trúc sâu cho các repo lớn: Areca, fcitx5-lotus, skey,
  fcitx5-bamboo, fcitx5-unikey, Gõ Nhanh, telebit, Ngó Sen.
- Đối chiếu chéo các tuyên bố trong README với cấu trúc source khi có thể.

**Chưa làm:**

- Chưa build/chạy tất cả trên máy thật. Các nhận định về "mượt", "ổn định",
  "chính xác" chỉ dựa trên kiến trúc, test và báo cáo, không phải benchmark.
- Chưa test từng app (Chrome, LibreOffice, terminal...) cho từng dự án.
- Stars/issue thay đổi; số liệu chỉ đúng tại snapshot.

**Cách đọc "trạng thái":**

- `Active`: repo còn phát triển (commit gần nhất trong ~3 tháng).
- `Archived`: repo archive/tombstone, không nhận thay đổi.
- `Inactive`: 3–6 tháng không commit.
- `Dead`: >6 tháng, hoặc repo đã bị xóa (404).

Lưu ý: tracker dùng ngưỡng commit để gắn nhãn, nên một repo archive vẫn có thể
hiện "Active". Bộ dữ liệu ở đây phân loại lại theo trạng thái thực tế.

---

## 2. Bức tranh tổng

### 2.1 Số liệu

| Chỉ số | Giá trị |
|---|---|
| Repository unique | 31 |
| Dòng dữ liệu (đa nền tảng) | 36 |
| Active / Archived / Inactive / Dead (repo) | 24 / 3 / 1 / 3 |
| Tổng stars (unique) | ~4.700 |
| Trung vị tuổi repo | phần lớn dưới 1 năm; 27/36 dòng tạo năm 2026 |
| Fcitx5 (dòng) | 16 |
| IBus (dòng) | 5 |
| Standalone Wayland (dòng) | 5 |
| Fcitx5 + IBus (dòng) | 4 |
| macOS native (dòng) | 3 |
| Windows native (dòng) | 3 |

Ngôn ngữ chính (dòng): C++ 12, Rust 12, Go 3, Makefile 3, Swift 3, Kotlin 1,
C# 1, HTML 1.

Giấy phép (dòng, theo cột tracker): GPL-3.0 18, MIT 9, BSD-3-Clause 3, không rõ 3,
0BSD 1, MPL-2.0 1, GPL-2.0 1. GitHub API có thể báo khác (ví dụ `fcitx5-bamboo`
ghi LGPL-2.1+ trong README nhưng API không thấy license).

### 2.2 Ba xu hướng

**Xu hướng 1 — Bùng nổ Fcitx5 sau Wayland.**
16/36 dòng là Fcitx5 addon. Trước 2025 chỉ có `fcitx5-unikey` và
`fcitx5-bamboo`; từ 2026 mọc thêm Lotus, Areca, VMK, SKey, telebit, Clak,
lilypad, Ngó Sen, CanType, pinakey... Lý do: Fcitx5 ổn định, tài liệu addon tốt,
và Wayland làm IBus/XIM lộ nhiều lỗi.

**Xu hướng 2 — Rust thay C++ ở lớp engine.**
12 repo Rust, phần lớn ra đời 2026. Nhưng đa số là engine nhỏ + adapter mỏng;
các dự án lâu năm (OpenKey, ibus-bamboo, fcitx5-unikey, Lotus) vẫn C++/Go.
Rust thắng ở FFI an toàn, cross-platform, nhưng chưa thắng ở số tính năng.

**Xu hướng 3 — Tách engine khỏi adapter.**
Các dự án mới đều có `core`/`engine` crate riêng + adapter mỏng:
Gõ Nhanh, Funput, TextVN, vnkey, CanType, pinakey, skey, telebit, Areca.
Đây là bài học chung của cả hệ sinh thái, không riêng ai.

### 2.3 Nghịch lý "nhiều dự án, ít test"

- 31 repo nhưng chỉ khoảng **8–10** có test tự động thực sự chạy trong CI.
- Chỉ **khoảng 4–5** có test adapter (giả lập InputContext): Areca, fcitx5-lotus,
  fcitx5-unikey (TestFrontend), ViFastKey, có thể thêm Ngó Sen.
- Phần lớn test engine (transform chuỗi), không test hành vi với app.
- Đây là lý do cùng một lỗi "mất chữ trên Chrome" xuất hiện lặp lại ở nhiều
  dự án qua nhiều năm.

---

## 3. Taxonomy: tích hợp ở đâu?

```text
┌─ Fcitx5 addon ─────────────────────────────────────────────┐
│ Engine nhúng trong addon, Fcitx5 gọi keyEvent.              │
│ Ưu: chuẩn, có surrounding text, preedit, status area.       │
│ Nhược: chỉ Linux; hành vi phụ thuộc frontend.               │
├─ IBus engine ──────────────────────────────────────────────┤
│ Daemon riêng nói D-Bus với IBus.                            │
│ Ưu: mặc định trên GNOME/Ubuntu.                             │
│ Nhược: Wayland hạn chế, surrounding text kém.               │
├─ Standalone Wayland ───────────────────────────────────────┤
│ Bắt evdev / dùng zwp_input_method_v2 / virtual keyboard.    │
│ Ưu: không cần Fcitx/IBus, kiểm soát hoàn toàn.              │
│ Nhược: cần quyền, dễ xung đột, phụ thuộc compositor.        │
├─ Native desktop ───────────────────────────────────────────┤
│ macOS IMK/CGEventTap, Windows TSF/hook.                     │
│ Ưu: tích hợp sâu, UX tốt.                                   │
│ Nhược: mỗi OS một codebase, signing/notarize.               │
└─ Cross-platform core + nhiều adapter ──────────────────────┘
```

Điểm quan trọng: **cùng một engine có thể xuất hiện ở nhiều nhóm**. Ví dụ
`bamboo-core` có mặt trong ibus-bamboo, fcitx5-bamboo, Areca, NepKey, và gián
tiếp trong Lotus (C++ port). Vì vậy đánh giá "bộ gõ" phải tách **engine** khỏi
**adapter**.

---

## 4. Nhóm Fcitx5

### 4.1 fcitx5-unikey — chuẩn mực cũ

- **Repo:** `fcitx/fcitx5-unikey` · C++ · GPL-2.0+ (README) · 132★ · 2019.
- **Là gì:** addon chính thức của Fcitx, dùng `unikey-lib` port từ x-unikey
  `ukengine` (inputproc, charset, mactab, ukengine, usrkeymap).
- **Tính năng:** Telex, VNI, VIQR, Microsoft Vietnamese, UserIM, Simple Telex,
  Simple Telex 2; 8 bảng mã (Unicode, TCVN3, VNI Win, VIQR, BK HCM 2, CString,
  NCR dec/hex); spell check; macro; `Process W at word begin`; auto-restore;
  modern style; free marking; preedit style None/Underline/Highlight;
  **experimental modify surrounding text**.
- **Đáng học:** test `TestFrontend` headless với bảng Telex kỳ vọng lớn; tách
  subconfig macro/keymap (`fcitx://config/addon/unikey/...`); editor tự reload.
- **Hạn chế:** engine và UX cũ; preedit-centric; surrounding edit chỉ thử nghiệm;
  không app profile, không timing; README 9 dòng.
- **Độ chín:** cao về tuổi đời và tích hợp Fcitx; thấp về tính năng hiện đại.

**Vì sao vẫn quan trọng:** đây là baseline mà mọi người dùng Linux so sánh. Nếu
bộ gõ mới không bằng fcitx5-unikey ở Telex/VNI cơ bản, đừng bàn chuyện khác.

### 4.2 fcitx5-bamboo — engine Go trong addon C++

- **Repo:** `fcitx/fcitx5-bamboo` · C++ + Go · LGPL-2.1+ (README) · 76★ · 2022.
- **Kiến trúc:** `bamboo-core` build `-buildmode=c-archive`, cgo `Handle` bọc
  trong RAII `CGoObject`; `BambooState` theo từng input context; dictionary
  `vietnamese.cm.dict` nạp vào Go map.
- **Tính năng:** Telex, VNI, VIQR, Custom keymap; Unicode + 16 bảng mã legacy
  (TCVN3, VNI, VIQR, VISCII, VPS, BKHCM, Vietware, CP1258, NCR...); spell check
  bằng từ điển; macro theo từng input method; `autoNonVnRestore`, `modernStyle`,
  `freeMarking`, `restoreKeyStroke`; preedit có gạch chân.
- **Đáng học:** macro file riêng `conf/bamboo-macro-<im>.conf`; status area cho
  input method/charset/spell/macro; option list lấy động từ Go.
- **Hạn chế:** chỉ preedit; không direct commit/surrounding/app profile/timing;
  Go runtime trong Fcitx5; `ddFreeStyle` bị ép true; addon không có test target.
- **Độ chín:** ổn định, chính thức, nhưng đứng yên về tính năng.

**Bài học:** đây là ví dụ sạch của "engine Go + adapter C++". Areca học mô hình
này nhưng thêm scheduler/backend.

### 4.3 fcitx5-lotus — nhiều mode nhất

- **Repo:** `LotusInputMethod/fcitx5-lotus` · C++20 · GPL-3.0 · 522★ · 2026-02.
- **Nguồn gốc:** **fork tối ưu từ VMK**, không phải từ fcitx5-bamboo. Engine
  `lotus/` là **bản port C++ của bamboo-core** (engine, composition,
  key_processor, spelling, macro_table, encoder, rules_parser, flattener,
  rebuild, dictionary).
- **7 mode:** OFF, Uinput (Smooth), Uinput (Super Smooth), Uinput (Select),
  Surrounding Text, Preedit, Emoji Picker.
- **Tính năng:** 8 kiểu gõ (Telex, VNI, Telex+VNI, VIQR, Microsoft, VNI French,
  Custom...); 17 bảng mã; dictionary spell check; macro `$TIME/$DATE`;
  auto-capitalize; double-space → period; double-hyphen → em dash; custom
  keymap; emoji history; **app rules** đặt mode + delay commit/backspace/post
  theo app.
- **Đáng học:** uinput server tách process, xác thực peer bằng `SO_PEERCRED`
  (UID + executable allowlist) trên abstract `SOCK_SEQPACKET`; 57 test headless
  gồm `app-rule-delay-wire`, `dbus-deferred-commit`,
  `focus-out-mid-replacement`, `chromium-address-bar`, golden files.
- **Hạn chế:** uinput/udev/ACL/setfacl phức tạp, bề mặt bảo mật rộng;
  `BUILD_TESTING` mặc định off; README mỏng, docs ở site ngoài.
- **Độ chín:** cao nhất trong nhóm Fcitx5 mới (522★, 22 contributor, CI nặng).

**So với Areca:** Lotus nhiều mode và nhiều tính năng người dùng hơn; Areca đi
sâu hơn vào trust probe và tính đúng đắn của từng transaction. Hai triết lý khác
nhau: Lotus tối ưu trải nghiệm, Areca tối ưu tính đúng.

### 4.4 Areca IME — adapter production-grade

- **Repo:** `xhkzeroone/ArecaIME` · C++20 + Go bridge · MIT · 20★ · 2026-07.
- **Kiến trúc:** 3 mode Rewrite/Preedit/Redirect; `KeyQueue` + `InputScheduler`
  (FIFO, single-flight, transaction barrier, post-commit 20 ms); 6 backend
  (Surrounding v1/v2, ForwardBackspace, UinputBackspace, UinputShiftSelect,
  NativeXTest); trust probe surrounding 150/80 ms; adaptive wait; AT-SPI;
  mouse tracker qua process libinput riêng.
- **Tính năng:** Telex/VNI/VIQR qua bamboo-core; spell check 3 mức; macro +
  capitalize + bypass modifier + revert; auto-cap; restore word từ surrounding;
  app override; 5 tab settings SDL3/ImGui, 9 theme.
- **Đáng học:** xem [`areca-ime.md`](areca-ime.md) — tài liệu riêng.
- **Hạn chế:** chỉ Fcitx5; phụ thuộc bamboo-core; phức tạp; compatibility list
  dài; heuristic address bar dễ vỡ; một maintainer.
- **Độ chín:** adapter chín nhất trong nhóm; engine không phải của họ.

### 4.5 VMK — tổ tiên của Lotus

- **Repo:** `thanhpy2009/VMK` · C++ · GPL-3.0 · 102★ · 2026-01 · alpha 0.9.31.
- **Kiến trúc:** addon `vmk.so` + `fcitx5_uinput_server` riêng; FLTK `sconfig`;
  dựa trên `fcitx5-bamboo` + `bamboo-core`.
- **4 mode:** VMK1 (Backspace ảo + delay), VMK1HC (state trong `/tmp` để sống
  qua XIM disconnect; cho IDE/Wine), VMK2 (Surrounding Text), VMK-Pre (preedit).
- **Tính năng:** Telex, Telex W, VNI; Unicode/UTF-8/VNI Windows/TCVN3/Unicode
  tổ hợp; macro; fix Chrome/Gemini/Chrome-X11.
- **Đáng học:** delay rõ ràng; fix race E-core; fix leak 100% CPU; chuyển từ
  socket world-writable (`666`) sang D-Bus.
- **Hạn chế:** alpha; không `git clone` đầy đủ (src-full cần `bamboo/` ngoài);
  không CI/test; README lộn xộn; từng có vấn đề socket world-writable (đã chuyển
  sang D-Bus).
- **Độ chín:** thấp, nhưng là nguồn gốc của Lotus và lilypad.

**Bài học:** VMK chứng minh nhu cầu "không gạch chân" rất lớn (102★ dù alpha).
Nhưng thiếu kỷ luật test/CI khiến nó bị fork và vượt qua.

### 4.6 SKey — 5 output mode + Qt6

- **Repo:** `collyn/skey` · C++17 + Rust · GPL-3.0 · 29★ · 2026-06.
- **Kiến trúc:** addon C++17 + `skey-engine` Rust **stateless** (~700 LOC,
  10 hàm FFI, `input string → output string`); CMake clone engine pin v0.2.2.
- **5 mode:** Auto, Surrounding Text, Uinput (in-process `/dev/uinput`), Native
  (XTest/libei+portal, mặc định tắt), Preedit.
- **Tính năng:** Telex/VNI (engine có VIQR); `w→ư`, `][→ươ`, free marking;
  14 bảng mã gồm NFD/TCVN3/VNI/VIQR; auto-restore với từ điển ~7.9k từ + user
  words; macro; per-app mode/disable/delay; profile riêng cho address bar
  Chromium; menu mode bằng backtick.
- **Đáng học:** engine stateless → FFI đơn giản, addon giữ buffer; ACL uinput
  qua `skey-uinput-access configure`; setup test trên 13 desktop.
- **Hạn chế:** engine stateless nghĩa là addon phải giữ state → dễ lệch nếu
  adapter phức tạp; Native cần portal; Auto phụ thuộc app/compositor; build cần
  mạng.
- **Độ chín:** cao; PPA + Fedora/openSUSE/Arch + NixOS, 8 file test/CTest.

**So với Areca:** SKey gọn hơn, nhiều mode tương đương, GUI Qt6 thay vì ImGui.
Areca có trust probe và scheduler chặt hơn; SKey có engine stateless dễ thay.

### 4.7 telebit — engine C++ độc lập + doctor

- **Repo:** `sonnam0904/telebit` · C++17 · MIT · 8★ · 2026-03 · v2.15.1.
- **Kiến trúc:** `vietnamese.*` + `EngineVietCpp` buffer theo phím;
  `canonicalize` (escape, tách initial/vần, chuẩn hóa `w`/`aa`/`ee`/`oo`, tách
  thanh); `rime_table` bảng nguyên âm + vị trí main vowel; `render_utf8`;
  `ai_client` libcurl; `tests.cpp`.
- **Tính năng:** Telex/VNI; Unicode; spell restore; `DirectCommitRollback`
  (direct commit, fallback preedit); VNI số `6/7/8/9`; modern tone; auto-cap;
  macro; AI assistant.
- **Đáng học:** **rebuild âm tiết** (initial + rime + tone) rồi validate, thay
  vì thay thế cơ học; `telebit doctor [--deep|--markdown]` với exit code
  0/1/2; AI key chỉ đọc từ env, không ghi config.
- **Hạn chế:** không app profile; direct-mode undo/redo khác; AI gửi text ra
  ngoài; base người dùng nhỏ.
- **Độ chín:** C++ test + APT repo 5 suite + Fedora RPM + docs site.

**Bài học:** telebit là một trong số ít dự án có "doctor" và xử lý rollback
nghiêm túc. Rất đáng đọc `canonicalize` nếu anh quan tâm thuật toán.

### 4.8 Clak IME — không preedit, nhiều kênh

- **Repo:** `versenilvis/ClakIME` · C++ + Rust · 0BSD · 2★ · 2026-10 · v0.7.7.
- **Kiến trúc:** addon C++ + `clak-engine` Rust staticlib qua C FFI; GUI Slint;
  CLI/diagnostics Rust.
- **Tính năng:** Telex, VNI, VIQR, Teip-VNI; NFC/NFD/VNI Win/TCVN3/CP1258/VIQR;
  spell/dictionary; macro; auto-cap; double-space period; auto-restore; modern
  tone. **Không preedit**; `ClakAction` = FORWARD / COMMIT / REPLACE /
  REPLACE_SURROUNDING / ADDRESS_BAR_FIX.
- **Đáng học:** routing theo app (`terminal.h`, `browser.h`, `steam.h`,
  `sites.h`); safety timer 50 ms; adaptive RTT pacing/calibration;
  `surrounding_verifier` phát hiện DOM desync; bypass Vim/Helix NORMAL qua
  Hyprland IPC.
- **Hạn chế:** rất mới; docs tự nhận AI-generated; heuristic app hardcode; phụ
  thuộc uinput/compositor; pre-1.0.
- **Độ chín:** thấp–trung; CI + Nix + packaging; ~114 Rust + 17 C++ + 4 E2E.

**Bài học:** ý tưởng "verify surrounding bằng DOM desync" và "RTT calibration"
là hướng đáng thử, nhưng hardcode app là nợ kỹ thuật.

### 4.9 fcitx5-lilypad — đo thời gian nuốt phím

- **Repo:** `chiconcota/fcitx5-lilypad` · C++ + Bamboo Go · GPL-3.0 · 6★ ·
  2026-08 · v2.3.6.
- **Kiến trúc:** addon C++ + Bamboo Go qua C FFI + `fcitx5-lilypad-server`
  uinput daemon (Unix socket, `SO_PEERCRED` UID auth) + Python/PyQt settings.
- **9 mode:** Off, Smooth, SuperSmooth, Uinput, SurroundingText, Preedit, Emoji,
  Minecraft, Sequence.
- **Đáng học:** đo `ΔT_swallow`; `per_bs_us = max(min, EMA, measured)`;
  N+1 sentinel Backspace; 70 ms post-commit settling; 250 ms watchdog
  `purgeContextEmergency`; IKI 35–150 ms; `ack_apps`; libinput mouse.
- **Hạn chế:** chỉ AUR; chỉ test Niri; 6★; một test Python; heuristic phức tạp;
  root daemon; README có link `file:///home/...`.
- **Độ chín:** thấp; nhưng ý tưởng đo thời gian thực rất đáng học.

### 4.10 Ngó Sen — "cứu chữ", "cứu dấu"

- **Repo:** `ngosen/ngosen` · C++17 + Rust `bamboo-rs` · GPL-3.0 · 51★ · 2026-10.
- **Kiến trúc:** addon Fcitx5 + engine Rust port bamboo-core; Qt + GTK settings;
  **không có uinput daemon**; có `ibus-ngosen` và GNOME Shell extension
  `forward-keys`.
- **Mode:** OFF, `Gõ Sen` (commit trực tiếp, tự nhận cách app nhận text),
  Preedit, Emoji Picker.
- **Tính năng:** Telex (daily-tested), VNI (chưa chắc); spell check; macro
  `$TIME/$DATE`; `w→ư`, `[`/`]`; **Cứu chữ** (gõ lại từ tiếng Anh sau khi
  chuyển sang tiếng Việt); **Cứu dấu** (thêm/đổi/bỏ dấu trên text bôi đen);
  giữ Ctrl tap-release để khóa từ; nhớ Việt/Anh theo app qua logout.
- **Đáng học:** state machine retype/retone/wait tường minh
  (`ngosen-state-*.cpp`); recorder; app-quirks layer; probe surrounding có
  event wait/timeout thay vì sleep cố định; settings lưu live + Undo.
- **Hạn chế:** maintainer tự nhận "vibecoded", không hứa support; chỉ Fcitx5;
  VNI chưa chín; Firefox Snap không dùng được Cứu dấu; Chromium Wayland mất
  preedit; Snap VS Code ra raw Telex.
- **Độ chín:** khá; 54 test headless, CI X ảo, repo COPR/APT/pacman/zypper,
  ARM64.

**Bài học:** "Cứu chữ/Cứu dấu" là UX rất Việt Nam, giải quyết đúng nỗi đau
chuyển Việt↔Anh. Đáng học cách họ làm state machine thay vì heuristic.

### 4.11 CanType — zero-preedit, invariant rõ

- **Repo:** `LumeWorks/CanType` · Rust · MPL-2.0 · 1★ · 2026-08.
- **Kiến trúc:** một crate với `cadence` (anti-corruption boundary), `sua`
  (sửa text đã commit, diff Unicode-safe trên byte/UTF-16/codepoint), `host`
  (`Host`, `BoiCanhNhap`, `KetQuaHost`, `HanhDong`), `phien` (session theo
  context), `fcitx5`, `ffi` (nơi duy nhất có `unsafe`); C++ shim.
- **Tính năng:** Telex + VNI; giữ code/chat (`sha256`, `1.2.3`); NFC/NFD;
  zero-preedit (chỉ insert/replace đã commit).
- **Đáng học:** `verify-before-mutate`; state mới của Cadence bị từ chối tới khi
  host áp dụng xong; host outcome không rõ → resync an toàn; tách `.so` khỏi
  Slint.
- **Hạn chế:** Phase 1–2; chưa IBus/TSF/macOS/uinput/package/hotkey; 1★.
- **Độ chín:** thấp về người dùng, cao về tài liệu invariant.

**Bài học:** CanType là ví dụ tốt nhất về "viết ra invariant trước khi viết
code" trong nhóm dự án nhỏ.

### 4.12 PinaKey — nói thật về nơi no-underline thất bại

- **Repo:** `trananhtung/pinakey` · Rust · chưa rõ license · 7★ · 2026-06.
- **Kiến trúc:** workspace `pinakey-core` (transform, spell, dictionary,
  charset), `pinakey-config`, `pinakey-emoji`, `pinakey-engine`
  (`process_key → (handled, Vec<Action>)`), `pinakey-ffi` (cbindgen),
  `pinakey-settings` (egui); C++ addon mỏng map Action.
- **Tính năng:** Telex/VNI/VIQR + Simple Telex; emoji fuzzy-trie ~11k key;
  macro `$DATE`/`$TIME`; spell "giải oan" + `dict.txt`; auto-skip app English/
  password; `transport-rules.conf`; 14 µs/key; 165 unit test.
- **Đáng học:** diff common-prefix cho no-underline; transport rules theo app;
  **công bố rõ** terminal/LibreOffice/GNOME Wayland phải dùng preedit; uinput
  opt-in.
- **Hạn chế:** GNOME Wayland buộc preedit; VTE/LibreOffice không tin được;
  uinput chưa ổn; Linux-only; một maintainer.
- **Độ chín:** trung bình; CI fmt/clippy/test/e2e, deb/rpm/AUR/Nix, docs
  ARCHITECTURE/USAGE/BENCHMARK, v2.1.2.

**Bài học:** pinakey là dự án duy nhất công bố "no-underline thất bại ở đâu".
Sự trung thực này đáng copy hơn mọi tuyên bố "100%".

### 4.13 UniLume — bia mộ có giá trị lịch sử

- **Repo:** `LumeWorks/UniLume` (trước `dismonjames/UniLume`) · 11★ · archive.
- **Là gì:** chỉ còn thông báo chuyển nhà. UI/runtime → `CanType`; engine →
  `Cadence`. Branch `legacy` là bản merge giữ **UniKey/x-unikey core**, Fcitx5
  addon, engine, test, benchmark — chỉ để tham khảo, không phát triển tiếp.
- **Đáng học:** quyết định tách engine/runtime được đưa ra ở đây; `legacy` là
  dấu vết công khai duy nhất của dòng x-unikey trong nhóm LumeWorks.
- **Hạn chế:** không code ở default branch; không build/test docs.

### 4.14 LotusVibe — archive của một fork

- **Repo:** `ngosen/ngosen-fork-archive` (trước `nguyenphivn/LotusVibe`) ·
  C++ · GPL-3.0 · 6★ · archive 2026-10-10.
- **Là gì:** fork pre-1.0 của Ngó Sen, dòng LotusVibe cũ; đã bị thay thế.
- **Giá trị:** ghi lại việc đổi tên `Uinput`/`Surrounding Text` → `Gõ Sen` và
  bỏ uinput service; config vẫn là `lotus*.conf`.
- **Hạn chế:** không fix; Nix chỉ test trên X ảo; README ngắn.

### 4.15 Các dự án Fcitx5 khác trong nhóm đa nền tảng

`Funput` và `TextVN` có adapter Fcitx5 nhưng thuộc nhóm cross-platform — xem
§7.3 và §7.4 để tránh trùng lặp.

---

## 5. Nhóm IBus

### 5.1 ibus-bamboo — nhiều người dùng nhất, đã ngừng

- **Repo:** `BambooEngine/ibus-bamboo` · Go · GPL-3.0 · **1508★** · 2018.
- **Kiến trúc:** engine Go + `bamboo-core`; daemon IBus; nhiều chế độ gõ.
- **Tính năng:** gần như đầy đủ nhất trong nhóm IBus — Telex/Telex W/Telex 2/
  VNI/VIQR/Microsoft; Unicode/TCVN3/VNI/VPS/VISCII/BKHCM/NCR; spell check;
  tone cũ/mới; bỏ dấu tự do; macro; **2666 emoji**; **5 chế độ gõ không gạch
  chân + 1 preedit**; nhớ mode theo app; `Shift+~` mở menu mode; loại trừ app.
- **Đáng học:** mô hình "một app một mode" rất sớm; menu mode bằng phím tắt;
  tích hợp emoji như một phần của IME, không phải app phụ.
- **Hạn chế:** README tự nói dự án đình trệ; Wayland IBus chưa tốt; khuyên
  chuyển sang fcitx5-unikey.
- **Độ chín:** cao về người dùng, thấp về bảo trì. **Bus factor = 1**.

**Bài học:** ibus-bamboo là lý do nhiều người vẫn nghĩ "IBus là đủ". Nó đúng
về tính năng, sai về tương lai.

### 5.2 ibus-lotus — đã chết

- **Repo:** `LotusInputEngine/ibus-lotus` · Go · GPL-3.0 · 118★ · 2025-07 →
  2026-01 (dead, repo 404).
- **Là gì:** engine IBus song song với fcitx5-lotus, cùng thương hiệu Lotus.
- **Bài học:** cùng một team làm hai framework → một cái chết. Chi phí duy trì
  hai adapter lớn hơn tưởng tượng.

### 5.3 TypeVN — IBus tối giản cho GNOME

- **Repo:** `vithanhlam/TypeVN` · Rust + C · MIT · 3★ · 2026-08 · v0.2.4.
- **Kiến trúc:** IBus only; adapter C (`c/main.c`, `c/engine.c`) do `build.rs`
  biên dịch; `typevn-core` Rust staticlib qua C ABI; settings Python GTK4.
- **Tính năng:** Telex/VNI; Unicode/VNI Win/VIQR/TCVN3; macro exact-match;
  Telex auto-repair; English passthrough + tone rewind; undo một phím.
- **Preedit:** IBus preedit tắt gạch chân; auto-commit sau idle 300–2000 ms
  (mặc định 800 ms). 0.2.0 thử commit-and-replace, 0.2.1 revert vì terminal
  không xóa surrounding đáng tin.
- **Đáng học:** xử lý đồng bộ từng phím, không queue/network; mục tiêu <1 ms;
  hotkey GNOME (`Shift+Space`, `Ctrl+Shift+1/2`).
- **Hạn chế:** không CI; không app profile; IBus/GNOME only; Snap pending.
- **Độ chín:** thấp–trung; test `telex.rs`, `vni.rs`, `stress.rs` + bench.

**Bài học:** việc revert commit-and-replace là quyết định đúng và đáng ghi
nhận. Nó xác nhận: trên terminal, preedit là lựa chọn an toàn.

### 5.4 NepKey — IBus + slash command cho AI chat

- **Repo:** `todo-laterx/nepkey` · Go · GPL-3.0 · 2★ · 2026-10.
- **Kiến trúc:** daemon IBus Go (`goibus` + `godbus`); settings GTK4/Python;
  engine dùng `bamboo-core` pin commit `b2e49a2` (2024-09).
- **Tính năng:** Telex 2/Telex/VNI/VIQR; Unicode; free tone; `òa/ùy`; spell;
  macro YAML hot-reload; V/E toggle.
- **Đáng học:** `rawTokenMode`/`slashCommandMode` + `isRawTokenContinuation()`
  bảo vệ `/image`, `/clear`; tách URL/path trước khi thêm dấu; auto-commit khi
  Left/Right/Home/End; `input-purpose` password bypass; output `clipboard`
  mặc định, `hex` (Ctrl+Shift+U) cho GTK.
- **Hạn chế:** IBus/GNOME only; clipboard injection race; macro 1:1; core pin
  cũ; 2★.
- **Độ chín:** thấp; `go test`, CI vet/test/build, `.deb`.

**Bài học:** NepKey là dự án duy nhất nhắm thẳng vào chat AI/CLI. Niche hẹp
nhưng thật.

---

## 6. Nhóm Wayland standalone

### 6.1 Unikey-Wayland — ring-0 IME

- **Repo:** `ubuntu2310fake/Unikey-Wayland` · Makefile/C++ · GPL-3.0 · **101★** ·
  2026-07 · v3.0.3.
- **Kiến trúc v3:** kernel module `ukw_driver` + root `ukw_daemon` + XKB layout
  `Vietnamese (Kernel Driver)`; legacy `wayland-client` (Bamboo Go qua CGO),
  `ibus-engine`, `x11-client`, Windows global hook vẫn trong tree; settings Qt6.
- **Tính năng:** Telex, Telex giản lược, VNI, VIQR; Unicode/TCVN3/VNI/VIQR;
  free marking; modern tone; spell; macro; preedit chỉ làm fallback terminal;
  Windows `SendInput` (không TSF).
- **Đáng học:** grab input ở kernel + `/dev/ukw` FIFO; XKB map keycode tùy biến
  (86/89/...) sang chữ dựng sẵn/dấu tổ hợp; DKMS; backend native từng OS;
  fix Omnibox selection.
- **Hạn chế:** `/dev/ukw` mode `0666` — **bất kỳ user nào cũng đọc/inject được
  phím**; root daemon; kernel module dễ vỡ; README nói Bamboo nhưng v3 ring-0
  là engine keycode C++ riêng; repo có `a.out`, `main.cpp.orig`, `fix*.py`;
  không unit test.
- **Độ chín:** cao về phân phối (deb/rpm/AUR, x86_64/ARM64, Windows OTA),
  thấp về vệ sinh mã nguồn.

**Cảnh báo:** nếu định dùng, hãy audit `/dev/ukw`. Đây là mô hình quyền nguy
hiểm nhất trong toàn bộ khảo sát.

### 6.2 Unikey-Wayland-Pro — fork mất giá trị

- **Repo:** `quannguyen247/Unikey-Wayland-Pro` (trước `...-Final`) · 1★ ·
  archive.
- **Là gì:** bản copy 2.x của Unikey-Wayland, không có `ring0-engine`.
- **Hạn chế:** README là bản upstream bỏ mục Ring-0; changelog dừng 2.0.10;
  download trỏ về upstream; không có tính năng riêng; không fix bảo mật.
- **Bài học:** đừng đặt tên "Pro" nếu không có gì mới. Fork không maintain =
  nợ bảo mật.

### 6.3 vi-ime — Rust, không Fcitx/IBus

- **Repo:** `nhanth87/vi-ime` · Rust · GPL-3.0 (API báo NOASSERTION) · 7★ ·
  2026-07 · v7.4.4.
- **Kiến trúc:** bind `zwp_input_method_v2` + `zwp_virtual_keyboard_v1`; grab
  evdev `/dev/input` + uinput fallback; single binary/AppImage.
- **Tính năng:** NFD/NFC algebra, không bảng nguyên âm, parse lại cả âm tiết
  mỗi phím; Telex/VNI/"Tự do"; Unicode NFC/NFD; `viet_dict`; 1070 từ Anh; 120+
  cheat; macro/hotkey macro; emoji.
- **Preedit:** theo app — terminal echo trực tiếp; Firefox preedit;
  LibreOffice/OnlyOffice evdev; Chrome trên Niri non-preedit. `APP_RULES.md`
  định nghĩa 5 tầng + timing 3–20 ms.
- **Đáng học:** một `rx.recv()` blocking, không timer; `--doctor`; phát hiện
  IME đối thủ + `--take-over` (opt-in); `godmod` JSONL telemetry; NFC clipboard
  tool; `learning.rs` cache theo app; `/proc/PID` advisor cho Electron.
- **Hạn chế:** không GNOME; không CI; AppImage prebuilt commit; README tự mâu
  thuẫn ("0.8ns/word" vs "17µs/keypress"); hardcode 45+ app; cần group `input`.
- **Độ chín:** thấp–trung; có unit/integration test, không CI.

### 6.4 vietc — "zero underline" nhưng có preedit

- **Repo:** `vndangkhoa/vietc` · Rust · MIT · 60★ · 2026-06 · v0.1.24.
- **Kiến trúc:** workspace `engine/protocol/daemon/uinputd/vietcctl`; systemd
  user daemon. Thứ tự đường: IBus-first trên Ubuntu GNOME → `zwp_input_method_v2`
  → EVIOCGRAB evdev → X11 XTEST; `wtype`/uinput + virtual keyboard native.
- **Tính năng:** Telex/VNI; `engine/bamboo.rs` kiểu Bamboo; auto-restore tiếng
  Anh; macro; giữ case; `uo→ươ`; Unicode only.
- **Preedit:** v2/IBus dùng preedit; evdev gõ rồi sửa. **README "Zero Underline"
  bị `docs/wayland-rootless.md` phản bác** — đường IBus vẫn hiện preedit.
- **Đáng học:** lọc `/dev/input/by-path/*-event-kbd`; AT-SPI password detector;
  state theo app (mặc định off); key dedup opt-in; OSD/tray/CLI; TOML config.
- **Hạn chế:** chọn đường phức tạp; stop IBus khi start (SIGKILL để lại IBus
  tắt); race khi inject; app profile mặc định off; claim "100%"; 6 issue mở.
- **Độ chín:** trung bình; CI build/release/AUR, "151 tests" badge, insta
  snapshot.

**Bài học:** vietc cho thấy "zero underline" là marketing, không phải kiến
trúc. Muốn không gạch chân thật, phải direct-rewrite + quản lý transaction.

### 6.5 codekeyvn — đã chết

- **Repo:** `tollaai2212hp22-lgtm/codekeyvn` · Rust · 7★ · tạo và chết trong
  cùng tháng 7/2026 (404).
- **Bài học:** repo một-lần-commit, không README để lại, không có gì để đánh
  giá. Đừng đưa vào tài liệu như một lựa chọn.

---

## 7. Nhóm native desktop & cross-platform

### 7.1 OpenKey — chuẩn mực macOS/Windows

- **Repo:** `tuyenvm/OpenKey` · C++ · GPL-3.0 · **969★** · 2019.
- **Nền tảng:** macOS (chính), Windows, Linux (đang phát triển).
- **Kỹ thuật:** **Backspace technique** — không gạch chân; gửi text một lần
  hoặc từng phím (tùy chọn "Gửi từng phím" cho app không tương thích).
- **Tính năng:** Telex/VNI/Simple Telex; Unicode/TCVN3/VNI Win/Unicode tổ hợp/
  CP1258; modern orthography; Quick Telex (`cc=ch`, `gg=gi`...); grammar check;
  spelling check; restore invalid word; macro không giới hạn; chuyển chế độ
  theo app; auto-cap; tắt tạm bằng Ctrl/Cmd; cho `f z w j` làm phụ âm đầu;
  gõ tắt phụ âm đầu/cuối; **công cụ chuyển mã**; nhớ bảng mã theo app
  (Photoshop/CAD); auto-update.
- **Đáng học:** UX menu bar rất chín; "gửi từng phím" như fallback tương thích;
  chuyển mã + nhớ charset theo app là tính năng người dùng thật, không phải
  demo.
- **Hạn chế:** 149 issue mở; macOS/Windows codebase tách nhánh; Linux chưa
  hoàn thiện; kỹ thuật Backspace có thể đua với app nhanh.
- **Độ chín:** rất cao về người dùng; là lựa chọn mặc định của nhiều người
  Việt trên macOS.

### 7.2 Gõ Nhanh — sản phẩm hoá tốt nhất

- **Repo:** `khaphanspace/gonhanh.org` · Rust + Swift/C# · BSD-3-Clause ·
  **772★** · 2025-12.
- **Nền tảng:** macOS (chính, Homebrew), Linux beta (Fcitx5), Windows beta.
- **Kiến trúc:** core Rust **validation-first**, pipeline 7 tầng: Stroke → Tone
  → Vowel mark → Remove → W-vowel → Normal letter → Shortcut; FFI C ABI; macOS
  CGEventTap, Windows `WH_KEYBOARD_LL`, Linux Fcitx5.
- **Tính năng:** Telex/VNI; tone mới; auto-cap; gõ tắt; **auto-restore tiếng
  Anh** (`text`, `expect`, `user`...); **ESC khôi phục**; nhớ ON/OFF theo app;
  tự tắt khi chuyển input source Nhật/Hàn/Trung; delay theo app (IDE/Claude
  Code); dark/light.
- **Đáng học:** validation-first (kiểm tra âm tiết trước khi biến đổi) là cách
  tránh "cướp từ tiếng Anh" tốt hơn dictionary; tài liệu thuật toán công khai;
  UX "ba không" (không phí/quảng cáo/theo dõi) tạo niềm tin.
- **Hạn chế:** auto-restore vẫn thua các từ tạo âm tiết hợp lệ (`mix → mĩ`);
  Windows/Linux còn beta; core Rust nhưng UI native từng OS.
- **Độ chín:** rất cao về sản phẩm; test/CI tốt; là đối thủ mạnh nhất của
  OpenKey trên macOS.

### 7.3 Funput — 5 nền tảng, crate sạch

- **Repo:** `Funput/Funput` · Rust + Kotlin/Swift · MIT · 52★ · 2026-06.
- **Nền tảng:** iOS keyboard extension, Android IME (Kotlin/JNI), macOS, Windows
  TSF, Linux Fcitx5 **và** IBus; settings GTK4/libadwaita.
- **Kiến trúc:** workspace `funput-core` (thuần: Telex/VNI,
  `TransformKind Pending/Applied/Reverted/Ignored`), `funput-engine` (stateful,
  `Action{None,Send,Restore}` + backspace + output), `funput-ffi`
  (`chars:[u32;64]`), `funput-jni`, `funput-config`, `funput-convert`,
  `funput-suggestions`, `funput-term`, `funput-lexicon-tool`, `funput-cli`.
- **Tính năng:** Telex, TelexAdvanced (`w` onset, `[`/`]`), VNI; tone cũ/mới;
  spell gate; smart + eager restore; `flip_composing()` (`card` ⇄ `cải`);
  shortcut; phụ âm đầu Z/F/W/J; Unicode/NFD/TCVN3/VNI Win; auto-cap;
  no-preedit với fallback theo app; gợi ý trigram/context/personal.
- **Đáng học:** tách pure/stateful cực rõ; backspace suy ra từ diff; overflow
  FFI xử lý ở boundary; có `benchmarks/results`.
- **Hạn chế:** không VIQR; "kiến trúc có thể thay đổi"; README gốc mỏng; 14
  issue mở; không CI badge.
- **Độ chín:** trung bình–cao; proptest + bench + docs từng nền tảng.

### 7.4 TextVN — nhiều tính năng nhất

- **Repo:** `hunglinhpt/TextVN` · Rust + Swift/C++ · GPL-3.0 · 9★ · 2026-09 ·
  v0.2.60 (60 release trong ~1 tháng).
- **Nền tảng:** Windows TSF (+ hook "Compatibility"), macOS IMK (+ CGEventTap
  chưa phát hành), Linux IBus + Fcitx5; settings GTK4.
- **Kiến trúc:** `core/` Rust độc lập (Telex/VNI/VIQR, tone, charset, macro),
  `ffi/` C ABI, `config/`; adapter từng OS; `corpus/*.keys` replay qua 4
  adapter simulator.
- **Tính năng:** Telex/VNI/VIQR/Simple Telex; Unicode/TCVN3/VNI/NFD; tone
  cũ/mới; macro theo case; per-app override; Tab completion + next-phrase
  prediction; 3000 từ Anh auto-restore; ESC raw restore; Ctrl+Shift+F9
  transcode vùng bôi đen; E28 anti-key-loop; **trợ lý ngữ pháp** 15 loại lỗi,
  Ctrl+Z 5 s; không gạch chân (`TF_LS_NONE`); không Backspace tổng hợp.
- **Đáng học:** adapter-simulation corpus replay; công bố luật xung đột ngôn
  ngữ; thiết kế chống AV; GPG + Sigstore signing.
- **Hạn chế:** RC (chưa Authenticode/notarize); thiếu RPM; README là changelog
  khổng lồ; repo ~740 MB.
- **Độ chín:** cao về tính năng và release velocity; thấp về ổn định kênh phát
  hành.

**Bài học:** TextVN cho thấy "grammar assistant" là hướng đi mới, nhưng cũng
cho thấy nguy cơ phình tính năng trước khi ổn định nền tảng.

### 7.5 VieLotus IM (Sen Việt) — open-core, store-first

- **Repo:** `stevedat/VieLotus-IM` · Swift + core nhị phân · MIT · 6★ ·
  2026-09.
- **Nền tảng:** macOS IMK, iOS/iPadOS `UIInputViewController`, Windows TSF
  (`ITfTextInputProcessorEx`, `ITfEditSession` atomic), Linux IBus+Fcitx5 qua
  `libVieLotusCAPI.so`, Android `InputMethodService`.
- **Kiến trúc:** shell Swift 6, nhưng engine là `VieLotusCore.xcframework` dựng
  sẵn; **mã nguồn core không công khai**; Windows wrapper link `.obj` commit
  sẵn.
- **Tính năng:** Telex/VNI; loại âm tiết bất khả (`turn` giữ nguyên); xử lý
  `rest`/`rét`, `if`/`ì`; per-app `typing-modes.yml` với `directReplacement` /
  `markedText` / `passthrough` / `automatic`; ring buffer 20 từ cho Backspace;
  UTF-16 cursor sync; p99 <0.2 ms (tự công bố).
- **Đáng học:** per-app behavior là **dữ liệu**, không phải code; precedence
  user override; rule sai bị bỏ qua thay vì crash.
- **Hạn chế:** không thực sự open source; binary (`.pkg`, `.exe`, `.deb`,
  `.obj`) commit trong `Binaries/`; "Pure Swift 6" không khớp C++ Windows;
  không macro/charset.
- **Độ chín:** cao về phân phối (App Store, Play, MS Store, Snap, PPA, AUR,
  Homebrew); thấp về mở.

**Cảnh báo:** nếu anh cần đọc/sửa engine, đây không phải lựa chọn. Nếu cần
dùng, nó ổn.

### 7.6 BambooMintKey — .NET/F# + TSF in-process

- **Repo:** `thatislg/BambooMintKey` · F#/.NET 10 + C++ · MIT (GitHub báo
  NOASSERTION) · 10★ · 2026-09.
- **Nền tảng:** Windows TSF in-process COM (`ITfKeyEventSink`, không inject
  key); Linux Fcitx5 addon C++ gọi C ABI `BambooMintKeyCore.so`; macOS IMK
  "TBD"; Steam Deck qua Flatpak Addon Extension.
- **Kiến trúc:** F# `BambooMintKey.Core` (Telex, syllable parser, Unicode
  tables) dùng chung 100%; C# NativeAOT bridge; Avalonia settings.
- **Tính năng:** Telex only; V/E toggle bằng `` ` `` + D-Bus `ModeChanged`;
  `config.json` hot-reload `inotify`; shared-memory config sync Windows;
  taskbar V/E icon.
- **Đáng học:** một functional core + bridge mỏng; Flatpak addon cho SteamOS
  read-only; single-owner D-Bus service.
- **Hạn chế:** không VNI/VIQR/macro/spell; macOS chưa build; toolchain .NET 10
  + NativeAOT + clang/lld nặng; license chưa rõ.
- **Độ chín:** trung bình; unit test F#, C++ addon test, DevHarness.

**Bài học:** TSF in-process là cách đúng để không inject key trên Windows —
đáng học nếu anh làm Windows IME.

### 7.7 vnkey — 4 adapter, dừng giữa đường

- **Repo:** `marixdev/vnkey` · Rust + C/ObjC · GPL-3.0 · 101★ · 2026-03 →
  inactive 2026-04.
- **Nền tảng:** Windows native, macOS IMKit, Fcitx5, IBus — cùng một core Rust
  `vnkey-engine` qua C FFI staticlib; Windows dùng Win32/Direct2D +
  `WH_KEYBOARD_LL`; macOS Objective-C IMKit; adapter C++/C mỏng; chung
  `config.json`.
- **Tính năng:** Telex, Simple Telex, VNI, VIQR, **MS Vietnamese**; 15 bảng mã;
  spell check/ASCII restore; free marking; modern style; **clipboard charset
  converter**; macro (lookup không phân biệt hoa/thường); mode **Ede (Tây
  Nguyên)**; `app_charsets` theo app; Windows app blacklist.
- **Direct/preedit:** Windows hook + `SendInput` backspace/Unicode rewrite;
  Fcitx5 nhận diện terminal → preedit để commit atomic, còn lại direct commit.
- **Đáng học:** một core + 4 adapter là mô hình đúng; `app_charsets` theo app
  rất thực dụng cho người dùng Photoshop/CAD.
- **Hạn chế:** hook/SendInput fragile (có workaround VMware/RDP trong code);
  IBus dùng `xdotool` + `/proc` để tìm app → X11-only; JSON parser viết tay;
  README bỏ sót macro/Ede.
- **Độ chín:** trung bình–cao về đóng gói (deb/rpm/pacman/Nix, CI, SignPath),
  nhưng **đã dừng** — đừng chọn làm nền tảng lâu dài.

**Bài học:** vnkey là ví dụ rõ nhất của "làm đúng kiến trúc nhưng hết người".
Nếu anh định làm IME đa nền tảng, hãy học cấu trúc của họ và chuẩn bị người
kế nhiệm.

---

## 8. Ma trận tính năng

Ký hiệu: ✅ có · ◐ một phần · — không · 🔌 adapter/OS.

| Dự án | Telex | VNI | VIQR | Charset | Spell | Macro | App profile | Preedit | Direct rewrite | Đa nền tảng |
|---|---|---|---|---|---|---|---|---|---|---|
| fcitx5-unikey | ✅ | ✅ | ✅ | 8 | ✅ | ✅ | — | ✅ | ◐ (experimental) | — |
| fcitx5-bamboo | ✅ | ✅ | ✅ | 17 | ✅ | ✅ | — | ✅ | — | — |
| fcitx5-lotus | ✅ | ✅ | ✅ | 17 | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| Areca | ✅ | ✅ | ✅ | bamboo | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| VMK | ✅ | ✅ | — | 5 | ◐ | ✅ | ◐ | ✅ | ✅ | — |
| SKey | ✅ | ✅ | ◐ | 14 | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| telebit | ✅ | ✅ | — | 1 | ✅ | ✅ | — | ✅ | ✅ | — |
| Clak | ✅ | ✅ | ✅ | 6 | ✅ | ✅ | ✅ | — | ✅ | — |
| lilypad | ✅ | ✅ | — | bamboo | ✅ | ✅ | ◐ | ✅ | ✅ | — |
| Ngó Sen | ✅ | ◐ | — | ◐ | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| CanType | ✅ | ✅ | — | 2 | ◐ | — | — | — | ✅ | — |
| pinakey | ✅ | ✅ | ✅ | ◐ | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| ibus-bamboo | ✅ | ✅ | ✅ | 17 | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| TypeVN | ✅ | ✅ | — | 4 | ◐ | ✅ | — | ✅ | — | — |
| NepKey | ✅ | ✅ | ✅ | 1 | ✅ | ✅ | — | ✅ | ◐ | — |
| Unikey-Wayland | ✅ | ✅ | ✅ | 4 | ✅ | ✅ | ◐ | ◐ | ✅ | ◐ (Win) |
| vi-ime | ✅ | ✅ | — | 2 | ◐ | ✅ | ✅ | ◐ | ✅ | — |
| vietc | ✅ | ✅ | — | 1 | ◐ | ✅ | ◐ | ◐ | ✅ | — |
| OpenKey | ✅ | ✅ | — | 6+ | ✅ | ✅ | ✅ | — | ✅ | ✅ (mac/Win) |
| Gõ Nhanh | ✅ | ✅ | — | 1 | ✅ | ✅ | ✅ | 🔌 | ✅ | ✅ (3 OS) |
| Funput | ✅ | ✅ | — | ◐ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (5 OS) |
| TextVN | ✅ | ✅ | ✅ | 4 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (3 OS) |
| VieLotus | ✅ | ✅ | — | 1 | ◐ | — | ✅ | ✅ | ✅ | ✅ (5 OS) |
| BambooMintKey | ✅ | — | — | 1 | — | — | — | ✅ | ◐ | ✅ (Win/Linux) |

Ghi chú:

- "Charset" là số bảng mã theo README; nhiều dự án chỉ hỗ trợ Unicode.
- "App profile" = cấu hình hành vi theo app, không chỉ bật/tắt.
- "Direct rewrite" = gõ trực tiếp, không gạch chân, có sửa text đã commit.
- Bảng này không đo chất lượng; một dấu ✅ có thể là tính năng mới, chưa chín.

---

## 9. Bản đồ dòng dõi engine

```text
bamboo-core (Go, MIT, Luong Thanh Lam)
├── ibus-bamboo            (Go, dùng trực tiếp)
├── fcitx5-bamboo          (Go c-archive)
├── Areca                  (Go c-archive + C++ adapter)
├── NepKey                 (Go, IBus)
├── fcitx5-lotus           (port C++ trong lotus/)
├── VMK                    (Go + C++ addon)
├── lilypad               (Go + C++ addon, fork VMK/Lotus)
├── Ngó Sen                (Rust port bamboo-rs)
└── vietc                  (Rust port kiểu Bamboo)

x-unikey / UniKey
├── fcitx5-unikey          (unikey-lib port)
├── Unikey-Wayland         (2.x dùng Bamboo; 3.x engine keycode riêng)
└── UniLume legacy         (x-unikey core, archive)

Engine riêng
├── OpenKey                (C++, lâu năm)
├── Gõ Nhanh               (Rust, validation-first)
├── telebit                (C++, rebuild âm tiết)
├── SKey                   (Rust stateless)
├── Funput                 (Rust workspace)
├── TextVN                 (Rust core + nhiều adapter)
├── vnkey                  (Rust engine)
├── CanType/Cadence        (Rust)
└── pinakey                (Rust)
```

**Nhận xét:** `bamboo-core` là engine được tái sử dụng nhiều nhất. Điều này
tốt cho chất lượng tiếng Việt (một chỗ sửa, nhiều nơi hưởng), nhưng tạo rủi ro
tập trung: nếu upstream chậm, cả hệ sinh thái chậm.

---

## 10. Mười bài học kỹ thuật lặp lại

1. **Tách engine khỏi adapter.** Dự án nào trộn hai thứ vào một file sẽ khó
   test và khó port. Bài học từ Areca, SKey, Funput, CanType, ViFastKey.
2. **Preedit là an toàn, direct rewrite là trải nghiệm.** Đừng chọn một;
   hãy cho người dùng chọn và tự fallback.
3. **Không tin capability.** App khai báo surrounding text không có nghĩa là
   nó xóa đúng. Areca probe; SKey/Lotus có mode riêng; pinakey công bố nơi
   thất bại.
4. **Transaction barrier là bắt buộc nếu direct rewrite.** Không có nó, gõ
   nhanh sẽ mất/lặp chữ. Xem Areca §6, Lotus `focus-out-mid-replacement`.
5. **Timing theo frontend, không một con số chung.** Wayland 3 ms, DBus 10 ms,
   post-commit 20 ms. Xem Areca §9, lilypad `ΔT_swallow`.
6. **Undo/restore phải có bằng chứng.** Chỉ coi phím lặp là undo khi phím trước
   thực sự biến đổi; nếu không, khôi phục raw. Xem ViFastKey COMPATIBILITY §4.
7. **Macro cần undo một phím + bypass modifier.** OpenKey, Areca, ViFastKey
   đều làm; dự án thiếu sẽ bị người dùng phàn nàn.
8. **Charset là module riêng, không nhúng vào transform.** Độ dài xóa phải
   tính trên chuỗi đã encode. Xem Areca §5.2.
9. **Test adapter, không chỉ test engine.** Cần `TestInputContext` giả lập
   commit/delete/forward/surrounding. Areca, Lotus, fcitx5-unikey, ViFastKey có.
10. **Đóng gói và doctor quan trọng ngang code.** Người dùng không debug được
    nếu không có log, không có `fcitx5-diagnose`, không có gói cài.

---

## 11. Repo đã chết và bài học

| Repo | Trạng thái | Bài học |
|---|---|---|
| `LotusInputEngine/ibus-lotus` | 404, dead | Làm hai framework cùng lúc → một cái chết |
| `furimeo/senkey` | 404, dead | Không để lại README/license → không ai kế thừa |
| `tollaai2212hp22-lgtm/codekeyvn` | 404, dead | Repo một lần commit không có giá trị lịch sử |
| `nguyenphivn/LotusVibe` | archive | Fork không maintain = nợ bảo mật |
| `quannguyen247/Unikey-Wayland-Final` | archive | Đổi tên "Pro" không tạo giá trị |
| `marixdev/vnkey` | inactive | 101★ nhưng dừng 4/2026; bus factor |

**SenKey** (`furimeo/senkey`, 0★, GPL-2.0) là trường hợp đáng tiếc nhất: repo
tồn tại chưa đầy một tháng, không README, không release, rồi bị xóa. Không có
gì để đánh giá — chỉ còn dòng trong tracker.

**Bài học chung:** trong 31 repo, 7 không còn phát triển (3 repo bị xóa, 3 archive,
1 inactive) — tức gần 1/4. Hệ sinh thái này có tỷ lệ churn rất cao. Khi chọn bộ
gõ để dùng lâu dài, hãy nhìn **lịch sử commit + test + packaging**, đừng nhìn stars.

---

## 12. Chọn bộ gõ nào?

| Nhu cầu | Gợi ý | Vì sao |
|---|---|---|
| Linux, muốn giống UniKey, không gạch chân | **Areca** hoặc **fcitx5-lotus** | Areca đúng đắn/ổn định; Lotus nhiều mode/tính năng |
| Linux, cần nhiều output mode + GUI | **SKey** | 5 mode, Qt6, per-app, 14 charset |
| Linux, GNOME/IBus | **ibus-bamboo** (nếu chấp nhận stale) hoặc **NepKey/TypeVN** | ibus-bamboo đầy đủ nhất; hai dự án kia mới hơn |
| Linux, không muốn Fcitx/IBus | **vi-ime** hoặc **vietc** | Standalone Wayland; chấp nhận cấu hình phức tạp |
| Linux, terminal/IDE nhiều | **Areca Preedit** hoặc **SKey Preedit** | Preedit an toàn cho VTE/Electron |
| macOS | **Gõ Nhanh** hoặc **OpenKey** | Gõ Nhanh hiện đại/auto-restore; OpenKey chín/lâu năm |
| Windows | **OpenKey** hoặc **TextVN** | OpenKey ổn định; TextVN nhiều tính năng (grammar) |
| Đa nền tảng, một engine | **Funput**, **TextVN**, **vnkey**, **ViFastKey** | Cùng core, nhiều adapter |
| Học kiến trúc adapter | **Areca** | Tài liệu + 6 backend + trust probe |
| Học thuật toán engine | **Gõ Nhanh**, **telebit** | Validation-first / rebuild âm tiết, có docs |
| Học kiến trúc tổng thể | **ViFastKey** | Oracle + conformance + nhiều adapter |

---

## 13. Kết luận

1. **Không có bộ gõ nào "tốt nhất" toàn diện.** Mỗi dự án tối ưu một trục:
   Areca (đúng đắn), Lotus (tính năng), SKey (mode), OpenKey (UX macOS),
   Gõ Nhanh (sản phẩm), TextVN (tính năng), ViFastKey (kỷ luật kỹ thuật).
2. **Engine tiếng Việt đã gần như commodity.** `bamboo-core` và các port đủ tốt
   cho 90% nhu cầu. Cạnh tranh thật nằm ở adapter, UX và phân phối.
3. **Test adapter là lợi thế cạnh tranh dài hạn.** Đây là thứ khó copy nhất và
   cũng là thứ người dùng cảm nhận rõ nhất (mất chữ, lặp từ, vỡ dấu).
4. **Hệ sinh thái churn cao.** 7/31 repo không còn phát triển (3 xóa, 3 archive,
   1 inactive). Đừng xây sản phẩm phụ thuộc vào một repo không có test/CI/người
   kế nhiệm.
5. **ViFastKey có nền tốt nhất về kỷ luật** (oracle + conformance + adapter
   contract), nhưng còn thiếu chiều sâu adapter Linux và phân phối so với
   Areca/Lotus/SKey. Xem [`comparison-vifastkey.md`](comparison-vifastkey.md).

---

## 14. Nguồn

- Tracker: <https://hthienloc.github.io/vietnamese-ime-tracker> ·
  `repos.yml` · `bo-go-tieng-viet.csv`
- GitHub API: metadata 31 repo (star, language, license, created, pushed).
- README của 28 repo còn tồn tại.
- Mã nguồn/kiến trúc: Areca (`952ddd8`), fcitx5-lotus, skey, fcitx5-bamboo,
  fcitx5-unikey, Gõ Nhanh, telebit, Ngó Sen.
- Tài liệu nội bộ ViFastKey: `docs/IME_FEATURE_RESEARCH.md`,
  `docs/research/*.md`, `docs/PLATFORMS.md`, `docs/COMPATIBILITY.md`.
