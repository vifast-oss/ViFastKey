# ViFastKey so với 31 bộ gõ tiếng Việt

> Snapshot 2026-11-01. Đây là so sánh **kỹ thuật và sản phẩm**, không phải
> benchmark. Điểm số (nếu có) là chủ quan, dựa trên đọc mã nguồn/tài liệu, không
> dựa trên đo tốc độ hay test app thật.
>
> ViFastKey **chưa có trong tracker** tại thời điểm này, nên không có stars/issue
> để so. Các nhận định về ViFastKey lấy từ `README.md`, `docs/PLATFORMS.md`,
> `docs/COMPATIBILITY.md`, `docs/CONFIG.md` và mã nguồn trong repo.

---

## 1. Tóm tắt cho người vội

- **ViFastKey mạnh nhất ở tầng engine + kỷ luật kiểm thử.** Không dự án nào
  trong 31 repo có oracle Python + conformance JSONL + port Rust đối chiếu
  100%. Đây là lợi thế khó copy.
- **ViFastKey yếu hơn ở tầng adapter Linux và phân phối.** Areca, fcitx5-lotus
  và SKey có nhiều backend/fallback hơn; Areca/Lotus/SKey/OpenKey/Gõ Nhanh có
  gói cài và kênh phát hành hoàn chỉnh hơn.
- **ViFastKey rộng nhất về nền tảng.** macOS + Windows + Linux (IBus/Fcitx5) +
  Android + iOS + Chrome + WASM + UEFI demo. Không dự án nào trong khảo sát có
  đủ bộ này.
- **ViFastKey ngang top về tính năng engine** (4 kiểu gõ, 16 bảng mã, macro
  nhiều từ + undo, app profile, preedit/replace, gợi ý local).
- **ViFastKey thua top về UX tiện dụng** (emoji picker, prediction, grammar,
  auto-update, ký/notarize) — những thứ không khó nhưng quyết định trải nghiệm.

---

## 2. ViFastKey là gì (và không là gì)

**Là:**

- Một **đặc tả hành vi chạy được**: Python là oracle, Rust là port phải qua
  100% `conformance/*.jsonl` (13.345 core + 3.000 engine tại snapshot).
- Một **engine thuần** trả `Action` (Pass/Preedit/Replace/Commit), không tự
  gửi phím, không tự vẽ UI.
- Một **bộ adapter** cho 8 đích: macOS IMK, Windows hook, Linux IBus, Linux
  Fcitx5, Android IME, iOS keyboard, Chrome MV3, UEFI demo.
- Một **bộ công cụ**: CLI, charset converter, suggest, emoji, settings egui,
  WASM playground.

**Không là:**

- Không phải bộ gõ "cài là chạy" trên Linux: chưa có `.deb`/`.rpm`/AUR.
- Không phải bản macOS đã ký/notarize: cần Developer ID.
- Không phải TSF trên Windows: hiện là hook-only.
- Không có backend uinput/Libei/XTest trên Linux.
- Không có emoji picker/prediction/grammar như Lotus/TextVN/Funput.

---

## 3. Ma trận năng lực (nhóm tiêu biểu)

Ký hiệu: ✅ mạnh · ◐ có nhưng chưa chín/giới hạn · — không có · 🔌 thuộc OS.

