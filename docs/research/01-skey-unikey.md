> **Báo cáo nghiên cứu nội bộ.** Tài liệu này chỉ ghi nhận hành vi, kiến trúc, UX và
> test fact của các dự án được khảo sát. Không có mã nguồn, bảng dữ liệu, từ điển hay test
> corpus GPL/AGPL nào được sao chép vào ViFastKey. Xem `THIRD_PARTY_NOTICES.md` và
> `docs/IME_FEATURE_RESEARCH.md`.

# Cross-project analysis: SKey, SKey-Engine, Bamboo Core, UniKey → ViFastKey

Prepared for: **ViFastKey** (MIT-licensed Vietnamese IME engine, Python oracle + Rust port)  
Mode: read-only behaviour extraction. **No source code from copyleft projects is copied into this report or should be copied into ViFastKey.** Test expectations below are re-stated as input/output facts.

Snapshots analysed:

| Project | Revision / version |
|---|---|
| `skey` | commit `2628606358eea80a5ffccb1be09362895c6da95f`, 2026-10-29 |
| `skey-engine` | commit `4481412f851343f45139feb69780aec1399ade55`, 2026-10-25 |
| `bamboo-vi_bamboo-core` | commit `06d4ff725f580e9e38034af2fa7e6400720b6951`, 2026-10-25 |
| `x-unikey` | 1.0.4 (ChangeLog, 2006) |
| `unikey362` | UniKey 3.62 Windows source (2005) |

---

## 0. Executive summary

- **SKey** has the strongest *adapter/product* layer: per-IC state, per-app output modes, Chromium address-bar handling, Uinput/surrounding-text/preedit/native backends, backspace acknowledgement, accessibility field detection, app delay learning, macros, user dictionary.
- **skey-engine** is a compact stateless `raw keys → Vietnamese string` core. It is useful as a second oracle for Telex/VNI/VIQR, tone placement, undo and toggle-off behaviour, but it has a serious configuration bug: its `set_free_marking` actually changes **tone style**, not free marking.
- **bamboo-core** is the best *MIT-licensed* algorithmic and test reference. It has a transformation-stack engine, 9 input presets, 17 charsets, 3-state spelling, rebuild/restore, snapshot backspace and a large, high-quality test suite. Some flag names/labels are inverted relative to UniKey, so use its **outputs**, not its labels.
- **Classic UniKey** (x-unikey 1.0.4) remains the authoritative reference for **semantics**: CVC validation, `gi`/`qu`, tone restrictions on stop codas, auto-restore, macro matching, user keymaps, and the 213-character charset pivot.
- **UniKey 3.62** is older engine code but a valuable reference for **legacy charset lists**, conversion/clipboard tooling, macro file format, and Windows-era options (`ToneNextToVowel`, `AlwaysMacro`, `ClipboardConvert`).
- **ViFastKey already has** 3-state spelling, old/new tone, undo/latch, smart caps, auto-capitalise, soft reset, app modes, preedit/replace, surrounding edit, NFD, C ABI/WASM and strong conformance. Do not rewrite those parts.
- **Highest-value ViFastKey gaps**: (1) `đ` abbreviations and `dd` after a consonant/uppercase (`ddc`, `vcdd`, `NDD` fail today); (2) legacy charset conversion; (3) the open `uơ`/`ươ` onset rule (`luowj` shows `luợ` instead of `lượ`); (4) dictionary-mode auto-restore with user words; (5) macro suffix/off-mode and adapter robustness.

---

## 1. Project inventory

| Project | License | Language / size | Architecture | Main contribution |
|---|---|---|---|---|
| **skey** | GPL-3.0 | C++17 + Qt6, ~26k LOC of `.cpp/.h` (including tests) | Fcitx5 addon. `SKeyState` per input context wraps a stateless `skey-engine` via C FFI. Separate output backends: surrounding text, Uinput, preedit, native XTest/libei. Qt6 settings app, a11y monitor, app-mode/delay config. | Adapter UX, mode auto-detection, injection/backspace transport, app profiles, macro/user-dict UI, real-world edge-case tests. |
| **skey-engine** | GPL-3.0 | Rust, ~5.1k LOC (`src`), ~1.0k LOC tests | Stateless `input → output` transform over a `Vec<Letter>`; four methods (Telex, VNI, VIQR, TeipVNI); 14 charsets; embedded 7,885-word dictionary; C FFI. | Compact letter-accumulator model, hook/roof gating, dictionary-prefix auto-restore, 70-VSeq parity tests. |
| **bamboo-core** | MIT | Rust, ~13k LOC (`src`), ~6.3k LOC tests | Stateful engine: `TransformationStack` of append/mark/tone operations, rule tables from input methods, lazy JIT DFA, 8-byte snapshots, commit/raw buffers, rebuild/restore, FFI + WASM. | MIT-compatible algorithms, 9 presets, 17 charsets, 3-state orthography, advanced test corpus, delta API. |
| **x-unikey** | LGPL-2.1+ engine, GPL-2.0 `vnconv`/GUI, permissive IMdkit | C/C++, ~25k LOC | Classic XIM server + GTK IM module + `UkEngine`. Output-character buffer with `WordInfo` (form, consonant/vowel offsets, VSeq/CSeq) plus raw keystroke buffer. VSeq/CSeq/VCPair lookup tables. | Authoritative classic semantics: CVC validation, tone target, hook/roof, undo, auto-restore, macro matching, user keymaps, 18 charset names. |
| **unikey362** | GPL-2.0 text + separate “non-commercial redistribution” notice; `rtfio.dll` proprietary | C++, ~15k LOC | Windows keyboard-hook engine (`VietKey`) with a 40-char output buffer and bit-packed `DT/BD/BK/BW/BT` tables; Win32 settings GUI, converters, RTF/HTML clipboard support. | Legacy charset list, conversion tooling, macro format, old options, docs/config samples. |

Architecture notes:

- **skey** is split deliberately: the Rust engine only transforms a complete raw word. All composition state, English bypass, macro matching, surrounding-text repair, deletion counts and per-app routing live in C++ (`src/engine.cpp`, `src/vietnamese.cpp`).
- **skey-engine** is stateless by design: every key calls `skey_engine_transform(raw_input)`. Its README says “~700 lines” but the current `src` is ~5.1k lines and includes charset conversion.
- **bamboo-core** keeps state and supports both `process_key` (key-by-key) and `process` (batch/testing only). The host owns preedit, commit, dictionaries, macros and platform integration (README “C and C++ integration”).
- **x-unikey** is the ancestor design that every later engine partially reimplements: one `WordInfo` per *output* character, plus a key-stroke buffer used only for auto-restore.
- **UniKey 3.62** uses a different, older buffer engine. Do not treat it as the same algorithm as x-unikey 1.0.4; its value is configuration, charset and UX history.

---

## 2. Feature inventory

### 2.1 Input methods

| Project | Methods / presets | Notes and references |
|---|---|---|
| **skey** | Telex, VNI in UI (`src/config.h:10-27`). Engine also supports VIQR and TeipVNI via FFI (`skey_engine.h` methods 2/3), not exposed by the addon. | `shortW` (bare `w→ư`), `bracketUO` (`[ ] → ơ ư`), VNI digits, no custom keymap. |
| **skey-engine** | Telex, VNI, VIQR, TeipVNI (combined Telex+VNI). | `src/engine.rs`: `convert_telex` (239), `convert_teip_vni` (555), `convert_viqr` (861), `convert_vni` (956). |
| **bamboo-core** | Telex, VNI, VIQR, Microsoft layout, Telex 2, Telex + VNI, Telex + VNI + VIQR, VNI French (AZERTY), Telex W. | Data-driven in `src/input_method/definitions.rs:6-182`; `InputMethodPreset` in `preset.rs`. |
| **x-unikey** | Telex, Simple Telex, VNI, VIQR, “MS VI” (Microsoft layout), USER keymap. | Mappings in `src/ukengine/inputproc.cpp:100-210`; user keymap actions in `usrkeymap.cpp` + `doc/keymap-syntax`. |
| **UniKey 3.62** | Telex, VNI, VIQR, VIQR*. | `keyhook/vietkey.cpp`, `keycons.h`; VIQR* uses `*` as horn. |
| **ViFastKey** | Telex, Simple Telex, VNI, plus optional Quick Telex (`cc=ch`, `gg=gi`, …). | `python/vfk_core/rules.py`; data-driven `InputMethodSpec`; no VIQR/custom keymap yet. |

Notable input-method details:

