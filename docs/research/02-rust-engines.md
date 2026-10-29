> **Báo cáo nghiên cứu nội bộ.** Tài liệu này chỉ ghi nhận hành vi, kiến trúc, UX và
> test fact của các dự án được khảo sát. Không có mã nguồn, bảng dữ liệu, từ điển hay test
> corpus GPL/AGPL nào được sao chép vào ViFastKey. Xem `THIRD_PARTY_NOTICES.md` và
> `docs/IME_FEATURE_RESEARCH.md`.

# Analysis 02 — Rust/C++ Vietnamese IME engines

**Scope:** `marixdev/vnkey` (Rust engine, GPL-3.0), `haidinhtuan/vigo` (Rust tiers, MIT/Apache-2.0), `nguyenrot/goviet` (Rust engine + macOS Swift, MIT), `uvie-project/uvie-mac` (Swift shell around `uvie-rs`), `sonnam0904/telebit` (C++ Telex/VNI core + fcitx5, MIT).

**Method:** source, tests, corpus data, docs, changelogs, and local behavioral probes. The Python ViFastKey oracle was run on the key sequences below (`transform(..., final=True)` unless stated). The Telebit core was compiled locally as a behavioral cross-check. No GPL code, tables, comments, or test files were copied. The UVIe engine itself (`uvie-rs`) is not present in the macOS clone, so its internals are inferred from the C ABI bridge, tests, and `AGENTS.md`.

**Snapshot commits**

| Project | Commit | Date | License |
|---|---|---|---|
| marixdev/vnkey | `b75cdc0` | 2026-04-03 | GPL-3.0-or-later |
| haidinhtuan/vigo | `06862b9` | 2026-02-27 | MIT OR Apache-2.0 (metadata; no LICENSE file in clone) |
| nguyenrot/goviet | `0da0b89` | 2026-09-10 | MIT |
| uvie-project/uvie-mac | `e0bd2c7` | 2026-10-21 | MIT OR Apache-2.0 per README (no LICENSE file in clone; engine separate) |
| sonnam0904/telebit | `969efe2` | 2026-09-15 | MIT (core); bundled dictionaries separately licensed |

---

## 1. Executive summary

1. **ViFastKey’s syllable/tone core is already strong.** Running Telebit’s curated 6,631-syllable Vietnamese dictionary through the ViFastKey Python oracle (modern tone style) produced only **16 mismatches**, all known Telex ambiguities: 13 `oo` words (`boong`, `xoong`, `coóc`, …), `GIF`, `HĐND`, and the run-together `palăng`. This is essentially Telebit’s own documented exception set. Do not replace ViFastKey’s CVC/spell model with Vigo’s or VnKey’s.
2. **The biggest correctness risk is not missing rimes; it is escape/undo semantics.** Any repeated Telex mark key (`a/e/o/d`) currently acts as an undo even when it is far from the original mark. This corrupts ordinary English and proper nouns: `banana → banna`, `Canada → Canda`, `tomorrow → tômrow`, `necessary → nêcsary`, `paragraph → pâgraph`, `acacia → accia`. Telebit keeps all of these unchanged. This is P0.
3. **VNI literal-number handling is broken.** `nam2024 → nãm`, `ba10 → ba`, `top10 → top`, while Telebit correctly returns the raw number. This silently corrupts dates, versions, and identifiers. P0.
4. **Restored/literal state must be sticky.** `text` is correctly restored at the word end, but pressing `s` afterwards produces `tét`. GõViệt’s `restored` flag and UVIe’s dictionary override avoid this preview churn. P0.
5. **The best borrowings are:** Telebit’s “only undo when a conversion actually happened” gate, its VNI digit rules, and its two-way dictionary round-trip test; GõViệt’s valid-prefix modifier gate and sticky restore; UVIe’s post-commit edit ring and diff/introspection FFI; VnKey’s charset hub and per-app charset API (behavior/architecture only, GPL); UVIe/GõViệt/VnKey adapter strategies for terminals, Chromium, Firefox, Office, secure input, and VM/RDP.
6. **Vigo’s SmartEngine (prediction, abbreviation, code-switching) is a demo, not a production design.** Its English detector flips after three characters, its built-in abbreviations include one-letter triggers (`k → không`, `v → với`), and its predictor has a tiny hand-written corpus. Use the toggled-feature architecture as inspiration, not the behavior.
7. **Licensing:** VnKey is GPL-3.0; clean-room reimplementation only. Vigo and UVIe advertise MIT/Apache but ship no LICENSE file in these clones — verify upstream before reuse. Telebit is MIT, but `dict/vietnamese` is GPL-2 test data and must not be shipped inside an MIT binary.

---

## 2. Project inventory

| Project | Stack | Approx. size | Tests / evidence | What it is best at |
|---|---|---:|---|---|
| **VnKey** (`marixdev_vnkey`) | Rust engine + Windows (Rust/Win32), macOS (Obj-C IMKit), fcitx5, IBus | ~10.7k Rust + 1.6k C/C++ | 38 integration tests; no `#[cfg(test)]` engine unit tests; extensive changelog of real-world app bugs | Dense CVC tables, charset conversion, per-app charset, soft reset, raw-key restoration, FFI for multiple hosts |
| **Vigo** (`haidinhtuan_vigo`) | Rust library/CLI + fcitx5 addon | ~6.5k Rust | 133 annotated `#[test]`s (README claims 137), a FastEngine/SyllableEngine parity test, Criterion benches | Engine tiers, zero-allocation fast path, no_std/heapless, action tables, allocation/size assertions |
| **GõViệt** (`nguyenrot_goviet`) | Rust engine + Swift macOS shell (CGEventTap) | ~2.0k Rust + 3.0k Swift | 10 TSV corpora (Telex, VNI, tone styles, English, edge, macros, autocaps, quick Telex, abbreviations), 20 Rust unit tests, 26 Swift tests, randomized stress | Sticky restore, valid-prefix gate, abbreviation preservation, data-driven corpus methodology, injection planning |
| **UVieMac** (`uvie-project_uvie-mac`) | Swift shell around prebuilt `uvie-rs` 2.8.2 | ~9.5k Swift; engine external | 151 XCTest cases across engine typing/state, dispatcher, helpers, features; `AGENTS.md` documents post-commit editing | Diff API, 8-word post-commit edit ring + mid-word edit, English dictionary override, app classes/injection strategies, AX mode, keyboard-layout monitor |
| **Telebit** (`sonnam0904_telebit`) | C++17 core + fcitx5 addon | ~2.8k C++ core, ~13.4k total | 396 `assert`s in `tests.cpp`; 6,631-syllable round-trip; 104,334-word English passthrough baseline; CLI/GUI tests | Rime canonicalization, prefix-set spell gate, evidence-gated escapes, VNI literal numbers, preedit/direct-commit modes, dictionary round-trip methodology |
| **ViFastKey** (current) | Python oracle + Rust crates + C ABI/WASM | ~3.1k Rust + 442 Python tests + 13,273 core / 3,000 engine conformance cases | Strong 3-state syllable model, conformance, property tests | Correct CVC/syllable/tone coverage; already better than Vigo/VnKey here |

### 2.1 VnKey