| Năng lực | ViFastKey | Areca | Lotus | SKey | ibus-bamboo | OpenKey | Gõ Nhanh | TextVN | Funput |
|---|---|---|---|---|---|---|---|---|---|
| Telex / VNI | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| VIQR | ✅ | ✅ | ✅ | ◐ | ✅ | — | — | ✅ | — |
| Simple Telex | ✅ | ◐ | ✅ | ◐ | ✅ | ✅ | — | ✅ | ✅ |
| Bảng mã legacy | 16 | ~17 (bamboo) | 17 | 14 | 17 | 6+ | 1 | 4 | ◐ |
| Spell check 3 trạng thái | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Macro nhiều từ | ✅ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ |
| Macro undo 1 phím | ✅ | ✅ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ |
| App profile (method/tone/spell) | ✅ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ | ✅ | ✅ |
| Preedit | ✅ | ✅ | ✅ | ✅ | ✅ | — | ◐ | ✅ | ✅ |
| Direct rewrite | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Fallback đa backend (Linux) | ◐ | ✅ | ✅ | ✅ | — | — | ◐ | ◐ | ◐ |
| Trust probe surrounding | ◐ | ✅ | ◐ | ◐ | — | — | — | ◐ | — |
| Transaction barrier | ✅ | ✅ | ✅ | ◐ | — | — | — | ◐ | ◐ |
| Emoji picker | — | — | ✅ | — | ✅ | — | — | — | — |
| Gợi ý từ (local) | ✅ | — | — | — | — | — | — | ✅ | ✅ |
| Prediction câu | — | — | — | — | — | — | — | ✅ | ✅ |
| Grammar assistant | — | — | — | — | — | ◐ | — | ✅ | — |
| Chuyển mã clipboard/file | ✅ CLI | — | — | — | — | ✅ | — | ✅ | ✅ |
| macOS native | ✅ IMK | — | — | — | — | ✅ | ✅ | ✅ | ✅ |
| Windows native | ◐ hook | — | — | — | — | ✅ | ◐ beta | ✅ TSF | ✅ TSF |
| Linux Fcitx5 | ✅ | ✅ | ✅ | ✅ | — | — | ◐ | ✅ | ✅ |
| Linux IBus | ✅ | — | — | — | ✅ | — | — | ✅ | ✅ |
| Android / iOS | ✅ / ✅ | — | — | — | — | — | — | — | ✅ |
| Web / WASM | ✅ | — | — | — | — | — | — | — | — |
| UEFI demo | ✅ | — | — | — | — | — | — | — | — |
| GUI cấu hình | ✅ AppKit + egui | ✅ ImGui | ✅ Qt/Py | ✅ Qt6 | ✅ | ✅ | ✅ | ✅ | ✅ |
| Packaging đa distro | — | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ◐ |
| Ký/notarize/auto-update | — | ◐ | ◐ | ◐ | ◐ | ✅ | ✅ | ◐ | ◐ |
| Test adapter tự động | ✅ | ✅ | ✅ | ✅ | — | — | — | ◐ | — |
| Conformance corpus | ✅ 16k+ | — | — | — | — | — | — | — | — |
| License | MIT / Apache-2.0 | MIT | GPL-3.0 | GPL-3.0 | GPL-3.0 | GPL-3.0 | BSD-3 | GPL-3.0 | MIT |

Ghi chú:

- "Bảng mã legacy" đếm theo tài liệu; một số dự án chỉ hỗ trợ Unicode.
- "Trust probe" = chờ bằng chứng app đã xóa trước khi commit; Areca đầy đủ
  nhất, ViFastKey có guard nhưng không probe ACK.
- "Test adapter" = test bằng InputContext/TestFrontend giả lập, không cần GUI.

---

## 4. So sánh theo tám trục

### 4.1 Độ đúng của engine

**Ai dẫn:** ViFastKey, Gõ Nhanh, telebit, OpenKey.

- ViFastKey có mô hình âm tiết CVC 3 trạng thái, đặt thanh, `allow_coda_k`,
  `đ` viết tắt, VNI literal number, sticky restore, open `uơ/ươ` theo âm đầu.
  Hành vi được ghi thành hợp đồng trong `COMPATIBILITY.md`.
- Gõ Nhanh dùng **validation-first**: kiểm tra âm tiết trước khi biến đổi, nên
  không "cướp" từ tiếng Anh. Đây là cách tiếp cận khác, rất tốt cho người gõ
  Anh nhiều.
- telebit **rebuild âm tiết** (initial + rime + tone) rồi validate — gần với
  cách ViFastKey làm.
- OpenKey có nhiều năm edge case thực tế, nhưng ít tài liệu.