- **Simple Telex** removes standalone `w→ư` and the `[ ] { }` bracket mappings; it is not just “Telex without tones”. Unikey `SimpleTelexMethodMapping` is the reference.
- **Telex Pro / user keymap** removes bracket keys and maps `w` to `Hook-Bowl`; actions are named (`Roof-A`, `Hook-UO`, `Telex-W`, `D-Mark`, `Tone0..5`, `a^`, `u+`, `dd`, …). A user method is a text file of one-key `key = action` lines; alpha keys apply to both cases.
- **Microsoft layout** maps digits and punctuation (`1=a(`, `2=a^`, `0=đ`, `[=ư`, `]=ơ`, …); it is a useful preset if ViFastKey later supports “other layouts”.
- **VIQR** uses `^ ( + *` for marks, `' \` ? ~ .` for tones, `dd` for đ, `\` as escape; the output side must escape punctuation that would otherwise change the previous vowel. Unikey’s VIQR decoder has a “smart/suspicious” mode for legacy mixed text.
- **Quick Telex** is an OpenKey convention, not a UniKey standard: `cc→ch`, `gg→gi`, `kk→kh`, `nn→ng`, `qq→qu`, `pp→ph`, `tt→th` at word start.

### 2.2 Tone, marks and special options

| Feature | Behaviour in the surveyed projects | ViFastKey status |
|---|---|---|
| Old/traditional tone | Open `oa`, `oe`, `uy` put the tone on the **first** vowel: `hòa`, `thúy`, `khỏe`. | `ToneStyle.OLD`; correct. |
| New/modern tone | Open `oa`, `oe`, `uy` put the tone on the **second** vowel: `hoà`, `thuý`, `khoẻ`. | `ToneStyle.NEW`; correct. |
| Free tone marking | Tone key may be typed after the vowel cluster/coda; tone target is recomputed. | Always enabled. |
| Free mark marking (`^ ( +`) | Unikey 3.62 `FreeMarking` allows marks after the base, not necessarily adjacent. x-unikey 1.0.4 ChangeLog says FreeMarking was removed (“now always enabled”), although the field remains. | Always enabled; no option. |
| Manual tone placement (`ToneManual` / `ToneNextToVowel`) | Windows UniKey 3.62: tone goes on the nearest vowel before the tone key (within 2 chars), bypassing normal cluster rules. x-unikey removed it as “not much of use”. | Not present; low priority. |
| Tone on stop codas | `c/ch/p/t` allow only sắc/nặng; `huyền/hỏi/ngã` are rejected (`batf` stays raw). With `allow_coda_k`, `k` joins the stop-coda set. | Present, including `allow_coda_k`. |
| `z` / VNI `0` | Classic UniKey: remove tone only; if no tone, the key is literal. ViFastKey currently removes a mark when no tone exists (`awz→a`, `aaz→a`), which diverges. | Needs a deliberate compatibility decision. |
| `gi` / `qu` | Treated as onset digraphs: `gi` + another vowel (`giá`), `qu` + another vowel (`quá`); `gi+i+coda` and `qu+u+coda` invalid. | Present. |
| `ưo` / `uơ` / `ươ` | The hardest cross-project area; see §3.4 and §4.6. | Partially present; open `uơ` collapse is too aggressive. |
| Tone retargeting | When adding a vowel/coda or changing a hook/roof, the tone is moved to the new correct target; “last tone key wins”. | Recomputed from state every render, so retargeting is naturally correct for complete words. |

**Tone-style naming trap:** all labels are ambiguous in prose. Use exact strings:

- old/traditional: `hòa`, `thúy`, `khỏe` — tone on the first vowel;
- new/modern: `hoà`, `thuý`, `khoẻ` — tone on the second vowel.

UniKey `ModernStyle = Yes` means **new/modern** (`hoà`). ViFastKey `ToneStyle.NEW` matches UniKey ModernStyle=Yes. SKey’s `modern=true` also matches, but its adapter’s `setFreeMarking(false)` accidentally forces `modern=false` (see §7).

### 2.3 Spelling validation

- **UniKey/x-unikey**: `VSeqList` has per-sequence `complete` (can stand alone) and `conSuffix` (can take a coda), plus `roofPos`/`hookPos` and `withRoof`/`withHook` links. `CSeqList` marks which consonants can be codas. `VCPairList` enumerates valid vowel+coda pairs. `isValidCV` restricts `k` to front vowels; `isValidVC`/`isValidCVC` handle `gi`/`qu` and exceptions (`quyn`, `quynh`, `gieng`, `giêng`).
  - References: `x-unikey/src/ukengine/ukengine.cpp:77-148` (VSeq), `156-186` (CSeq), `207-256` (VCPair), `342-419` (validators).
- **skey-engine**: bitmask tables for first consonant (`FC_0..4`), vowel (`VO_0..7`), last consonant (`LC_0..4`), `CV_ALLOWED_MASKS`, `VC_ALLOWED_MASKS`; CVC split with `gi`/`qu` absorption; `k` front-vowel rule; stop-coda tone check is separate. `spelling.rs:440` is the entry point.
- **bamboo-core**: 3-state `VALID/PARTIAL/INVALID`; relaxed prefix sets (`_RELAXED_ANY`, `_RELAXED_FOR_CODA`) allow free marking and mid-word prefixes. It also supports loanword onsets (`kr`), final `k` and the `ueu` rime; with `auto_correct=false` these forms (including host-supplied dictionary words) can compose instead of being restored. References: `src/orthography/spelling.rs`, `src/orthography/syllable.rs:94-227`, tests `free_onset_spelling.rs`, `dict_word_composition.rs`.
- **UniKey 3.62**: older table-driven buffer engine, no 3-state result and no newer spell-check/auto-restore layer; it builds the output buffer and applies marks/tone directly. Useful for historical charset/config/UX reference, not for modern free-marking validation.
- **ViFastKey**: 3-state model is already strong; it handles free marking, `gi/qu`, stop codas, `allow_coda_k`, and relaxed prefixes. Its main spelling gap is **onset-specific `uơ`** (see §3.4) and optional loanword/rare-rime support.

Important spelling cases worth keeping/porting:

| Keys | Expected | Note |
|---|---|---|
| `gim` / `gin` | raw | `gi + i + coda` is not a valid syllable. |
| `quu` | raw | `qu + u` invalid. |
| `batf` | `batf` | stop coda rejects huyền. |
| `achs` / `achj` | `ách` / `ạch` | sắc/nặng allowed. |
| `aaps` / `aapj` | `ấp` / `ập` | same rule. |
| `ieecs` / `ieecj` | `iếc` / `iệc` | same rule. |
| `tawsk` (with coda-k) | `tắk` | `ă` + `k` allowed only sắc/nặng. |
| `buks` (with coda-k) | `búk` | `u` + `k` allowed. |
| `tak` | invalid/raw | `a + k` is not a rime even with coda-k. |
| `kroong` | `krông` | loanword onset, optional. |
| `khueeuf` | `khuều` | rare `uêu` rime, optional. |

### 2.4 Undo, backspace, restore and word history

- **UniKey/x-unikey** keeps an output-character buffer and a raw key buffer. Backspace deletes the last **output character** (`tiếng → tiến`) and may move the tone if the deletion changes the tone target; raw key strokes are kept for auto-restore. References: `ukengine.cpp:1919-1976` (`processBackspace`), `2143-2193` (`restoreKeyStrokes`).
- **skey-engine** is stateless; the wrapper pops the last **raw key** and recomposes (`tiếng → tiêng`). This is *not* the same UX as UniKey. Bamboo exposes both behaviours: `undo_last_key()` (raw-key undo, snapshot) and `remove_last_output_char()` (display-character delete). References: `bamboo/src/engine/mod.rs:1207-1297`.
- **Repeating a transform key** cancels it once:
  - Telex `aaa→aa`, `eee→ee`, `ooo→oo`, `ddd→dd`, `ww→w`; after the toggle the key is literal for the rest of the word.
  - `ass` cancels the tone and types `as`.
  - SKey’s implementation uses `reverted`, `circ_toggled`, `d_toggled`, `horn_toggled`; Bamboo uses `english_bypass` after an undo **only if the result is not a valid prefix** (so `thooongf → thoòng` still composes).
  - ViFastKey uses `undone`, `latched` (tone undo latches the rest of the word literal) and `locked` (mark key locked). This is a good model; test it against the cases in §4.3.
- **Non-Vietnamese / literal mode**:
  - SKey: once a mark is toggled off, the rest of the word stays raw (`doww → dow`, `dowwnload → download`, `ww → w`, `wwayland → wayland`).
  - x-unikey: `m_reverted` and `m_singleMode` (the latter disables spell checking for abbreviations).
  - Bamboo: `english_bypass`; backspacing back to a valid prefix clears it (`eete< → et`, then `e` → `êt`).
  - ViFastKey: `literal` after backspace into a non-rebuildable word; `undone` prevents restore.
- **Soft reset / previous word**:
  - SKey stores a 16-entry `RetroWord` history; after deleting a separator, a tone key can reopen the previous word and change it (retroactive tone editing).
  - ViFastKey stores one `last` word and restores it when Backspace is pressed after a space (`Chaof ` + BS + `f` → `Chaof`); this works in REPLACE, not PREEDIT.
  - Bamboo has no word history; committed text/raw buffers are separate.
- **Rebuild from text**: Bamboo `rebuild_from_text()` can load arbitrary pre-existing text as if typed, so later keys can change tone/marks and `restore_last_word_as(RawKeystrokes)` can recover letters. Useful for adapters if they need to attach to a field with existing text.

### 2.5 Macro / auto-text

| Project | Matching | File/format | Case | Special |
|---|---|---|---|---|
| **x-unikey** | At word end, scan backwards up to 15 chars; tries the suffix and the suffix+1; can cross word breaks, so multi-word macro keys work. | `key:text`, one per line; old format VIQR, version 1 UTF-8 header; key buffer 16 (effective ≤15), 1024 text chars, 1024 entries, 128 KB. | No automatic case; users add `Cntt` and `cntt` entries. | `Shift+Space`/`Shift+Enter` bypasses macro; `alwaysMacro` allows expansion when Vietnamese is off. |
| **skey** | Exact raw input of current word, lowercased; `lookupMacro` exact then lowercase. | Fcitx config list `Key=Value`; UI tab; `macroInOffMode` and `capitalizeMacro` options (`capitalizeMacro` is wired; no engine use of `macroInOffMode` was found, so treat it as UI-only/unfinished). | Optional auto-capitalise first char. | Macro expansion happens on Enter/Tab/space and can use `surroundingCommit`; macro digits are not a separate option. |
| **UniKey 3.62** | At word stop, same suffix scan; key contains composed characters (tones included). | `key:text`, VIQR or UTF-8 depending on header/version. | Explicit entries. | `AlwaysMacro`; `macroEnabled`; macro text can be multi-word. |
| **ViFastKey** | Exact current composed word or raw input, lowercased. | Python dict / JSON `macros`; CLI `--macro`; C ABI config. | Auto: all-upper → upper; first-upper → first-upper; else as stored. | `macro_digits` keeps digits in the word (`mp3`, `k8s`); VNI unaffected; digits cannot start a word. |

ViFastKey gaps: no suffix/multi-word keys, no macro when Vietnamese is off, no Shift+Space bypass, no explicit “exact case” mode. These are P2 unless users rely on phrase macros.

### 2.6 Character sets

See the full matrix in **Appendix A**. Summary:

- **skey / skey-engine**: 14 charsets — Unicode, Unicode NFD, TCVN3, VNI-Windows, VISCII, VPS, BKHCM1, BKHCM2, Vietware-F, Vietware-X, ISC, VNI-MAC, WinCP1258, VIQR. No NCR or C-string.
- **bamboo-core**: 17 charsets — Unicode, Unicode “tổ hợp” (composite, not full NFD), TCVN3, VNI-Windows, Windows-1258, VIQR, VISCII, VPS, BKHCM1, BKHCM2, Vietware-X, Vietware-Full, UTF-8 literal, NCR decimal, NCR hex, Unicode C-string hex, Unicode C-string decimal. No ISC/VNI-MAC.
- **x-unikey**: 18 names — BKHCM1/2, ISC, NCR-DEC/HEX, TCVN3, UNI-COMP, UNICODE, UTF-8, UVIQR, VIETWARE-F/X, VIQR, VISCII, VNI-MAC, VNI-WIN, VPS, WINCP-1258.
- **UniKey 3.62**: 17 — Unicode, TCVN3, VNI Windows, VIQR, CP1258, UTF-8 Literal, NCR Decimal, NCR Hex, Composite Unicode, VISCII, VPS, BK HCM 1/2, Vietware X/F, X UTF-8, Unicode C String.
- **ViFastKey**: Unicode NFC and NFD only. No legacy charsets, no NCR, no C-string, no VIQR output, no clipboard conversion.

Architecture ideas:

- SKey/skey-engine use a **213-character internal index**: 186 Vietnamese letters (12 vowels × 6 tones × 2 cases, plus consonants/đ) and 27 Western symbols. Each charset has a 213-entry output table; decode builds reverse maps.
- Bamboo uses a **Unicode char → encoded string** map. Decode matches longest encoded sequence, leaves ambiguous codes undecoded, folds case-only shared codes to lowercase, and adds VISCII control-code capitals. This is safer than “last table entry wins” reverse maps.
- x-unikey’s `vnconv` uses the same 213 pivot but supports 1-byte, 2-byte (base+tone), Unicode, UTF-8, NCR, NFD/composite, VIQR and C-string classes; `VnConvert` can convert memory, files and clipboard.
- **Clipboard/file conversion** is a separate product feature, not just an encoder: UniKey 3.62’s Toolkit converts clipboard/file, supports “swap charset”, writes RTF and HTML clipboard formats, handles CF_TEXT/CF_UNICODETEXT/CF_LOCALE, and stores a conversion signature so it can remember the last charset. `uvconvert` is the standalone file converter. x-unikey has `VnFileConvert` in `vnconv` but no comparable clipboard UI. This is a good model for a ViFastKey “Convert text…” tool.
- Legacy charsets are **not interchangeable Unicode**; TCVN3/ABC and Vietware Full re-use codes for upper/lowercase (font trick), VNI/CP1258 use two bytes (base plus tone/combining), VIQR is ASCII with escapes, NCR/C-string are textual encodings. Any converter must define:
  - encode fallback for unmappable characters;
  - decode policy for shared/ambiguous codes;
  - whether `Unicode NFD` means full canonical NFD or “precomposed base + combining tone” (Vietnamese “tổ hợp”);
  - byte-vs-string return types.

### 2.7 Configuration and per-app profiles

- **SKey** (`src/config.h`, `src/settings/config_io.h`): input method, shortW, bracketUO, charset, output mode, native/uinput replacement, free marking, auto restore, dictionary, show preedit, a11y auto-enable, Chromium address-bar mode, debug, auto-delay, macro enable/capitalise/off-mode, mode-menu key, icon.
  - `skey-app-modes.conf`: `program=Auto|Uinput|Surrounding Text|Preedit|Native|Excluded`.
  - `skey-app-delay-overrides.conf`: `app@x=auto|paceMs,preCommitMs,postCommitMs`.
  - `skey-app-delays.conf`: learned round-trip statistics (`samples`, `rtUsec`, `lastSleepUsec`).
  - `skey-macro.conf`: key/value list. `user-dict.txt`: one word per line, `#` comments.
- **Bamboo** (`src/config.rs`): `free_tone_marking` (default true), `std_tone_style` (default true; exact output caveat), `auto_correct` (true), `w2u_mode` (Disabled/NonStart/Everywhere), `bracket_mode` (Disabled/NonStart/Everywhere). No macro/dictionary in the core; host owns them.
- **x-unikey** (`doc/options`, `doc/unikeyrc`, `src/xim/ukopt.c`): input, charset, free style, modern style, macro file, user keymap, spell check, auto-restore non-VN, commit method (XSendEvent/forward), XIM flow, bell, locales, GTK-alone, window position. `ToneManual` exists in the sample but is not in the active option map (removed as low value).
- **UniKey 3.62** (`newkey/userpref.cpp`, `keydlg.cpp`): FreeMarking, ToneNextToVowel, ModernStyle, MacroEnabled, AlwaysMacro, UseUnicodeClipboard, UseIME, charset, switch key, autostart, GUI language, conversion in/out charset, clipboard convert, rich-text/minimal-font options.
- **ViFastKey** (`python/vfk_engine/ports.py`, `config.example.json`): input method, tone style, spell check, w_as_u, bracket keys, quick telex, coda-k, auto-restore English, macro enable/digits/table, auto capitalise, excluded apps, per-app on/off memory, app profiles, edit-previous-word, restore-after-space, smart caps, NFC/NFD. AppProfile: output mode hint, backspace delays, wait-ack, edit-previous-word, forced Vietnamese on/off.

SKey’s per-app model is the richest: the app profile selects an **output transport**, not a typing method; typing config remains global. Its mode values are:

| SKey mode | Meaning |
|---|---|
| `Auto` | Detect surrounding/preedit/Uinput/native from app + field + runtime failures. |
| `Uinput` | Backspace injection through `/dev/uinput`, with ack/settle. |
| `Surrounding Text` | Replace through Fcitx5 surrounding-text API. |
| `Preedit` | Compose in preedit, commit atomically. |
| `Native` | XTest/libei/portal injection. |
| `Excluded` | No Vietnamese processing. |

ViFastKey’s `AppProfile` is close in spirit but is mostly advisory today:

| ViFastKey AppProfile field | Current meaning |
|---|---|
| `output_mode` | `"preedit"` / `"replace"` hint; adapter must apply it. |
| `backspace_delay_ms` | Base delay between synthetic backspaces. |
| `delay_per_char_ms` | Extra delay per deleted character. |
| `wait_ack` | Wait for the app to process backspaces before inserting. |
| `edit_previous_word` | Allow surrounding-text previous-word editing. |
| `vietnamese` | Force Vietnamese on/off for this app. |

Gaps vs SKey: no address-bar mode, no per-app delay override user file, no learned delay, no failure/ack state, and `excluded_apps`/`app_modes` matching is exact rather than normalised.

### 2.8 Adapter / platform layer

- **SKey output modes** (`src/config.h`; `src/engine.cpp`):
  - **Auto** chooses between surrounding text, Uinput, preedit and native based on app, field capabilities, browser/terminal detection, accessibility snapshots and runtime failures.
  - **Surrounding Text** replaces the tracked word via Fcitx5 surrounding text; computes old/new word diff; falls back on invalid cache.
  - **Uinput** writes backspaces through `/dev/uinput`, tracks app loopback backspaces, has a delete acknowledgement (`uinput_delete_ack.h`), safety timeout, deferred commit and ordered replay so fast typing is not corrupted.
  - **Native** uses XTest on X11 or libei/portal on Wayland; off by default on Wayland because of portal permission prompts.
  - **Preedit** composes in a preedit region and commits atomically.
  - Address-bar special paths: Chromium first-word/full-replace, select-and-overtype on GNOME omnibox, Firefox address-bar detection, Ctrl+L/Ctrl+U invalidation, autocomplete repetition guards.
  - Field/app detection: terminal list plus `/proc` descendant scan, accessibility (AT-SPI2) text-entry/password detection, browser identity, Google Sheets cell tracking.
- **Bamboo** exposes a clean adapter contract:
  - `process_key_edit()` returns `delete_chars`, `delete_bytes`, `insert`; the host applies a suffix diff. `process_key()` + `output()` is the simple path.
  - `undo_last_key()` vs `remove_last_output_char()`; `restore_last_word_as(Vietnamese|RawKeystrokes)`; `rebuild_from_text()`.
  - `OutputOptions`: `TONE_LESS`, `MARK_LESS`, `LOWER_CASE`, `FULL_TEXT`, `PUNCTUATION_MODE`, `RAW`.
- **x-unikey** distinguishes `UkCharOutput` (commit composed text) from `UkKeyOutput` (send raw key events). `getSeqSteps()` computes backspace count in the **target charset** (1 for Unicode/UTF-8, byte length for legacy, half for decomposed), which matters for legacy apps.
- **ViFastKey** already has `Preedit`, `Replace(delete_before, insert, pass_key)`, `Commit(text, pass_key)`, `PassThrough`; FFI converts delete counts to UTF-16. Its AppProfile hints cover terminal/Chromium/Firefox/Electron/games, but the actual platform adapters are still to be built.

### 2.9 Tests and docs

- `skey/src/test_vietnamese.cpp` (~1,470 lines): Telex basics, compounds, common words, undo, backspace, auto-restore, dictionary, VNI, ShortW, BracketUO, cross-method, stress. Note: the wrapper sets `FreeMarking(false)`, which in skey-engine forces traditional tone placement, so some “modern/traditional” labels in comments are easy to misread; other expectations (`eesf`, `huef`, `thoaws`, `thuyt`) were corrected in Bamboo’s parity suite.
- `skey-engine/tests`: 70 x-unikey vowel sequences (`vseq_parity.rs`), tone placement/rare sequences (`parity.rs`), cross-validation against VSeq/VCPair (`cross_validate.rs`), stress (`stress_test.rs`), charset FFI (`charset_ffi.rs`).
- `bamboo/tests`: 28 files, ~6.3k lines. Highest value: `skey_parity.rs`, `skey_repo_tests.rs`, `horn_placement.rs`, `free_onset_spelling.rs`, `tone_on_invalid_word.rs`, `backspace_keeps_tone_target.rs`, `undo_keeps_tone.rs`, `bracket_transform.rs`, `w2u_mode.rs`, `charset_decode.rs`, `vi_uvie_comprehensive_suite.rs`, `vni_english_mode.rs`, `rebuild_from_text.rs`.
- `x-unikey/doc`: `options`, `unikeyrc`, `keymap-syntax`, `im-samples/{telex-std,telex-pro,vni,vni-new,viqr,microsoft}`, `ukmacro`, `manual`.
- `unikey362`: `readme.txt`, `license.txt`, `keyhook/keycons.h`, `newkey/encode.cpp` charset table, `newkey/userpref.cpp`, `newkey/mactab.cpp`, `newkey/keydlg.cpp`, `vnconv/`, `uvconvert/`.
- `ViFastKey`: 442 Python tests, 13,273 core conformance cases + 3,000 engine scenarios, golden TSV, corpus, Rust parity. This is already better test infrastructure than any surveyed project.

---

## 3. Algorithms / important implementation ideas

### 3.1 Four buffer models

| Model | Project | What it stores | Consequence |
|---|---|---|---|
| Output-character buffer + raw key buffer | x-unikey, UniKey 3.62 | One `WordInfo` per displayed character (`form`, offsets, `cseq`, `vseq`, tone, caps) plus raw key entries. | Natural display backspace (`tiếng → tiến`); auto-restore replays keys; form transitions must be maintained incrementally. |
| Letter accumulator, stateless | skey-engine | `Letter { c, is_vowel, variant, tone, is_dstroke, upper, flags }`; each call replays the whole raw string. | Simple, easy to audit, O(n²) per word but n ≤ ~24–128; no incremental state bugs. |
| Transformation stack | bamboo-core | Append/Mark/Tone operations with `target_idx`, effect value, case bit; snapshots per key; flatten to output. | Exact undo/backspace, O(1) snapshot restore, delta API; more complex. |
| Word state + prefix snapshots | ViFastKey | `WordState` letters + tone + flags; `Composer` caches per-prefix states; `rebuild()` for edits. | Good balance; already property-tested; keep. |

**Takeaway:** ViFastKey’s model is closest to skey-engine plus snapshots, and it is sufficient. Do not replace it with UniKey’s form buffer. Borrow from Bamboo only the operation-target idea if profiling shows backspace/recompute is a bottleneck.

### 3.2 CVC validation pipeline

A robust validator does all of the following, in order:

1. Normalize to lowercase/toneless for structure checks; keep tone separately.
2. Split trailing consonants (coda), then trailing vowel run, then leading onset.
3. Absorb `gi` and `qu` into the onset when followed by another vowel; reject `gi+i+coda` and `qu+u+coda`.
4. Reject `k` before non-front vowels (`k` only with `e ê i y` and their sequences).
5. Look up onset, vowel sequence and coda in tables; check CV and VC compatibility matrices.
6. Track `complete` (can stand alone) vs `can_take_coda`; use relaxed prefix sets for mid-word `PARTIAL`.
7. Restrict stop codas (`c ch p t`, plus `k` when enabled) to sắc/nặng.
8. Return `VALID`, `PARTIAL` or `INVALID`; do not collapse `PARTIAL` into invalid while the user may still complete the word.

UniKey’s VSeq table encodes 70 vowel sequences (single, diphthongs, triphthongs) with roof/hook positions. SKey’s bitmask tables are a compact equivalent. Bamboo’s 3-state version is the most useful for free marking and Vietnamese/English coexistence.

### 3.3 Tone target selection

A practical, cross-checked rule set:

1. If the vowel cluster contains `ơ` (`ươ`, `uơ`), put the tone on `ơ`.
2. Else if the cluster contains a marked vowel (`ê`, `â`, `ă`, `ô`, `ơ`, `ư`), use the project’s preference order; most use the rightmost/marked main vowel: `iê`, `uê`, `uô`, `ây`, `âu`, `ưa`, `ưu`, `uyê`.
3. If a coda is present, put the tone on the last vowel of the cluster (`toán`, `huỳnh`, `tiếng`).
4. If the cluster has 3 vowels, put it on the middle (`oái`, `khuỷu`, `ngoeo`), except `uyê` → `ê`.
5. If the cluster is open `oa`, `oe`, `uy`, use the first vowel for old/traditional and the second for new/modern.
6. Otherwise put it on the first vowel (`mùa`, `kìa`, `hai`).
7. After adding a vowel/coda or changing a hook/roof, recompute the target and move the existing tone; do not drop it.

Project nuances:

- **UniKey** `getTonePosition`: `roofPos` first, then `hookPos`, triphthong middle, modern `oa/oe/uy` second, else open first / closed last; `ươ`-family special-cases tone to the second vowel.
- **SKey** `tone_vowel_index`: rightmost marked variant in clusters of 1–2; triphthong middle except `uyê`; coda last; open `oa/oe/uy` style-dependent; default first. It also has explicit retargeting on new vowel push, `gi/qu`, triphthong expansion and trailing consonant.
- **Bamboo** `find_tone_target`: one vowel; two vowels prefer `ơ/ê` when both horns; otherwise `TONE_PREFERRED = [ơ, ê, ô, â, ă]`; `TONE_ON_SECOND = [oa, oe, uy, ue, uo]` for the non-std path; coda last; three vowels middle except `uye` third.
- **ViFastKey** `tone_index`: `ơ` first; first marked vowel; coda last; 3 vowels middle; `oa/oe/uy` style; default first. This matches the common cases; add tests for `ươ`, `uơ`, `uyê`, `gì`, `quýt`, `khuỷu`, `thuở`.

**Important:** `std_tone_style` / `modern` names are not portable. Assert exact strings (`hòa` vs `hoà`) in conformance.

### 3.4 Marks, hooks and the `uo` family

- **Telex `w`** is two-phase:
  1. Try to apply horn/breve to the current vowel cluster using a vowel-sequence table.
  2. If no target, optionally insert standalone `ư` (`short_w`/`w_as_u`), else treat `w` as literal.
- **Vowel-sequence table** (SKey `vseq.rs`): `uoi/uou → both u+o`; `uo → both`; `ua/ui/uy/ue → u`; `oa → a`; `oe → o`; single `a→ă`, `o→ơ`, `u→ư`. If no table entry, scan right-to-left for the first `a/o/u`.
- **Horn propagation:** when a horned vowel is followed by a compatible vowel, share the horn (`ư+o→ươ`, `ơ+u→ươ`); mark propagated horns so another `w` “confirms” rather than toggles off.
- **Hook ↔ circumflex swap:** `w` on `ô` becomes `ơ` (`uô→ươ`, `luộc + w → lược`); `aa/ee/oo` toggles circumflex; a second same key unmerges once; a third is literal.
- **Crossing rules:** a circumflex `a` may cross `y/u` (`ây`, `âu`), `e` across `u` (`êu`), `o` across `i` (`ôi`, `uôi`). Other vowels block the roof. Unikey uses VSeq `withRoof`; SKey encodes the allowable crossings inline.
- **Validity gates:** SKey only crosses/merges when the resulting CVC is a valid syllable (`crossing_roof_valid`) or unmerges when the result is valid (`unmerge_valid`). This prevents `speedtest → spedtest` and `avata → avta`.
- **`uo` / `uơ` / `ươ`**:
  - `uow` is the mechanical Telex step to `uơ` in Bamboo/uvie; SKey instead emits `ươ` always.
  - Unikey applies **both** horns for `uo + w`, except when the word is open and prefixed by `th`, where it puts the horn only on `o` (`thuơ`); its `getTonePosition` still lands on `ơ`.
  - Bamboo uses an **onset-class matrix**: open `uơ` is allowed only after `c h k kh qu th`; after other onsets the second horn spreads (`duơ → dươ`, `tuơ → tươ`, `nguơ → ngươ`). `th` is special-cased.
  - ViFastKey always collapses auto-horned `ươ` to `uơ` at word end. That is correct for `thuở`, `huơ`, `khuơ`, but wrong for `lượ`/`dươ`/`tườ`: it produces `luợ`, `duơ`, `tuờ`. A tone-aware, onset-aware collapse is needed.
- **Tone reposition after hook:** when `w` changes the vowel structure, recompute tone target and move the tone (`uô`→`ươ`, `ă`→`â` swaps, etc.). SKey has `reposition_tone_after_hook`; Unikey does it inline.

### 3.5 Undo / literal-mode algorithms

Use explicit state, not ad-hoc comparisons:

- `reverted` / `literal`: set when a mark is toggled off; all later transform keys in the word are literal.
- `circ_toggled`, `d_toggled`, `horn_toggled`: one-shot flags prevent re-merging after an undo.
- `undone`: user has intentionally undone; suppress auto-restore for this word.
- `latched`: tone undo (`ass`) makes the remainder literal; mark undo should only lock that mark key (ViFastKey already distinguishes this).
- `locked`: per-key literal set (`xooongj` keeps the cancelled `o` literal).
- `english_bypass`: Bamboo’s variant; enter only if post-undo output is *not* a valid prefix, and leave it when backspace returns to a valid prefix.
- `m_singleMode`: Unikey’s “spell-check off for this word” after an undo; prevents the engine from reinterpreting abbreviations.

### 3.6 Auto-restore decision trees

**UniKey (word end)**:

```
if auto_restore is off: keep composed
if last word has no Vietnamese mark: keep composed (nothing to restore)
if last word is non-Vietnamese by form/validation: replay raw keystrokes
else keep composed
```

It only restores at a word break and only if at least one key was converted; it uses the raw key buffer, not a dictionary.

**SKey engine (real time)**:

```
composed = transform(input)
if composed == input: return composed
if composed is all ASCII: return composed        # intentional English/toggle result
if has_vn_markers(input): return composed        # dd, aaa/eee/ooo, aww/oww/uww
if not valid(composed): return input             # rule mode
if dict_mode and not dict_known(composed): return input
```

`has_vn_markers` treats word-initial `dd`, `dd` after a consonant, triple vowels, and `aww/oww/uww` as intentional. `dd` after a vowel is English (`address`, `add`) and does not block restore. `dict_known` has two tiers:

- tone-marked output must be an exact dictionary/user word, or its tone-stripped base must be a **strict prefix** of a dictionary base (so `lượ` is kept while typing `lược`, but wrong-tone `lước` is not accepted just because `lược` exists);
- tone-less output is accepted if it is a dictionary base or a prefix of one (`phươc` while typing `phước`).

**Bamboo (real time)**:

```
if key caused an undo: do not auto-correct
if auto_correct and composition has transforms and is not a valid prefix:
    replace composition with raw keys; set english_bypass = true
```

`english_bypass` lasts until word boundary or until backspace returns to a valid prefix (`eete< → et`, then `e → êt`). This is an elegant way to avoid repeatedly fighting the user.

**ViFastKey (current)**:

- core: invalid transformed word → raw; at word end, `PARTIAL` with a vowel → raw;
- engine: at word end, if raw is English and final is not a Vietnamese common word → raw;
- undo respected via `undone`/`literal`.

**Recommended**: add a dictionary mode resembling SKey’s tiered predicate, and add a small “intentional VN marker” exception for `đ` abbreviations. Do not make auto-restore depend solely on English dictionaries.

### 3.7 Macro matching

UniKey’s algorithm is the most flexible:

1. Only at word-break/Enter/Tab; if Shift is held with Space/Enter, skip macro and pass through.
2. Start at the current last character; walk back to the word start; build a key sequence of up to 15 characters.
3. Look up the sequence and the sequence with its first character skipped; if not found, step back across a word break and try a longer, multi-word suffix.
4. On hit, replace from the matched start; append the triggering separator/key as appropriate; reset composition.
5. `alwaysMacro` allows matching when Vietnamese is off.

SKey’s simpler exact-current-word lookup is adequate for word macros; ViFastKey can add suffix/multi-word support without changing the table format. If ViFastKey adds macro files, use a versioned header or a documented UTF-8 format; do not inherit the old VIQR default.

### 3.8 Charset conversion architecture

A robust design is a small pure module:

```
VietChar = (base, mark, tone, upper)  # or a 213-index canonical table
encode(text, charset) -> bytes/string
decode(bytes/string, charset) -> text
remove_tone(text) -> text
to_nfd(text) / to_composite(text)
```

Recommendations:

- Use a **Unicode-character → encoded-sequence map** (Bamboo style) rather than a 213-entry table per charset, unless you need exact legacy byte compatibility. A canonical `VietChar` index makes tables 213 entries and is easier to validate against iconv, but it forces every charset into one layout.
- Decode **longest match first**. Fold case-only shared codes to lowercase; if a code is shared by different letters (e.g. BKHCM1 `Ặ`/`Ụ`), leave the encoded text undecoded rather than guessing.
- Add VISCII’s control-code capitals as decode-only entries (VISCII maps some uppercase vowels onto C0 bytes).
- Encode unmappable characters by passing them through unchanged (Bamboo) or by a documented fallback; never silently drop them.
- Keep **NFC** for internal state. Provide three output forms explicitly:
  - NFC,
  - full canonical NFD,
  - Vietnamese composite (“tổ hợp”: precomposed base with mark + combining tone).
  `Unicode NFD` in SKey is full NFD; Bamboo’s `UnicodeCompound` is the composite form; Unikey `UNI-COMP` is composite.
- NCR: decimal `&#7879;`, hex `&#x1EC7;` (Unikey uses uppercase hex digits); only non-ASCII characters are escaped.
- C-string: `\x` + up to 4 hex digits for characters that are non-ASCII or ambiguous (`0-9A-FxX`); this is a source-code encoding, not a display encoding.
- VIQR: escape `\`, tone/mark punctuation after a vowel, and punctuation that would otherwise extend the previous letter; decode `\` before ASCII punctuation.
- Do **not** feed legacy bytes into a normal UTF-8 commit API. SKey wraps each legacy byte as Latin-1 UTF-8 for Fcitx; that only works with legacy fonts/apps and is easy to corrupt. Keep conversion for clipboard/file export, or add an explicit legacy-commit adapter.

### 3.9 Adapter transaction algorithm (from SKey)

A safe replace transaction:

1. Track the last committed word (`old_word`), its display length and raw keys.
2. On a key, compute `new_word`; derive the minimal suffix diff.
3. Choose a transport:
   - surrounding text available and reliable → `deleteSurroundingText(old_len - common_prefix, …)` + commit suffix;
   - Uinput/native → send backspaces, wait for loopback/ack (or timeout), then commit;
   - preedit → update preedit and commit atomically on boundary/focus loss;
   - no safe transport → preedit.
4. If the app reports surrounding text that does not end with `old_word` after the operation, mark the transport/app broken and fall back; do not keep deleting blind.
5. On focus change/cursor move/field change: commit or discard preedit, clear word state, re-resolve app/field.
6. On repeated failure in Chromium/Electron, prefer preedit or select-and-overtype; on terminals, use preedit.
7. Backspace pacing: base delay + per-character delay; learn per-app round-trip time; never send a commit before the preceding backspaces are acknowledged.

Bamboo’s `process_key_edit` contract is a good model for the engine side: return `delete_chars`, `delete_bytes`, `insert`; the adapter applies the diff. ViFastKey already has `diff` at the Python layer; the C ABI exposes UTF-16 delete counts.

---

## 4. Concrete test cases worth porting

Source tags: **[S]** `skey/src/test_vietnamese.cpp`; **[SE]** `skey-engine/tests`; **[B]** `bamboo/tests`; **[U]** x-unikey docs/code; **[W]** UniKey 3.62; **[V]** ViFastKey current tests/docs. “raw” means the exact ASCII keys are returned.

### 4.1 Tone and compound vowels (Telex, old style unless noted)

| Keys | Expected | Source / note |
|---|---|---|
| `as` `af` `ar` `ax` `aj` | `á` `à` `ả` `ã` `ạ` | [S] basics; [SE] |
| `aas` `aws` `ees` `oos` `ows` `uws` | `ấ` `ắ` `ế` `ố` `ớ` `ứ` | [S] |
| `ais` `aos` `aus` `ays` | `ái` `áo` `áu` `áy` | [SE] |
| `aaus` `aays` `eeus` | `ấu` `ấy` `ếu` | [SE] |
| `ias` `ius` `ois` `oois` `owis` | `ía` `íu` `ói` `ối` `ới` | [SE] |
| `uas` `uis` `uwas` `uwis` `uwus` | `úa` `úi` `ứa` `ứi` `ứu` | [SE] |
| `uuw` `uuws` `uwu` | `ưu` `ứu` `ưu` | [SE] |
| `tieengs` `tieeps` `toans` | `tiếng` `tiếp` `toán` | [S] common words |
| `hoawcj` `khoer` `xuaans` | `hoặc` `khỏe` `xuấn` | [SE] |
| `thuees` `buoons` `cuowcs` | `thuế` `buốn` `cước` | [SE] |
| `ieeus` `oais` `oays` `oeos` | `iếu` `oái` `oáy` `oéo` | [SE] triphthongs |
| `uaays` `uoois` `khuyas` `khuyus` | `uấy` `uối` `khuýa` `khuýu` | [SE] |
| `nguoiws` `ruowus` `yeeus` | `ngưới` `rướu` `yếu` | [SE] tone target |
| `huowuf` `hueef` `thuees` | `hườu` `huề` `thuế` | [SE] rare |
| `eesf` | `ề` | [SE] tone override (s then f) |
| `saosf` `saojs` `toansr` | `sào` `sáo` `toản` | [B] `skey_repo_tests.rs` |
| `gisa` `gifa` `quisa` | `giá` `già` `quía` | [B] rapid retarget |
| `tofan` `toafn` `hoafn` | `toàn` `toàn` `hoàn` | [B] |
| `gias` `quas` `gif` `quyts` | `giá` `quá` `gì` `quýt` | [S]/[B] gi/qu |
| `uyeef` `uyees` `huyeenf` `nguyeexn` | `uyề` `uyế` `huyền` `nguyễn` | [SE] |

### 4.2 Old/new tone style (exact strings)

| Keys | Old/traditional | New/modern | Source |
|---|---|---|---|
| `hoaf` | `hòa` | `hoà` | [SE] `vseq_parity` |
| `oas` | `óa` | `oá` | [SE] |
| `oes` | `óe` | `oé` | [SE] |
| `uys` | `úy` | `uý` | [SE] |
| `khoer` | `khỏe` | `khoẻ` | [SE] |
| `thuys` | `thúy` | `thuý` | [V] golden |
| `hoafn` | `hoàn` | `hoàn` | closed coda, same |
| `toans` | `toán` | `toán` | closed, same |
| `quyt` | `quyt` | `quyt` | no tone |

### 4.3 Undo / toggle / non-Vietnamese mode

| Keys | Expected | Source / note |
|---|---|---|
| `ass` | `as` | [B] `undo_keeps_tone`; [V] |
| `looixfsx` | `lỗi` | [B] last tone wins |
| `aaa` `ddd` `eee` `ooo` | `aa` `dd` `ee` `oo` | [S] undo patterns; [B] |
| `uww` `uwfw` `ojww` | `uw` `ùw` `ọw` | [B] mark undo keeps tone |
| `bass` `stress` `jazz` `txt` `rx` `sx` | `bas` `stress` `jazz` `txt` `rx` `sx` | [B] `vi_uvie` |
| `aww` `ddoww` | `aw` `đow` | [B] double `w`/`dd` undo |
| `ww` `WW` | `w` `W` | [S]/[B] case-preserving undo |
| `xooongj` | `xoọng` | [V] locked mark key |
| `thooongf` | `thoòng` | [B] triple `o` stays composable |
| `doww` | `dow` | [SE]/[V] toggle-off |
| `dowwnload` | `download` | [SE]/[V] rest of word literal |
| `wwayland` `wwindow` | `wayland` `window` | [SE]/[V] short-w toggle |
| `resstore` (with P5 + restore) | `restore` | [S] wrapper-specific; do not copy P5 blindly |
| `ressponse` | `response` | [S] wrapper-specific |

### 4.4 Auto-restore, dictionaries and abbreviations

| Keys | Expected | Source / note |
|---|---|---|
| `world` `computer` `wood` `reboot` | same raw | [S] real-time restore |
| `aloo` `vaai` `ook` `wtf` | same raw | [S] invalid intermediate |
| `address` `add` `ladder` `odd` | same raw | [S] `dd` after vowel is English |
| `speedtest` `banana` | same raw | [SE] no letter loss |
| `baa` `mee` `caw` `mow` | `bâ` `mê` `că` `mơ` | [S] valid syllabic results kept |
| `ddc` `vcdd` `nddm` | `đc` `vcđ` `nđm` | [S] ddFreeStyle/abbrev |
| `ddi` `NDD` `NDd` `DDD` | `đi` `NĐ` `NĐ` `DD` | [S] uppercase/mixed |
| `kdd` `cmdd` `avcdd` | `kđ` `cmđ` `avcđ` | [S] dd after consonant |
| `luwocs` (rule mode) | `lước` | [S] structurally valid |
| `luwocs` (dictionary mode) | `luwocs` | [S] not a real word |
| `truwocs` `khoong` | `trước` `không` | [S] real words |
| `need` `force` | `need` `force` | [SE] non-words |
| `luow` `luowj` `luowjc` | `lươ` `lượ` `lược` | [S] dictionary prefixes |
| `phuowc` `phuowcs` | `phươc` `phước` | [S] base-prefix |
| Add user word `lước` | `luwocs → lước`; prefix `luowj` stays `lượ` | [SE] user-word override |
| `VieetjNam` auto-correct on/off | `VieetjNam` / `ViệtNam` | [B] camelcase |
| `data her mix this did ` | `data her mix thí đi ` | [V] English vs Vietnamese |
| `text law windows boss ` (dict off) | `text law windows bos ` | [V] no dictionary |
| `server boss pass miss ass ` | `server boss pass miss as ` | [V] dictionary |

### 4.5 Backspace, soft reset and rebuild

| Operation | Expected | Source / note |
|---|---|---|
| `tieengs` + BS | `tiến` | [V] display delete; UniKey-style |
| `tieengs` + BS + `g` | `tiếng` | [V] mid-word edit |
| `nguwowif` + BS | `ngườ` | [V] tone stays on ơ |
| `nguwowif` + BS + `ix ` | `ngưỡi ` | [V] replacing vowel changes tone target |
| `text` + BS | `tex` (stays literal) | [V] |
| `text` + BS + `t ` | `text ` | [V] |
| `abc vieejt` + BS×3 | `abc v` | [V] |
| `vieejt` + BS under NFD | NFD of `việ` | [V] UTF-16/NFD count |
| `Chaof ` + BS + `f` | `Chaof` | [V] soft reset |
| `tieeng ` + BS + `s ` | `tiếng ` | [V] soft reset + tone |
| `ko ` + BS + `s` with macro `ko` | `khôngs` | [V] soft reset must not undo macro |
| `eete<` / `eete<e` | `et` / `êt` | [B] backspace re-enters valid prefix |
| `tieengs` + BS (raw-key model) | `tiêng` | [S] skey wrapper; **not** UniKey default |
| `aws` + BS (raw-key model) | `ă` | [S] |
| `dduwowcj` + BS (display model) | `đượ` | [U]/[V]; raw-key model gives `đươc` |

### 4.6 The `uo` / `uơ` / `ươ` family (conflict table)

This is the most important place not to “unify” expectations blindly.

| Keys | skey-engine | bamboo | ViFastKey today | Notes |
|---|---|---|---|---|
| `uow` | `ươ` | `uơ` | `uơ` | SKey always both horns; Bamboo/uvie mechanical step. |
| `khuow` | `khươ` | `khuơ` | `khuơ` | Unikey special only for `th`; Bamboo restricts open `uơ` to `c h k kh qu th`. |
| `thuow` | `thươ` | `thuơ` | `thuơ` | `th` special in Unikey/Bamboo. |
| `huow` | `hươ` | `huơ` | `huơ` | Bamboo allows `h` class. |
| `duow` | `dươ` | `dươ` | `duơ` | ViFastKey bug: should spread to `dươ`. |
| `tuow` | `tươ` | `tươ` | `tuơ` | same. |
| `nguow` | `ngươ` | `ngươ` | `nguơ` | same. |
| `luow` | `lươ` | `lươ` | `luơ` | same; prefix of `lược`. |
| `luowj` | `lượ` | `lượ` | `luợ` | current preedit is wrong. |
| `tuowf` | `tườ` | `tườ` | `tuờ` | tone must land on `ơ` after both horns. |
| `tuowfj` | `tượ` | `tượ` | `tuợ` | same. |
| `uowr` | `uở` | `uở` | `uở` | no onset, uơ allowed. |
| `thuowr` | `thưở` | `thuở` | `thuở` | `th` special. |
| `huouw` | `hươu` | `hươu` | `hươu` | horn spread before coda. |
| `thuoiwx` | `thưỡi` | `thưỡi` | `thưỡi` | tone target. |
| `huwowu` | `hươu` | `hươu` | `hươu` | explicit both horns. |
| `truwowjt` | `trượt` | `trượt` | `trượt` | long form. |
| `dduowngf` | `đường` | `đường` | `đường` | completed word. |
| `uwo` / `uwon` | `ưo` / `ươn` | `ưo` / `ươn` | `ươ` / `ươn` (mid-word) | ViFastKey's display forces `ươ`; Bamboo keeps `ưo` until coda. |
| `uo7` (VNI) | `uơ` | `uơ` | `uơ` | same family. |
| `duo7` (VNI) | `dươ` | `dươ` | `duơ` | same bug. |

**Recommendation:** adopt a named onset-class policy (`uơ` allowed only after `c h k kh qu th`, plus empty onset; `th` special), apply it only when the cluster is open, and do not collapse when a tone is present unless the onset allows `uơ`. Add both `luowj → lượ` and `thuowr → thuở` as tests.

### 4.7 Spelling, loanwords and rare rimes

| Keys | Expected | Source |
|---|---|---|
| `boawjm` `khoawsm` `noawfm` | `boặm` `khoắm` `noằm` | [B] free onset pairs |
| `nhoawngf` | `nhoằng` | [B] |
| `kroong` | `krông` | [B] loanword onset |
| `buks` / `tawsk` (coda-k) | `búk` / `tắk` | [B] |
| `tak` (coda-k) | raw/invalid | [B] |
| `khueeuf` | `khuều` | [B] `ueu` rime |
| `gim` `gin` `quu` | raw | [S]/[B] invalid |
| `VieetjNam` (auto-correct off) | `ViệtNam` | [B] camelcase |
| `enlf` `eds` `olkf` | raw | [B] tone after invalid word is a letter |
| `enlaf` | `enlà` | [B] valid syllable after invalid prefix; note ViFastKey treats whole word and returns raw |
| `mymfyk` | `mỳmyk` | [B] tone stays on earlier syllable; policy decision, not recommended blindly |
| `batf` | `batf` | [S]/[B] stop coda |

### 4.8 VNI

| Keys | Expected | Source |
|---|---|---|
| `a1` `a2` `a3` `a4` `a5` | `á` `à` `ả` `ã` `ạ` | [S]/[B] |
| `a6` `e6` `o6` `a8` | `â` `ê` `ô` `ă` | [S]/[B] |
| `o7` `u7` `d9` | `ơ` `ư` `đ` | [S]/[B] |
| `a61` `a81` `e63` `o71` `u72` | `ấ` `ắ` `ể` `ớ` `ừ` | [S] |
| `tie6ng1` `Vie6t5` | `tiếng` `Việt` | [S]/[B] |
| `d9u7o7c5` `kho6ng` | `được` `không` | [S]/[B] |
| `ngu7o7i2` `nhu7ng4` `tru7o7ng2` | `người` `những` `trường` | [S] |
| `go366` | `gỏ6` | [B] mark undo keeps tone |
| `enl2` | `enl2` | [B] tone after invalid word |
| `u7o7ng` `uo7ng` | `ương` | [B] |
| `uo7` `khuo7` | `uơ` `khuơ` | [B] |
| `uo73` `huou7` | `uở` `hươu` | [B] |

### 4.9 Charset conversion

| Operation | Expected | Source |
|---|---|---|
| All vowels round-trip TCVN3, VNI-Win | identical | [SE] `charset.rs` tests |
| `remove_tone("tiếng Việt")` | `tiêng Viêt` | [SE] |
| `remove_tone("được")` | `đươc` | [SE] |
| VNI-Win `"tiếng Việt"` | `"tieáng Vieät"` | [B] |
| TCVN3 `"TIẾNG Việt"` encode→decode | `"TIếNG Việt"` | [B] shared uppercase/lowercase codes |
| VIQR `"mưa."` | `mu+a\.` (then decode back) | [B] |
| VIQR `"Hà Nội?"` | ``Ha` No^.i\?`` (decode back) | [B] |
| VIQR input containing an ordinary backslash | encode each `\` as `\\`; decode back | [B] |
| VIQR decode `"tie^'ng Vie^.t"` | `tiếng Việt` | [B] |
| NCR decimal `Vi&#7879;t` | `Việt` | [B] |
| NFD `e\u302\u301` | unchanged by composite charset; `\uEA\u301` → `ế` | [B] |
| WinCP1258 `"Ứ"` | bytes `0xDD 0xEC` (`Ýì` in Latin-1 view) | [B] |
| WinCP1258 `"Đ"` | `0xD0` | [B] |
| VISCII bytes `02 05 06 A0 14 19 1E` | `ẲẴẪÕỶỸỴ` | [B] |
| BKHCM1 `"Ụ"` | encode/decode returns the encoded sequence unchanged (ambiguous) | [B] |

### 4.10 Macro, config and adapter scenarios

| Scenario | Expected | Source |
|---|---|---|
| `ko Ko KO ` (macro `ko`) | `không Không KHÔNG ` | [V] |
| `vn ` (macro `vn`) | `Việt Nam ` | [V] |
| `mp3 k8s MP3 123 ` with `macro_digits` | expansions + `MÁY NGHE NHẠC` etc.; `123` not a macro start | [V] |
| `mp3 ` without `macro_digits` | `mp3 ` | [V] |
| VNI `a1 ` with `macro_digits` | `á ` | [V] VNI unaffected |
| `ko ` + BS + `s` | `khôngs` | [V] soft reset not after macro |
| `Shift+Space` after macro key | macro bypassed, normal space | [U] |
| `alwaysMacro` while Vietnamese off | macro expands | [U]/[W] |
| app toggle `notepad` off, focus `chrome` | re-enabled on chrome, off again on notepad | [V] |
| password/number/email/URL field | pass-through, no preedit | [V] |
| focus out during preedit | commit word | [V] |
| cursor move/click | flush/commit, no stuck preedit | [V] |
| modifier alone (Shift/Option) | word preserved | [V] |
| broken JSON config | defaults | [V] |
| unknown config field | ignored | [V] |
| unknown input method | Telex fallback | [V] |
| `auto_capitalize` after `. ` / `? ` / Enter | next word capitalised | [V] |
| `smart_caps` `JSON`, `HTTP` | literal, not Vietnamese | [V] |

---

## 5. ViFastKey gap analysis

### 5.1 Already strong — do not regress

- 3-state spelling (`VALID/PARTIAL/INVALID`) and free tone placement.
- Old/new tone style with correct exact strings (`hòa` vs `hoà`).
- Telex/VNI/Simple Telex/Quick Telex; `w_as_u`, bracket keys, `allow_coda_k`.
- Undo/latch/locked semantics; `ass`, `aaa`, `doww`, `wwayland`.
- Display-character backspace and rebuild; NFD handling; UTF-16 FFI.
- English restore via `Dictionary`; macro case/digits; smart caps; auto-capitalise.
- App modes/field kinds/soft reset/surrounding edit; preedit/replace actions.
- 442 Python tests + large conformance oracle + Rust parity.

### 5.2 Gaps / failing sequences (current Python oracle)

| Area | ViFastKey current | Expected / reference | Evidence | Priority |
|---|---|---|---|---|
| `đ` abbreviation after consonant | `vcdd`, `nddm`, `kdd`, `cmdd`, `avcdd` stay raw | `vcđ`, `nđm`, `kđ`, `cmđ`, `avcđ` | [S] `ddFreeStyle`; [SE] `has_vn_markers`; [U] `processDd` | **P0** |
| Uppercase `DD` after consonant | `NDD`, `NDd` stay raw | `NĐ`, `NĐ` | [S] uppercase tests | **P0** |
| Open `uơ` onset rule | `duow→duơ`, `tuowf→tuờ`, `luowj→luợ`, `nguow→nguơ`, `phuow→phuơ` | `dươ`, `tườ`, `lượ`, `ngươ`, `phươ` (unless onset allows `uơ`) | [B] `horn_placement.rs`; [B] `skey_repo_tests.rs` | **P1** |
| `d` at any position | only first `d` can become `đ`; final `d` shortcut works | `duowngd→đương`, `duowngfd→đường`, but `vcdd`/`NDD` missing | [U]/[S] | **P0** (same fix) |
| Dictionary mode | only small English/Vietnamese lists; structurally valid nonwords kept (`luwocs→lước`) | dictionary mode: `luwocs` raw, `truwocs` kept, prefix `phuowc`/`luowj` kept | [S] `dict_known`; [SE] | **P1** |
| User dictionary | no user word file/API | `user-dict.txt`; user words override embedded list | [S] `loadUserDict` | **P1** |
| Macro suffix / multi-word | exact current word only | `cn` inside `xyzcn`, phrase keys; suffix scan | [U] `macroMatch` | **P2** |
| Macro when Vietnamese off | not expanded (session returns PassThrough) | `alwaysMacro` / `macroInOffMode` | [U]/[W]/[S] config | **P2** |
| Macro Shift+Space bypass | none | skip macro, pass space | [U] | **P2** |
| Raw-key undo / restore last word | display BS only; one soft-reset word | `undo_last_key`, `restore_last_word_as(RawKeystrokes)` for adapters | [B] | **P1** API |
| Word history / retro tone editing | one `last` word; REPLACE only | 16-word ring; tone key after BS edits previous word | [S] `RetroWord` | **P2** |
| Charset conversion | NFC/NFD only | legacy charsets, NCR, VIQR, clipboard | §2.6/Appendix A | **P0** core |
| VIQR / other presets / custom keymap | Telex/Simple Telex/VNI only | VIQR, Microsoft, Telex 2, VNI French, user keymap | [B]/[U] | **P2** |
| Shifted bracket keys | `{`/`}` not mapped (`NG}{I` raw) | `NGƯƠI`, `{s→Ớ`, `}f→Ừ` | [B] `bracket_transform.rs` | **P1** small |
| Loanwords / rare rimes | `kroong`, `khueeuf`, `buks` rejected | `krông`, `khuều`, `búk` via options/dictionary | [B] | **P2** |
| `z` no-tone semantics | `awz→a`, `aaz→a` | UniKey: `z` literal when no tone; remove mark only by retyping it | [U]/[B]/[SE] | **P2** decision |
| Long-word/preedit overflow | >24 raw keys = literal | Bamboo commits head, continues; SKey 128-char buffer | [B] `preedit_overflow` | **P2** |
| App profile runtime | static hints only; no address-bar/delay/ack/fallback | SKey Auto mode, app modes/delay files, ack | [S] `engine.cpp` | **P1** adapter |
| App key normalisation | `excluded_apps`/`app_modes` exact; profiles normalise | path/basename/case/`.exe`/`.app`; WM_CLASS fallback | [S] `appProgram` | **P2** |
| Clipboard conversion | none | UniKey Tools/clipboard, RTF/HTML, signature | [W] | **P1** adapter |
| Preedit previous-word edit | REPLACE only | `DeleteSurrounding` action in preedit | [V] roadmap | **P2** |
| Accessibility / auto-delay / native injection | not built | SKey a11y, learned delay, Uinput/libei | [S] | **P2** adapter |
| Performance | O(n²) per key, max 24 | Bamboo snapshot/DFA, skey stateless | [B]/[SE] | **P2** |

---

## 6. Prioritized recommendations

### 6.1 P0 — correctness and foundational capability

| ID | Layer | Recommendation | Acceptance / rationale |
|---|---|---|---|
| **P0-1** | **Engine** | Fix `đ`/`Đ` handling: transform `dd`/`DD` when the preceding character is not a vowel (start or consonant), preserve uppercase, support `d` at word end as a shortcut, and exempt `đ` abbreviations from auto-restore. | `ddc→đc`, `vcdd→vcđ`, `nddm→nđm`, `kdd→kđ`, `cmdd→cmđ`, `avcdd→avcđ`, `NDD→NĐ`, `NDd→NĐ`, `DDD→DD`, `ddi→đi`, `add/address/ladder` stay raw. Port tests from [S] and [SE]. |
| **P0-2** | **Engine** | Design and implement a charset module: canonical `VietChar`/mapping, `encode`, `decode`, `remove_tone`; core set Unicode NFC, full NFD, TCVN3, VNI-Windows, VIQR, VISCII, WinCP1258, NCR dec/hex. Add raw-byte C ABI (`vfk_encode_charset`, `vfk_decode_charset`) with explicit length. | Per-letter and known-text round-trip for each charset; VIQR punctuation escapes; TCVN3 case ambiguity; VNI/CP1258 two-byte; NCR; unmappable policy. Do not copy GPL tables — see §7. |
| **P0-3** | **Tests** | Add a cross-engine golden suite: 70 VSeq × old/new, undo/toggle, auto-restore, abbrev, backspace, `uo` family, VNI, charset, macro. Keep expected I/O pairs in a neutral JSONL/TSV with source attribution. | All P0/P1 cases above run in Python and Rust; conformance export includes them; test comments cite source project, not copied code. |
| **P0-4** | **Docs** | Write a compatibility contract: tone style exact examples (`hòa` vs `hoà`), backspace model (display char vs raw key), macro matching, auto-restore levels, charset byte semantics, app profile fields, `z` decision. | New `docs/compatibility.md` + config reference; no ambiguous “old/new” without examples. This prevents future ports from importing SKey/Bamboo flag bugs. |

### 6.2 P1 — high-value engine and adapter work

| ID | Layer | Recommendation | Acceptance / rationale |
|---|---|---|---|
| **P1-1** | **Engine** | Implement an onset-class rule for open `uơ`/`ươ`: allow `uơ` after empty/`c h k kh qu th`; otherwise spread both horns; keep the `th` special case; do not collapse when a tone is present unless the onset allows `uơ`. Apply to VNI `7` as well. | `uow→uơ`, `khuow→khuơ`, `thuow→thuơ`, `huow→huơ`, `duow→dươ`, `tuowf→tườ`, `nguow→ngươ`, `luowj→lượ`, `phuow→phươ`, `duo7→dươ`; `thuowr→thuở`, `luowjc→lược`. |
| **P1-2** | **Engine + data** | Add optional dictionary mode: embedded ~7–8k Vietnamese syllables + user-word list; two-tier validity (exact tone word or strict base-prefix; tone-less base prefix); user words override; keep rule mode as default. | `luwocs` raw in dict mode, `truwocs→trước`, `phuowc→phươc`, `phuowcs→phước`, `luowj→lượ`, user word `lước` accepts `luwocs`; prefix real-time stays visible. Use an independently licensed word list. |
| **P1-3** | **Engine** | Extend macro matching: longest suffix of the current word, optional multi-word keys crossed by spaces (bounded), macro when Vietnamese is off, Shift+Space/Enter bypass, optional exact-case mode; keep `macro_digits`. | Match UniKey `key:text` semantics without requiring its file format; add tests for suffix, phrase, off-mode, bypass, digits, case. |
| **P1-4** | **Engine/API** | Expose dual backspace/restore APIs for adapters: keep display-character `Backspace` as default, add `undo_last_key()` (raw recompose) and `restore_last_word_as(RawKeystrokes|Vietnamese)`; optionally a 16-word history. | `tieengs` display BS `tiến`, raw undo `tiêng`; `aws` raw undo `ă`; adapters can choose. Port Bamboo’s contract (`delete_chars`, `delete_bytes`, `insert`) conceptually, not code. |
| **P1-5** | **Engine** | Support shifted brackets `{`/`}` in bracket mode: `{→Ơ`, `}→Ư`, uppercase output; keep pair-undo semantics. | `NG}{I→NGƯƠI`, `{s→Ớ`, `}f→Ừ`, `[[→[`, `][→ươ`; bracket mode in VNI optional. |
| **P1-6** | **Adapter** | Build the real adapter profile layer: terminal→preedit; Chromium address bar→preedit or select-replace; Firefox→no previous-word edit; default fallback surrounding→replace→preedit; commit preedit on focus/cursor/field change; detect password/URL/email/number; per-app mode overrides; backspace pacing/wait-ack. | Final text matches PREEDIT/REPLACE property; no lost word on focus change; no duplicate address-bar word; no preedit in password fields; app profile JSON is consumed. |
| **P1-7** | **Adapter** | Add clipboard/selection charset conversion as a separate tool: read text, choose input/output charset, convert, write back; start with plain text, then RTF/HTML if needed. | Round-trip known text; VIQR escapes; no formatting loss in plain text; user-visible charset names match Appendix A. |
| **P1-8** | **Docs** | Document config schema and migration: JSON fields, AppProfile fields, macro/dictionary file formats, default values, unknown-field behaviour, charset names, and a UniKey/OpenKey mapping table. | JSON schema or generated reference; examples for old/new tone, macro digits, app profiles, charset conversion. |

### 6.3 P2 — compatibility, advanced features, performance

| ID | Layer | Recommendation | Notes |
|---|---|---|---|
| **P2-1** | Engine | Add VIQR input method and additional presets: Microsoft layout, Telex 2, Telex+VNI, VNI French; generic custom keymap (`key=action` or data spec). | VIQR needs punctuation/escape handling; custom keymap is a larger design; keep `InputMethodSpec` data-driven. |
| **P2-2** | Engine | Optional relaxed spelling/loanword mode: `kr`/`cr` onsets, `ueu` rime, dictionary-driven words; keep `allow_coda_k`. | Tests `kroong→krông`, `khueeuf→khuều`, `buks→búk`; do not weaken strict mode. |
| **P2-3** | Engine | Decide `z`/VNI `0` compatibility: default UniKey behaviour (tone only; literal if no tone) or an explicit `remove_mark` option. Also optional `free_marking`/`tone_manual` for legacy users. | Current `awz→a`/`aaz→a` diverges; document or gate. `ToneManual` is low demand (removed from x-unikey). |
| **P2-4** | Engine | Long-word policy: commit the head and keep composing, or keep current literal fallback; add `preedit_overflow` tests; avoid key loss. | Bamboo 32-char window; SKey 128; ViFastKey 24. |
| **P2-5** | Adapter | A11y monitoring, auto-enable toolkit accessibility, learned per-app delay, native XTest/libei injection, Uinput ack/settle/replay, Sheets cell tracking, XIM handback. | SKey is the reference; platform-specific and high complexity. Do not port until a concrete platform adapter exists. |
| **P2-6** | Adapter | RTF/HTML clipboard conversion, font/charset signature, file conversion. | UniKey 3.62 tools; RTF library is proprietary, implement independently or skip. |
| **P2-7** | Performance | Replace `Vec` with fixed arrays (`MAX_RAW`), add prefix snapshots/DFA if profiling warrants; keep zero-alloc output. | Bamboo’s DFA/SWAR/LTO is overkill for a 24-char word; use its snapshot idea only if backspace latency matters. |
| **P2-8** | Docs/tests | Add provenance/third-party notices, a source-attribution table for test expectations, and a fuzz/property corpus for raw key sequences, NFD, charsets, macro. | Required for MIT distribution if any MIT code/tables are reused. |

### 6.4 Suggested order

1. **Milestone A (correctness)**: P0-1, P0-3, P0-4. These are small, high-impact, and expose the real gaps.
2. **Milestone B (charsets)**: P0-2 core module + C ABI; then P1-7 clipboard tool. Verify against independent standards/iconv, not GPL tables.
3. **Milestone C (engine depth)**: P1-1, P1-2, P1-3, P1-4, P1-5.
4. **Milestone D (adapter/product)**: P1-6, P1-8; then P2-5/P2-6 as platform work.
5. **Milestone E (advanced)**: P2-1/2/3/4, performance.

---

## 7. Risks and licensing constraints

### 7.1 License matrix

| Source | License | Can ViFastKey copy code/tables? | Safe use |
|---|---|---|---|
| **skey** | GPL-3.0 | **No.** | Behaviour, UX patterns, test input/output facts. |
| **skey-engine** | GPL-3.0 | **No.** | Algorithms/ideas; reimplement independently. Its embedded dictionary is GPLv3/CC-BY-SA-derived — do not import. |
| **bamboo-core** | MIT | **Yes, with copyright notice**, but verify provenance. | Algorithms, test cases, charset tables with attribution; still prefer independent implementation to keep the MIT repo clean. |
| **x-unikey** | LGPL-2.1+ engine, GPL-2.0 `vnconv`/GUI; IMdkit permissive | **No** for practical purposes; LGPL code would impose relinking/source requirements; `vnconv` is GPL. | Behaviour, semantics, test facts. |
| **UniKey 3.62** | GPL-2.0 plus “non-commercial redistribution” notice; `rtfio.dll` proprietary | **No.** | Behaviour, charset names/formats, docs/config ideas. |
| **ViFastKey** | MIT | — | New code must remain MIT-compatible; add NOTICE for MIT-derived material. |

Important:

- Do **not** copy the SKey/skey-engine dictionary (`data/vietnamese.cm.dict`) into an MIT project. Its own license file says it is part of ibus-bamboo under GPLv3 and derived from GPL/CC-BY-SA sources. Use an independently licensed Vietnamese word list (public domain, permissively licensed, or user-provided), or keep it as a separate optional data package with its own license.
- Do **not** copy charset tables verbatim from `skey-engine/src/charset.rs`, `x-unikey/vnconv`, or `unikey362/newkey/encode.cpp`. Derive tables from standards (TCVN 5712, VNI, VISCII, Unicode, Microsoft CP1258) or from MIT-licensed Bamboo tables with attribution and independent verification. Mapping facts are not copyrightable in the abstract, but table selection/ordering/comments can be.
- Do not copy test harness code or long comments. Re-state expected input/output pairs as facts; cite the source project in a provenance table.
- `unikey362/rtfio.dll` is proprietary; do not link or redistribute it.
- “UniKey”, “VNI”, “TCVN3”, “VISCII”, “BAMBOO” names: avoid implying endorsement; use nominative references only.

### 7.2 Behavioural traps found in the surveyed code

1. **SKey `FreeMarking` is not free marking.** `skey/src/vietnamese.cpp:81` calls `skey_engine_set_free_marking`; `skey-engine/src/lib.rs:267` maps `v != 0` to `set_modern`. So the option changes old/new tone style, not mark placement. SKey has no separate tone-style option. Do **not** reproduce this coupling; ViFastKey’s free-marking behaviour and `tone_style` are independent.
2. **Bamboo’s `std_tone_style` naming does not line up with UniKey.** Tests show `std_tone_style=true` → `hoaf` = `hòa` (tone on first vowel) and `false` → `hoà` (tone on second). UniKey `ModernStyle=Yes` → `hoà`. Bamboo’s docs label `hòa` as “standard (new)”, which conflicts with UniKey’s terminology. Treat flags as project-local; assert exact strings.
3. **SKey wrapper backspace is raw-key undo, not UniKey display backspace.** `tieengs` BS → `tiêng`, not `tiến`; `dduwowcj` BS → `đươc`, not `đượ`. Do not port SKey’s backspace expectation as UniKey behaviour.
4. **The `uo` family differs by project.** SKey always `ươ`; Bamboo/uvie mechanical `uơ` with onset-class spread; UniKey only special-cases `th`; ViFastKey currently always collapses. Pick one documented policy and test both prefix and complete-word cases.
5. **`dd` rules differ.** UniKey allows `dd` in a non-VN sequence if the preceding character is not a vowel (abbreviations); SKey uses `has_vn_markers`; Bamboo has its own auto-correct bypass; ViFastKey currently only supports word-initial `d`. Use the desired UX tests (`vcdd` yes, `add` no) to define the rule.
6. **Dictionary auto-restore semantics differ.** SKey can reject structurally valid but non-word `lước`; UniKey only restores raw-key sequences without a dictionary; Bamboo auto-correct rejects invalid prefixes, not nonwords. Decide whether “dictionary mode” is optional and what counts as a word.
7. **Macro file formats and matching differ.** UniKey `key:text` with VIQR/UTF-8 versions and suffix/multi-word matching; SKey exact raw config list; ViFastKey exact composed/raw. Do not assume one format is compatible.
8. **`z`/`0` mark removal differs.** ViFastKey removes a mark if no tone; UniKey/Bamboo usually type literal. Document or gate.
9. **NFD is not one thing.** Full canonical NFD (SKey, ViFastKey) vs Vietnamese composite/precomposed-base + combining tone (Bamboo `UnicodeCompound`, UniKey `UNI-COMP`). Mixing them breaks delete counts and round-trips.
10. **Legacy charsets are ambiguous by design.** TCVN3/Vietware/VPS share uppercase/lowercase codes; BKHCM1 shares `Ặ`/`Ụ`; VISCII has control-code capitals. Decode policy must be explicit; never guess silently.
11. **UTF-16 delete counts vs Unicode code points vs charset bytes.** ViFastKey FFI already converts to UTF-16; a charset adapter must also count target-encoding bytes (UniKey `getSeqSteps`). Do not reuse a Unicode count for a legacy charset.
12. **`xoong` is not `xông`.** UniKey/OpenKey require `xooong` for the loanword (`oo` is a vowel sequence); typing `xoong` produces `xông`. Document this, do not “fix” it accidentally.
13. **`th`/open `uơ` special case** is easy to lose in a refactor; keep explicit tests (`thuow→thuơ`, `thuowr→thuở`, `tuowf→tườ`).
14. **Macro expansion at boundaries must not trigger soft reset or auto-restore.** ViFastKey already tests `ko ` + BS; keep that invariant.
15. **Tagged “test data” can contain known errors.** SKey test comments admit corrections (`eesf`, `huef`, `thoaws`, `thuyt`); do not treat every SKey expectation as truth. Prefer Bamboo’s corrected parity suite for vowel sequences, and UniKey semantics for classic cases.

### 7.3 Platform/adapter risks

- **Uinput** requires `/dev/uinput` permissions/udev rules; **Wayland native** libei/portal triggers an input permission dialog; **XTest** works on X11 only. SKey defaults Native off on Wayland for this reason.
- **Surrounding text** is unreliable in Chromium/Electron/Firefox and in canvas editors; SKey’s code is a catalogue of fallbacks (ack, deferred commit, select-and-overtype, a11y snapshots). A naive `deleteSurroundingText` + commit can corrupt text under fast typing.
- **Focus/cursor changes** must commit or discard preedit; otherwise words get stuck or duplicate. SKey’s `SKeyState::deactivate`, pending-preedit map and `checkCellSelection` are the reference behaviours.
- **Password/email/URL/number fields** must bypass the IME; ViFastKey already models `FieldKind`, but real adapters must detect them reliably.
- **Legacy charset commit** may corrupt UTF-8 apps if bytes are wrapped as Latin-1 (SKey’s `charset.cpp` does this deliberately for Fcitx legacy fonts). Prefer conversion tools/clipboard, or an explicit opt-in legacy app mode.
- **Accessibility APIs** can be slow/private; cache snapshots and avoid blocking key handling.
- **Macro expansion** writes arbitrary user text; keep it plain text, no command execution, and cap length. Password fields must never be macro-expanded.
- **Privacy**: a11y text caches and surrounding text can contain sensitive content; keep them in memory only, scope per IC, clear on focus changes.

---

## Appendix A — Charset support matrix

Legend: ✓ supported; — not supported; “composite” = Vietnamese precomposed base + combining tone, not full NFD; “NFD” = canonical decomposition.

| Charset | skey / skey-engine | bamboo-core | x-unikey | UniKey 3.62 | ViFastKey |
|---|---|---|---|---|---|
| Unicode NFC | ✓ | ✓ | ✓ | ✓ | ✓ |
| Full Unicode NFD | ✓ `UnicodeDecomposed` | — (has composite) | — (UTF-16 composite) | — (composite) | ✓ |
| Unicode composite (“tổ hợp”) | — | ✓ `UnicodeCompound` | ✓ `UNI-COMP` | ✓ `Composite Unicode` | — |
| UTF-8 output / byte view | ✓ `Unicode` (actual UTF-8) | ✓ `UTF-8` (bytes as Latin-1) | ✓ `UTF-8` | ✓ `UTF-8 Literal`, `X UTF-8` (escape form) | ✓ (UTF-8 is the only output) |
| TCVN3 (ABC) | ✓ | ✓ | ✓ | ✓ | — |
| VNI-Windows | ✓ | ✓ | ✓ | ✓ | — |
| VISCII | ✓ | ✓ | ✓ | ✓ | — |
| VPS | ✓ | ✓ | ✓ | ✓ | — |
| BKHCM1 | ✓ | ✓ | ✓ | ✓ | — |
| BKHCM2 | ✓ | ✓ | ✓ | ✓ | — |
| Vietware-F / Full | ✓ `VietwareF` | ✓ `Vietware Full` | ✓ `VIETWARE-F` | ✓ `Vietware F` | — |
| Vietware-X | ✓ | ✓ | ✓ | ✓ | — |
| ISC | ✓ | — | ✓ | — | — |
| VNI-MAC | ✓ | — | ✓ | — | — |
| WinCP1258 | ✓ | ✓ | ✓ | ✓ | — |
| VIQR | ✓ | ✓ | ✓ (+UVIQR) | ✓ | — (input method also absent) |
| NCR decimal | — | ✓ | ✓ | ✓ | — |
| NCR hex | — | ✓ | ✓ | ✓ | — |
| Unicode C-string hex/dec | — | ✓ | — | ✓ `Unicode C String` | — |

Notes:

- SKey’s `TCVN3` table is validated against TCVN5712-1/iconv for 74 letters but intentionally differs from historical `.VnTime`/x-unikey tables in some slots. Bamboo’s TCVN3 table and x-unikey’s table also differ. Choose one standard, document it, and test against the fonts/apps you target.
- Bamboo’s decode is the safest reference for ambiguous legacy text: longest match, shared case-only → lowercase, shared distinct letters → leave encoded, VISCII control-code capitals decode-only.
- VIQR output must escape punctuation; VIQR input has “smart” legacy heuristics. A naive `string.replace` implementation will corrupt prose.

## Appendix B — Source map (high-value files)

| Area | Files |
|---|---|
| SKey config | `skey/src/config.h`, `skey/src/settings/config_io.h`, `skey/src/settings/config_io.cpp` |
| SKey engine wrapper | `skey/src/vietnamese.cpp`, `vietnamese.h`, `viet_undo.cpp`, `viet_recompose.cpp`, `viet_restore.cpp`, `viet_fallback.cpp` |
| SKey adapter | `skey/src/engine.cpp`, `engine.h`, `uinput_injector.*`, `native_injector.*`, `a11y_monitor.*`, `surrounding_cursor.h`, `input_field_policy.h`, `browser_identity.h` |
| SKey tests | `skey/src/test_vietnamese.cpp`, `test_engine_performance.cpp`, `test_charset.cpp`, `test_a11y_*`, `scripts/TESTING.md` |
| skey-engine core | `skey-engine/src/engine.rs`, `vseq.rs`, `spelling.rs`, `tables.rs`, `charset.rs`, `lib.rs` |
| skey-engine tests | `tests/vseq_parity.rs`, `parity.rs`, `cross_validate.rs`, `stress_test.rs`, `charset_ffi.rs` |
| Bamboo core | `src/engine/mod.rs`, `engine/restore.rs`, `engine/rebuild.rs`, `engine/state.rs`, `engine/snapshot.rs`, `engine/rules.rs`, `orthography/spelling.rs`, `orthography/syllable.rs`, `orthography/phonetics.rs` |
| Bamboo input/charset | `src/input_method/definitions.rs`, `preset.rs`, `rule.rs`; `src/encoder/mod.rs`, `encoder/tables.rs`; `src/config.rs` |
| Bamboo tests | `tests/skey_parity.rs`, `skey_repo_tests.rs`, `horn_placement.rs`, `free_onset_spelling.rs`, `tone_on_invalid_word.rs`, `backspace_keeps_tone_target.rs`, `undo_keeps_tone.rs`, `bracket_transform.rs`, `w2u_mode.rs`, `charset_decode.rs`, `vi_uvie_comprehensive_suite.rs`, `vni_english_mode.rs`, `rebuild_from_text.rs` |
| UniKey engine | `x-unikey/src/ukengine/ukengine.cpp`, `inputproc.cpp`, `mactab.cpp`, `usrkeymap.cpp`, `keycons.h`, `vnlexi.h` |
| UniKey config/docs | `x-unikey/doc/options`, `doc/unikeyrc`, `doc/keymap-syntax`, `doc/im-samples/*`, `doc/ukmacro`, `doc/manual` |
| UniKey charset | `x-unikey/src/vnconv/{charset,convert,data,pattern}.cpp`, `vnconv.h` |
| UniKey 3.62 | `unikey362/Uk362/keyhook/{vietkey,keyhook}.cpp`, `keyhook/keycons.h`; `newkey/{encode,userpref,keydlg,mactab,macrodlg}.cpp`; `vnconv/`, `uvconvert/` |
| ViFastKey | `python/vfk_core/{rules,transform}.py`, `python/vi_syllable/__init__.py`, `python/vfk_engine/{ports,session}.py`, `python/vfk_ffi/`, `python/tests/`, `python/conformance/`, `rust/crates/*`, `rust/README.md` |

---

## Appendix C — Quick mapping of ViFastKey config to surveyed concepts

| ViFastKey field | Closest concept | Notes |
|---|---|---|
| `input_method` | UniKey `Input`; Bamboo preset | Telex/Simple Telex/VNI; add VIQR/custom later. |
| `tone_style` | UniKey `ModernStyle`; SKey `modern`; Bamboo `std_tone_style` | ViFastKey `OLD` = first vowel (`hòa`), `NEW` = second (`hoà`). SKey/Bamboo flag names are unreliable; assert strings. |
| `spell_check` | UniKey `EnableSpellCheck`; Bamboo `auto_correct` partly | ViFastKey separates validity from restore; good. |
| `w_as_u` | SKey `ShortW`; Bamboo `W2uMode::Everywhere`/`NonStart` | ViFastKey boolean = everywhere; Bamboo has a non-start mode. |
| `bracket_keys` | SKey `BracketUO`; Bamboo `BracketMode` | ViFastKey boolean = everywhere; missing shifted `{ }` and non-start mode. |
| `quick_telex` | OpenKey convention | No direct UniKey equivalent. |
| `allow_coda_k` | Bamboo final-`k` relaxation | ViFastKey good. |
| `auto_restore_english` | UniKey `AutoRestoreNonVn`; SKey `AutoRestore`; Bamboo `auto_correct` | ViFastKey uses dictionary; SKey has rule/dict modes; UniKey word-end only. |
| `macro_enabled`/`macros` | UniKey `MacroFile`; SKey macro table | Add suffix/off-mode/bypass. |
| `macro_digits` | No standard | ViFastKey-specific; keep. |
| `auto_capitalize` | No standard | ViFastKey-specific; keep. |
| `excluded_apps`/`app_modes` | SKey app modes; UniKey no profiles | Normalise app keys. |
| `app_profiles` | SKey app modes/delay | Add runtime transport/delay/address-bar fields. |
| `edit_previous_word` | SKey reclaim; UniKey none | Works in REPLACE; preedit needs `DeleteSurrounding`. |
| `restore_after_space` | SKey soft reset/reclaim | Works in REPLACE; document. |
| `smart_caps` | No standard | Keep. |
| `unicode_form` | SKey NFD; Bamboo composite; UniKey UNI-COMP | Define whether NFD is canonical or composite; `unicode_form` currently NFC/NFD canonical. |
