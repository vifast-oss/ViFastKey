> **Báo cáo nghiên cứu nội bộ.** Tài liệu này chỉ ghi nhận hành vi, kiến trúc, UX và
> test fact của các dự án được khảo sát. Không có mã nguồn, bảng dữ liệu, từ điển hay test
> corpus GPL/AGPL nào được sao chép vào ViFastKey. Xem `THIRD_PARTY_NOTICES.md` và
> `docs/IME_FEATURE_RESEARCH.md`.

# VnKey deep analysis — behavior, architecture, tests, licensing

**Target:** `marixdev/vnkey` at `thư mục clone tạm/marixdev_vnkey`
**Snapshot:** commit `b75cdc0` (single squashed history), version **1.0.3**, dated 2026-04-03
**License:** GPL-3.0-or-later (`LICENSE`, `vnkey-engine/Cargo.toml:6`, `vnkey-windows/Cargo.toml:6`)
**Purpose:** extract behavior/UX/architecture/test facts for the MIT-licensed ViFastKey. No GPL code, tables, word lists, comments, or assets are reproduced here or should be copied into ViFastKey.

**Method:** full source read of the engine and all four adapters; `cargo test --offline` (38/38 pass); a throwaway probe crate at `probe tạm` using only the public `Engine`/`charset` API to exercise behaviors not covered by the repo tests. Probe results below are labeled **[probe]**; repo assertions are labeled **[test]**.

---

## 0. Executive summary

VnKey is a **Rust core + four thin platform adapters** product. Its engine is a fairly direct Rust port of UniKey's `ukengine`/`vnconv` data model and charset tables. It is strongest where ViFastKey is weakest today: shipped adapters, legacy Vietnamese charsets, VIQR input, clipboard conversion, per-app charset, packaging, and a real desktop GUI. It is weaker where ViFastKey is strongest: dictionary-based English restoration, configurable input behavior, NFC/NFD, preedit/replace abstraction, FFI safety, and conformance-scale tests.

Highest-value facts for ViFastKey:

1. **VnKey's macOS app is not an IMKit input method.** It is a `CGEventTap` background app (`vnkey-macos/src/main.m:1-8`, `VnKeyEventTap.m:322-370`, `CMakeLists.txt:23-63`). README/CHANGELOG/CONTRIBUTING still say IMKit. This is the single largest documentation/architecture discrepancy.
2. **`config.json` is not truly shared.** Windows uses `%APPDATA%\VnKey\config.json`; Fcitx5/IBus use `$XDG_CONFIG_HOME/vnkey/config.json`; macOS uses `NSUserDefaults`. Only a subset of keys overlap, and `macros` is an array on Windows but a tab-separated string on Linux.
3. **VnKey has no dictionary-based English restore.** It only has structural restoration. `text` → `tẽt`, `test` → `tét`, `rest` → `rét`, `post` → `pót` **[probe]**. ViFastKey's dictionary layer is materially better; do not regress it.
4. **`MS Vietnamese` is advertised but is a no-op.** `InputProcessor::set_im` clears the key map for `MsVi`/`UserDefined` and returns (`vnkey-engine/src/input.rs:172-185`). `vieejt` stays `vieejt` under `MsVi` **[probe]**. Do not copy this feature claim.
5. **Legacy charset support is broad (19 engine charsets) but inconsistently exposed and partly broken.** Engine has 19 (`charset/mod.rs:18-38,99-108`), README says 15, Fcitx5/IBus menus list 16, Windows tray/hotkeys list 12, Windows config GUI lists 12 labels backed by only 11 IDs, macOS has zero charset selection.
6. **`backspaces_bytes` exists but no adapter consumes it.** Multi-byte charsets (VNI-Windows, BKHCM2, VietWare-X, VNI-Mac) emit two code units per Vietnamese character, while adapters delete `backspaces` (character count), not `backspaces_bytes`. Live editing in those charsets is therefore suspect.
7. **NFC/NFD is not a general feature.** `from_utf8`/`to_utf8` short-circuit for UTF-8, so NFD passes through unchanged; only the legacy-conversion decode path has partial composition support. `e + U+0301` can encode to TCVN3, but `e + U+0302 + U+0323` becomes `e##` **[probe]**.
8. **VIQR input is fragile.** `a'b` → `b` **[probe]**, because tone punctuation is classified as a word break and the restore key history stops there. VIQR has no input-method tests; only charset round-trip tests.
9. **Macros do not work on Space in Fcitx5/IBus.** Space commits the preedit and soft-resets without calling `engine.process(' ')` (`vnkey-fcitx5.cpp:970-974`, `vnkey-ibus.c:1024-1028`). They can expand on punctuation because punctuation is processed through the engine.
10. **The engine and charset data are GPL-derived from UniKey.** The `VnLexiName`/`VowelSeq`/`ConSeq` layout, 71 vowel sequences, 29 consonant sequences, 213-character tables, constants, and charset IDs match UniKey's `vnlexi.h`, `ukengine.cpp`, `vnconv/charset.h`, and `data.cpp`. GPL-3 is compatible with UniKey's LGPL-2+/GPL-2+, but notices are missing; for MIT ViFastKey this is a **no-copy** area.

---

## 1. Repository layout and architecture

### 1.1 Tree and roles

| Path | Language | Role | Size |
|---|---|---|---|
| `vnkey-engine/` | Rust | Core state machine + C FFI; `lib` + `staticlib` (`Cargo.toml:8-10`) | ~5.6k LOC incl. 1.7k charset data |
| `vnkey-windows/` | Rust | Win32 `WH_KEYBOARD_LL` hook, SendInput, tray, WebView2 GUI | ~4.0k LOC |
| `vnkey-macos/` | Objective-C | `CGEventTap` app + status item + AppKit preferences (not IMKit) | ~1.0k LOC |
| `vnkey-fcitx5/` | C++ | Fcitx5 addon/engine, per-input-context state | ~1.1k LOC |
| `vnkey-ibus/` | C | IBus engine executable | ~1.2k LOC |
| `ede-fonts/` | TTF | 49 renamed fonts for Tây Nguyên/Êđê; license/provenance unclear | 16 MB |
| `tests/firefox_test.py` | Python | Manual Selenium/pyautogui Firefox test on Windows | 487 lines |
| `.github/workflows/` | YAML | Build/release matrix for Windows/macOS/deb/rpm/Arch/Nix | 543 lines |

Engine source split:

- `engine.rs` 2031 lines — state machine, tone/diacritic handling, macro, auto-restore, Ede.
- `vnlexi.rs` 592 — `VnLexiName`, 71 `VowelSeq` definitions, 29 `ConSeq` definitions, VC table, validity.
- `charset/mod.rs` 887 — hub-and-spoke conversion, 19 charsets, multiple decode/encode families.
- `charset/data.rs` 800 — 213-entry Unicode/VIQR/composite/no-tone/single/double/CP1258 tables.
- `input.rs` 287 — key maps, event types, char classification.
- `ffi.rs` 641 — singleton + instance C ABI, macros, charset, app charset.
- `macro_table.rs` 187 — case-insensitive macro table, text/JSON serialization.
- `app_charset.rs` 99 — global exe→charset map.
- `lib.rs` 41 — `Options` defaults.
- `tests/integration.rs` 479 — 38 tests (3 empty).

### 1.2 Data flow

```
platform key event
  -> platform maps to ASCII / key code
  -> vnkey_engine_process(engine, key_code, out_buf, ...)
       -> InputProcessor::key_code_to_event
       -> Engine::process
            prepare_buffer, snapshot_output, macro check
            dispatch_event (roof/hook/dd/tone/w/map/escape/append)
            auto_non_vn_restore fallback
            write_output from change_pos..current (UTF-8)
       -> ProcessResult { backspaces, backspaces_bytes, output, processed }
  -> adapter converts UTF-8 to target charset
  -> adapter replaces text (SendInput / CGEvent / deleteSurroundingText+commit)
```

`Engine` keeps a per-character `WordInfo` buffer (`engine.rs:79-109`) with:

- `form`: `Empty | NonVn | C | V | CV | VC | CVC`
- `c1_offset`, `v_offset`, `c2_offset` relative to the current end
- `vseq` (vowel sequence) or `cseq` (consonant sequence)
- `caps`, `tone` (0–5), `vn_sym` (internal `VnLexiName`), `key_code` (original key)

It also keeps `key_strokes: Vec<KeyBufEntry>` for macro extraction and auto-restore, `change_pos`/`backs` for diff output, `single_mode`, `to_escape`, and soft-reset saved state. `MAX_BUFFER` is 128 (`engine.rs:10`).

Important architectural property: unprocessed keys are **passed through natively** by adapters, while processed keys are **replaced** by backspaces + output. `Engine::process` returns `processed=false` for ordinary first letters (`append_vowel` returns false for ASCII alphabetic keys, `engine.rs:964-966`), so the adapter can let the OS type them; once a transformation happens, the engine emits a replacement diff. This is why Windows can inject only transformed output and pass ordinary keys through, and why `backspaces` matters only on transformations.

### 1.3 C ABI shape

`ffi.rs` exposes two APIs:

- Singleton: `vnkey_setup`, `vnkey_cleanup`, `vnkey_process`, `vnkey_backspace`, `vnkey_reset`, setters, `vnkey_add_macro`, `vnkey_clear_macros`.
- Instance: `vnkey_engine_new/free/process/backspace/reset/soft_reset/feed_context`, setters, macro add/remove/clear/load/save, charset conversion, app charset.

`VnKeyEngine` is an opaque alias for `Engine` (`ffi.rs:193-194`). There is **no ABI version**, no `catch_unwind`, and no single shared header: each adapter has a hand-copied `vnkey-engine.h` (Fcitx5, IBus, macOS), and the macOS header omits app-charset functions. `ffi.rs:4-6` says “cdylib” but `Cargo.toml` only builds `lib` + `staticlib`; adapters statically link `libvnkey_engine.a`.

`vnkey_setup` uses `ENGINE.lock().unwrap()` and can panic on a poisoned mutex (`ffi.rs:36-39`). `with_engine` silently returns `R::default()` on poison (`ffi.rs:24-27`). Instance calls dereference raw pointers after a null check on the engine only; `out_buf` is checked, but `actual_len == NULL` silently drops output. A panic crossing `extern "C"` aborts the host process (Rust behavior), so this ABI is less defensive than ViFastKey's panic-guarded FFI.

### 1.4 Build, packaging, signing

- Windows: `cargo build --release`, embeds `v.ico`/`e.ico` and manifest via `build.rs`/`resources.rc`; GUI is WebView2 (`tao` + `wry`). Manifest requests `asInvoker`, despite a comment claiming Administrator; elevation is an optional runtime toggle (`vnkey.manifest`, `tray.rs:59-79`).
- macOS: CMake builds an `.app` with `LSUIElement=true`; install target is `/Applications` (`CMakeLists.txt:92-95`), but `build.sh install` copies to `~/Library/Input Methods` (`build.sh:102-110`), and the release DMG links `/Library/Input Methods`. The app is ad-hoc signed (`codesign --force --deep --sign -`, `CMakeLists.txt:97-101`); not notarized.
- Windows release signing uses SignPath with `continue-on-error: true` (`.github/workflows/build.yml:39-50`), so a release can ship unsigned even though `SECURITY.md` says Windows binaries are signed.
- Linux: CMake/CPack for deb/rpm; Fcitx5 postinst can auto-install Fcitx5 and overwrite `/etc/environment.d` + `/etc/profile.d` (`vnkey-fcitx5/scripts/postinst`), which is aggressive for a package.
- No auto-update mechanism in any adapter. No crash reporting/telemetry (privacy claim is true), but Windows has an opt-in debug key logger (see §4.1).

### 1.5 The “shared config.json” claim vs reality