**Kết luận:** ViFastKey ngang top về độ phủ; Gõ Nhanh/telebit có triết lý khác
đáng học. Không ai vượt trội tuyệt đối.

### 4.2 Kiểm thử và đặc tả

**Ai dẫn:** ViFastKey (rõ ràng), sau đó TextVN/Areca/Lotus.

- ViFastKey: 676 test Python, 16k+ ca conformance, adapter-conformance dùng
  chung cho mọi port, golden/corpus. Đây là mức kỷ luật hiếm thấy trong toàn
  bộ khảo sát.
- TextVN: 230+ replay scenario, 190+ assistant test, adapter simulator 3 OS.
- Areca: 22 CTest adapter, fake InputContext — tốt nhất về **hành vi adapter**.
- Lotus: 57 test headless, golden files, CI nặng.
- ibus-bamboo/OpenKey: nhiều người dùng nhưng test tự động ít.

**Kết luận:** ViFastKey dẫn về test engine/đặc tả; Areca/Lotus dẫn về test
adapter. Hai loại test khác nhau, không thay thế nhau.

### 4.3 Adapter Linux

**Ai dẫn:** Areca, fcitx5-lotus, SKey.

- Areca: 6 backend, trust probe, adaptive wait, mouse tracker, app
  compatibility, preedit/replace/redirect.
- Lotus: 3 chế độ uinput + surrounding + preedit, app rules có delay riêng,
  emoji picker.
- SKey: 5 output mode, per-app, Qt6.
- ViFastKey: preedit mặc định + surrounding guard + transaction barrier +
  field policy + AppProfile timing. Đủ cho P0 nhưng **không có fallback** khi
  app không xử lý surrounding, không uinput/native, không browser/address bar.
- pinakey công bố rõ no-underline thất bại ở terminal/LibreOffice/GNOME
  Wayland — điều ViFastKey cũng sẽ gặp nếu mở rộng direct rewrite.

**Kết luận:** ViFastKey đã thu hẹp khoảng cách sau đợt học Areca, nhưng vẫn
xếp sau 3 dự án trên về độ sâu adapter.

### 4.4 macOS

**Ai dẫn:** OpenKey, Gõ Nhanh.

- OpenKey: lâu năm, menu bar chín, auto-update, convert tool, nhớ charset theo
  app.
- Gõ Nhanh: sản phẩm hoá tốt, auto-restore tiếng Anh, ESC khôi phục, nhớ
  ON/OFF theo app, Homebrew, notarize (theo mô tả sản phẩm).
- ViFastKey: IMK thật, universal2, menu Telex/VNI, macro undo, app profile;
  **chưa ký/notarize**, chưa có event-tap fallback.
- VieLotus: native Swift, nhiều store, nhưng core đóng.

**Kết luận:** ViFastKey đủ dùng về kỹ thuật nhưng thua về độ chín sản phẩm
(signing, auto-update, UX). Nếu phát hành cho người dùng phổ thông, đây là
việc P0.

### 4.5 Windows

**Ai dẫn:** TextVN, OpenKey, BambooMintKey (TSF); ViFastKey ở nhóm hook-only.

- TextVN: TSF in-process, MS Store, grammar, prediction; nhiều tính năng nhất.
- OpenKey: hook + SendInput, ổn định, auto-update.
- BambooMintKey: TSF in-process COM, không inject key — kiến trúc đúng.
- ViFastKey: hook-only một binary, HKL EN-US, VI/JP pass-through, tray; TSF
  legacy không đóng gói.
- Gõ Nhanh: beta.

**Kết luận:** ViFastKey chọn hook-only để tránh cài đặt/phức tạp — đúng cho
bản portable, nhưng thua TSF về độ tương thích app hiện đại. Nếu muốn Windows
nghiêm túc, cần TSF hoặc giải thích rõ giới hạn hook.

### 4.6 Tính năng người dùng

**Ai dẫn:** Lotus, ibus-bamboo, TextVN, OpenKey.