- Engine layout: `vnkey-engine/src/engine.rs` (~2,031 lines), `vnlexi.rs` (~592), `input.rs` (~287), `charset/` (~1,687), `ffi.rs` (~641), `macro_table.rs` (~187), integration tests (~479).
- Character model: `VnLexiName` is a contiguous `i16` enum where even indices are uppercase and odd indices lowercase; tones are encoded as offsets 0..5 within each vowel family (`a`, `a1`…`a5`, `ar`, `ar1`…). This makes case/tone manipulation integer arithmetic.
- Vietnamese model: 71 `VowelSeqInfo` entries with `len`, `complete`, `con_suffix`, `roof_pos`, `hook_pos`, `with_roof`, `with_hook`; 29 `ConSeqInfo` entries; a sorted `VC_PAIR_LIST`; special `is_valid_cv` rules for `k`, `gi`, `qu`; special `is_valid_cvc` rules for `quynh`, `gieng`.
- Word state: `WordInfo` stores form (`Empty`, `NonVn`, `C`, `V`, `CV`, `VC`, `CVC`), offsets `c1_offset`, `v_offset`, `c2_offset`, plus `vseq`/`cseq`/`tone`/`caps`/raw `key_code`. `mark_change()` tracks the earliest changed position so only the affected suffix is re-emitted; `get_seq_steps()` turns that into backspace count.
- Undo/restore: a parallel raw-key buffer (`key_strokes`) plus `saved_*` state for soft reset; `to_escape` for VIQR `\`; `single_mode` for literal `dd` in non-Vietnamese text; `auto_non_vn_restore` reconstructs raw ASCII keys but deliberately refuses to restore all-consonant abbreviations (`đc`, `đk`).
- Options: `free_marking` (default true), `modern_style` (true), `spell_check_enabled` (true), `auto_non_vn_restore` (true), `strict_spell_check` (declared but unused), `macro_enabled` (false), `ede_mode` (false).
- Input methods: Telex, Simple Telex, VNI, VIQR, `MsVi`, `UserDefined`. `MsVi` is effectively unimplemented: `set_im(MsVi)` resets the key map to all `Normal`, so letters pass through.
- Charsets: 19 IDs in code (README says 15): Unicode/UCS-2, UTF-8, NCR dec/hex, decomposed, CP-1258, C-string, VIQR, UTF8-VIQR, TCVN3, VPS, VISCII, BKHCM1, VietWare-F, VNU/ISC, VNI-Win, BKHCM2, VietWare-X, VNI-Mac. Hub-and-spoke through a 213-character internal `StdVnChar` alphabet. Per-app charset map in `app_charset.rs`.
- FFI: both a global singleton (`vnkey_setup`, `vnkey_process`, …) and an instance API (`vnkey_engine_new`/`process`/`free`); `ProcessResult` carries `backspaces`, `backspaces_bytes`, output bytes, processed flag; `feed_context`, soft reset, macros, charset conversion, app charset.
- Real-world test evidence is mostly the changelog: `windows`/`workflow` non-Vn guard, Firefox U+202F workaround, Notion/Office VK_BACK, terminal preedit, VM/RDP pass-through, `WW` uppercase fix, `Ctrl+Shift+N/V` false toggle.

### 2.2 Vigo

- Three tiers:
  - `FastEngine`: zero heap allocation, `[u8;32]` raw, `[u8;128]` UTF-8 out, max 12 chars, two-pass render; byte-indexed action maps; `size_of::<FastEngine>() <= 168` test; `Send + Sync`.
  - `SyllableEngine`: action-based, `Syllable` with initial/vowel/final/modifications, history stack, bypass, triple-key and double-tap undo, CVC tone placement.
  - `SmartEngine`: wraps `SyllableEngine`, adds validation, prediction, abbreviations, code-switch detection, all individually toggleable.
- Input rules are declarative: `Action::{AddTone, ModifyLetter, ModifyLetterOnFamily, InsertU, ResetInsertedU, RemoveTone, AppendChar}` and ordered action lists per key. Telex `w` tries `ResetInsertedU → Horn → Breve → InsertU`; VNI maps `1..9`.
- `FastEngine` toggle logic: tracks `last_mod_key`/`last_tone_key` and `prev_lower`; only consecutive same keys toggle. This is directly relevant to ViFastKey’s current over-eager undo.
- `SyllableEngine` case preservation: first-uppercase or all-uppercase derived from raw letters, not per-letter state.
- Smart features are toy-quality: static 100-word validation list, 60-word prediction table, 40-word abbreviation table, heuristic English set; see §4.12.
- Build: `std`/`heapless` exactly-one feature, optional `ffi` via cbindgen, optional TUI/CLI. Benchmarks compare against `uvie-rs` and `vi-rs`; README claims 258 ns simple word, 1.46 µs medium sentence, versus 274 ns/1.65 µs for uvie-rs; zero allocations.
- License metadata is MIT OR Apache-2.0, but no LICENSE file is present in the clone.

### 2.3 GõViệt

- Rust engine (`rust/engine`) is a pure state machine returning `Action::{PassThrough, Replace { backspaces, text, forward }}`. No I/O, no globals.
- `Composer` stores `Vec<Letter>` and one `Tone`. `Letter` has `base`, `mark`, `upper`, `from_w`. Rendering applies tone at the computed nucleus index.
- `valid_prefix()` is used *before* accepting any modifier. If the result cannot still grow into a valid Vietnamese syllable, the key lands literally instead. This is the inverse of “transform first, restore later”.
- English auto-restore is stateful:
  - During typing: if the word was transformed and is no longer a valid prefix, restore raw and set `restored`.
  - At commit: if transformed but not a valid full syllable, restore raw and forward the separator.
  - `suppress_restore` after an explicit revert/backspace prevents unwanted restore.
  - `kept_abbreviation()` preserves `đc`, `đn`, `đt`, `đl`, `kô`, `òh`, `àh`, `ùh` instead of restoring to `ddc`/`ofh`.
  - `auto_capitalized` is preserved through restore (`a. text → a. Text`), while ESC intentionally drops it (`a. hello⎋ → a. hello`).
- Backspace rebuilds the composer from rendered text (`from_rendered`), then re-enables composition if the remainder is a valid prefix or kept abbreviation. This gives the nuanced `muowa⌫⌫wif → mười` and `user⌫r → user` behavior.
- `MAX_WORD_LEN = 32`; longer words become literal (URLs/passwords).
- Tests are data-driven TSV with special keys `⌫` and `⎋`; the stress test uses a deterministic LCG, 4 configs, 2,000 iterations × 80 events, and asserts `backspaces <= visible chars` (no over-delete), including emoji, `é`, navigation, Command, Tab, Enter.
- Swift shell adds `TextInjectionPlanner` (grapheme-safe chunks, CRLF/Tab as real key steps, bounded delay), `InjectionScheduler` (serializes slow injection and defers later physical keys), `AppProfiles` (`fast`, `slow`, `selectAndRetype`), secure-input monitor, tap watchdog, and stable signing so Accessibility permission survives rebuilds.

### 2.4 UVieMac

- Engine is external `uvie-rs` 2.8.2 behind a diff C ABI:
  - `uvie_engine_feed(ch, out, cap) -> backspaces`, `backspace`, `commit`, `is_composing`, `committed_text`, `current_output`, `raw_chars`.
  - `uvie_engine_edit_at(caret_back, ch, out, cap, &forward_deletes)`: post-commit editing.
  - Options: input method, modern orthography, relaxed coda, quick Telex, quick start, English dictionary override.
- Engine behavior locked by tests:
  - `chaof → chào`, `toans → toán`, `ranhr → rảnh`, `tieengs → tiếng`.
  - `duowc/dduowc/duwowc → dươc/đươc/gươn` (interesting partial behavior), `nguowif → người`.
  - `ass → as`, `aww → aw`; `AS → Á`, `As → Á`, `aS → á`, `VIEJS → VIEJS`, `Viets → Viết` (centering-diphthong resolution).
  - `ghost/good/book/safari/character` type literally via ~15k English dictionary; `user → uể`, `chaos → cháo`, `reset → rết` are documented collisions.
  - `neebo → nêbo` (V-C-V auto-split), `banana → bânna`, `master` raw chars cover whole word across split.
  - `vieejt` backspace walk: `việ → viê → vie → vi → v → ""`; each backspace returns the display delta.
- Post-commit editing ring: records `(raw, rendered)` for the last 8 committed words; `caretBack` matches a word-end boundary or a mid-word position; mid-word edit returns `forwardDeletes` and re-renders the whole word; editing a word truncates it and all newer entries.
- App classes: bypass (loginwindow, securityagent, screensaver, systemuiserver, iOS Simulator), compound (Safari, Notes, TextEdit, Mail, iWork, Chromium family), Chromium (selection-based replace), AX (Spotlight), excluded (user).
- Injection: fast backspace+text, slow paced per grapheme with total budget, select-and-retype; Chromium native field vs web content distinction; U+202F sentinel for autocomplete; `warmUpSyntheticKeycodePath`; event tagging; non-ASCII pass-through; English-mode synthetic re-post; keyboard layout monitor; per-app language memory packed into one int (1 bit language + 3 bits code table).
- No engine source in the clone; tests are the specification. `AGENTS.md` warns that stale `Frameworks/*.a` can make correct tests look like missing engine features.

### 2.5 Telebit

- Pipeline (`vietnamese.cpp`, `canonicalize.cpp`, `render_utf8.cpp`, `rime_table.cpp`):
  1. Split input into words; only alpha (and digits in VNI) are convertible.
  2. `applyEscapeRules`: doubled tone keys, `ww`, `aaa/eee/ooo/ddd`, trailing hat escape; gated by whether the keys would actually convert.
  3. `extractTone`: any tone key after the first vowel; **last one wins**; all tone keys removed.
  4. `normalizeTripleVowels`: collapse `eee/aaa/ooo`.
  5. `splitOnsetRime`: longest onset, special `dd`, `gi`/`qu` rejoin.
  6. `canonicalizeRimeByTable`: direct table match; otherwise bounded BFS (depth 2) moving duplicated `a/e/o` adjacent and moving `w` after `a/o/u`; 2,048-entry cache.
  7. `applyShapesRime`: ASCII placeholders `B=â, A=ă, E=ê, O=ô, Q=ơ, U=ư, D=đ`; `uow` pair; bare `UQ` becomes `uQ` (`uơ`).
  8. `renderRimeUtf8`: main-vowel index from rime table; modern override for open `oa/oe/uy`; apply tone; preserve case.
  9. Spell gate: valid onset; empty rime OK; first rime char must be a vowel; no `W`/`D` mid-rime; exact rime or prefix of a valid rime; đ-initial bare vowel relaxation.
- VNI is a separate translator: doubled digit escape; trailing digit run with ≥2 tone digits is raw; shape digits 6/7/8/9; tone 1–5; 0 clears; leading digit literal.
- Macros match **raw keystrokes**, case-insensitively, at commit; expansion is verbatim.
- fcitx5 addon supports preedit and direct commit + rollback; force-preedit per app; macros; auto caps; per-input-context state.
- Tests: 396 asserts plus two-way dictionary harness; 6,631 Vietnamese syllables, 104,334 English words; baseline tracks “newly broken”, “changed result”, “fixed” per word, not just counts.
- MIT core; dictionaries are separate test data (`hunspell-vi` GPL-2; `wamerican` BSD-like).

---

## 3. Feature inventory

Legend: ✅ full; ◐ partial/optional; — absent; `!` demo-quality.

### 3.1 Input methods and core semantics

| Capability | VnKey | Vigo | GõViệt | UVie | Telebit | ViFastKey |
|---|---|---|---|---|---|---|
| Telex | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Simple Telex | ✅ | — | ✅ | ✅ | — | ✅ |
| VNI | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| VIQR | ✅ | — | — | — | — | — |
| MS Vietnamese | ◐ declared, no mapping | — | — | — | — | — |
| User-defined key map | ✅ | — | — | — | — | — |
| Free marking | ✅ option | ✅ always | ✅ always | ✅ always | ✅ always | ✅ always, no toggle |
| Old/new tone style | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Double/triple undo | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Delayed/anywhere modifiers | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `allow_coda_k` | — | — | — | — | — | ✅ |
| Relaxed coda `g→ng`, `h→nh` | — | — | — | ✅ option | — | — |
| Quick Telex | — | — | ✅ | ✅ (also `hh→nh`) | — | ✅ |
| Quick start `f/j/w` | — | — | — | ✅ option | — | — |
| Bracket shortcuts | ✅ `[→ơ, ]→ư`, `{→Ơ, }→Ư` | ✅ `[→ư, ]→ơ` | — | — | — | ✅ `[→ơ, ]→ư` |

Notes:
- Bracket conventions disagree: VnKey and ViFastKey use `[ → ơ`, `] → ư`; Vigo uses the opposite. Pick one and document it; ViFastKey currently matches VnKey, not Vigo.
- VnKey’s `MsVi` enum is a placeholder: `InputProcessor::set_im` resets all keys to `Normal`, so do not treat it as a reference implementation.
- Vigo has only `InputMethod::{Telex, Vni}`; its README overstates “Telex and VNI” but not Simple Telex.

### 3.2 Spelling, English, and restoration

| Capability | VnKey | Vigo | GõViệt | UVie | Telebit | ViFastKey |
|---|---|---|---|---|---|---|
| Spell validation | ✅ boolean + CVC tables | ! fixed lists | ✅ onset/rhyme/tone-coda, prefix | ✅ internal | ✅ rime table + prefix set | ✅ 3-state |
| English auto-restore | ✅ invalid word → raw keys | ! heuristic code-switch | ✅ live + commit, sticky | ✅ ~15k dictionary | ✅ spell gate | ✅ core + 227-word dictionary |
| ESC restores raw | ◐ adapter reset | — | ✅ | ✅ | ◐ clears buffer | ✅ |
| Sticky literal after restore | ◐ `NonVn` guard | ◐ bypass after undo | ✅ `restored` | ✅ dictionary commit | ◐ per-word pipeline | — |
| Keep abbreviations (`đc`, `đn`, `òh`) | ✅ all-consonant guard | — | ✅ `kept_abbreviation` | — | — | — |
| Tone-coda restriction (`c/ch/p/t` only sắc/nặng) | ✅ | ◐ | ✅ | ✅ | ◐ rime table only | ✅ |
| Progressive `đ` (đie/đuo) | — | — | — | — | ✅ | — |
| Centering diphthong inference (`Viets → Viết`) | — | — | — | ✅ | — | — |
| V-C-V auto-split (`neebo → nêbo`) | — | — | — | ✅ | — | — |
| VNI literal numbers / doubled digits | — | — | — | — | ✅ | — |
| Trailing hat escape (`dataa`, `mongoo`) | — | — | — | — | ✅ | ✅ |
| Double-tone evidence gate (`coffee` raw) | ◐ | — | ✅ | ✅ | ✅ | — |

### 3.3 Smart features

| Capability | VnKey | Vigo | GõViệt | UVie | Telebit | ViFastKey |
|---|---|---|---|---|---|---|
| User macros | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Macro digits (`mp3`, `k8s`) | ◐ raw buffer only, no digits in Telex | — | — | ◐ | — | ✅ `macro_digits` |
| Macro import/export | ✅ tab/JSON | — | ✅ JSON | ✅ JSON | ✅ UI | ◐ config map |
| Abbreviation expansion | — | ! built-in aggressive | — | — | — | — |
| Word prediction | — | ! unigram/bigram | — | — | — | — |
| Code-switching detection | ◐ restore only | ! heuristic | ✅ invalid-prefix restore | ✅ dictionary override | ✅ spell gate | ◐ commit dictionary |
| Spell suggestions | — | ! Levenshtein over ~100 words | — | — | — | — |
| Tây Nguyên/Êđê | ✅ option | — | — | — | — | — |

### 3.4 Platform, editing, and API

| Capability | VnKey | Vigo | GõViệt | UVie | Telebit | ViFastKey |
|---|---|---|---|---|---|---|
| Preedit | ✅ fcitx/IBus | ✅ fcitx5 | — (replace) | ✅ AX/IMK? shell uses replace | ✅ | ✅ |
| Replace/direct commit | ✅ | — | ✅ | ✅ | ✅ rollback | ✅ |
| Post-commit edit | — (only feed context) | — | — | ✅ 8-word ring + mid-word | — | ◐ one previous word |
| Surrounding text | ✅ `feed_context` | — | — | ✅ | — | ✅ |
| Soft reset after space | ✅ | — | — | — | — | ✅ |
| App profiles | ✅ backspace/charset/blacklist | — | ✅ injection strategy | ✅ compound/Chromium/AX/bypass + memory | ✅ force preedit | ◐ engine hints only |
| Charset conversion | ✅ 19 | — | — | ◐ code table memory | — | — |
| Per-app charset | ✅ | — | — | ◐ memory | — | — |
| UTF-16 FFI lengths | ◐ UTF-8 bytes | — | ✅ | ✅ | — | ✅ |
| C ABI | ✅ singleton + instance | ✅ opaque + alloc strings | ✅ fixed result + large pending | ✅ diff + editAt | — (C++) | ✅ per-session |
| no_std / zero alloc | — | ✅ `heapless` | — | ✅ engine claimed no_std/no-alloc | — | — |
| Conformance/golden | — | few | ✅ 10 TSV corpora | ✅ 151 XCTest | ✅ 396 asserts | ✅ 13,273 + 3,000 |
| Benchmarks | — | ✅ Criterion | — | — | ✅ dictionary harness | ◐ simple bench |

---

## 4. Algorithms and architecture worth adopting

### 4.1 Ordered action tables per key (Vigo, VnKey)

Both engines model a keystroke as an ordered list of possible actions, tried until one succeeds:

- Vigo: `TELEX['w'] = [ResetInsertedU, ModifyLetter(Horn), ModifyLetter(Breve), InsertU]`; `TELEX['a'] = [ModifyLetterOnFamily(Circumflex, 'a')]`; VNI maps digits.
- VnKey: a 256-entry `key_map` maps a key to a `KeyEvType` (`RoofA/E/O`, `HookUO`, `Bowl`, `Dd`, `Tone0..5`, `TelexW`, `MapChar`, `EscChar`, `Normal`). The input method is data, not `if method == Telex` branches.

**Why adopt:** adding VIQR/MS/custom layouts becomes a table, not a new transform path. ViFastKey already has `InputMethodSpec` and `MarkRule`; extend it with:

- `extra_word_keys` for VIQR punctuation (`'`, `` ` ``, `?`, `~`, `.`, `^`, `+`, `(`, `\`),
- `escape_char`,
- `uppercase_inserts` (`{`/`}` → `Ơ`/`Ư`),
- `digit_mode` and `literal_number_rule`,
- optional `tone_after_coda`/`infer_nucleus` flags.

**Avoid:** VnKey’s `UserDefined` 256-entry map is platform-specific; a compact `InputMethodSpec` is enough for ViFastKey.

### 4.2 Dense CVC sequence tables (VnKey)

VnKey’s model is the most complete “data structure” reference:

- 71 vowel sequences, each with `len`, `complete`, `con_suffix`, `roof_pos`, `hook_pos`, `with_roof`, `with_hook`, and a `sub[]` sequence of prefix VowelSeqs.
- 29 consonant sequences, each with `len`, `c[3]`, and `suffix` (can be a coda).
- `VC_PAIR_LIST` sorted by `(VowelSeq, ConSeq)` and binary-searched.
- `is_valid_cv`: `gi` cannot precede `i`; `qu` cannot precede `u`; `k` only precedes a fixed set of front vowels/glides.
- `is_valid_cvc`: special `qu + y + n/nh` and `gi + e/ê + n/ng`.
- `get_tone_position`: single vowel → 0; `roof_pos`; `hook_pos` (with `ươ` exceptions); 3-vowel → middle; modern `oa/oe/uy` → second; else open/closed rule.

**Why adopt:** this is a precise, testable specification of Vietnamese orthography, and the `with_roof`/`with_hook` transitions make toggling marks O(1). ViFastKey’s 3-state parser already covers the same ground, so do not replace it; use these tables as a **test oracle** and, if needed, a fast path for prefix validation.

**Licensing:** the tables and names are GPL-3.0. Build an independent table from Vietnamese orthography/Unicode and validate against ViFastKey’s existing corpus; do not copy the table rows.

### 4.3 Rime canonicalization + prefix-set gate (Telebit)

Telebit’s `canonicalizeRimeByTable` is a compact solution to “modifier keys can be typed anywhere”:

1. Try the raw rime directly in the rime table.
2. If it fails, search the vowel cluster with BFS depth 2:
   - move duplicated `a/e/o` adjacent (to simulate a hat key typed apart from its vowel),
   - move `w` after each `a/o/u` in the cluster.
3. Return the first candidate that matches a valid rime; otherwise return the raw rime.
4. Cache results in a 2,048-entry map; clear when full.

The accompanying `getRimeMainVowelPrefixSet()` precomputes every strict prefix of every valid rime, so partial typing (`tiê` on the way to `tiên`) is validated with one hash lookup. `isValidSyllable` accepts exact rimes or prefixes; an đ-initial bare-vowel rime is additionally accepted so `ddie` can show `đie`.

**Why adopt:** the prefix set is cheap and makes “modifier accepted/rejected” decisions consistent; the BFS is a fallback for free-order marks. ViFastKey’s `Composer` already re-evaluates from raw, so the prefix set is more useful than the BFS. Add it as an internal validation index, not a replacement parser.

**Caveat:** Telebit’s gate does not enforce tone-coda restrictions, so `text → tẽt` (Telebit) while ViFastKey correctly keeps `text`. Combine the prefix gate with ViFastKey’s `vi_syllable.check`.

### 4.4 Incremental valid-prefix gate (GõViệt)

GõViệt applies a modifier only if the resulting letters are still a **valid prefix**:

- onset must be a prefix of a valid onset (`q`, `ng`, `ngh`, …),
- rest must be vowels followed by consonants,
- rhyme must be a prefix of a valid rhyme,
- accept transitional `ie/ye/ue/uye/uo` only when tone is none,
- accept `ưo` as a prefix of `ươ`,
- enforce `gh/ngh` before `i/e/ê`,
- stop codas only allow tone none/sắc/nặng once complete.

If the modifier fails, the key lands literally instead of being transformed and later restored. This produces much less flicker than “always transform, restore at commit”.

**Why adopt:** it complements ViFastKey’s 3-state check and is exactly what is missing around English words that look like Telex prefixes (`user`, `text`, `use`). It also avoids transient invalid renders.

**Avoid:** treating “valid prefix” as “valid syllable”; GõViệt’s `valid_prefix` deliberately accepts incomplete forms. Keep ViFastKey’s `VALID/PARTIAL/INVALID` distinction.

### 4.5 Evidence-gated escape/undo (Telebit; the most important fix)

Telebit only treats a doubled modifier as an escape when the keys would actually have converted:

- Tone double (`ss`, `ff`, `rr`, `xx`, `jj`): there must be a vowel before it, the prefix through the first key must convert, and — when spell-check restore is on — the whole word must also convert. This keeps `ass → as`, `soffa → sofa`, but leaves `coffee`, `mission`, `essay`, `address` intact.
- Mark double (`aaa`, `eee`, `ooo`, `ddd`): only adjacent triple keys.
- `ww`: unconditional collapse to `w` (since `w` is never a plain Vietnamese letter).
- Trailing hat escape: only if the doubled `a/e/o` is at the end **and** the prefix already produced the corresponding hat. Thus `data → dât`, `dataa → data`, `mongo → mông`, `mongoo → mongo`; but `zoo`, `igloo`, `agree`, `kangaroo`, `coffee` are untouched.
- VNI doubled digits: collapse and output literally (`a11 → a1`).
- VNI trailing digit run: if it contains ≥2 tone digits (0–5), the whole word is raw (`nam2024`, `ba10`, `top10`); one tone digit plus shape digits still converts (`viet65`).

**Why adopt:** this is exactly where ViFastKey’s current parser loses letters. Current ViFastKey treats any later `a/e/o` as an undo of an earlier mark; examples observed with the Python oracle:

| Input | ViFastKey now | Correct (Telebit verified; GõViệt also aims for raw English) |
|---|---|---|
| `banana` | `banna` | `banana` |
| `Canada` | `Canda` | `Canada` |
| `tomorrow` | `tômrow` | `tomorrow` |
| `necessary` | `nêcsary` | `necessary` |
| `paragraph` | `pâgraph` | `paragraph` |
| `acacia` | `accia` | `acacia` |
| `amalgam` | `amlgam` | `amalgam` |
| `anaconda` | `anconda` | `anaconda` |
| `coffee` | `cofee` | `coffee` |
| `cheese` | `chée` | `cheese` |
| `mission` | `mision` | `mission` |
| `essay` | `esay` | `essay` |
| `daddy` | `dady` | `daddy` |
| `toolbox` | `tolbox` | `toolbox` |
| `airport` | `aiport` | `airport` |

**Design for ViFastKey:** track `last_mark_key`, `last_mark_pos`, and whether it produced a transformation (Vigo’s `last_mod_key`/`prev_lower` pattern). A mark undo is legal only when the repeated key is the immediately preceding raw key and the action actually changed the word. For tone double, additionally require the structural whole-word conversion gate (onset + rhyme valid, ignoring the tone-coda restriction so `ass` still works). Never set a global `undone` flag when the key merely landed as a literal.

**Undo-history alternatives.** Vigo keeps a `HistoryEntry { key, transformation, syllable_before }` stack: a triple key pops one snapshot and enters bypass (`aaa → aa`), a double tone key restores the pre-tone state and appends the literal (`ass → as`). It is precise but allocates a `Syllable` clone per action. VnKey instead keeps a parallel raw-key buffer and rebuilds state from it; soft reset snapshots the buffer indices. ViFastKey currently uses flags (`undone`, `latched`, `locked`); that is cheap but caused the over-broad undo above. A middle ground is a small fixed ring of `(key, action, prior_mark/tone)` records, enough to support one-step undo and debug traces without per-key heap allocation.

### 4.6 Sticky restored/literal state (GõViệt, UVIe)

GõViệt’s `restored` flag makes the engine inert until the next word break. UVIe’s dictionary override commits the English word and passes the rest through. ViFastKey currently restores only for the current `compute()` call; the next keystroke can re-transform.

Observed: `text` is restored, then `texts` becomes `tét`. Correct behavior is `texts`. Add a session-level `literal/restored` state:

- set when core or dictionary restores a word,
- append subsequent characters raw,
- on backspace, remove one raw char and re-enable composition only if the remainder is a valid prefix/known abbreviation (GõViệt’s `muowa⌫⌫wif` rule),
- clear on word break or ESC return,
- preserve auto-capitalization presentation through restore.

This is the session-level equivalent of Vigo’s `bypass` after a triple undo, VnKey’s `to_escape`/`single_mode`, and GõViệt’s `restored`. Keep it separate from “user explicitly undid a key”; the two states have different lifetime rules.

### 4.7 Diff actions, backspace walk, and post-commit ring (GõViệt, UVIe)

- GõViệt’s `Action::Replace { backspaces, text, forward }` is the smallest correct adapter contract. `diff(old, new)` keeps the LCP, deletes the rest, inserts the suffix. `forward` re-posts the original space/Enter after macro expansion.
- UVIe’s `editAt(caretBack, char)` records committed words `(raw, rendered)` and matches `caretBack` to a boundary (`rendered_len + 1` per older word) or a mid-word position. Mid-word edits return `forwardDeletes` plus a full re-render. Editing truncates that word and all newer entries.
- UVIe’s `rawChars()` returns the whole word’s keystrokes even after V-C-V split or dictionary commit; ESC uses it. An empty raw means the word outgrew the 24-key buffer, so ESC cancels instead of deleting unknown text.
- UVIe’s scratch-buffer test is a good FFI regression: after an empty suffix, the caller buffer must be empty, not stale.

**Why adopt:** ViFastKey has the right `Replace` diff already; adding a committed-word ring and `edit_at` would close the gap with UVIe while keeping the current one-word surrounding adoption as a fallback.

### 4.8 Zero-allocation fast tier and no_std (Vigo)

Vigo’s FastEngine is a useful blueprint if ViFastKey needs an embedded/hot-loop tier:

- fixed stack buffers: raw 32 bytes, UTF-8 out 128 bytes, max 12 chars;
- byte-indexed action maps for ASCII keys;
- one pass builds `chars[]`, tracks pending tone and modification toggles;
- second pass finds vowel bounds and applies tone;
- no heap allocation; `size_of` and `Send + Sync` assertions;
- `heapless` feature for no_std.

ViFastKey’s Rust `Composer` already measures ~0.50 µs/key vs ~1.03 µs/key for full transform on this machine. A fixed-array `FastSession` would mainly help allocation-sensitive hosts (WASM/embedded/IME hooks), not desktop correctness. Do it behind a feature, with a parity test against `Composer`.

### 4.9 Charset hub-and-spoke and per-app charset (VnKey)

VnKey’s design is the most complete charset layer in the set:

- internal `StdVnChar` = 213-character alphabet (186 Vietnamese letters + 27 Western symbols), values ≥ offset are internal indices, below are passthrough;
- decode/encode for Unicode, UTF-8, NCR decimal/hex, decomposed, CP-1258, C-string, VIQR, UTF8-VIQR, TCVN3, VPS, VISCII, BKHCM1, VietWare-F, VNU/ISC, VNI-Win, BKHCM2, VietWare-X, VNI-Mac;
- single-byte reverse maps, double-byte sorted lookup, NFD combining handling;
- `convert(from, to)`, `from_utf8`, `to_utf8`;
- `backspaces_bytes` counts 2 bytes per non-ASCII char for VNI-Win/BKHCM2/VietWare-X/VNI-Mac;
- per-app map `exe → charset_id`, JSON serialization, `CS_IDS`/`CS_NAMES` for UI;
- FFI conversion and app-charset functions.

**Why adopt:** legacy documents and Office/WPS users need it; it also forces the adapter contract to distinguish Unicode code points from output bytes. ViFastKey should add a separate `vfk-charset` crate that converts only at output; the engine remains Unicode/NFC internally.

**Licensing:** VnKey’s tables are GPL. Generate equivalent tables independently from the published charset standards and verify with round-trip tests. Do not copy `data.rs`.

### 4.10 FFI design

| Design | Strength | Weakness |
|---|---|---|
| VnKey instance + singleton | Two integration styles; `backspaces_bytes`; context feed; app charset | GPL; API surface large |
| Vigo opaque + C strings | Simple; cbindgen | Allocates and requires free; no config/introspection |
| GõViệt fixed `VkResult` | No allocation across boundary; 256 UTF-16 cap + large pending; `forward` flag | One global mutex; large-text indirection |
| UVIe diff API | `backspaces` + suffix; `editAt` + forward deletes; `rawChars`, `currentOutput`, `isComposing` | Fixed 128-byte output buffer (adequate for one word); engine source separate |
| ViFastKey current | Per-session, UTF-16 lengths, `pass_key`, panic guard, pointer into session | Missing `edit_at`, `forward_delete`, raw/current introspection, charset/delete-bytes, preedit cursor after edits |

**Recommendation:** keep ViFastKey’s per-session opaque handle and `VfkAction`. Add:

- `vfk_action_forward_delete_utf16` (or a new action field),
- `vfk_edit_at(handle, caret_back, char, *forward_delete, *delete_before, *insert)`,
- `vfk_raw_chars`, `vfk_current_output`, `vfk_committed_text`, `vfk_is_composing`,
- `vfk_soft_reset`, `vfk_set_charset`, `vfk_charset_convert`, `vfk_set_app_charset`,
- `vfk_action.delete_bytes` for legacy charsets,
- ABI `struct_size`/version field so adapters can evolve safely.

### 4.11 Adapter and injection strategies

The engine is only half the product; these repos contain the hard-won platform knowledge:

**VnKey (Windows/macOS/Linux):**
- Default replace path uses Shift+Left selection to avoid Chrome autocomplete swallowing VK_BACK; fall back to VK_BACK for WPS, MS Office, LibreWolf/Waterfox/Mullvad, Notion.
- Firefox-based browsers get a U+202F sentinel before backspace, plus one extra backspace to remove it.
- Console apps (`cmd`, PowerShell, Windows Terminal, conhost) use VK_BACK + `WriteConsoleInputW`, because `KEYEVENTF_UNICODE` is ignored by PSReadLine.
- VM/RDP clients (VMware, VirtualBox, mstsc, VNC) pass through entirely; synthetic keys break guest login.
- Mark synthetic events (`dwExtraInfo`/global flag); don't re-inject unhandled keys through `SendInput` (breaks Kanata remapping).
- `Ctrl+Shift` toggle is deferred to keyup and cancelled if another key intervenes, to avoid `Ctrl+Shift+N/V`.
- Option/Shift alone must not reset the composing word.
- Terminal/Electron apps use preedit/atomic commit to avoid split backspace+commit.

**GõViệt (macOS):**
- Strategies: `.fast`, `.slow`, `.selectAndRetype`, `.passthrough`.
- Defaults: terminals and Electron editors (VS Code, Cursor) slow; Chrome/Edge/Arc/Brave/Vivaldi/Chromium/Cốc Cốc select-and-retype; Spotlight/Raycast slow.
- Slow injection is serialized off the tap thread; later physical keys are consumed and replayed behind the replacement, with PID targeting; a `pending` counter preserves order.
- Text chunks never split a grapheme; CRLF/Tab become real key events; per-character for short text, 64-unit bulk for long; total delay budget 2 s.
- Secure input is polled; tap watchdog re-enables; stable signing avoids TCC resets; event source suppression interval 0 and filter allows hardware events.
- Manual test matrix covers TextEdit, Notes, Safari, Chrome omnibox, Google Docs, VS Code terminal, iTerm2, Spotlight, Excel, Messages, password fields, games.

**UVIeMac (macOS):**
- Classes: bypass system UI + iOS Simulator; compound (selection-based atomic replace) Safari/Notes/TextEdit/Mail/iWork + Chromium; Chromium native field vs web content detection; AX for Spotlight; excluded user list.
- Compound replacement: Shift+Left selection + postText for native fields; plain backspace + U+202F sentinel for Chromium web content; forward-delete tail for mid-word edits (selection + one forward delete, or plain forward deletes in web content).
- English mode re-posts character keys as synthetic events to suppress autocomplete/autocorrect.
- Non-ASCII keys are never swallowed: commit composing word, reset, pass through; empty diff/no composition also passes through.
- Keyboard layout monitor prefers input-source language codes over ID keywords; auto-disables on CJK/Cyrillic/etc.
- Per-app memory stores language + code table; Fn tap and modifier-only chord toggle; custom hotkey detected inside the tap so apps with custom text engines can't swallow it.
- Event tap self-heals on `tapDisabledByTimeout/UserInput` and via watchdog; synthetic event tagging; warm-up for first keycode post after a rebuild.

**Telebit (Linux fcitx5):**
- Preedit vs direct commit + rollback; direct commit is default but per-app force-preedit for Firefox and other incompatible apps.
- Per-input-context state so windows don't share buffers.
- Macros can trigger at space/Enter/punctuation; expansion is verbatim.
- 2 s total budget / per-app pacing is a useful pattern if ViFastKey adds a slow path.

**ViFastKey adaptation:** implement `AppProfile` with `strategy`, `output_mode`, `backspace_delay_ms`, `delay_per_char_ms`, `wait_ack`, `edit_previous_word`, `vietnamese`, `charset`, plus app classes (terminal, chromium, ax, bypass, compound). The engine should only expose hints; the adapter owns injection.

### 4.12 Smart features: what to adopt, what to avoid

- **Vigo prediction:** unigram/bigram `HashMap`s with 60 hand-written words and a few dozen bigrams; fallback to global unigrams; no model persistence or pruning. Architecture (toggleable, prefix filter) is fine; the data and latency are not production-ready. If ViFastKey ever adds a “smart” tier, make it a separate optional crate, disabled by default, with a real corpus, memory cap, and latency budget.
- **Vigo abbreviation expansion:** built-ins include `k→không`, `v→với`, `r→rồi`, `ok→được`; this would make normal typing impossible if enabled broadly. ViFastKey’s explicit user macros are safer. Keep expansions opt-in, exact-match, and never auto-expand a valid Vietnamese word.
- **Vigo code-switching:** switches to English when `raw.len() >= 3` and a static dictionary/heuristic says so; clears the Vietnamese engine and buffers raw. This can flip mid-word and lose composition state. Prefer UVIe’s dictionary override at word level with a Vietnamese-common-word exclusion, or GõViệt’s invalid-prefix restore.
- **Vigo spell suggestions:** Levenshtein over ~100 common syllables. Useful as a demo, not as a core dependency.
- **GõViệt keep-abbreviation list:** small, explicit, and effective (`đc`, `đn`, `đt`, `đl`, `kô`, `òh`, `àh`, `ùh`). Adopt the behavior, not a large hardcoded list.
- **Telebit two-way dictionary test:** the best quality gate in the set. Adopt the methodology, not the GPL dictionary.

---

## 5. Concrete test cases

### 5.1 Telex core (should already pass; use as regression)

| Keys | Expected | Source |
|---|---|---|
| `vieejt` | `việt` | Vigo, GõViệt, UVIe, Telebit, VnKey |
| `Vieejt Nam` | `Việt Nam` | Vigo/GõViệt |
| `VIEETJ` | `VIỆT` | Vigo |
| `xin chaof` | `xin chào` | all |
| `tieengs Vieejt` | `tiếng Việt` | GõViệt/VnKey |
| `ddaay` | `đây` | GõViệt |
| `truowngf` | `trường` | GõViệt |
| `thuowngr` | `thưởng` | GõViệt |
| `nguyeenx` | `nguyễn` | GõViệt/Telebit |
| `quynhf` | `quỳnh` | GõViệt/Telebit |
| `quas` | `quá` | GõViệt |
| `quaan` | `quân` | GõViệt |
| `nawm` | `năm` | GõViệt |
| `laji` | `lại` | GõViệt |
| `muoonj` | `muộn` | GõViệt |
| `ddoongf` | `đồng` | GõViệt |
| `nghieeng` | `nghiêng` | GõViệt |
| `gifn` | `gìn` | GõViệt |
| `giaf` | `già` | GõViệt |
| `hoas` | `hóa` (old) / `hoá` (new) | GõViệt/Telebit |
| `hoaf` | `hòa` / `hoà` | GõViệt |
| `tuyeejt` | `tuyệt` | GõViệt |
| `uwa` | `ưa` | GõViệt |
| `ows` | `ớ` | GõViệt |
| `muwowjn` | `mượn` | GõViệt |
| `thuowng` | `thương` | GõViệt/Telebit |
| `tuwong` | `tương` | GõViệt |
| `nguwowif` | `người` | GõViệt/Telebit |
| `hueej` | `huệ` | GõViệt |
| `khuyur` | `khuỷu` | GõViệt |
| `toansz` | `toan` | GõViệt |
| `DDoongf` | `Đồng` | GõViệt/Telebit/ViFastKey |
| `dDoongf` | `Đồng` | GõViệt only; **ViFastKey and Telebit both give `đồng`** (deliberate case-policy difference, not a universal bug) |
| `ddieerm` | `điểm` | GõViệt |
| `ddieenj` | `điện` | GõViệt |
| `ddieeuf` | `điều` | GõViệt |
| `dduooi` | `đuôi` | GõViệt |
| `dduooir` | `đuổi` | GõViệt |
| `1 xius` | `1 xíu` | GõViệt |
| `1 xisu` | `1 xíu` | GõViệt |
| `cos xius taoj huongws` | `có xíu tạo hướng` | GõViệt |
| `cos cos cos` | `có có có` | GõViệt |
| `as casc cacs af ar ax aj aan awn aans awns` | `á các các à ả ã ạ ân ăn ấn ắn` | GõViệt |

### 5.2 Telex undo/escape (critical)

| Keys | Expected | Why |
|---|---|---|
| `aaa` | `aa` | Triple hat undo |
| `eee` | `ee` | Triple hat undo |
| `ooo` | `oo` | Triple hat undo |
| `ddd` | `dd` | Triple stroke undo |
| `uww` | `uw` | Double-w undo |
| `ww` | `w` | ww undo |
| `w` | `ư` | standalone w |
| `wa` | `ưa` | w + vowel |
| `ass` | `as` | Double tone undo |
| `aff` | `af` | Double tone undo |
| `asz` | `a` | z removes tone |
| `tesst` | `test` | double s undo |
| `data` | `dât` | delayed hat |
| `dataa` | `data` | trailing hat escape |
| `gamaa` | `gama` | trailing hat escape |
| `mongo` | `mông` | delayed hat |
| `mongoo` | `mongo` | trailing hat escape |
| `tepee` | `tepe` | trailing escape escalation |
| `tepeee` | `tepee` | escalation |
| `caa` | `câ` | adjacent aa is hat, not escape |
| `salaam` | `salaam` | trailing double not escape |
| `bazaar` | `bazaar` | same |
| `aardvark` | `aardvark` | same |
| `Isaac` | `Isaac` | same |
| `Canaan` | `Canaan` | same |
| `graal` | `graal` | same |
| `caan` | `cân` | adjacent aa hat |
| `khuaay` | `khuây` | same |
| `sofa` | `sòa` | tone conversion |
| `soffa` | `sofa` | evidence-gated double tone |
| `javaa` | `javaa` | no hat to undo |
| `pandaa` | `pandaa` | no hat to undo |
| `zoo` | `zoo` | no hat |
| `igloo` | `igloo` | no hat |
| `voodoo` | `voodoo` | no hat |
| `kangaroo` | `kangaroo` | no hat |
| `agree` | `agree` | no hat |
| `moo` | `mô` | adjacent oo hat |
| `tree` | `trê` | adjacent ee hat |
| `mongoose` | `mongoose` | only trailing double escapes |
| `mongoos` | `mongoos` | same |
| `coffee` | `coffee` | **ViFastKey now `cofee`** |
| `toffee` | `toffee` | **now `tofee`** |
| `office` | `office` | **now `ofice`** |
| `mission` | `mission` | **now `mision`** |
| `essay` | `essay` | **now `esay`** |
| `assign` | `assign` | **now `asign`** |
| `afford` | `afford` | **now `aford`** |
| `affair` | `affair` | **now `afair`** |
| `suffer` | `suffer` | **now `sufer`** |
| `difference` | `difference` | **now `diference`** |
| `daddy` | `daddy` | **now `dady`** |
| `toolbox` | `toolbox` | **now `tolbox`** |
| `cheese` | `cheese` | **now `chée`** |
| `airport` | `airport` | **now `aiport`** |
| `workflow` | `workflow` | **now `wokflỏw`** |
| `banana` | `banana` | **now `banna`** |
| `Canada` | `Canada` | **now `Canda`** |
| `tomorrow` | `tomorrow` | **now `tômrow`** |
| `necessary` | `necessary` | **now `nêcsary`** |
| `paragraph` | `paragraph` | **now `pâgraph`** |
| `acacia` | `acacia` | **now `accia`** |
| `amalgam` | `amalgam` | **now `amlgam`** |
| `anaconda` | `anaconda` | **now `anconda`** |

### 5.3 Tone placement and rimes (regression strength)

| Keys | Expected | Notes |
|---|---|---|
| `tusaan` / `tuanas` | `tuấn` | any-position tone |
| `ngojc` | `ngọc` | delayed tone |
| `hoanfg` | `hoàng` | tone after coda |
| `hongof` | `hồng` | tone after coda |
| `buowir` | `bưởi` | pair horn |
| `uoirw` / `uoiwr` | `ưởi` | w position |
| `vayaj` | `vậy` | delayed â |
| `lieuej` | `liệu` | delayed ê |
| `kieuer` | `kiểu` | delayed ê |
| `lieuje` | `liệu` | tone inside then ê |
| `huaws` | `hứa` | ưa |
| `chuaw` | `chưa` | ưa |
| `hopwj` | `hợp` | ơ |
| `hungws` | `hứng` | ư |
| `bapws` | `bắp` | ă |
| `unws` | `ứn` | ưn |
| `unwx` | `ữn` | ưn + ngã |
| `uasw` | `ứa` | ưa |
| `uowns` | `ướn` | ươn |
| `uowngs` | `ướng` | ương |
| `oorn` | `ổn` | ô + coda |
| `ngoaij` | `ngoại` | oai |
| `xoays` | `xoáy` | oay |
| `huow` / `khuow` / `quow` | `huơ` / `khuơ` / `quơ` | bare uơ |
| `thuowr` / `uowr` | `thuở` / `uở` | uơ + tone |
| `nguowif` | `người` | ươi |
| `ruowuj` | `rượu` | ươu |
| `uownf` | `ườn` | ươn |
| `thuowngf` | `thường` | ương |
| `uwow` / `huwow` / `nguwow` | `uơ` / `huơ` / `nguơ` | order agreement |
| `buoomf` | `buồm` | uôm |
| `nhuoomj` | `nhuộm` | uôm |
| `luoomj` | `luộm` | uôm |
| `thuoomj` | `thuộm` | uôm |
| `ngoamj` | `ngoạm` | oam |
| `khoawmf` | `khoằm` | oăm |
| `soajp` | `soạp` | oap |
| `hueenh` | `huênh` | uênh |
| `khueechs` | `khuếch` | uêch |
| `ngueechj` | `nguệch` | uêch |
| `gieengs` / `gieeng` | `giếng` / `giêng` | êng |
| `yeems` / `yeengr` | `yếm` / `yểng` | yêm/yêng |
| `huwmf` / `uwmf` | `hừm` / `ừm` | ưm |
| `ngoaos` / `ngoems` / `xoengr` | `ngoáo` / `ngoém` / `xoẻng` | oao/oem/oeng |
| `tuyps` | `tuýp` | uyp |
| `huynhf` / `khuynhr` / `luynhs` / `huychs` | `huỳnh` / `khuỷnh` / `luýnh` / `huých` | uy + coda |
| `quynhf` / `quyts` / `quytj` | `quỳnh` / `quýt` / `quỵt` | qu + uy |
| `quys` / `quyf` | `quý` / `quỳ` | bare quy |
| `quyeenr` | `quyển` | quyên |
| `quyaf` / `quyuf` | `quyaf` / `quyuf` (Telebit) | **ViFastKey `quỳa`/`quỳu`** |
| `gif` / `gis` / `gir` / `gix` / `gij` | `gì` / `gí` / `gỉ` / `gĩ` / `gị` | gi as g+i |
| `gi` / `gia` / `giaf` / `gioongs` | `gi` / `gia` / `già` / `giống` | gi onset |
| `ddie` / `dduo` | `đie` / `đuo` (Telebit) | **ViFastKey raw** |
| `ddieecs` / `ddieets` / `dduoocs` / `dduowcj` | `điếc` / `điết` / `đuốc` / `được` | đ + iê/uô |
| `ddog` / `ddz` | `ddog` / `ddz` | invalid đ words restore |

### 5.4 VNI

| Keys | Expected |
|---|---|
| `vie65t` / `viet65` | `việt` |
| `d9i` / `di9` | `đi` |
| `nga2y` / `ngay2` | `ngày` |
| `tuo7ng1` | `tướng` |
| `hoa8c5` | `hoặc` |
| `que6n` | `quên` |
| `a1` | `á` |
| `1a` | `1a` |
| `la2m` | `làm` |
| `nguoi72` | `người` |
| `mu7a` | `mưa` |
| `toan1` | `toán` |
| `d9ie63m` | `điểm` |
| `1 xiu1` / `1 xi1u` | `1 xíu` |
| `tua6n` | `tuân` |
| `toi6` | `tôi` |
| `tu7o7ng` | `tương` |
| `a8n` | `ăn` |
| `nguye64n` | `nguyễn` |
| `Vie65t` / `VIE65T` | `Việt` / `VIỆT` |
| `a11` | `a1` |
| `nam2024` | `nam2024` |
| `ba10` | `ba10` |
| `top10` | `top10` |
| `2024` | `2024` |
| `box1` | `box1` |
| `toa10n` | `toan` |
| `person` / `address` / `cheese` / `caan` | unchanged |
| `u7ow1` | `u7ow1` |

**Observed ViFastKey failures:** `nam2024 → nãm`, `ba10 → ba`, `top10 → top`, `a6 → a6` at final, `taw → taw` at final. `a11 → a1` and `u7ow1` already pass.

### 5.5 Tone styles

| Keys | Old | New |
|---|---|---|
| `hoas` | `hóa` | `hoá` |
| `hoaf` | `hòa` | `hoà` |
| `khoer` | `khỏe` | `khoẻ` |
| `thuys` | `thúy` | `thuý` |
| `oaf` | `òa` | `oà` |
| `uys` | `úy` | `uý` |
| `hoanfg` | `hoàng` | `hoàng` |
| `ngoaij` | `ngoại` | `ngoại` |

### 5.6 English restore and known limitations

**Must stay unchanged (auto-restore / dictionary):**

`text`, `add`, `address`, `boundary`, `example`, `keyboard`, `world`, `water`, `good`, `for`, `just`, `software`, `expression`, `function`, `class`, `first`, `strong`, `string`, `map`, `window`, `render`, `user`, `and`, `are`, `that`, `what`, `when`, `you`, `with`, `work`, `quit`, `new`, `two`, `die`, `pie`, `use`, `user`, `dies`, `texted`, `hello`, `hello123`, `viet`, `ghost`, `book`, `safari`, `character`, `permission`, `system`, `coffee`, `cheese`, `mission`, `essay`, `assign`, `afford`, `affair`, `suffer`, `difference`, `daddy`, `toolbox`, `airport`, `workflow`, `banana`, `Canada`, `tomorrow`, `necessary`, `paragraph`, `acacia`, `amalgam`, `anaconda`.

**Known deliberate collisions (do not chase to zero):**

`mix → mĩ`, `did → đi`, `was → ứa`, `wow → ươ` (GoViệt) or `wow` (ViFastKey/Telebit), `how → hơ`, `now → nơ`, `this → thí`, `as → á`, `is → í`, `os → ó`, `those → thóe`, `off → of`, `test → tét`, `there → thể`, `password → pasword`, `tow → tơ`, `reset → rết`, `chaos → cháo`, `most → mót`, `air → ải`.

**Keep abbreviations (GõViệt):**

`ddc → đc`, `ddn → đn`, `ddt → đt`, `ddl → đl`, `ofh → òh`, `koo → kô`, and `ddc ` / `ddc.` / `ddn `; but `add → add` and `two → two`.

### 5.7 Engine/session behavior

| Scenario | Expected |
|---|---|
| `vieejt` + backspace × 6 | `việ → viê → vie → vi → v → ""` (UVIe) |
| `vieejt⌫t` | `việt` |
| `vieejt⎋` | `vieejt` |
| `text` then `s` | `texts`, **not `tét`** |
| `text` + space | `text` (commit restore) |
| `text` + backspace | `tex` literal; re-enable if remainder valid |
| `muowa⌫⌫wif` | `mười` |
| `muowa⌫⌫⌫owif` | `mười` |
| `user⌫r` | `user` |
| `no` + space + backspace + `s` | `nó` (soft reset) |
| `Chào` + space + backspace + `f` | `Chaof` (soft reset) |
| `Việt` + `f` | `Việtf` (not `vieetjf`) |
| `1 xius` after number | `1 xíu` |
| `1 xius 2 chuts` | `1 xíu 2 chút` |
| `chaof. anh` (auto caps) | `chào. Anh` |
| `a. ddi` | `a. Đi` |
| `xin chaof! toi` | `xin chào! Toi` |
| `a? b` | `a? B` |
| `a, b` | `a, b` |
| `a. hello` / `a. text` | `a. Hello` / `a. Text` |
| `a. hello⎋` / `a. text⎋` | `a. hello` / `a. text` |
| `vn` + space | `Việt Nam ` |
| `BTW` + space | `BY THE WAY ` |
| `Btw` + space | `By the way ` |
| `btwx` + space | `btwx ` (no prefix match) |
| `mp3` + space with macro | expansion, including digit |
| macro expansion >65,535 UTF-16 units | not truncated (UVIe) |

### 5.8 Adapter / injection cases

| Scenario | Requirement |
|---|---|
| Chrome omnibox `1 xius`, `1 xisu` | `1 xíu`, initial consonant not lost; select-and-retype or sentinel |
| Chrome Google Docs contenteditable | plain backspaces + sentinel; no duplicate chars |
| Safari/Notes/TextEdit/Mail/iWork | selection-based atomic replace; no flicker |
| VS Code / Cursor / iTerm2 / Terminal / Ghostty | paced injection; terminal preedit/atomic commit |
| Spotlight | AX injection or slow path; no swallowed first char |
| Password/secure input | pass through; reset engine; badge state |
| Excluded app/game/VM/RDP | pass through entirely |
| Non-Latin keyboard layout | pass through + reset; don't swallow keystrokes |
| App switch while slow queue pending | replacement and deferred keys go only to original PID |
| Emoji/combining grapheme between words | chunks never split a grapheme; no over-delete |
| Macro with 200+ chars then `abc`/click | expansion lands first; no lost keys; no tap timeout |
| `Cmd+A` / Shift+arrows selection | reset diff state |
| `Option+Backspace` | OS word delete; reset engine |
| `Shift+Backspace` | char-by-char delete preserving case |
| Non-ASCII key (`é`, Cyrillic) | commit composing word, reset, pass through |
| Mouse down / click into text | reset composing state; per-app sentence-start handling |
| Tap disabled by timeout/user input | re-enable + reset stale state |

### 5.9 Property, stress, and dictionary methodology

- **GõViệt stress:** deterministic LCG, 4 configs (`default`, `SimpleTelex + quick_telex`, `VNI new`, `auto_capitalize + no English restore`), 2,000 engines × 80 events, content includes letters, digits, `é`, emoji, space, punctuation, newline, tab, backspace, ESC, Left, Command+A; assert `backspaces <= visible` every step, no panic. This is a strong template for ViFastKey.
- **Telebit dictionary round-trip:** Vietnamese syllable → generated Telex keys → engine → compare; English word → engine → byte-identical. Run both old/new tone styles; count a Vietnamese failure only if it fails in both (hunspell mixes conventions). Compare per-word lists, not just totals: “newly broken”, “changed result”, “fixed”. Baseline files make behavior changes reviewable.
- **Vigo benchmarks:** Criterion groups for batch transform, incremental feed, sentence with commits, throughput bytes/s, engine comparisons; 60 WPM ≈ 100 ms/keystroke budget; zero-allocation assertion.
- **UVIe:** tests drive the real Swift↔Rust FFI with a simulated screen; separate `RecordingSink` for injection plans; `editAt` decline cases; stale scratch-buffer regression; test seams for AX/app detector.
- **VnKey:** screen simulation applies returned `backspaces` and appends output; good for FFI correctness, but lacks randomized/property coverage.
- **ViFastKey current:** 13,273 core conformance + 3,000 engine scenarios + 442 Python tests + property checks; strong. Missing full Vietnamese dictionary round-trip, full English passthrough baseline, and randomized over-delete stress.

---

## 6. What ViFastKey currently lacks

| Gap | Concrete evidence | Impact | Priority |
|---|---|---|---|
| VNI literal numbers | `nam2024 → nãm`, `ba10 → ba`, `top10 → top`; Telebit expects raw; `viet65` still converts | corrupts dates/versions/IDs | **P0** |
| Undo/escape gating | `banana → banna`, `Canada → Canda`, `coffee → cofee`, `cheese → chée`, `daddy → dady`, `airport → aiport` | silent letter loss in English/proper nouns | **P0** |
| Sticky restore | `text` then `s → tét`; GõViệt `restored` prevents | preview jumps/corruption | **P0** |
| English dictionary | 227 English + 602 Vietnamese words; UVIe uses ~15k English; roadmap wants 20k + 7k | weak auto-restore; frequent false transforms | **P0/P1** |
| KEEP abbreviations | no `đc`/`đn`/`òh` rule; GõViệt preserves | abbreviations restore to `ddc`/`ofh` | **P1** |
| Free-marking toggle | always free; VnKey option; `muonf` differs | compatibility with strict UniKey modes | **P1** |
| Legacy charsets | none; VnKey 19, roadmap high priority | Office/WPS/legacy documents | **P1** |
| Per-app charset | none; VnKey map + FFI | mixed legacy apps | **P1** |
| Post-commit edit ring | only one previous word; UVIe 8 + mid-word + `forwardDeletes` | editing older words/typos | **P1** |
| Real adapters | engine profiles are hints only | terminal/Chromium/Firefox/Office reliability | **P1** |
| VIQR / extra layouts | only Telex/Simple/VNI | input-method coverage | **P1/P2** |
| Case policy | `dDoongf → đồng` (GõViệt wants `Đồng`; Telebit matches ViFastKey); `wW → w` | proper names/acronyms; pick/document a policy | **P1/P2** |
| Progressive đ | `ddie → ddie` vs Telebit `đie` | partial-typing UX | **P2** |
| Coda-based nucleus inference | `Viets → Viets` vs UVIe `Viết` | UniKey-style shorthand | **P2** |
| Relaxed coda/quick start | UVIe `g→ng`, `h→nh`, `f→ph`, `j→gi`, `w→qu` | typing speed | **P2** |
| Quick Telex `hh→nh` | UVIe includes it; ViFastKey doesn’t | consistency | **P2** |
| no_std / zero alloc | Vigo FastEngine; ViFastKey uses Vec | embedded/hot path | **P2** |
| Prediction/abbrev | Vigo only, demo; ViFastKey macros cover explicit expansion | optional | **P2 (not default)** |
| Stress/dictionary tests | current conformance strong but no full dict/stress | regression risk | **P1/P2** |

**Not gaps:** ViFastKey’s 3-state spell model, CVC tone placement, `ươ/uơ` handling, `allow_coda_k`, NFC/NFD, preedit/replace, soft reset, smart caps, macros with digits, and conformance are already stronger than most of the set. Do not replace them.

---

## 7. Prioritized recommendations

### P0 — correctness/data loss (do first, Python oracle first, then Rust + conformance)

| Area | Recommendation | Concrete work |
|---|---|---|
| **Engine** | Implement VNI literal-number state | Add a VNI pre-scan: leading digit literal; doubled digit collapses and is literal (`a11 → a1`); trailing run with ≥2 tone digits (0–5) makes the whole word raw (`nam2024`, `ba10`, `top10`); one tone digit + shape digits still converts (`viet65`). Mirror in `python/vfk_core/rules.py` + `transform.py`, then Rust `rules.rs`/`transform.rs`; add golden rows. |
| **Engine** | Fix escape/undo semantics | Track last mark key/position and whether it actually transformed; allow mark undo only for the immediately preceding same key (and triple for `aaa/eee/ooo/ddd`); gate tone double on structural whole-word conversion; never set `undone` for a literal key. Add Telebit’s `coffee/cheese/daddy/banana/Canada/tomorrow/airport` corpus. |
| **Engine** | Make restored state sticky | Add `literal/restored` to `ImeSession`; append raw chars until word break; backspace can re-enable composition when remainder is a valid prefix/known abbreviation; preserve auto-capitalization. Add `text`+`s → texts`, `text`+space, `text`+backspace, `muowa⌫⌫wif`, `user⌫r`. |
| **Engine/data** | Load real dictionaries | Replace 227/602-word fallback with ~20k English + ~7k Vietnamese sets; keep a compact default for tests; add prefix/trie or word-level override to reduce preview churn; exclude common Vietnamese words from English override (`this`, `as`, `did`). |
| **Docs/tests** | Add regression corpus | Independent TSV rows for the cases above; no copying GPL tests. Add a “known collisions” file; do not chase zero. |

### P1 — product completeness

| Area | Recommendation | Concrete work |
|---|---|---|
| **Engine** | `free_marking` option | Add `free_marking: bool` to `TransformOptions`/`Config`; default true. False = only apply tone when rime complete and mark only at target position (VnKey behavior). Add `muonf`/strict tests. |
| **Engine** | Keep-abbreviation policy | Port GõViệt behavior conceptually: no-vowel + stroke (`đc`, `đn`, `đt`, `đl`), `kô`, toned vowel + `h` (`òh`, `àh`, `ùh`); don’t restore them to raw. Add `add`/`two` negative cases. |
| **Engine/FFI** | Charset hub | New `vfk-charset` crate: TCVN3, VPS, VISCII, BKHCM1, VietWare-F, VNU/ISC, VNI-Win, BKHCM2, VietWare-X, VNI-Mac, CP1258, VIQR, NCR, decomposed. Convert only at output; add `delete_bytes` and per-app charset map. Independently derive tables; no VnKey tables. |
| **Engine/FFI** | Post-commit edit ring | Record `(raw, rendered)` for last 8 words; add `edit_at(caret_back, char)` with `forward_delete`; mid-word re-render; truncate newer entries; expose `raw_chars`, `current_output`, `committed_text`, `is_composing`. Add UVIe test cases (`don`+space+←+`s`, `do|n`+`s`, off-boundary decline). |
| **Adapter** | Implement `AppProfile` strategies | At least: terminal → preedit/atomic; Chromium → select-and-retype / native-vs-web; Firefox → U+202F sentinel or disable edit-previous; Office/WPS/Notion → VK_BACK; console → WriteConsoleInput; VM/RDP/game → bypass; secure input/non-Latin → pass-through; per-app language/charset memory. Add manual test matrix from GõViệt. |
| **Engine** | Case policy | Choose and document one policy. GõViệt treats `dD`/`Dd`/`DD` as `Đ` when either key is uppercase; Telebit currently yields `đ` for `dD`. Also ensure `wW` does not lose the mark; test macro case adaptation, auto-caps + restore + ESC. Add `dDoongf`, `DDi`, `WW`, `wW`. |
| **Engine** | VIQR input method | Extend `InputMethodSpec` with `extra_word_keys` and `escape_char`; map `' ` ? ~ . ^ + ( \`; support `DD` and uppercase inserts; test escaped punctuation and round-trip. |
| **Tests** | Full dictionary + stress | Add Telebit-style two-way dictionary harness with independently licensed data; add GõViệt-style randomized over-delete stress (emoji, nav, Command, reset); add UVIe backspace/scratch-buffer tests. |
| **Docs** | Compatibility/provenance | `BEHAVIOR.md`, `COMPATIBILITY.md`, `ADAPTERS.md`, `CHARSETS.md`, `TESTING.md`, `THIRD_PARTY_NOTICES.md`; document bracket convention, free-marking default, VNI numbers, escape rules, known collisions, app profile matrix. |

### P2 — optional/performance/experimental

| Area | Recommendation |
|---|---|
| **Engine** | Progressive `đ` and coda-based nucleus inference (`ddie → đie`, `Viets → Viết`) behind flags; default off until compatibility is measured. |
| **Engine** | Relaxed coda `g→ng`, `h→nh`; quick start `f→ph`, `j→gi`, `w→qu`; `hh→nh`; uppercase bracket `{`/`}`; Ede/Tây Nguyên characters. |
| **Engine** | Optional `FastSession`/`no_std` tier with fixed arrays, zero alloc, parity tests vs `Composer`; keep current `Composer` as default. |
| **Engine** | Optional `vfk-smart` crate for prediction/abbreviation/code-switch; disabled by default; real corpus, memory/latency caps; no one-letter abbreviations. |
| **Tests** | Criterion benchmarks in CI (p50/p99 per key, FFI overhead, allocation count) with thresholds; compare against a frozen baseline. |
| **Docs** | Add “why this behavior” notes and test provenance; list GPL sources as behavior-only, not code. |

---

## 8. Licensing constraints

| Source | License | What can be reused | Constraints |
|---|---|---|---|
| **VnKey** | GPL-3.0-or-later | Behavior, architecture, API ideas, test scenarios (rewritten) | **Do not copy code, comments, tables, or tests into MIT ViFastKey.** Its charset tables and CVC tables are GPL. Clean-room re-derive. Linking/porting GPL code into an MIT binary is not compatible. |
| **Vigo** | MIT OR Apache-2.0 (Cargo/README) | Code, tables, tests with notice | No LICENSE file in the clone; fetch upstream license before copying. If no license text, treat as unlicensed. |
| **GõViệt** | MIT | Code/tests with copyright notice | `docs/ATTRIBUTION.md` says behavior learned from UniKey (GPL), vi-rs (MIT), bamboo-core (MIT), goxkey (BSD-3), Gõ Nhanh (inconsistent metadata), OpenKey (GPL issues only). The code is new; preserve MIT notice. |
| **UVIeMac** | MIT OR Apache-2.0 per README | Swift shell, tests with notice | No LICENSE file in clone; engine `uvie-rs` is separate and must be checked. `AGENTS.md` is documentation, not a license. |
| **Telebit** | MIT | C++ core, tests, docs with notice | `dict/vietnamese` is GPL-2 (`hunspell-vi`), `dict/american-english` BSD-like; both are test data, not linked. Do not ship GPL dictionary data in ViFastKey. The AI client may use network; out of scope. |
| **ViFastKey** | MIT | — | If incorporating MIT code, add `THIRD_PARTY_NOTICES.md` and SPDX headers. If any GPL code/tables are used, the combined distribution cannot remain MIT-only. |

**Recommended clean-room process:**

1. Write behavior specs and tests from docs/tests/public standards; do not copy source structure, names, comments, or table rows.
2. For VnKey, use its tests only as a checklist of scenarios; write independent inputs/expected values and code.
3. For charset tables, derive from published TCVN3/VISCII/VNI/CP1258 mappings and validate with round-trip tests; do not copy `data.rs`.
4. For dictionaries, use a license-compatible source (public domain/CC0/MIT) or generate a small curated list; keep GPL dictionaries outside the repo/binary.
5. Keep third-party code in separate modules with original headers; document provenance in `THIRD_PARTY_NOTICES.md`.
6. If distributing test fixtures from MIT repos, include their copyright/license text; if from GPL repos, don’t.

---

## 9. Benchmark and regression methodology to adopt

### 9.1 Engine benchmark

Vigo’s Criterion setup is a good template:

- Inputs: `vieetj`, `thuwowngf`, `nghieeeng`, `xin chaof`, medium sentence, 24-word sentence.
- Modes: batch transform, incremental `feed`, sentence with commits, throughput bytes/s, engine tiers, comparison against frozen competitors.
- Budget: 60 WPM ≈ 100 ms/keystroke; all current engines are far below, so measure regressions, not absolute need.
- Assertions: zero heap allocations per keystroke where promised; `size_of` bound; `Send + Sync`; no panic on empty/backspace.
- ViFastKey current local measurement: Rust `transform` ~1.03 µs/key, `Composer` ~0.50 µs/key; Python ~31/17 µs. Add Criterion and CI thresholds (e.g. p99 < 5 µs/key Rust) to catch regressions.

### 9.2 Behavioral regression

- Golden TSV: `options<TAB>keys<TAB>expected`, special keys for backspace/ESC, one row per scenario (ViFastKey already).
- Conformance JSONL: Python oracle → Rust, compare every Action, not just final text (ViFastKey already, 13,273 + 3,000).
- Dictionary round-trip: all Vietnamese syllables through generated Telex keys; all English words passthrough; per-word diff baseline (Telebit).
- Randomized stress: deterministic LCG; configs across methods/options; 80-event sequences; assert no over-delete/panic; include emoji/NFD/nav/Command/reset (GõViệt).
- FFI tests: apply returned backspaces to a screen buffer; stale scratch buffer; empty diff; large macro; null handles; panic safety (VnKey/UVIe/ViFastKey).
- Adapter tests: `RecordingSink`/`StubAXInjector`/`StubAppDetector` seams; test consume/pass-through and injection plans without touching the OS (UVIe/GõViệt).
- Manual matrix: TextEdit, Notes, Safari, Chrome omnibox, Google Docs, VS Code, iTerm2/Terminal, Spotlight, Excel, Messages, password, game/excluded (GõViệt).

### 9.3 Test cases to port as behavior (not code)

1. Telebit escape/English set: `coffee`, `cheese`, `mission`, `essay`, `daddy`, `toolbox`, `airport`, `banana`, `Canada`, `tomorrow`, `necessary`, `paragraph`, `dataa`, `mongoo`, `soffa`, `tepee`.
2. Telebit VNI set: `a11`, `nam2024`, `ba10`, `top10`, `2024`, `box1`, `viet65`, `toa10n`.
3. GõViệt engine set: `text`+`s`, `text`+space, `muowa⌫⌫wif`, `user⌫r`, `a. text`, `ddc`, `ddn`, `ofh`, `add`, `two`.
4. UVIe set: `viets`, `viets`→`Viết` if adopting inference, `neebo`, `banana`, `vieejt` backspace walk, `editAt` mid-word, scratch buffer.
5. VnKey set: `windows` not `windoư`, `how/now/cow` valid, `muonf` strict/free marking, `WW` case, `dDoongf` case.
6. ViFastKey existing set: keep all golden/conformance; add the above as new rows; don’t replace existing behavior without a spec decision.

---

## 10. Source map

| Area | Files |
|---|---|
| VnKey engine | `vnkey-engine/src/engine.rs`, `vnlexi.rs`, `input.rs`, `charset/mod.rs`, `charset/data.rs`, `macro_table.rs`, `ffi.rs`; `tests/integration.rs` |
| Vigo tiers | `src/fast_engine.rs`, `syllable_engine.rs`, `syllable.rs`, `tone.rs`, `action.rs`, `definitions.rs`, `smart_engine.rs`, `validation.rs`, `prediction.rs`, `abbreviation.rs`, `codeswitching.rs`, `ffi.rs`; `benches/benchmark.rs`, `vs_uvie_rs.rs` |
| GõViệt engine | `rust/engine/src/engine.rs`, `compose.rs`, `validation.rs`, `tone.rs`, `autocaps.rs`, `macros.rs`, `syllable.rs`, `config.rs`; `tests/corpus/*.tsv`, `tests/stress_test.rs` |
| GõViệt macOS | `macos/Sources/Inject/TextInjectionPlan.swift`, `TextInjector.swift`, `AppProfiles.swift`, `InjectionScheduler.swift`, `EventRouting.swift`; `docs/TESTING.md`, `docs/ATTRIBUTION.md` |
| UVIeMac | `Sources/UVieMac/Core/EngineBridge.swift`, `EventTap/EventTap+Handle.swift`, `EventTap+AutoCapitalize.swift`, `EventTap+SyntheticOutput.swift`, `AppClassification.swift`, `KeyboardLayoutMonitor.swift`, `Features/MemoryManager.swift`, `Utils/AppDefaults.swift`; `Tests/UVieMacTests/*`; `AGENTS.md` |
| Telebit | `canonicalize.cpp`, `rime_table.cpp`, `render_utf8.cpp`, `vietnamese.cpp`, `engine.cpp`, `tests.cpp`; `docs/concepts/spell-check.md`, `docs/guide/telex-vni.md`, `docs/dev/architecture.md`, `scripts/dict-roundtrip/*` |
| ViFastKey current | `rust/crates/vfk-core/src/transform.rs`, `rules.rs`; `rust/crates/vi-syllable/src/lib.rs`; `rust/crates/vfk-engine/src/session.rs`, `ports.rs`; `rust/crates/vfk-ffi/src/lib.rs`; `python/tests/golden/*.tsv`, `python/conformance/*.jsonl`; `python/EDGE_CASES.md` |

---

## Appendix A — concrete observed ViFastKey core results (2026-10-29)

Method: Python oracle `transform(input, TELEX|VNI, TransformOptions(), final=True)`. Session/dictionary may alter some results at commit; these are core-transform observations.

| Input | Observed | Expected (source) |
|---|---|---|
| `banana` | `banna` | `banana` (Telebit) |
| `Canada` | `Canda` | `Canada` |
| `tomorrow` | `tômrow` | `tomorrow` |
| `necessary` | `nêcsary` | `necessary` |
| `paragraph` | `pâgraph` | `paragraph` |
| `acacia` | `accia` | `acacia` |
| `amalgam` | `amlgam` | `amalgam` |
| `anaconda` | `anconda` | `anaconda` |
| `coffee` | `cofee` | `coffee` |
| `cheese` | `chée` | `cheese` |
| `mission` | `mision` | `mission` |
| `essay` | `esay` | `essay` |
| `daddy` | `dady` | `daddy` |
| `toolbox` | `tolbox` | `toolbox` |
| `airport` | `aiport` | `airport` |
| `workflow` | `wokflỏw` | `workflow` |
| `texts` | `tét` | `texts` (GõViệt sticky) |
| `dDoongf` | `đồng` | `Đồng` (GõViệt; Telebit agrees with ViFastKey) |
| `quyaf` | `quỳa` | `quyaf` (Telebit) |
| `quyuf` | `quỳu` | `quyuf` (Telebit) |
| `ddie` | `ddie` | `đie` (Telebit) |
| `dduo` | `dduo` | `đuo` (Telebit) |
| `viets` / `Viets` | `viets` / `Viets` | `viết` / `Viết` (UVIe) |
| `muonf` | `muonf` | `muòn` (VnKey free marking; compatibility choice) |
| `aw` | `aw` | `ă` (Telebit/VnKey; standalone partial) |
| `a6` (VNI) | `a6` | `â` (Telebit) |
| `nam2024` (VNI) | `nãm` | `nam2024` |
| `ba10` (VNI) | `ba` | `ba10` |
| `top10` (VNI) | `top` | `top10` |
| `a11` (VNI) | `a1` | `a1` (already correct) |

**Syllable corpus:** 6,631 curated Vietnamese syllables, modern tone, only 16 mismatches (13 `oo` words, `GIF`, `HĐND`, `palăng`). This is a strong result and argues for preserving ViFastKey’s parser.

**English corpus:** 74,585 alphabetic entries from `wamerican`; 7,251 changed (9.72%) by the core; lowercase subset 63,875 → 6,366 changed (9.97%). Words without Telex modifier letters still changed 103/9,500 (1.08%), which isolates the repeated-`a/e/o` undo bug. Telebit’s committed English baseline is 1,203 exceptions out of 104,334 (~1.15%); the comparison is directional, not apples-to-apples, but the gap is large. ViFastKey’s engine-level dictionary can restore some words at commit, but not the preview churn or words that render as valid Vietnamese.