| Platform | Config location/format | Notes |
|---|---|---|
| Windows | `%APPDATA%\VnKey\config.json` (`config.rs:10-16`) | Full schema, line-based parser, debounce save |
| Fcitx5 | `$XDG_CONFIG_HOME/vnkey/config.json` or `~/.config/vnkey/config.json` (`vnkey-fcitx5.cpp:25-39`) | Subset; no `viet_mode`, hotkeys, blacklist |
| IBus | same XDG path (`vnkey-ibus.c:83-103`) | Subset; same limitations |
| macOS | `NSUserDefaults` keys (`VnKeyEventTap.m:30-77`) | No config.json, no charset, no per-app config |

Shared keys: `input_method`, `output_charset`, `spell_check`, `free_marking`, `modern_style`, `ede_mode`, `macro_enabled`, `macros`, `app_charsets`. Even these differ in type/format. Details in §5.

---

## 2. Engine in depth

### 2.1 Input methods and key maps

`InputMethod` has `Telex`, `SimpleTelex`, `Vni`, `Viqr`, `MsVi`, `UserDefined` (`input.rs:8-15`). Built-in maps:

- **Telex** (`input.rs:85-101`): `z`=tone0 (remove), `s/f/r/x/j`=tones 1–5, `w`=`TelexW`, `a/e/o`=roof, `d`=đ, `[`/`]`=ơ/ư, `{`/`}`=Ơ/Ư.
- **Simple Telex** (`input.rs:103-115`): same tones, but `w`=`HookAll` (not `TelexW`), so standalone `w` is literal; `uw`→ư, `ow`→ơ, `aw`→ă. No bracket mappings.
- **VNI** (`input.rs:117-128`): `0` remove tone, `1–5` tones, `6` roof-all, `7` hook uo, `8` breve/bowl, `9` đ.
- **VIQR** (`input.rs:130-143`): `0` remove, `'` sắc, `` ` `` huyền, `?` hỏi, `~` ngã, `.` nặng, `^` roof, `+`/`*` hook, `(` breve, `D` đ, `\` escape.
- **MsVi**: `set_im` resets the map to all `Normal` and returns (`input.rs:172-185`). No mapping exists. It is effectively English/pass-through, not Microsoft Vietnamese.
- **UserDefined**: same no-op; no FFI API to set a custom map (`set_user_key_map` exists in Rust but is not exported).

`KeyEvType` includes `RoofAll/A/E/O`, `HookAll/UO/U/O`, `Bowl`, `Dd`, `Tone0..5`, `TelexW`, `MapChar`, `EscChar`, `Normal` (`input.rs:18-41`). `VNE_COUNT` is the boundary for character-map actions.

Character classification (`input.rs:209-230`):

- `<= 32` → `Reset`
- any of `, ; : . " ' ! ? space < > = + - * / \ _ @ # $ % & ( ) { } [ ] |` → `WordBreak` (`input.rs:146-150`)
- `a..z/A..Z`, except `j/f/w` → `Vn`; `j/f/w` default to `NonVn` (but tone/roof events still work because dispatch uses `ev_type`, not `ch_type`).
- everything else → `NonVn`.

This classification has important consequences: `'` and `.` are both VIQR tone keys and `WordBreak`; digits are `NonVn`; `[`/`]`/`{`/`}` are both `WordBreak` and Telex map keys.

### 2.2 Word state machine

`process()` (`engine.rs:269-369`):

1. Clears `has_saved_state`.
2. `prepare_buffer()` to avoid overflow.
3. Sets `backs=0`, `change_pos=current+1`, `reverted=false`.
4. Saves `prev_current`; snapshots current output for multi-byte backspace calculation.
5. Converts key to `KeyEvent`.
6. If macros enabled, table non-empty, previous word exists, and event is `WordBreak` or `Reset`, tries macro expansion and returns early.
7. Dispatches (or handles `to_escape`).
8. If spell check is off or `single_mode` and the current entry became `NonVn` while a `Vn` key arrives, reclassifies it via `process_no_spell_check` (`engine.rs:312-320,1788-1803`).
9. Appends the key to `key_strokes` (with `ch_type` recomputed from `key_code`).
10. If not handled, tries `try_auto_restore`; if it returns true, emits restored ASCII.
11. Otherwise writes output from `change_pos..current`.

`append_vowel` (`engine.rs:822-969`) is the heart:

- Canonicalizes diacritics and case, extracts tone from a precomposed `VnLexiName` (only relevant for map chars).
- At word start or English mode → form `V`.
- After `NonVn`, `CVC`, `VC` → form `NonVn`.
- After `V`/`CV`: computes candidate `VowelSeq` from previous sequence plus new vowel. Three-vowel sequences cannot extend; two-vowel sequences can extend to three only via table lookup; one-vowel extends via `lookup_vseq2`.
- If candidate is `Nil` or invalid `CV`, the new entry becomes `NonVn`; otherwise it inherits the form, updates offsets/vseq, and moves/applies tone.
- For ordinary ASCII alphabetic keys it returns `false` even when the buffer changed, enabling native pass-through.

`append_consonant` (`engine.rs:971-1140`):

- At start/English → `C`.
- After `NonVn` → `NonVn`.
- After `V`/`CV`: looks up the new coda and validates `is_valid_cvc`; if valid, becomes `VC`/`CVC`, updates offsets; special-cases `u+o`/`u+ơ` auto-horn to `ươ` and moves tone.
- After `C`/`VC`/`CVC`: extends onset/coda sequence; validates CVC; invalid → `NonVn`.
- Returns `false` for consonants; only the `u+o` auto-horn path returns `true`.

`process_word_end` (`engine.rs:813-820`) appends an `Empty` slot holding the trigger key and returns `false`; the trigger passes through natively unless a macro consumed it.

### 2.3 Vowel/consonant tables and spelling rules

`VSEQ_LIST` has **71** entries: 12 single vowels, 37 two-vowel sequences, 22 three-vowel sequences (`vnlexi.rs:287-363`). Each entry has `len`, `complete`, `con_suffix`, `v[3]`, `sub[3]`, `roof_pos`/`with_roof`, `hook_pos`/`with_hook`. Key facts:

- `complete=false` marks sequences that need more input to be valid (e.g. `ua`, `ue`, `uo`, `uu`, `ye`, `ieu`, `uay`, `uoi`, `uou`, `uhoi`, `uhou`, `yeu`).
- `con_suffix=false` blocks coda attachment entirely for that sequence (e.g. `ai`, `ao`, `au`, `ay`, `eo`, `eu`, `ia`, `iu`, `oi`, `ui`, `uu`, `uoi`, `uou`, `oai`, `oay`, `oeo`, `uay`, `uyu`, `uhou`, `uhoi`, `yeu`).
- `roof_pos`/`hook_pos` encode where a diacritic lives for the sequence; `with_roof`/`with_hook` transition to another sequence or `Nil`.

`CSEQ_LIST` has **29** entries (`vnlexi.rs:366-396`). `suffix=true` only for `c, ch, m, n, ng, nh, p, t`; `k` is **not** a suffix, so `allow_coda_k` has no equivalent. `b, d, đ, dz, g, gh, gi, gin, k, kh, l, ph, q, qu, r, s, th, tr, v, x` are onsets only.

`VC_PAIR_LIST` (`vnlexi.rs:399-434`) enumerates legal vowel+coda pairs and is searched with binary search (`vnlexi.rs:493-508`). `is_valid_cv` (`vnlexi.rs:511-531`) only enforces:

- `gi` + `i` invalid; `qu` + `u` invalid;
- `k` only with a fixed list of front-vowel sequences (`e, i, y, ê, eo, eu, êu, ia, ie, iê, ieu, iêu`);
- otherwise true.

This is a structural check, not a lexicon. It allows many non-Vietnamese onsets (`ce`, `ka`, `ko`, `ku`), which is why English words such as `text` can survive as “valid” CVC. `is_valid_cvc` (`vnlexi.rs:534-563`) special-cases `quyn/quynh` and `gieng/giêng` when the VC pair fails.

### 2.4 Tone placement

`get_tone_position(vs, terminated)` (`engine.rs:495-516`):

1. Single vowel → 0.
2. If `roof_pos != -1` → that position.
3. If `hook_pos != -1` → that position, except `VS_UHOH/UHOHI/UHOHU` → 1.
4. Three vowels → 1 (middle).
5. If `modern_style` and `VS_OA/OE/UY` → 1.
6. Else `terminated ? 0 : 1`.

Examples:

| Input | Default (modern) | `modern_style=false` |
|---|---|---|
| `hoas` | `hoá` | `hóa` |
| `hoaf` | `hoà` | `hòa` |
| `thuys` | `thuý` | `thúy` |
| `tuys` | `tuý` | `túy` |
| `loas` | `loá` | `lóa` |
| `khoer` | `khoẻ` | `khỏe` |
| `toans` | `toán` | `toán` |
| `toanf` | `toàn` | `toàn` |

Note VnKey defaults to **modern** style (`Options::default`, `lib.rs:29-40`), while ViFastKey defaults to **old** (`ToneStyle::Old`). This alone changes many outputs (`hoá` vs `hóa`).

`process_tone` (`engine.rs:1142-1209`):

- At word start or English mode → append literally.
- Special `gi`/`gin` handling: tone goes on `i` (for `gi`) or the preceding vowel (for `gin`); pressing the same tone removes it and appends the tone key literally.
- If current entry has no vowel offset → literal.
- If spell check + not free marking + incomplete sequence → literal (`muonf` stays `muonf` when free marking is off).
- If current form is `VC`/`CVC` and coda is `c/ch/p/t`, tones 2/3/4 (huyền/hỏi/ngã) are rejected; only sắc/nặng are allowed.
- Same tone pressed again removes the tone and appends the key literally (`ass` → `as`, `aaa` is roof undo, not tone).
- Otherwise sets tone at computed position.

### 2.5 Diacritics and special keys

**Roof** (`process_roof`, `engine.rs:1211-1336`): target-specific `RoofA/E/O`; `RoofAll` has target `NonVnChar` and accepts any roof position. Special `ươ/ươi/ươu` → `uô...` double change. If `with_roof == Nil`, removes an existing roof if target matches; recalculates vseq from raw symbols and moves tone. Free-marking-off requires the change position to be the current end. Validates CVC.

**Hook/bowl** (`process_hook`, `engine.rs:1338-1482`; `process_hook_with_uo`, `1484-1608`): `HookUO`/`HookAll`/`HookU`/`HookO`/`Bowl`; special `uo` family; `th + o` special-case gives `ơ` rather than `ươ`; removing a hook reverses to `u/o/a`. Validates event-type-specific target, CVC, free marking; moves tone.

**đ** (`process_dd`, `engine.rs:1610-1662`): allows `dd` in a non-Vietnamese abbreviation (`single_mode`) to become `đ` even when the word is `NonVn` (so `đc`, `đk` survive instead of restoring `ddc`, `ddk`); otherwise finds the onset at `current - c1_offset` and toggles `d`↔`đ`. Pressing `d` again after `đ` removes the stroke and appends literal `d` (`ddd` → `dd`).

**Bracket/map chars** (`process_map_char`, `engine.rs:1664-1714`): appends the mapped character; if the result is not a valid word form, removes it and retries as a normal key. Consequences: `[` alone → `ơ`, `]` alone → `ư`, `{`→`Ơ`, `}`→`Ư`, `th[`→`thơ`, `[s`→`ớ`, but `a[`→`a[` and `u[`→`uơ` (the latter is a valid `uơ` sequence, not ư). This is not a general insert-anywhere key like ViFastKey's optional bracket mode.

**Telex `w`** (`process_telex_w`, `engine.rs:1716-1775`):

- `ww` undoes a standalone `ư` created by `w` back to literal `w` (with case preserved).
- If a vowel is present, delegates to `process_hook` with `HookAll`.
- If current form is `NonVn`, passes `w` through (prevents `windows` → `windoư`).
- Otherwise standalone `w` → `ư`.

Observed: `w`→`ư`, `ww`→`w`, `www`→`ww`, `wa`→`ưa`, `wow`→`ươ`, `windows`→`windows` **[probe]**.

**Escape** (`process_esc_char`, `engine.rs:1777-1786`): only sets `to_escape` when in-word and form is not `Empty`/`NonVn`; then `process_append` is called. The next key handling is in `process` (`engine.rs:298-310`). VIQR uses `\`, but the behavior is inconsistent at word start: `\a` stays `\a`, `a\?`→`a?`, `\?`→`\?` **[probe]**. Telex has no `EscChar` mapping, so `\` is literal.

### 2.6 Backspace and soft reset

`soft_reset` (`engine.rs:210-220`) saves `current`, `key_current`, `single_mode`, `to_escape`, sets `has_saved_state = current >= 0`, then clears current state. `process()` clears `has_saved_state` on any new key. `process_backspace` (`engine.rs:372-458`):

- If English mode or `current < 0`, tries `restore_saved_state`; if successful returns `processed=false` with no output (adapter deletes the space natively), so the next key can apply a tone to the restored word.
- Otherwise marks change at current, checks form/previous form; for a simple character or onset-boundary, just decrements and syncs key history.
- If deleting the last character changes the vowel sequence, computes old/new tone positions; if the tone should move, moves it; otherwise decrements.
- `backs > 1` signals a replacement; `processed` is `backs > 1`.

Observed: type `no`, space, backspace, `s` → `nó` **[test `test_backspace_after_space_restores`]**. `tiếng` backspace → `tiến`; further backspaces → `tiế`, `ti`, `t` **[probe]**.

### 2.7 Auto non-Vietnamese restore (structural, not dictionary)

`try_auto_restore` (`engine.rs:663-741`) runs only when a key was **not** handled (`ret == false`) and all of:

- `auto_non_vn_restore` and `viet_key` and `spell_check_enabled`;
- current form is `NonVn`;
- the word has at least one vowel;
- at least one “modification” exists: any tone, or `â/ê/ô/ơ/ư/ă/đ`.

It finds word start by walking back until `Empty`, finds the key-history start by walking back until a `WordBreak`, then rebuilds the word from original `key_code`s as `NonVn` ASCII. `backs` = number of characters already on screen before the new key; the new key is suppressed and represented in the rebuilt string. It deliberately does **not** restore all-consonant words (`đc`, `đk` abbreviations).

This is purely structural. It catches `fix`, `linux`, `flex`, `project`, `windows`, `power`, `software`, `workflow` **[test `test_telex_non_vietnamese_words`]**, but not words that remain structurally valid:

| Input | VnKey default | ViFastKey (dictionary) |
|---|---|---|
| `text` | `tẽt` | `text` |
| `test` | `tét` | `test` |
| `rest` | `rét` | `rét` (small dict; not restored) |
| `post` | `pót` | `pót` |
| `most` | `mót` | — |
| `cast` | `cát` | — |
| `this` | `thí` | `thí` (intentional Vietnamese priority) |

So VnKey's “English restore” is **not** dictionary-based, contrary to how a user might read the feature name. ViFastKey's two-layer restore is a genuine advantage.

With `spell_check=false`, transformations proceed even in invalid sequences: `fix`→`fĩ`, `linux`→`linũ`, `project`→`proẹct`, `windows`→`ưindớ` **[probe]**. With `auto_non_vn_restore=false`, `windows`→`ưindows` **[probe]**.

### 2.8 Macros (gõ tắt)

`MacroTable` (`macro_table.rs`):

- Case-insensitive keys, stored original case; max 1024 entries, max key length 256 bytes (`:5-6,30-46`).
- `add` updates existing key value; `remove` uses `swap_remove` and fixes the swapped index.
- `load_from_text` uses tab-separated lines, ignores blank/`#`; `to_text` same.
- `to_json`/`load_from_json` are hand-rolled, not robust JSON: values containing `"`/control chars/escaped backslashes can break; no `\u` escapes; parser looks for the first `"` after `"key":"`.

Expansion (`engine.rs:603-660`):

- Triggered only when `ev.ch_type` is `WordBreak` or `Reset`, previous word exists, macro table non-empty.
- Extracts original key codes from `buffer[word_start..=prev_current]`, so digits are included even though they are `NonVn`.
- Case rules: all lower → expansion as-is; first upper only → capitalize each word (after spaces); all upper → `to_uppercase()`.
- Backspaces = number of on-screen chars in the word; resets engine; output = expansion + trigger key if `0x20 <= key < 0x80`.
- Trigger punctuation is appended raw; the platform must not also send it.

Observed: `bc `→`báo cáo `, `Bc `→`Báo Cáo `, `BC `→`BÁO CÁO `, `bc,`→`báo cáo,`, `a1 `→`một ` with macro `a1`, `b2b.`→`doanh nghiệp.` **[probe]**. Digits work without a separate `macro_digits` option because they remain in the word buffer.

Limitations:

- No expansion on Enter/Tab in real adapters because they reset/commit before calling the engine. In pure engine, `process('\n')` would expand and **swallow the newline** because the trigger is only appended when `>= 0x20` (`engine.rs:648-651`).
- Fcitx5/IBus do not call `process(' ')` on Space (`vnkey-fcitx5.cpp:970-974`, `vnkey-ibus.c:1024-1028`), so macros do not expand on Space there; they do expand on punctuation.
- Windows saves macros as a JSON array; Fcitx5/IBus save a tab-separated JSON string; macOS saves a string in `NSUserDefaults`. The formats are mutually incompatible.
- No macro import/export UI beyond the Windows macro table; macOS has a checkbox but no editor.

### 2.9 Tây Nguyên / Êđê mode

`try_ede` (`engine.rs:1813-1920`) runs before normal dispatch when `ede_mode` and Vietnamese mode:

- `Bowl` (VNI 8), `TelexW`, or `HookAll`: `â`→ẁ, `ê`→ẅ, `ô`→ö, `ơ`→ő, `ư`→ů, `e`→ĕ, `i`→ĭ; `c`→č.
- `Tone0` (VNI 0, Telex z): `o`→ŏ, `u`→ŭ.
- `Tone4` (VNI 4, Telex x): `n`→ñ.
- `Dd` (VNI 9, Telex d): `b`→ẃ.

It emits a Unicode char as a `NonVn` buffer entry.

Observed Telex: `aaw`→ẁ, `eew`→ẅ, `oow`→ö, `oww`→ő, `uww`→ů, `cw`→č, `ew`→ĕ, `iw`→ĭ, `oz`→ŏ, `uz`→ŭ, `nx`→ñ, `bd`→ẃ **[probe]**.
Observed VNI: `a68`→ẁ, `e68`→ẅ, `o68`→ö, `o78`→ő, `u78`→ů, `c8`→č, `e8`→ĕ, `i8`→ĭ, `n4`→ñ, `b9`→ẃ; **`o8`/`u8` do not produce ŏ/ŭ** **[probe]**.

CHANGELOG 1.0.3 claims `bb`/`b9`→ẃ and VNI `o8`/`u8`→ŏ/ŭ, but the code supports Telex `bd` (not `bb`) and VNI `b9`, while VNI `o8`/`u8` are `Bowl` and not handled by the Ede `Bowl` branch (only `Tone0` handles o/u). Documentation is out of sync.

### 2.10 Context feeding

`feed_context(text)` (`engine.rs:244-266`) resets, feeds only printable ASCII `0x21..=0x7E`, resets and skips on any non-ASCII, then sets `backs=0`, `change_pos=current+1`. It is used by IBus before each printable key (`vnkey-ibus.c:943-992,1082-1084`) to recover a prefix such as `g` after committed “giá” was partially deleted. Fcitx5 has an equivalent `trySurroundingContext()` but **never calls it** (`vnkey-fcitx5.cpp:884-935`; no call site), so the Fcitx5 context-loss bug remains. Windows/macOS never call it.

`feed_context` processes each char through the full engine, so it can apply transformations to the prefix; it does not just seed raw state. It caps the caller's context to 10 ASCII letters (IBus/Fcitx).

### 2.11 Dead/unused code and invariants

- `Options::strict_spell_check` is declared and defaulted but **never read** (`lib.rs:25,37`; no other use).
- `Engine::reverted` is set in many branches but **never read** (`engine.rs:138,274,1160,...`).
- `KeyBufEntry::_converted` is stored but **never read** (`engine.rs:115,331`).
- `InputMethod::UserDefined` exists but has no FFI path.
- `get_backspaces_for_multi_byte` (`engine.rs:544-564`) counts every non-ASCII char as 2, which is wrong for single-byte charsets (TCVN3/VPS/VISCII/etc.) and only approximately right for double-byte charsets.
- `prepare_buffer` (`engine.rs:566-585`) has a complex compaction heuristic; no tests near `MAX_BUFFER=128`.

---

## 3. Input methods and charsets

### 3.1 Input-method support matrix

| Method | Engine | Windows | macOS | Fcitx5 | IBus | Notes |
|---|---|---|---|---|---|---|
| Telex | yes | yes | yes | yes | yes | default; bracket keys always on |
| Simple Telex | yes | yes | yes | yes | yes | no standalone `w`→ư; no brackets |
| VNI | yes | yes | yes | yes | yes | digits as modifiers |
| VIQR | yes | yes | yes | yes | yes | fragile input; charset conversion separate |
| MS Vietnamese | no-op | menu item | menu item | no | no | advertised but no mapping |
| Ede | yes | checkbox | checkbox | toggle | toggle | VNI o8/u8 mismatch |
| User-defined | Rust API only | no | no | no | no | not exported |

### 3.2 Charset inventory (19 engine IDs)

From `charset/mod.rs:18-38` and `99-108`:

| ID | Enum | `name()` | Family | Notes |
|---:|---|---|---|---|
| 0 | `Unicode` | `UNICODE` | UCS-2LE | Decode/encode 16-bit LE; no surrogate handling |
| 1 | `Utf8` | `UTF-8` | UTF-8 | identity fast path |
| 2 | `NcrDec` | `NCR-DEC` | HTML numeric | `&#NNN;` for >=128 |
| 3 | `NcrHex` | `NCR-HEX` | HTML numeric | `&#xHHHH;` for >=256, **raw byte for 128–255** |
| 4 | `UniDecomposed` | `UNI-COMP` | UTF-16LE decomposed | uses composite table |
| 5 | `WinCP1258` | `WINCP-1258` | Windows CP1258 | double-byte table + pre table for decode |
| 6 | `UniCString` | `UNI-CSTRING` | C escapes | `\xHHHH` |
| 10 | `Viqr` | `VIQR` | text | VIQR |
| 11 | `Utf8Viqr` | `UTF8VIQR` | mixed | decode accepts UTF-8 + VIQR; encode always VIQR |
| 20 | `Tcvn3` | `TCVN3` | single byte | ABC/T CVN3 |
| 21 | `Vps` | `VPS` | single byte | |
| 22 | `Viscii` | `VISCII` | single byte | |
| 23 | `Bkhcm1` | `BKHCM1` | single byte | |
| 24 | `VietwareF` | `VIETWARE-F` | single byte | |
| 25 | `Isc` | `VNU` | single byte | ISC/VNU |
| 40 | `VniWin` | `VNI-WIN` | double byte | |
| 41 | `Bkhcm2` | `BKHCM2` | double byte | |
| 42 | `VietwareX` | `VIETWARE-X` | double byte | |
| 43 | `VniMac` | `VNI-MAC` | double byte | |

`from_name` also accepts aliases: `UCS-2/UCS2`, `UTF8`, `NCR`, `CP1258`, `ABC` for TCVN3, `VNU` for ISC, `VNI` for VNI-WIN (`charset/mod.rs:48-72`).

### 3.3 Conversion architecture

`convert(input, from, to)` = `decode` → `Vec<StdVnChar>` → `encode` (`charset/mod.rs:123-145`). `StdVnChar` is either a plain scalar or `0x10000 + index` into a **213-entry canonical table** (`TOTAL_VNCHARS=213`, `data.rs:3`). `INVALID_STD_CHAR=0xFFFFFFFF`.

Decode families:

- **UTF-8** (`decode_utf8`, 176-202): precomposed lookup first; if next char is a combining mark in `0x0300..=0x0323`, tries `(mark<<16)|base` composite lookup; only **one** combining mark is consumed. Raw non-Vietnamese scalars pass through.
- **UCS-2LE** (`decode_unicode`, 204-217): every 2 bytes; no endian detection, no surrogate pairs.
- **NCR** (`decode_ncr`, 219-263): `&#` + decimal or optional `x` hex, max 5 digits, optional `;`; non-entities pass through as bytes. Hex entities are only accepted by `NcrHex`.
- **UniDecomposed** (`decode_uni_decomposed`, 265-296): UTF-16LE; tries two-word composite then precomposed then single-word composite.
- **UniCString** (`decode_uni_cstring`, 298-322): `\x` + up to 4 hex digits; other bytes pass through.
- **Single-byte** (`decode_single_byte`, 324-342): builds a 256-entry reverse map from the charset table, with duplicate handling based on `table[i] != table[i+1]`; unmapped bytes pass through as scalars.
- **Double-byte** (`decode_double_byte`, 344-419): builds a sorted 16-bit lookup of `(low byte, high byte)` pairs; high byte alone marks a lead byte; invalid pairs become `INVALID_STD_CHAR`.
- **VIQR** (`decode_viqr`, 425-519): stateful parser with `at_word_beginning`, `got_tone`, `\` escape, `DD` for đ, diacritic+tone composition; a number of ad-hoc offsets.
- **UTF8-VIQR** (`decode_utf8_viqr`, 521-568): bytes >0xBF are treated as UTF-8, other runs are decoded as VIQR.

Encode families:

- UTF-8/Unicode/NCR/CString/UniDecomposed/VIQR/single/double (572-849).
- Unmappable characters become `PAD_CHAR = b'#'` (`charset/mod.rs:119,709-780`), **silently**. There is no error return for lossy conversion.
- `encode_ncr_hex` pushes a raw byte for `code < 256` (658-665), so `à` (U+00E0) becomes byte `0xE0`, not ASCII `&#xE0;`.
- `encode_utf8` only handles up to 3-byte sequences; codepoints >0xFFFF are emitted incorrectly (no 4-byte branch, 595-615).
- `encode_viqr` has a state machine to escape literal tone/diacritic characters; it is not exercised by input tests.

`std_vn_to_upper/lower/remove_tone` (`charset/mod.rs:854-878`) use index parity and `STD_VN_NO_TONE`.

### 3.3.1 Family-specific behavior

- **TCVN3 (ABC)**: single-byte; accented lowercase glyphs occupy bytes such as `0xB5` (à), `0xB8` (á), `0xB9` (ạ); uppercase accented vowels share the same byte as lowercase in this table, so case is not round-trippable for `Ạ`/`Ả`/etc. TCVN3 page 0x80–0x9F is Western symbols. Encode is font-oriented; do not treat it as a Unicode code page.
- **VPS**: single-byte like TCVN3, different byte assignments; same uppercase-collision class (probe `Ạ`→`ạ`).
- **VISCII**: single-byte, preserves uppercase/lowercase distinctions and round-trips the sample; control range is used for some Vietnamese characters, so clipboard/binary safety matters.
- **VNI-Windows**: two-byte; low byte is the base letter, high byte is the tone/diacritic modifier (e.g. `ê` is base `e` + modifier). It is not UTF-16 and not a code page. Every Vietnamese character is two bytes; this is the main reason `backspaces_bytes` exists (and the reason adapters that delete characters are wrong).
- **VNI-Mac / BKHCM2 / VietWare-X**: same two-byte model with different byte assignments.
- **BKHCM1 / VietWare-F / VNU (ISC)**: single-byte legacy tables; VNU/ISC differs from TCVN3 only in some assignments. Round-trip sample works for BKHCM1/VNU; VietWare-F shares the uppercase collision.
- **VIQR**: plain-text convention, not a byte encoding. `DD` = Đ, `dd` = đ, `^` roof, `+`/`*` hook, `(` breve, tone marks `` ' ` ? ~ . ``, `\` escapes a literal tone/diacritic character. Encode/decode is stateful; the decoder has `at_word_beginning`/`got_tone` heuristics and is not the same code path as VIQR *input*.
- **NCR-Dec/Hex**: HTML numeric character references; decimal uses `&#NNN;` for non-ASCII; hex intends `&#xHHHH;`. Hex is buggy for U+0080–U+00FF (raw byte instead of entity) and the decoder accepts at most 5 digits. It is ASCII text, not a legacy font encoding.
- **CP1258**: Windows Vietnamese code page; uses base characters plus combining diacritics for many letters, with a `WIN_CP1258_PRE` table for decode. Encoding via this table produces multi-byte sequences for accented letters; it is not “Latin-1 bytes for a VNI font”. Adapters that do Latin-1 passthrough will not render it correctly.
- **Unicode/UTF-8**: `Unicode` is UCS-2 **little-endian** (no BOM, no surrogate handling); `Utf8` is ordinary UTF-8 with an identity fast path.
- **UniDecomposed (UNI-COMP)**: UTF-16LE with base + combining mark sequences; `UNICODE_COMPOSITE` drives it. This is not a general NFD string format and is not UTF-8.
- **UniCString**: C-style `\xHHHH` escapes for non-ASCII; ASCII printable characters pass through. Decoder accepts up to 4 hex digits.
- **UTF8-VIQR**: decoder accepts mixed UTF-8 and VIQR; encoder always emits VIQR. It is not a distinct wire format with a normative spec.

### 3.4 Charset findings from round-trip probes

Sample: `Việt Nam: đệ ơ ư ă â ê ô Ạ ỹ Đ`.

| Charset | Round-trip | Observation |
|---|---|---|
| Unicode, UTF-8, NCR-Dec, NCR-Hex, UniDecomposed, CP1258, UniCString, VIQR, UTF8-VIQR | yes |  |
| VISCII, BKHCM1, VNU | yes |  |
| VNI-Win, BKHCM2, VietWare-X, VNI-Mac | yes |  |
| **TCVN3, VPS, VietWare-F** | **no** | uppercase `Ạ` decodes back as lowercase `ạ` (legacy tables share bytes for upper/lower accented vowels) |

`à` → NCR-Hex gives raw byte `E0` **[probe]**, invalid UTF-8; `String::from_utf8` fails and adapters that assume UTF-8 will misbehave. NCR-Dec correctly gives `&#224;`.

Unmappable text: `😀`/`中`/`€` → TCVN3/VISCII produce `#` **[probe]**.

UCS-2/UniDecomposed are little-endian and do not handle BOM/surrogates; `Unicode` is not UTF-16 with surrogates.

### 3.5 NFD/NFC behavior

There is no general normalization API.

- `from_utf8(s, Utf8)` returns `s.as_bytes()` immediately (`charset/mod.rs:129-135`).
- `to_utf8(input, Utf8)` returns the input bytes immediately (`:138-145`).
- Only `decode_utf8` (used when converting UTF-8 to a legacy charset) attempts composition, and only one combining mark.
- Probes:
  - `e + U+0301` → TCVN3 `D0` (works, one mark).
  - `e + U+0302 + U+0323` → TCVN3 `65 23 23` (`e##`, lost).
  - `ê + U+0323` → TCVN3 `D6` (works, precomposed base + one mark).
  - `from_utf8(NFD, Utf8)` returns NFD unchanged.

ViFastKey's explicit `UnicodeForm::{Nfc,Nfd}` at output and NFC-internal model is strictly more capable. If ViFastKey adds legacy conversion, it should normalize to NFC (or define a documented policy) before lookup, and support full NFD input.

### 3.6 Charset exposure mismatch (important UX/docs finding)

| Surface | Count | IDs |
|---|---:|---|
| Engine enum | 19 | all above |
| README | 15 | “Unicode UTF-8, TCVN3, VNI Windows, VISCII, VPS, VIQR, NCR, CP-1258, …” |
| Fcitx5 `CS_DEFS` | 16 | 1,40,20,10,4,5,2,3,22,21,41,23,42,24,25,6 |
| IBus `CHARSETS` | 16 | same as Fcitx5 |
| Windows tray/hotkeys/app-charset | 12 | 0,1,2,3,5,10,20,21,22,25,40,43 |
| Windows converter | 12 | same |
| Windows config GUI | **12 labels / 11 IDs** | `gui.rs:22,107-112` bug |
| macOS | 0 | always UTF-8 |

**Windows config GUI bug:** `CS_IDS` has 11 entries `[0,1,2,3,5,10,20,21,22,40,43]`, but `cs_names` has 12 labels including VNU, VNI Windows, VNI Mac. After VISCII, labels and IDs shift: selecting “VNU” stores 40 (VNI Windows), selecting “VNI Windows” stores 43 (VNI Mac), selecting “VNI Mac” falls back to 0 (Unicode). This is a concrete, high-impact GUI bug (`gui.rs:22-24,107-112,189-195`).

The Windows converter/`charset_from_id` supports only 12 and omits BKHCM1/2, VietWare X/F, UniDecomposed, CString, Utf8Viqr, despite engine support (`converter.rs:32-47`). The Fcitx5/IBus menus omit Unicode (0), Utf8Viqr (11), VNI-Mac (43).

### 3.7 Clipboard converter

| Platform | Mechanism | Behavior/limits |
|---|---|---|
| Windows | Win32 `CF_UNICODETEXT` (`converter.rs:100-136`); UI + hotkey | Converts text or clipboard; source legacy is assumed to be Latin-1/mojibake (casts chars to `u8`, `:73-94`), so converting real Unicode text *from* TCVN3 gives wrong bytes. NCR-Hex invalid for Latin-1. UI has source/target dropdowns, swap, convert. Default source index 0 (Unicode), target index 6 (TCVN3) — both are indices, not charset IDs. |
| Fcitx5 | `popen` `wl-paste`/`xclip`/`xsel` (`vnkey-fcitx5.cpp:697-733`) | Converts to/from current output charset; legacy bytes are lifted to Latin-1 UTF-8 for display; requires external tools; no notification/UI besides status menu. |
| IBus | same shell tools (`vnkey-ibus.c:337-379`) | same. |
| macOS | none | no clipboard converter. |

Windows converter labels 12 charsets but `charset_from_id` can’t convert BKHCM1/2/VietWare; Fcitx5/IBus can use 16 but mishandle UniDecomposed (see §4.3).

### 3.8 Multi-byte backspace bookkeeping

`ProcessResult` carries both `backspaces` (characters) and `backspaces_bytes` (`engine.rs:120-128`, `ffi.rs:53-79`). `get_backspaces_for_multi_byte` computes 2 for every non-ASCII output char (`engine.rs:544-564`). No adapter reads `backspaces_bytes` (grep across all adapters finds only header comments). Consequences:

- VNI-Win/BKHCM2/VietWare-X/VNI-Mac output two UTF-16 code units per Vietnamese char; adapters delete `backspaces` characters, leaving one unit behind or deleting too few.
- For TCVN3/VPS/VISCII (one byte per char), `backspaces_bytes` would over-delete (2 vs 1) if an adapter used it.
- The API should expose a **code-unit delete count for the actual adapter encoding**, not a hardcoded “non-ASCII = 2” rule.

This is one of the highest-value adapter/engine contract findings.

---

## 4. Platform integrations

### 4.1 Windows (`vnkey-windows`)

**Hook and event pipeline** (`hook.rs`):

- Installs `WH_KEYBOARD_LL` and `WH_MOUSE_LL` (`:47-69`). Mouse-down resets engine (`:597-625`).
- Ignores its own injected events by `dwExtraInfo == "VNKY"` and a `SENDING` flag (`:162-169`; `send.rs:39-47`).
- VM/RDP pass-through **before any logging** to avoid logging guest passwords (`:171-177`); VM list in `send.rs:79-85` (VMware, VirtualBox, RDP, VNC, Hyper-V).
- Heartbeat and 5-second watchdog: reinstall keyboard hook only if no key event for >5s (`:23-24,84-128`), because Windows can silently unhook a slow low-level hook.
- 5-second typing-gap reset (`:236-257`), chosen over focus/foreground reset because Chrome autocomplete popups change foreground/hwndFocus mid-word (changelog 1.0.1: “đc” → “ddc”).
- Blacklist check every keydown (`:226-231`), exact exe name, case-insensitive (`blacklist.rs:17-21`).
- Ctrl/Alt/Win: reset engine and pass through (`:351-364`); Shift allowed.
- Built-in toggle `Ctrl+Shift` is deferred to keyup and canceled by any non-modifier key (`:286-301,343-346`), fixing accidental toggle on Ctrl+Shift+N/V (#15).
- Custom toggle hotkey is handled inside the hook to avoid focus loss, and the original key is blocked (`:303-330`).
- Enter/Esc/Tab reset engine; Space soft-resets, or calls `process(0x20)` when macros are enabled to allow expansion (`:368-417`).
- Backspace: calls `process_backspace`; if `processed && backspaces > 1`, replaces; otherwise native/injected (`:494-528`).
- Printable keys: `vk_to_ascii` (`:530-557`); if 0, reset and pass/inject; otherwise `engine.process(ascii)`. If processed with output/backs, convert charset and `send_output`; if unprocessed, pass through (console) or let native handle (GUI).
- **Every transformed key is blocked and reinjected**, so all text in GUI apps goes through one SendInput path, avoiding mixed native/SendInput backspace bugs (`:444-492`).
- Mutex is `try_lock`; if the GUI holds it, keys pass through rather than deadlocking.

**Key-to-ASCII bug/limitation:** letters `VK_A..VK_Z` are short-circuited to `vk ± 32` (`hook.rs:537-540`), bypassing `ToUnicode`. The changelog claims full layout support via `ToUnicode`, but only non-letter keys use it. Non-QWERTY layouts may still produce wrong letters; worth not copying this shortcut.

**Output path** (`send.rs`):

- Default replacement uses Shift+Left selection, not VK_BACK, because Chrome Omnibox/autocomplete can swallow the first backspace (`:203-224`). VK_BACK fallback for apps that block Shift+Left.
- App-specific lists:
  - `VK_BACK_APPS`: WPS (`wps.exe`, `wpp.exe`, `et.exe`), MS Office, LibreWolf/Waterfox/Mullvad, Notion (`:49-56`).
  - `RECOMMEND_FIX_APPS`: Firefox/Zen; sends U+202F before backspace to break spellchecker state, then one extra backspace (`:58-65,237-245`).
  - `CONSOLE_APPS`: cmd, PowerShell, pwsh, Windows Terminal, conhost; uses `AttachConsole` + `WriteConsoleInputW` because `KEYEVENTF_UNICODE` is ignored by PSReadLine (`:67-73,304-397`).
  - `VM_APPS`: native pass-through; no injection.
- `send_backspaces_and_raw` sends each legacy byte as a UTF-16 code unit; this is the likely source of the multi-byte backspace issue (§3.8).
- `SENDING`/tag prevents feedback loops.
- `make_key_input` sets scan code via `MapVirtualKeyW`; `make_unicode_input` uses `KEYEVENTF_UNICODE`.

**Tray/UI** (`tray.rs`, `gui.rs`, `hotkey.rs`, `osd.rs`, `info.rs`, `blacklist.rs`, `converter.rs`, `app_charset.rs`):

- Tray left click toggles Vi/En; double click opens config; right click menu with input method, charset, options, tools, admin toggle, exit (`tray.rs:143-327`).
- Config GUI is WebView2 (`tao`+`wry`), WinUI-ish, with charset/input-method dropdowns, checkboxes, tool buttons (`gui.rs:104-179`). **It opens on every launch** (`main.rs:200-201`), not just first run; no “start minimized” option.
- GUI charset bug described in §3.6.
- Macro manager is a separate WebView2 window; it also implements Vietnamese input inside its own inputs via JS/IPC because SendInput doesn’t work in WebView2 (`gui.rs:283-434`). This is a special self-passthrough path (`hook.rs:270-275`, `send.rs:101-109`).
- Hotkeys: built-in Ctrl+Shift; optional toggle, clipboard-convert, and per-charset/per-input-method hotkeys (`hotkey.rs:12-55,93-139`). Custom toggle is handled in hook; others use `RegisterHotKey` on the hidden window.
- OSD toast for Vi/En and charset/input-method changes (`osd.rs:19-125`); 1.5s, top-center, layered, no activation. Comment says rounded but painting is rectangular (`osd.rs:160-163`).
- Admin: optional `run_as_admin`; on startup relaunches via `ShellExecuteW("runas")` and exits if not elevated (`main.rs:153-166`). This matters for elevated apps/VMware. Manifest itself is `asInvoker`.
- Auto-start: HKCU `Run` key via `reg add` (`gui.rs:28-50`).
- Single instance via named mutex `Global\VnKeyInputMethod`; second instance posts `WM_VNKEY_SHOW_CONFIG` (`main.rs:121-141`).
- **No surrounding-text API, no secure-input/password-field detection, no UIAccess, no auto-update.** Password fields in normal apps are intercepted like any text field. Debug logging can record all keys if `%TEMP%\vnkey_debug` exists (`debug_log.rs:12-29,67-80`): vk/scan/modifiers plus `engine: ascii=... output=...`. VM/RDP is excluded, but not password fields. This is a privacy/security concern.
- No per-app Vietnamese mode; only blacklist and per-app charset. No full app profile (delays/output mode).

**Windows behavior worth adopting (as behavior, not code):** Shift+Left instead of VK_BACK for Chrome-like autocomplete; VK_BACK list for Office/WPS/Notion; Firefox U+202F trick; console `WriteConsoleInputW`; VM/RDP pass-through; deferred Ctrl+Shift toggle; mouse reset; hook watchdog; single SendInput path for transformed keys; `try_lock` pass-through.

### 4.2 macOS (`vnkey-macos`)

**Architecture reality:** `main.m` starts `NSApplication` with `VnKeyAppDelegate`; no `IMKServer`, no `IMKInputController`, no InputMethodKit framework. `CMakeLists.txt:39` explicitly says “CGEventTap cần ApplicationServices, không cần InputMethodKit”. `Info.plist` has `LSUIElement=true` and no `tsInputMethod*`/`ComponentInputModeDict` keys. README/CONTRIBUTING/CHANGELOG are stale on this point.

**Event tap** (`VnKeyEventTap.m`):

- Creates `CGEventTapCreate` at `kCGHIDEventTap`, falls back to `kCGSessionEventTap`; mask includes keyDown/keyUp/flagsChanged (`:333-355`).
- Requires Accessibility (`AXIsProcessTrustedWithOptions`), prompts and retries after 3s (`VnKeyAppDelegate.m:33-58,72-96`).
- Re-enables tap on timeout/user-input disable (`:148-155`).
- Ignores its own events via `kCGEventSourceUserData` tag `"VNKY"` (`:26,162-165`).
- In English mode returns event unchanged (`:167-170`).
- Command/Control: reset engine, pass through (`:175-179`). Option: reset only on Delete/ForwardDelete/arrows, pass through; this is the latest fix (commit title) so VNI keys with Option are not over-reset (`:183-195`).
- Enter/Esc/Tab/arrows/Home/End/PageUp/PageDown reset and pass (`:197-206`).
- **Mouse clicks do not reset**; the comment says “detect via flagsChanged or skip here” (`:208`). Clicking between fields can carry word state until a special key or mode change. No timeout reset.
- Backspace: `process_backspace`; if processed and `backspaces > 1`, sends `backspaces-1` extra deletes, optionally output, then passes the native backspace (`:210-241`). If `backspaces == 1` with output, the output is ignored because the branch is `> 1`.
- Space: calls `process(' ')` for macro; if processed, sends backspaces+output and returns `NULL` (swallows space); otherwise soft-resets and passes space (`:243-269`).
- Printable ASCII `0x21..0x7E`: `engine.process`; if processed, sends backspaces, optionally output, returns `NULL` to swallow the original key (avoids race with async apps); otherwise passes through (`:271-318`). Non-ASCII resets and passes.
- `sendBackspaces`: CGEvent delete down/up, then `usleep(1500)` (1.5ms) so Electron apps (Zen, VS Code) process deletes before replacement text (`:81-105`).
- `sendUnicodeString`: chunks of 20 UTF-16 code units, down/up with `CGEventKeyboardSetUnicodeString` (`:107-139`), similar to OpenKey/XKey.
- No secure-input check (`IsSecureEventInputEnabled`), no app blacklist, no per-app charset, no surrounding text, no input-source integration, no forward-delete handling outside Option, no mouse reset.

**Preferences** (`VnKeyPreferences.m`): input method popup, spell/free/modern/auto-restore/Ede/macro checkboxes, Accessibility button, version. Uses `NSUserDefaults` keys `VnKeyInputMethod`, `VnKeyVietMode`, `VnKeySpellCheck`, `VnKeyFreeMarking`, `VnKeyModernStyle`, `VnKeyAutoRestore`, `VnKeyEdeMode`, `VnKeyMacroEnabled`, `VnKeyMacros` (`VnKeyEventTap.m:30-77`). No charset, no macro editor, no hotkey, no per-app, no excluded apps.

**Status item** (`VnKeyAppDelegate.m`): text “Vi”/“En”; menu toggle, input-method radio items, Preferences, Accessibility, Quit. Selecting an input method posts `VnKeyPreferencesChanged` so all controllers reload (fix #17).

**Packaging contradictions:** `CMakeLists.txt` installs to `/Applications`; `build.sh install` copies to `~/Library/Input Methods`; release DMG symlinks `/Library/Input Methods` (`.github/workflows/build.yml:130-145`). It is ad-hoc signed, not notarized. Users may need right-click Open and Accessibility permission.

### 4.3 Fcitx5 (`vnkey-fcitx5`)

- Addon factory, one `VnKeyEngine` per addon, one `VnKeyState` per input context; each state has its own `vnkey_engine_new()` (`:94-103,539-547`).
- Config loaded once in constructor (`:107-149`); saved on menu changes (`:151-187`). No hot reload.
- Menu/status area: input method, charset, spell/free/modern/Ede/macro toggles, clipboard convert actions (`:234-389`). Radio labels prepend “●” for GNOME AppIndicator workaround (`:225-232`).
- `syncSettings` applies input method on change/gen, always sets `auto_restore=1`, applies options every key, loads macros only if non-empty (`:549-579`). Removing all macros does not clear stale engine macros.
- Activation: sets Vietnamese on, syncs, resets, clears preedit, detects terminal via a hardcoded list, updates app charset from `ic_->program()` basename lowercased (`:581-609`).
- Key handling (`:937-1126`):
  - Ctrl+Space toggles Vietnamese and consumes.
  - Ctrl/Alt/Super: commit preedit and pass through; Shift allowed.
  - Enter/Esc/Tab: commit preedit and pass.
  - Space: `commitPreedit(true)` (soft reset) and pass — **no macro check**.
  - Backspace: terminal preedit edit; GUI direct mode uses `deleteSurroundingText(-backspaces, backspaces)` if capability, then `directCommit`; if no SurroundingText, logs a warning but still commits (possible duplicates).
  - Printable ASCII: `engine.process`; terminal preedit update (commit when at word boundary); GUI direct commit after `deleteSurroundingText`; unprocessed char committed raw; resets engine when at word boundary.
  - Non-ASCII/non-printable: commit preedit, pass.
- **English mode still consumes printable keys**: there is no `vietMode_` early return; with engine `viet_key=false`, `process` returns false and the adapter commits the raw char itself. This breaks app-level key handling and means the toggle only disables transformations, not interception. `vietMode_` is set true on activation and not persisted.
- Direct commit conversion (`:789-849`): applies `app_charset` override, converts via FFI; for “UTF-8-safe” charsets it commits bytes directly, otherwise `bytesToUtf8` (each legacy byte → Latin-1 UTF-8). `isUtf8Charset` includes id 4 (Unicode Composite) but id 4 output is UTF-16LE, not UTF-8 (`:646-655`), so decomposed output is mojibake. Same class of issue for CP1258.
- Terminal detection list (`:851-869`) includes Alacritty, Kitty, Foot, GNOME Terminal, Konsole, xterm, urxvt, WezTerm, st, Terminator, Tilix, XFCE4, Mate, LXTerminal, Sakura, Terminology, Cool-Retro-Term, Deepin, QTerminal, Guake, Yakuake, Hyper, Tabby, Rio, Ghostty, Contour.
- `trySurroundingContext()` exists (`:884-935`) but has **no call site**; context recovery is dead code.
- Clipboard conversion via `wl-paste`/`xclip`/`xsel` (`:697-787`), status-menu actions only.
- Per-app charset from `ic_->program()`; no `.exe` stripping, so Windows-style keys don’t match. No blacklist/excluded apps, no secure field check.
- `addon.conf` says `Configurable=False`, so no separate config UI.

### 4.4 IBus (`vnkey-ibus`)

- One engine process; `VnkIBusEngine` per engine instance, each with its own `vnkey_engine_new()`; preedit buffer 4096 bytes (`:464-471,793-801`).
- Config loaded once at startup (`:199-229`), saved on property changes (`:231-278`).
- Properties: input method, charset, spell/free/modern/Ede/macro toggles, clipboard actions (`:655-766`). No hotkey config.
- Focus in: syncs, registers properties, detects focused exe via `xdotool getactivewindow getwindowpid` + `/proc/PID/exe`, lowercases basename, sets terminal flag, updates app charset (`:849-862`). **Wayland: xdotool fails → no per-app charset/terminal detection.**
- Key handling (`:994-1134`):
  - Ctrl+Space toggles and consumes.
  - Ctrl/Alt commit preedit and pass; Shift allowed.
  - Enter/Esc/Tab commit and pass.
  - Space `commit_preedit_ex(self,1)` — **no macro check**.
  - Backspace: terminal preedit edit; GUI uses `ibus_engine_delete_surrounding_text(-backspaces, backspaces)` then `direct_commit`; if no surrounding capability, the delete call may fail silently and output may duplicate.
  - Printable ASCII: `try_surrounding_context` first (only GUI), then `engine.process`; terminal preedit or GUI direct commit; reset at word boundary.
  - Non-ASCII -> commit preedit, pass.
- **English mode still consumes printable keys** for the same reason as Fcitx5; no early return.
- `is_utf8_charset` includes 4/6/10/11/2/3/1 (`:282-286`); id 4 output is UTF-16LE and is committed as UTF-8 → mojibake.
- Clipboard via shell tools (`:337-457`).
- No blacklist/excluded apps, no secure field check, no app profile beyond terminal.
- `postinst` refreshes icon cache and restarts IBus if a display is present.

### 4.5 Cross-platform integration comparison

| Concern | Windows | macOS | Fcitx5 | IBus |
|---|---|---|---|---|
| Integration | LL keyboard hook | CGEventTap HID/session | Fcitx5 addon | IBus engine |
| Preedit | none (direct replace) | none | terminal only | terminal only |
| Replace mechanism | Shift+Left / VK_BACK / SendInput | CGEvent deletes + Unicode | deleteSurroundingText + commit | deleteSurroundingText + commit |
| Surrounding text | no | no | delete only; context function dead | delete + feed_context |
| Secure input | no | no | framework-dependent | framework-dependent |
| App exclusion | exact exe blacklist | none | none | none |
| Per-app charset | yes | no | yes (`program()`) | yes (xdotool, X11) |
| English-mode passthrough | yes | yes | no (consumes) | no (consumes) |
| State persistence of Vi/En | yes | yes | no (reset on activate) | no (reset on enable) |
| Context reset | mouse + 5s timeout + special keys | special keys only; no mouse/timeout | focus/activate + framework reset | focus/activate + framework reset |
| Backspace delay | immediate batch | 1.5 ms after deletes | framework commit | framework commit |
| Macro on Space | yes | yes | **no** | **no** |
| Charset count exposed | 11/12 (GUI bug) | 0 | 16 | 16 |
| Accessibility | hook may need admin for elevated apps | Accessibility permission | none | none |

---

## 5. Config schema, defaults, import/export

### 5.1 Windows `config.json` (full schema)

Saved by `config.rs:203-306`; loaded by `:20-173`.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `input_method` | int | 0 | 0 Telex, 1 Simple Telex, 2 VNI, 3 VIQR, 4 MS Vietnamese (no-op) |
| `output_charset` | int | 1 | charset ID; 1 UTF-8 |
| `viet_mode` | bool | true | Vi/En |
| `spell_check` | bool | true | structural spell check |
| `free_marking` | bool | true | tone anywhere |
| `modern_style` | bool | true | `hoá` vs `hóa` |
| `ede_mode` | bool | false | Tây Nguyên |
| `macro_enabled` | bool | false | macro expansion |
| `macros` | JSON array | `[]` | `[{"key":"bc","value":"báo cáo"}]` |
| `run_as_admin` | bool | false | relaunch elevated |
| `conv_from` | int | 0 | clipboard source **index** into `CS_IDS` (0 = Unicode) |
| `conv_to` | int | 6 | clipboard target **index** into `CS_IDS` (6 = TCVN3, not charset ID 6) |
| `toggle_vk` | int | 0 | custom toggle VK; 0 = built-in Ctrl+Shift |
| `toggle_mods` | int | 0 | bit 1 Alt, 2 Ctrl, 4 Shift |
| `hk_conv_vk` / `hk_conv_mods` | int | 0 | clipboard convert hotkey |
| `custom_hotkeys` | JSON array | `[]` | `{k:"cs"|"im", id, vk, mods}` |
| `blacklist` | JSON array | `[]` | exact exe names |
| `app_charsets` | JSON object | `{}` | `{"wps.exe":20,...}` |

Parser is line-based (`config.rs:310-323`): it splits on the first `:` per line and trims quotes; it is not a real JSON parser. Saved output puts each key on its own line, so its own files round-trip; hand-edited or minified JSON may not. `macros`/`blacklist`/`custom_hotkeys` are parsed with ad-hoc string splitting (`:116-165,344-352`). `app_charsets` delegates to `app_charset::from_json`, also ad-hoc.

Defaults in `EngineState::new` (`main.rs:66-91`): Telex, UTF-8, Vietnamese on, spell/free/modern on, Ede/macro off. Options struct defaults agree (`lib.rs:29-40`).

### 5.2 Fcitx5/IBus schema

Saved by `vnkey-fcitx5.cpp:151-187` / `vnkey-ibus.c:231-278`:

```json
{
  "input_method": 0,
  "output_charset": 1,
  "spell_check": true,
  "free_marking": true,
  "modern_style": true,
  "ede_mode": false,
  "macro_enabled": false,
  "macros": "bc\tbáo cáo\n",
  "app_charsets": {"firefox": 1}
}
```

No `viet_mode`, `run_as_admin`, hotkeys, blacklist, `conv_*`, `toggle_*`, `custom_hotkeys`. `macros` is a JSON **string** with `\n`/`\t` escapes, not an array. Both load at process start only. IBus loads before `start_component`; Fcitx5 in the addon constructor. Editing the file while running has no effect.

### 5.3 macOS preferences

`NSUserDefaults` keys listed in §4.2. No charset, no config.json, no app charset, no blacklist, no hotkeys, no `viet_mode` file (stored as `VnKeyVietMode`). `VnKeyMacros` is a tab-separated string.

### 5.4 Per-app behavior

- **Windows:** `app_charsets` map exact lowercase exe → charset ID; updated on foreground change from the hook (`hook.rs:259-267`, `app_charset.rs:15-20`). `blacklist` exact exe → pass-through. No per-app input method/options/output mode/delay.
- **Fcitx5:** `app_charsets` keyed by `ic_->program()` basename lowercased; no `.exe` stripping; no blacklist; no per-app options.
- **IBus:** keyed by xdotool-derived basename; X11 only; no blacklist/options.
- **macOS:** none.

Cross-platform sharing of `app_charsets` is unreliable because Windows keys include `.exe` and Linux keys typically do not; Fcitx5 uses app IDs, IBus uses process names.

### 5.5 Import/export and migration

There is **no import/export**, no config schema version, no migration, and no backup. The only “export” is the raw config file / macro table. Cross-platform migration is manual and lossy (macros format, charset IDs, app keys, hotkeys). If ViFastKey adopts a config file, use a versioned schema and a real JSON parser; do not inherit these ad-hoc parsers.

---

## 6. Tests, regressions, and observed bugs

### 6.1 Test suite

`vnkey-engine/tests/integration.rs` has **38 tests**, all pass (`cargo test --offline`):

- Telex basic vowels: `taan`→`tân`, `toon`→`tôn`, `teen`→`tên`.
- `thuee` bug: `thuee`→`thuê`, `khuee`→`khuê`, `muoons`→`muốn`.
- Per-key `thuee` detail: second `e` processes with 1 backspace and output `ê`.
- Tones: `bas/baf/bar/bax/baj` → `bá/bà/bả/bã/bạ`.
- Free marking after coda: `muonf`→`muòn`, `muoons`→`muốn`, `chaof`→`chào`, `toons`→`tốn`, `banr`→`bản`, `tangf`→`tàng`.
- `ddi`→`đi`.
- `tuw`→`tư`, `tow`→`tơ`.
- `taw`→`tă`.
- `w`→`ư`, `ww`→`w`.
- Issue #10: `muowjt`, `muowtj`, `muotwj` → `mượt`.
- Space/backspace soft reset: `no` + space + backspace + `s` → `nó`.
- English structural restore: `fix`, `linux`, `flex`, `project`, `windows`, `power`, `software`, `workflow` unchanged; `www`→`ww`; `how/now/cow`→`hơ/nơ/cơ`.
- VNI: `ba1`→`bá`, `ba2`→`bà`, `a6`→`â`.
- Reset state; macro table lookup/remove/text load; VnLexi tone/case; input-method switching; charset round-trips (TCVN3, VISCII, VNI-Win, VIQR), NCR dec/hex, TCVN3 byte decode/encode, 19 charsets list.

**Empty/skipped tests:** `test_telex_ww_then_continue_typing`, `test_telex_w_consecutive`, `test_telex_w_undo_with_tone` have no assertions (`integration.rs:155-168`). They count as passes but test nothing.

**Missing coverage:** no VIQR *input* tests, no MsVi, no Ede, no macro *expansion*, no app charset, no FFI/ABI tests, no multi-byte backspace, no `feed_context`, no `MAX_BUFFER`, no config parsing, no adapter behavior. CI (`build.yml`) builds packages but does not run `cargo test`. The Python Firefox test is manual and Windows-only, not wired into CI.

### 6.2 Changelog regressions fixed (useful knowledge)

- 1.0.1: UB from unchecked `transmute`; `auto_non_vn_restore`; soft reset; Windows `try_lock`; `ToUnicode` for layouts; Win+D; Facebook first-click; WPS double chars; Chrome autocomplete; Kanata conflict (#13); Ctrl+Shift+N/V (#15); hook watchdog.
- 1.0.2: app_charset; `backspaces_bytes`; all-consonant abbreviations not restored (`đc`/`đk`); `VS_UY` VC order; `w` standalone/`ww`/windows guard (#14); WebView2 GUI; LibreWolf/Waterfox/Mullvad; Fcitx5/IBus direct commit; terminal preedit; per-app charset.
- 1.0.3: uppercase `WW` (#16); macros + case; Ede; macro on Space; Firefox/Notion; Fcitx5/IBus terminal preedit; macOS VNI selection (#17), icon, Ede/macro prefs.
- HEAD: macOS Option over-reset fix; 1.5ms backspace delay for Electron.

These are good regression candidates for ViFastKey adapter tests, but write them independently.

### 6.3 Bugs/edge cases found by this analysis

| # | Finding | Evidence | Severity |
|---|---|---|---|
| 1 | `MS Vietnamese` is a no-op | `input.rs:172-185`; probe `vieejt`→`vieejt` | High (false feature) |
| 2 | `text`/`test` become `tẽt`/`tét`; no dictionary restore | probe; structural `VC_PAIR_LIST` | High vs ViFastKey |
| 3 | `qusa`→`qusa`, not `quá`; tone immediately after `qu` unsupported | probe; `engine.rs:794-803` treats `u` after `q` as consonant, no vowel offset | Medium |
| 4 | VIQR `a'b`→`b`; tone punctuation is `WordBreak`, key history stops | probe `input.rs:146-150`; `engine.rs:715-736` | High for VIQR |
| 5 | VIQR `\?` at word start stays `\?`; escape only inside word | probe; `engine.rs:1777-1786` | Medium |
| 6 | Windows config GUI charset mapping shifts after VISCII | `gui.rs:22-24,107-112` | High UX |
| 7 | NCR-Hex emits raw Latin-1 byte for U+0080–U+00FF | `charset/mod.rs:658-665`; `à`→`E0` | High for NCR |
| 8 | UniDecomposed output committed as UTF-8 on Fcitx5/IBus | `isUtf8Charset` includes 4; output is UTF-16LE | High |
| 9 | `backspaces_bytes` unused; multi-byte charset editing wrong | grep adapters; `engine.rs:544-564` | High for legacy |
| 10 | Macros don’t expand on Space in Fcitx5/IBus | no `process(' ')` in space branch | High for macro |
| 11 | macOS not IMKit despite docs; no input-source bundle | `main.m`, `CMakeLists.txt:39`, `Info.plist` | High docs/product |
| 12 | macOS no mouse/timeout reset; no secure input; no blacklist/per-app | `VnKeyEventTap.m:208`; no APIs | Medium |
| 13 | macOS forward delete not reset/handled outside Option | `:183-206` | Low |
| 14 | English mode still consumes keys on Fcitx5/IBus | no early return; direct commit raw | Medium |
| 15 | TCVN3/VPS/VietWare-F uppercase Ạ→ạ round-trip | probe; duplicate legacy bytes | Medium/legacy-inherent |
| 16 | No NFC/NFD; partial NFD composition only | `from_utf8`/`to_utf8` early returns; probe | High vs ViFastKey |
| 17 | `strict_spell_check` unused | grep | Low |
| 18 | `reverted`/`_converted` dead | grep | Low |
| 19 | Windows letter keys bypass ToUnicode | `hook.rs:537-540` | Medium on non-QWERTY |
| 20 | Windows debug log can record all typed text | `debug_log.rs`; marker file | Security/privacy |
| 21 | Windows config opens every launch | `main.rs:200-201` | UX |
| 22 | Windows GUI `cs_value` fallback silently selects Unicode | `gui.rs:24` | High UX |
| 23 | Converter/`charset_from_id` can’t convert BKHCM1/2/VietWare despite labels | `converter.rs:32-47` | Medium |
| 24 | Macro JSON parser fragile; cross-platform macro format incompatible | `macro_table.rs:118-180`; configs | Medium |
| 25 | `feed_context`/`trySurroundingContext` dead on Fcitx5 | no call site | Medium |
| 26 | `get_backspaces_for_multi_byte` counts TCVN3 chars as 2 | `engine.rs:544-564` | Medium if used |
| 27 | FFI no panic guard/ABI version; headers duplicated | `ffi.rs`, headers | Medium |
| 28 | macOS ad-hoc signing/not notarized; docs imply installable IME | `CMakeLists.txt:97-101` | Product |
| 29 | Windows signing `continue-on-error` may ship unsigned | workflow | Security/docs |
| 30 | Ede VNI `o8/u8` and Telex `bb` don’t match changelog | probe; `engine.rs:1867-1883` | Low/docs |
| 31 | CHANGELOG says Firefox/Zen added to VK_BACK, but code uses the U+202F `RECOMMEND_FIX` path instead | `send.rs:58-65`; CHANGELOG 1.0.3 | Low/docs |

---

## 7. Licensing: what is derived, and what ViFastKey must avoid

### 7.1 VnKey's own license

GPL-3.0-or-later (`LICENSE`; Cargo manifests; source SPDX headers in adapters). The project is a combined work; all linked adapters are GPL-3.0-or-later. Distributing binaries requires source and notices. There is no separate trademark/licensing statement for the Ede fonts.

### 7.2 Evidence of UniKey derivation

The engine is not merely “inspired”; it is structurally and numerically a port:

1. **`VnLexiName`** (`vnkey-engine/src/vnlexi.rs:9-51`) has exactly UniKey's `vnl_*` enum order and integer layout (`vnl_nonVnChar=-1`, `A,a,A1,a1...`, `LastChar`), with case/tone encoded in parity and suffix letters (`r` roof, `b` breve, `h` hook). Compare `x-unikey/src/ukengine/vnlexi.h`.
2. **`VowelSeq`/`ConSeq`** (`:183-254`) have the same ordering and names as UniKey (`vs_a, vs_ar, ...`; `cs_b, cs_c, ...`).
3. **`VSEQ_LIST`** has 71 entries with the same field layout and values/order as `VSeqList` in `x-unikey/src/ukengine/ukengine.cpp:77-149`; **`CSEQ_LIST`** has the same 29 entries (`ukengine.cpp:156-186`).
4. **`VC_PAIR_LIST`** and `is_valid_vc/cv/cvc` correspond to UniKey's spelling tables and rules (`ukengine.cpp`).
5. **Charset data** (`charset/data.rs`) matches UniKey `vnconv`: `TOTAL_VNCHARS=213`, `VnStdCharOffset=0x10000`, `INVALID_STD_CHAR=0xFFFFFFFF`, `UNICODE_TABLE`, `VIQR_TABLE`, `UNICODE_COMPOSITE`, `STD_VN_NO_TONE`, `SingleByteTables`, `DoubleByteTables`, `WinCP1258`, `WinCP1258Pre`; charset IDs 0,1,2,3,4,5,6,10,11,20–25,40–43 correspond to UniKey `CONV_CHARSET_*`.
6. **Engine control flow** (`processRoof`, `processHook`, `processDd`, `processTone`, buffer/tone-position logic) is a close translation of UniKey `ukengine.cpp`, with Rust-specific state and new features (soft reset, auto restore, macros, Ede).

UniKey's `ukengine` is LGPL-2+; `vnconv` is GPL-2+ (`x-unikey/src/vnconv/data.cpp` header; `ukengine/vnlexi.h` header). Both are GPL-3-compatible (`or later`), so VnKey's GPL-3-or-later choice is legally coherent **if** it preserves notices and provides source. The source files here generally do not carry the original copyright/license headers, and README only says “inspired by UniKey”; that is a compliance gap, not a permission for MIT reuse.

### 7.3 Other data/assets

- **No bamboo-core, OpenKey, or third-party dictionary data** was found in the engine. There is no English word list; macro data is user-provided. Comments cite OpenKey/PHTV/XKey for adapter techniques, not copied data.
- **`ede-fonts/`**: 49 TTF files renamed `Vnk*` (Verdana, Times New Roman, Arial, Segoe UI, Calibri, Candara, Consolas, Tahoma, Georgia, Noto Sans, Roboto). Noto/Roboto are OFL, but the Microsoft-named families are proprietary; no font license files or provenance. CHANGELOG says Ede mode references “VnKey cũ của Trần Thanh Bình (VietSID)”. Redistributing these fonts under GPL is legally risky; **do not copy any of them**.
- **Test data**: `tests/integration.rs` is project-authored; `tests/firefox_test.py` is project-authored. No large GPL corpus.
- **Charset tables**: even if one argues raw mappings are factual, the selection/order/duplicate handling and table compilation are part of the GPL work. Treat all of `charset/data.rs` as GPL-derived.

### 7.4 Constraints for ViFastKey (MIT)

1. **No code, tables, comments, enum layouts, test suites, or fonts from VnKey or UniKey.** Reimplement from standards/specs (Unicode, TCVN 6909, VISCII spec, VNI/VIQR conventions) with independent tables and independent tests.
2. **Behavior facts are not copyrightable**, but do not copy large example sets or table arrangements. Write your own conformance cases from first principles.
3. If ViFastKey ever uses a GPL/UniKey-derived table, the whole distributed binary must comply with GPL; that would break the MIT-only goal. Avoid.
4. Add a `THIRD_PARTY_NOTICES.md` only for genuinely independent, permissively licensed data (e.g. Unicode data under Unicode License) with provenance.
5. Do not reuse VnKey's `ede-fonts`, icons, or other assets.

---

## 8. Feature gaps vs ViFastKey and recommendations

### 8.1 What VnKey does better than ViFastKey today

- **Shipped adapters** for Windows, macOS, Fcitx5, IBus, with real-world workarounds: Chrome autocomplete, Firefox spellchecker, WPS/Office, Notion, Electron, terminals, VM/RDP.
- **Legacy charset breadth**: 19 engine charsets, NCR/CP1258/TCVN3/VNI-Win/VISCII/VPS/VIQR/CString/UniDecomposed/BKHCM/VietWare/VNU, plus clipboard conversion.
- **VIQR input** (advertised and present, albeit fragile) and **VIQR charset conversion**.
- **Ede/Tây Nguyên mode** (Êđê, Jarai, M’Nông, K’Ho, Xêđăng, Sán Chỉ) with 12 character pairs.
- **Per-app charset overrides** on Windows/Linux.
- **Desktop product surface**: tray, OSD, hotkeys, config GUI, macro manager, portable exe, admin elevation, auto-start.
- **Packaging/release**: deb/rpm/Arch/Nix, Windows SignPath, universal macOS build.
- **Soft reset on Space** (ViFastKey also has this).
- **Direct commit + terminal preedit** adapters (ViFastKey has the abstraction but no adapters).

### 8.2 What ViFastKey does better

- **Dictionary-based English restore**, including `text`, `data`, `boss`, `server`; no structural false positives for common words.
- **Configurable engine**: `w_as_u`, `bracket_keys`, `quick_telex`, `allow_coda_k`, `macro_digits`, `auto_capitalize`, `smart_caps`, `restore_after_space`, `edit_previous_word`, `excluded_apps`, `app_modes`, `default_vietnamese`, `unicode_form`.
- **NFC/NFD** at output, NFC internal model, UTF-16 delete counts at FFI.
- **Preedit/replace Action abstraction** with `PASS/REPLACE/PREEDIT/COMMIT`, `pass_key`, cursor; adapter-ready.
- **Panic-guarded C ABI, cdylib + staticlib, header, WASM**, JSON config patch.
- **Conformance-scale tests**: 442 Python tests, 13,273 core cases, 3,000 engine scenarios, golden/corpus/property tests.
- **Three-state spell check** (`VALID/PARTIAL/INVALID`) rather than structural tables; better handling of typing order.
- **`qusa` → `quá`**, tone immediately after `qu`.
- **MIT license** and clean-room data path.
- **App profiles** (output mode, delays, wait_ack, edit_previous_word, Vietnamese off) vs VnKey’s charset-only profile.

### 8.3 Prioritized recommendations

Legend: **E** = engine/library repo, **A** = adapter/product, **D** = docs/tests. “Feasible in engine repo” noted.

#### P0 — do first (correctness and licensing)

1. **[D, repo-wide] Licensing firewall and clean-room policy.**
   State that VnKey/UniKey are reference-only; ban copying code/tables/comments/test corpora/fonts. Add a short `docs/reference-projects.md` with provenance and a checklist for new charset/dictionary data. This prevents accidental MIT contamination. Feasible in the engine repo.

2. **[E, tests] Add regression cases for the traps VnKey exposes.**
   Independently write tests asserting ViFastKey’s intended behavior:
   - `text`→`text`, `test`→`test`, `data`→`data`, `boss`→`boss`, `server`→`server` (dictionary restore); `this`→`thí` (Vietnamese priority).
   - `qusa`→`quá`; `quas`→`quá`.
   - `muowjt`/`muowtj`/`muotwj`→`mượt`; `thuee`→`thuê`.
   - `windows`/`power`/`software`/`workflow` unchanged; `đc`/`đk` not restored.
   - Space + backspace + tone soft reset.
   These are P0 because they protect ViFastKey’s differentiators against a future “VnKey parity” refactor.

3. **[D, product] Publish a support matrix.**
   Do not advertise `MS Vietnamese` or `VIQR` until implemented; list Telex/Simple Telex/VNI as supported, legacy charsets as roadmap. VnKey’s no-op MsVi and fragile VIQR are cautionary examples. Feasible in engine repo docs.

#### P1 — high-value features/contracts

4. **[E] Independent legacy charset codec module.**
   Implement a hub-and-spoke converter with:
   - a documented subset (at least UTF-8, Unicode, NCR-Dec/Hex, VIQR, TCVN3, VNI-Windows, VISCII, VPS, CP1258; optionally BKHCM1/2, VietWare F/X, VNU, CString, UniDecomposed);
   - **independent tables** with provenance and license headers;
   - explicit unmappable policy (error or U+FFFD, not silent `#`);
   - NCR-Hex always ASCII entities; no raw Latin-1;
   - full NFC/NFD normalization before lookup;
   - code-unit delete counts for the target encoding (UTF-16/UTF-8/legacy bytes) rather than “non-ASCII=2”;
   - round-trip and lossy-case tests, including uppercase TCVN3 behavior.
   This is the biggest functional gap vs VnKey and is feasible in the engine repo. Do not copy `data.rs`.

5. **[E] VIQR input method (only after charset codec).**
   Data-driven tone/mark keys, proper `\` escape at any position, punctuation-vs-tone disambiguation, word-break handling, and tests such as `a'b`, `a\?`, `mot? hai`. VnKey’s `a'b`→`b` is a bug to avoid, not a behavior to copy. Feasible.

6. **[E] App-profile extension: per-app output charset + adapter hints.**
   ViFastKey already has `AppProfile` (output mode, delays, wait_ack, edit_previous_word, Vietnamese). Add an optional `charset`/`output_charset` field and normalise app keys (strip path, `.exe`, `.app`; lowercase) so Windows/Linux/macOS keys can share. Feasible; don’t inherit VnKey’s `.exe`/no-`.exe` mismatch.

7. **[D/A] Adapter contract and reference recipes.**
   Document/implement independently:
   - replace path with Shift+Left selection vs VK_BACK fallback for Chrome/Office/WPS;
   - U+202F spellchecker workaround for Firefox;
   - console `WriteConsoleInputW` path;
   - VM/RDP pass-through;
   - 1–5 ms backspace delay for Electron;
   - terminal preedit/atomic commit;
   - deferred modifier-toggle logic;
   - hook watchdog;
   - secure-field/password detection.
   This is adapter/product work, **not feasible in an engine-only repository**; keep it in `docs/adapters/` or example adapters.

8. **[E] FFI hardening.**
   Keep ViFastKey’s panic guards; add an ABI version, a single generated/shared header, null-pointer validation, and documented ownership. VnKey’s duplicated headers and unguarded `extern "C"` are a cautionary tale. Feasible.

#### P2 — opportunistic / niche

9. **[E] Ede/Tây Nguyên mode.** Niche but concrete; implement as a data-driven character map, not copied tables. Ensure VNI/Telex key mapping is coherent (avoid VnKey’s o8/u8 changelog mismatch).
10. **[E] MS Vietnamese mapping.** Only if there is user demand; VnKey doesn’t actually implement it. Do not ship a menu item that does nothing.
11. **[A] Clipboard converter.** App-level utility using the engine’s charset codec; Windows native clipboard, Linux `wl-paste`/`xclip`, macOS `NSPasteboard`. Not an engine feature.
12. **[A] Desktop GUI/tray/OSD/hotkeys/packaging/signing.** Product work; VnKey shows the expected surface but also the pitfalls (config GUI opens every launch, GUI charset mapping bug, ad-hoc macOS signing, optional Windows signing, aggressive Linux postinst).
13. **[E] Performance.** Cache charset reverse maps and double-byte lookups; avoid per-call 213-entry sort. Low priority compared to correctness.
14. **[D] Security/privacy.** If adding debug logging, never log key text by default; VnKey’s marker-file logger can record passwords. Document secure-input behavior per platform.

### 8.4 What not to copy (explicit)

- `vnkey-engine/src/vnlexi.rs`, `charset/data.rs`, `engine.rs` tone/diacritic tables and control flow.
- The 213-character tables, 71/29 sequence tables, VC pair list, charset ID layout, `VnLexiName` enum layout.
- VIQR/UniKey/TCVN3/VNI table arrangements, even if individual mappings look factual.
- `ede-fonts/` and icons.
- Macro test values? They are short facts, but write your own examples.
- Adapter code; reimplement the *behaviors* listed in §8.3/§4.

---

## 9. Quick reference appendix

### 9.1 Engine options/defaults

| Option | Default | Used? |
|---|---|---|
| `free_marking` | true | yes |
| `modern_style` | true | yes |
| `macro_enabled` | false | yes |
| `spell_check_enabled` | true | yes |
| `auto_non_vn_restore` | true | yes |
| `strict_spell_check` | false | **no** |
| `ede_mode` | false | yes |
| `viet_key` | true | yes |
| `single_mode` | false | internal |

### 9.2 FFI entry points (instance)

`vnkey_engine_new/free/process/backspace/reset/soft_reset/feed_context/set_input_method/set_viet_mode/set_options/at_word_beginning/add_macro/remove_macro/clear_macros/load_macros/save_macros/free_string`; charset `vnkey_charset_from_utf8/to_utf8`; app charset `vnkey_app_charset_from_json/to_json/free_string/update/get_current`. Singleton equivalents exist for most.

### 9.3 Reproduction commands

```bash
cd thư mục clone tạm/marixdev_vnkey/vnkey-engine
cargo test --offline          # 38 passed

cd probe tạm
cargo run --offline --bin charset   # charset round-trips, NFD, MsVi, macro, Ede
cargo run --offline --bin viqr      # VIQR/escape/caps probes
cargo run --offline --bin ede       # Ede key traces
cargo run --offline --bin bs        # backspace/tone migration
```

### 9.4 Key file references

- Engine: `vnkey-engine/src/engine.rs`, `vnlexi.rs`, `charset/mod.rs`, `charset/data.rs`, `ffi.rs`, `input.rs`, `macro_table.rs`, `app_charset.rs`.
- Tests: `vnkey-engine/tests/integration.rs`.
- Windows: `vnkey-windows/src/hook.rs`, `send.rs`, `config.rs`, `gui.rs`, `tray.rs`, `hotkey.rs`, `blacklist.rs`, `converter.rs`, `app_charset.rs`, `osd.rs`, `debug_log.rs`.
- macOS: `vnkey-macos/src/VnKeyEventTap.m`, `VnKeyAppDelegate.m`, `VnKeyPreferences.m`, `main.m`, `resources/Info.plist`, `CMakeLists.txt`, `build.sh`.
- Linux: `vnkey-fcitx5/src/vnkey-fcitx5.cpp`, `vnkey-fcitx5/src/vnkey-fcitx5.h`, `vnkey-ibus/src/vnkey-ibus.c`, component/addon data.
- Docs: `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, workflows.

---

## 10. VnKey vs ViFastKey — compact matrix

Status: ✅ present, ◐ partial/limited, ❌ absent, ⚠ present but broken/risky.

| Feature | VnKey | ViFastKey | Notes / action for ViFastKey |
|---|---|---|---|
| Telex | ✅ | ✅ | VnKey modern tone default; ViFastKey old default |
| Simple Telex | ✅ | ✅ | VnKey no standalone w; no bracket |
| VNI | ✅ | ✅ | VnKey VNI digits; Ede o8/u8 mismatch |
| VIQR input | ⚠ | ❌ | VnKey `a'b`→`b`; implement cleanly if desired |
| MS Vietnamese | ⚠ no-op | ❌ | Do not advertise until implemented |
| Ede/Tây Nguyên | ✅ | ❌ | Niche P2; reimplement mapping |
| Legacy charsets | ✅ 19 engine / inconsistent UI | ❌ | P1 independent codec; do not copy tables |
| NCR-Dec/Hex | ✅/⚠ | ❌ | NCR-Hex raw Latin-1 bug |
| CP1258 | ✅ | ❌ | P1 codec |
| TCVN3/VNI-Win/VISCII/VPS/VIQR charset | ✅ | ❌ | P1 codec; TCVN3 uppercase case loss |
| BKHCM1/2, VietWare F/X, VNU, CString, UniDecomposed | ✅ engine / ◐ UI | ❌ | P2 unless demanded |
| Clipboard converter | ✅ Win/Linux | ❌ | Adapter utility; not engine |
| Spell check | ◐ structural | ✅ 3-state | VnKey false positives `text` |
| Free marking | ✅ option | ✅ option | |
| Old/new tone | ✅ `modern_style` | ✅ `tone_style` | Defaults differ |
| `w_as_u` | ◐ Telex always / SimpleTelex off | ✅ option | |
| Bracket keys | ◐ always in Telex, not insert-anywhere | ✅ option | |
| Quick Telex | ❌ | ✅ option | VnKey only standard double vowels |
| `allow_coda_k` | ❌ | ✅ option | VnKey `k` never a coda |
| Macro | ✅ | ✅ | VnKey Linux Space bug; formats incompatible |
| Macro incl digits | ✅ implicit | ✅ `macro_digits` | VnKey digits are NonVn but stay in word |
| English restore | ◐ structural only | ✅ dictionary | ViFastKey substantially better |
| Auto capitalize | ❌ | ✅ | |
| Smart caps | ❌ | ✅ | |
| App profiles | ◐ charset only (+Windows blacklist) | ✅ output mode/delays/off | ViFastKey model better |
| Preedit | ◐ terminal only | ✅ action | VnKey no general preedit |
| Replace | ✅ | ✅ | VnKey adapters ship it |
| Surrounding edit | ◐ Fcitx/IBus delete; context dead on Fcitx | ✅ port | ViFastKey should implement adapter contract |
| Soft reset | ✅ | ✅ | |
| Undo/backspace | ✅ | ✅ | VnKey multi-byte delete bug |
| NFC/NFD | ❌ general | ✅ | VnKey partial conversion only |
| C ABI | ✅ staticlib, weak | ✅ cdylib+staticlib, panic-safe | Keep ViFastKey’s |
| WASM | ❌ | ✅ | |
| Conformance/tests | ◐ 38, no adapters/CI | ✅ 442 + 16k cases | Huge ViFastKey advantage |
| Windows adapter | ✅ | ❌ | VnKey recipes valuable |
| macOS adapter | ⚠ CGEventTap, not IMK | ❌ | Do IMK or document CGEventTap |
| Fcitx5 adapter | ✅ | ❌ | Reference only |
| IBus adapter | ✅ | ❌ | Reference only |
| Tray/OSD/GUI | ✅ Windows; menu on macOS/Linux | ❌ | Product work |
| Hotkeys | ✅ Windows | ❌ | Adapter work |
| Packaging/signing | ✅ broad; signing optional/ad-hoc | ◐ CLI/WASM | Don’t copy signing gaps |
| Config format | ◐ 3 incompatible schemas | ✅ versioned JSON | Use one versioned schema |
| Import/export | ❌ | ◐ JSON/CLI | Add migration/versioning |
| License | GPL-3.0-or-later, UniKey-derived | MIT | **No code/table/asset reuse** |

**Bottom line for ViFastKey:** keep the engine (it is already ahead on behavior, testing, and licensing), and mine VnKey only for **adapter behavior recipes**, **legacy-charset product requirements**, and **real-world edge cases**. Implement those independently. The two P0 engineering protections are a clean-room/licensing firewall and regression tests for the English-restore/`qusa`/VIQR traps that VnKey gets wrong.