- Lotus: 7 mode, 8 kiểu gõ, 17 charset, emoji picker, macro `$DATE`, double
  space → period, custom dict/keymap.
- ibus-bamboo: 2666 emoji, 5 mode không gạch chân, nhiều charset.
- TextVN: grammar assistant, prediction, transcode vùng bôi đen.
- OpenKey: convert tool, nhớ charset theo app, gõ tắt phụ âm.
- Funput: gợi ý trigram/context, flip composing.
- ViFastKey: 4 kiểu gõ, 16 charset, macro nhiều từ + undo + digits, app
  profile, gợi ý từ/emoji local, charset CLI. **Thiếu** emoji picker trong IME
  desktop, prediction, grammar.

**Kết luận:** ViFastKey đủ cho người dùng kỹ thuật, thiếu "delight" cho người
dùng phổ thông.

### 4.7 UX và cấu hình

**Ai dẫn:** OpenKey, Gõ Nhanh, Areca, Lotus, SKey.

- Areca: 5 tab, 9 theme, menu chọn backend theo app, preset "Best performance".
- SKey/Lotus: Qt6/Python GUI, mode menu bằng backtick.
- OpenKey: menu bar + control panel rất chín.
- ViFastKey: macOS AppKit + egui 6 tab (Windows/Linux), tray, import/export,
  screenshots; nhưng chưa có trên Linux dưới dạng gói, và chưa có mode menu
  kiểu Areca/SKey.

**Kết luận:** ViFastKey ngang về năng lực cấu hình, thua về độ hoàn thiện
trình bày và discoverability.

### 4.8 Phân phối và vận hành

**Ai dẫn:** Areca, Lotus, SKey, OpenKey, Gõ Nhanh, TextVN.

- Areca: installer tự nhận distro, .deb/.rpm/pacman/AUR, udev, CI matrix đa distro.
- Lotus/SKey: PPA/repo riêng, Nix, packaging.
- OpenKey/Gõ Nhanh: auto-update, Homebrew, store.
- TextVN: 60 release, MS Store, AUR, PPA, ký GPG/Sigstore.
- ViFastKey: web download, Windows zip, macOS package; **chưa có repo Linux,
  chưa auto-update, chưa ký**.

**Kết luận:** đây là khoảng cách lớn nhất giữa ViFastKey và top sản phẩm.

---

## 5. So sánh trực tiếp với sáu đối thủ chính

### 5.1 ViFastKey vs Areca

| Trục | ViFastKey | Areca |
|---|---|---|
| Engine | Riêng, 4 kiểu gõ, 16 charset, conformance | bamboo-core, Telex/VNI/VIQR |
| Test | 16k+ conformance + 676 test + adapter | 22 CTest adapter |
| Adapter Linux | Preedit + surrounding + barrier | 6 backend + probe + adaptive |
| Preedit mặc định | ✅ | ❌ (Rewrite mặc định) |
| Direct rewrite | Chỉ khi Replace | Mặc định |
| App compatibility | Capability + preset nhỏ | Danh sách hàng trăm app |
| Packaging | Chưa | Đầy đủ |
| License | MIT / Apache-2.0 | MIT |

**Kết luận:** Areca là **playbook adapter**; ViFastKey là **playbook engine +
đặc tả**. Hai dự án bổ sung nhau. Nếu ViFastKey muốn vượt Areca trên Linux,
phải làm fallback backend + trust probe + test app thật, không phải viết lại
engine.

### 5.2 ViFastKey vs fcitx5-lotus

| Trục | ViFastKey | Lotus |
|---|---|---|
| Tính năng người dùng | Đủ | Rất nhiều (7 mode, emoji, 17 charset) |
| Mode | Preedit/Replace | 3 uinput + surrounding + preedit + emoji |
| Test | Engine nặng, adapter vừa | 57 headless, adapter + golden |
| UX | egui 6 tab | Qt/Python, emoji picker |
| Engine | Riêng | C++ port bamboo-core |
| License | MIT / Apache-2.0 | GPL-3.0 |

**Kết luận:** Lotus thắng về trải nghiệm và số mode; ViFastKey thắng về tính
đúng engine, đặc tả và license. Nếu người dùng Linux muốn "nhiều tính năng",
Lotus là lựa chọn; nếu muốn "engine đúng, adapter an toàn", ViFastKey có nền
tốt hơn nhưng phải hoàn thiện adapter.

### 5.3 ViFastKey vs SKey

| Trục | ViFastKey | SKey |
|---|---|---|
| Engine | Riêng, 16 charset | Rust stateless, 14 charset |
| Output mode | 2 | 5 |
| GUI | egui | Qt6 |
| Test | 16k+ conformance | 8 file CTest + setup test |
| Packaging | Chưa | PPA/Fedora/openSUSE/Arch/Nix |
| License | MIT / Apache-2.0 | GPL-3.0 |

**Kết luận:** SKey là "Areca nhẹ": ít tham vọng hơn nhưng đủ mode và đóng gói
tốt. ViFastKey hơn về engine/test; thua về mode và phân phối.

### 5.4 ViFastKey vs ibus-bamboo

| Trục | ViFastKey | ibus-bamboo |
|---|---|---|
| Framework | Fcitx5 + IBus | IBus |
| Tính năng | Đủ, hiện đại | Rất nhiều (emoji, 5 mode) |
| Bảo trì | Đang phát triển | Đình trệ |
| Test | Mạnh | Yếu |
| Engine | Riêng | bamboo-core |

**Kết luận:** ibus-bamboo vẫn là lựa chọn GNOME phổ biến vì quen và nhiều
tính năng, nhưng tương lai không sáng. ViFastKey có thể thay thế nếu adapter
IBus được test kỹ trên Wayland.

### 5.5 ViFastKey vs OpenKey

| Trục | ViFastKey | OpenKey |
|---|---|---|
| macOS | IMK, chưa ký | Chín, auto-update |
| Windows | Hook-only | Hook + SendInput |
| Linux | Fcitx5/IBus | Chưa hoàn thiện |
| Tính năng | Charset/CLI/test | Convert tool, per-app charset |
| License | MIT / Apache-2.0 | GPL-3.0 |

**Kết luận:** OpenKey thắng tuyệt đối về độ chín macOS. ViFastKey thắng về
engine/test/đa nền tảng. Trên macOS, người dùng phổ thông nên chọn OpenKey/Gõ
Nhanh; người cần charset/CLI có thể chọn ViFastKey.

### 5.6 ViFastKey vs Gõ Nhanh

| Trục | ViFastKey | Gõ Nhanh |
|---|---|---|
| Engine | 4 kiểu gõ, 16 charset, 3 trạng thái | Validation-first, auto-restore |
| Auto-restore tiếng Anh | Escape Telex, không dictionary | ✅ tự động + ESC |
| macOS UX | IMK cơ bản | Sản phẩm hoàn thiện |
| Windows/Linux | Hook / Fcitx5+IBus | Beta |
| Test | 16k+ conformance | Test + CI, ít conformance |
| License | MIT / Apache-2.0 | BSD-3 |

**Kết luận:** Gõ Nhanh thắng về trải nghiệm gõ Anh/Việt và macOS; ViFastKey
thắng về độ phủ tiếng Việt, charset và đa nền tảng. Đây là đối thủ gần nhất
về triết lý "engine tốt + sản phẩm gọn".

### 5.7 Các dự án khác đáng chú ý

- **TextVN:** nhiều tính năng nhất (grammar, prediction, transcode), nhưng
  nặng và RC. Nếu ViFastKey muốn mở rộng tính năng, đây là bản đồ.
- **Funput:** kiến trúc crate sạch nhất; đáng học cách tách pure/stateful và
  gợi ý local.
- **telebit:** thuật toán rebuild âm tiết + doctor; đáng học nếu cải thiện
  engine.
- **CanType:** invariant rõ; đáng học cách viết hợp đồng trước khi code.
- **pinakey:** trung thực về giới hạn no-underline; đáng học cách công bố
  failure mode.

---

## 6. Scorecard (chủ quan)

Thang 1–5. Điểm dựa trên đọc code/docs, **không** phải đo thực tế. Dùng để
thấy hình dạng, không để tranh cãi điểm lẻ.

| Dự án | Engine | Test | Linux adapter | macOS | Windows | Tính năng | UX | Phân phối |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| **ViFastKey** | 5 | 5 | 3.5 | 3.5 | 3.5 | 4 | 3.5 | 2.5 |
| Areca | 3.5 | 4 | 5 | 1 | 1 | 3.5 | 4.5 | 4.5 |
| fcitx5-lotus | 4.5 | 4 | 5 | 1 | 1 | 5 | 4.5 | 4.5 |
| SKey | 4 | 4 | 4.5 | 1 | 1 | 3.5 | 4.5 | 4.5 |
| ibus-bamboo | 4 | 2 | 1 | 1 | 1 | 5 | 4 | 3.5 |
| OpenKey | 4.5 | 2 | 2 | 5 | 4 | 4.5 | 5 | 4 |
| Gõ Nhanh | 4 | 4 | 3.5 | 5 | 3 | 4 | 5 | 4.5 |
| TextVN | 4 | 4.5 | 3.5 | 4 | 4.5 | 5 | 3 | 4 |
| Funput | 4 | 4 | 3.5 | 3 | 3 | 4 | 3.5 | 2.5 |

**Đọc bảng:**

- ViFastKey có cột **Engine/Test** cao nhất và cột **Phân phối** thấp nhất.
- Không dự án nào có tất cả cột cao. Đây là lý do hệ sinh thái vẫn phân mảnh.
- Nếu mục tiêu là "sản phẩm cho người dùng phổ thông", Gõ Nhanh/OpenKey/Lotus
  đang dẫn. Nếu mục tiêu là "nền tảng kỹ thuật đúng", ViFastKey dẫn.

---

## 7. ViFastKey mạnh / yếu / ngang (tóm tắt)

### Mạnh hơn tất cả

1. **Oracle + conformance**: không ai có.
2. **Đặc tả hành vi**: `COMPATIBILITY.md` chi tiết đến mức kỳ lạ (theo nghĩa
   tốt).
3. **Bảng mã + CLI chuyển mã**: 16 charset, encode/decode, detect.
4. **Đa nền tảng thật**: 8 đích, cùng một engine.
5. **Adapter-core dùng chung**: field policy, surrounding guard, action plan,
   timing — một lần viết, nhiều port.
6. **License MIT / Apache-2.0** trong khi đa số đối thủ GPL.
7. **UEFI demo**: duy nhất trong khảo sát.

### Ngang top

1. **Engine Telex/VNI/VIQR + chính tả 3 trạng thái**.
2. **Macro** (nhiều từ, undo, digits, off-mode).
3. **App profile** (method/tone/spell).
4. **Preedit/replace + surrounding guard**.
5. **Test adapter** (ngang Areca/Lotus, hơn phần còn lại).

### Yếu hơn top

1. **Fallback backend Linux** (Areca/Lotus/SKey hơn).
2. **Trust probe ACK** (Areca hơn).
3. **uinput/Libei/XTest** (Areca/Lotus/SKey hơn).
4. **Browser/address bar heuristics** (Areca/SKey hơn).
5. **Finalized-word recovery** (Areca hơn).
6. **Adaptive wait** (Areca/Lotus hơn).
7. **Emoji picker** (Lotus/ibus-bamboo hơn).
8. **Prediction/grammar** (TextVN/Funput hơn).
9. **Packaging Linux** (Areca/Lotus/SKey hơn).
10. **Ký/notarize/auto-update** (OpenKey/Gõ Nhanh/TextVN hơn).
11. **UX macOS** (OpenKey/Gõ Nhanh hơn).
12. **TSF Windows** (TextVN/BambooMintKey/OpenKey hơn).

---

## 8. Khoảng trống và roadmap đề xuất

### P0 — trước khi nói "dùng được cho người thật"

| # | Việc | Vì sao | Tiêu chí |
|---|---|---|---|
| 1 | Ký + notarize macOS, stable signing | Gatekeeper/TCC; không ký thì người dùng không mở được | Cài trên máy sạch, không cảnh báo |
| 2 | Gói Linux `.deb`/`.rpm`/AUR + repo | Người dùng Linux không cài từ source | `apt install` chạy, có gỡ sạch |
| 3 | Fallback backend Fcitx5 | App không nhận surrounding sẽ mất chữ | Có ít nhất Preedit fallback + ForwardBackspace |
| 4 | Test app thật (Chrome, LibreOffice, terminal, VS Code) | Test giả lập không bắt được frontend | Checklist app + log |
| 5 | Auto-update hoặc thông báo phiên bản | Người dùng không tự cập nhật | Có cơ chế check/update |

### P1 — để ngang top

| # | Việc | Học từ |
|---|---|---|
| 6 | Trust probe surrounding + cache verdict | Areca |
| 7 | Browser/address bar guard | Areca/SKey |
| 8 | Finalized-word recovery | Areca |
| 9 | Emoji picker + gợi ý trong IME desktop | Lotus/ibus-bamboo |
| 10 | Chuyển mã GUI (không chỉ CLI) | OpenKey/SKey |
| 11 | Doctor script (`vfk-doctor`) | Areca/telebit |
| 12 | macOS event-tap fallback | OpenKey/Gõ Nhanh |

### P2 — khác biệt hoá

| # | Việc | Học từ |
|---|---|---|
| 13 | Auto-restore tiếng Anh thông minh | Gõ Nhanh |
| 14 | Prediction/grammar local | TextVN/Funput |
| 15 | "Cứu chữ"/"Cứu dấu" | Ngó Sen |
| 16 | Adaptive wait theo app | Areca/lilypad |
| 17 | uinput/Libei opt-in | Areca/SKey |

---

## 9. Kết luận

1. **ViFastKey không thua ở đâu về kỹ thuật nền.** Engine, test, đặc tả, đa
   nền tảng, license — đều ngang hoặc hơn top.
2. **ViFastKey thua ở tầng "sản phẩm"**: cài đặt, ký, auto-update, UX tiện
   dụng, và độ sâu adapter Linux. Đây là những việc không khó về ý tưởng nhưng
   tốn thời gian và cần test thật.
3. **Đừng cố thắng mọi trục.** Areca thắng adapter, Lotus thắng tính năng,
   OpenKey/Gõ Nhanh thắng macOS. ViFastKey nên giữ lợi thế **engine đúng +
   conformance + MIT/Apache-2.0 + đa nền tảng**, rồi chọn 2–3 trục để đuổi:
   Linux fallback, macOS signing, packaging.
4. **Học Areca ở adapter, không học ở engine.** Engine ViFastKey đã có nhiều
   edge case mà bamboo-core không có (`allow_coda_k`, `đ` viết tắt, VNI literal
   number, open `uơ/ươ`); bê nguyên Areca sẽ làm chậm và phình.
5. **Học Gõ Nhanh ở UX, không học ở engine.** Auto-restore/ESC là thứ người
   dùng cảm nhận ngay; nhưng validation-first của họ không thay thế được mô
   hình âm tiết của ViFastKey.

---

## 10. Nguồn

- [`survey-all-imes.md`](survey-all-imes.md) — phân tích 31 repo.
- [`areca-ime.md`](areca-ime.md) — sổ tay Areca.
- ViFastKey: `README.md`, `docs/PLATFORMS.md`, `docs/COMPATIBILITY.md`,
  `docs/CONFIG.md`, `docs/IME_FEATURE_RESEARCH.md`, `docs/research/`,
  `linux/fcitx5/README.md`, `rust/crates/vfk-ffi/include/vifastkey.h`.
- Tracker: <https://hthienloc.github.io/vietnamese-ime-tracker>.
