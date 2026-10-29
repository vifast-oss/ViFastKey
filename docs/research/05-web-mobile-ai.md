> **Báo cáo nghiên cứu nội bộ.** Tài liệu này chỉ ghi nhận hành vi, kiến trúc, UX và
> test fact của các dự án được khảo sát. Không có mã nguồn, bảng dữ liệu, từ điển hay test
> corpus GPL/AGPL nào được sao chép vào ViFastKey. Xem `THIRD_PARTY_NOTICES.md` và
> `docs/IME_FEATURE_RESEARCH.md`.

# Analysis 05 — Web, mobile and AI ideas for ViFastKey

**Scope:** five public Vietnamese IME projects cloned under `thư mục clone tạm`:
`1ec5_avim`, `ducngg_v7`, `Funput_Funput`, `phatMT97_VKey`, `ngkhhuy_bamboo-viet`.

**Method:** read READMEs, design docs, engine sources, browser/DOM adapters, platform
shells and tests. Behaviour, UX, architecture and test cases are extracted. No source
code is copied, and no GPL/AGPL implementation is reused.

**Snapshot:** AVIM `18572ba` (v5.8.3, 2020-06-25); v7 `4404b70` (2025-05-13);
Funput `e63ac55` (2026-10-28); VKey `4399b89` (2026-10-29); Bamboo Viet `7164155`
(2026-08-25).

**Current ViFastKey baseline used for the gap analysis:** Python oracle + Rust port,
Telex / Simple Telex / VNI, old/new tone, spell check, `w_as_u`, bracket keys, Quick
Telex, `allow_coda_k`, macros (including digits), English restore, app profiles,
preedit/replace, surrounding edit, soft reset, smart caps, auto-capitalize, NFC/NFD,
C ABI/WASM, web playground with replace/preedit, macro editor and the current option
set.

---

## 0. Executive summary

1. **The largest concrete gap is the browser adapter, not the Vietnamese rules.**
   ViFastKey's playground is a `keydown` adapter over a `<textarea>` that rewrites
   `textarea.value` on every key. AVIM's old, MIT-licensed DOM layer shows the missing
   pieces: plain text vs. `contenteditable`, collapsed-selection checks, `maxLength`,
   word-level undo transactions, a synthetic `input` event for framework/autocomplete
   state, and per-editor shims for Google Docs/Zoho/Ace/iCloud. A modern version must
   use `beforeinput`, composition events and a real undo journal.
2. **Mobile is a separate product problem, and Funput has the best public blueprint.**
   The important decisions are marked text vs. direct commit as a user setting, a
   touch journal with down-time ordering, touch evidence captured at touch-down,
   field-type policy, local-only suggestions, and iOS Full Access/privacy handling.
3. **AI should not enter `vi-syllable` or `vfk-core`.** v7's strongest reusable idea is
   constrained decoding: analyse the raw input into `(consonant family, rhyme family,
   tone)`, let a small local model propose words, then accept only candidates that match
   the typed structure. Its 10.9M-parameter, 43.6 MB model is a demo-grade optional
   add-on, not a default dependency.
4. **Personal n-gram suggestions are a better first intelligence feature than a neural
   model.** Funput's four-slot follower table, generation tags, aging and “predict only
   when dominant” rule are small, deterministic, local and testable.
5. **Macro UX has a clear upgrade path.** Funput and VKey both converge on: exact /
   smart-case matching, longest key wins, boundary guard, optional Space/Enter/Tab
   triggers, and **one Backspace undoes a just-expanded macro while keeping the
   boundary**. ViFastKey has none of the undo/refusal semantics today.
6. **Auto-capitalize needs a shared sentence scanner.** Funput's rules cover `…`,
   closing quotes, newlines, `1.5`, abbreviation guards and numeric sentence starts;
   ViFastKey currently tracks only `.` `!` `?` followed by whitespace.
7. **UX lessons are platform-independent.** UniKey-style control panel, app presets,
   settings search, quick tiles, Full Access gating, contrast warnings, floating V/E
   icon, hotkey capture, versioned config import/export and a CLI that mirrors the GUI.
8. **Licensing is favourable for exactly two code sources:** AVIM and Funput are MIT;
   v7 is Apache-2.0. VKey and Bamboo Viet are GPL-3.0 — behaviour and test cases only.
   Data/model licences need a separate audit before anything is bundled.

---

## 1. Project inventory

| Project | Stack / form | License | Size | What it is | Best material for ViFastKey |
|---|---|---|---|---|---|
| **1ec5_avim** | XUL/JS Firefox/Thunderbird extension, Python build | **MIT** | 119 files, ~1 MB | Legacy but mature browser IME; pure transformer + DOM frame script | Browser input/contenteditable adapter, word transactions, site blacklist, per-editor shims |
| **ducngg_v7** | Python 3.12 + PyQt5, PyTorch | **Apache-2.0** | 56 files, ~10 MB | Research IME: dictionary mode + tiny GPT “v7gpt”; 8-tone phonology | Constrained next-word decoding, 8-tone analysis/synthesis, candidate paging |
| **Funput_Funput** | Rust workspace + iOS/Android/macOS/Windows/Linux shells | **MIT** | 3,492 files, ~59 MB | Production-oriented cross-platform IME with extensive design docs/tests | Mobile input pipeline, marked/direct composition, suggestions, typo correction, config interchange, privacy |
| **phatMT97_VKey** | C++20 Windows hook + TSF, Sciter/Classic UI | **GPL-3.0** | 671 files, ~30 MB | Modern Windows IME with per-app/domain modes and rich UX | Per-app/domain policy, macro casing/undo, auto-cap, commit undo, OSD, convert tool |
| **ngkhhuy_bamboo-viet** | Go IBus/Fcitx5, GTK3/Python control panel | **GPL-3.0** (wrapper); `bamboo-core` MIT | 99 files, ~3 MB | Linux IME with UniKey-style GUI, CLI, app presets, emoji, fuzzer | Input-mode matrix, app presets, GUI/CLI parity, macro file reload, emoji trie |

### 1.1 AVIM (1ec5_avim)

- **Architecture:** `components/transformer.js` is a pure string transformer
  (`applyKey(prefix, context) -> {value, changed}`). `content/frame.js` is the browser
  adapter; `content/avim.js` holds preferences, toolbar/status UI and shortcuts.
- **Why it still matters:** it is a catalogue of browser edge cases from the XUL era.
  It distinguishes HTML `<input>`/`<textarea>` from `designMode`/`contenteditable`,
  handles collapsed selections only, clamps to `maxLength`, dispatches a synthetic
  `input` event after `preventDefault`, scrolls the caret into view, merges undo
  transactions per word, and carries special adapters for Google Docs (Kix), Zoho
  Writer/Show, Ymacs, Ace, iCloud and Silverlight.
- **Limits:** pre-WebExtensions, dead on Firefox 57+, no modern `beforeinput`/
  composition-event model, no mobile, no suggestions, no AI.

### 1.2 v7 (ducngg_v7)

- **Architecture:** `utils/vietnamese.py` maps words to
  `(consonant family, rhyme family, tone)` and back; `imethod/v7.py` is dictionary
  prediction; `imethod/v7ai.py` is model-assisted prediction; `app/app.py` is a PyQt
  candidate picker with 9 items/page, number keys, arrow paging and context buffer.
- **Model:** `v7gpt` is an 8-layer, 8-head, 256-embedding, 32-token-context,
  17,789-vocabulary GPT-like model: 10,880,768 parameters, 43.6 MB. Inference is local
  (CUDA if present, otherwise CPU); the checkpoint is downloaded separately.
- **Why it matters:** it demonstrates a clean split between a **phonological
  constraint** and a **language model**, and an 8-tone analysis that is more precise
  than the usual six-tone UI model.
- **Limits:** Windows/macOS only, PyTorch dependency, Linux unsupported, demo UI rather
  than a system IME, checkpoint/dataset licences need separate verification.

### 1.3 Funput (Funput_Funput)

- **Architecture:** one Rust workspace (`funput-core`, `funput-engine`,
  `funput-suggestions`, `funput-config`, `funput-ffi`, `funput-jni`, desktop helpers),
  with thin platform shells. `docs/features/*.md` are design documents that record
  trade-offs and measurements, not just marketing.
- **Most relevant material:**
  - iOS input pipeline: touch ingest, intent journal, ordering, marked vs. direct
    composition, echo tracking, field policy (design blueprint; only phase 0 is
    implemented as of the snapshot).
  - Suggestions: personal word store, English lexicon, bigram followers, rerank vs.
    predict, dominance threshold.
  - Typo correction: touch evidence, cost model, ambiguity, undo/refuse, opt-in.
  - Config interchange: portable vs. platform-specific blocks, merge import, UniKey
    macro import.
  - UX: onboarding, Full Access, quick settings, settings search, themes with contrast
    warnings, clipboard, emoji, voice, privacy manifests.
- **Limits:** large surface area; many features are experiments or platform-specific;
  some docs are Vietnamese-only; the Rust core is MIT but the lexicon data has its own
  notices.

### 1.4 VKey (phatMT97_VKey)

- **Architecture:** C++20 Windows application with a low-level keyboard hook and an
  optional TSF text service; Sciter UI plus a native “Classic” Win32 UI; a companion
  browser extension/native-messaging host routes per-domain V/E mode.
- **Most relevant material:**
  - per-app method/charset/send-method overrides, hard E/V locks, Smart Switch memory;
  - browser per-domain mode with session state and hard rules;
  - macro: long/multiline expansions, smart case, title case, optional trigger keys,
    Backspace undo;
  - auto-capitalize fail-closed decision tables;
  - commit undo, digit-led-word bypass, CJK/foreign-layout suspension, game mode;
  - floating V/E icon, tray icon colors, hotkey capture, sequential convert tool;
  - pure decision modules with dense unit tests (`AutoCapDecision`, `CjkSwitchDecision`,
    `PerAppModeDecision`, `MacroCase`, `MacroContextMatch`, `CommitUndoExemption`).
- **Limits:** GPL-3.0; Windows-only; the “advanced” Rust engine is a closed component;
  some UX is Windows-specific and should not be copied literally.

### 1.5 Bamboo Viet (ngkhhuy_bamboo-viet)

- **Architecture:** Go IBus engine + Fcitx5 addon; a Python GTK3 “UniKey style” control
  panel; a `bamboo-viet-config` CLI; `bamboo-core` (MIT) for the Vietnamese rules;
  emoji trie and macro table.
- **Most relevant material:**
  - per-app input-mode presets: browsers/Electron/Office → surrounding text; terminals
    → forward-as-commit; 7 modes including “US” exclusion;
  - GUI/CLI parity: `status`, `set-method`, `set-mode`, `set-app`, `list-presets`,
    `restart`;
  - macro file format and hot reload; UniKey-style macro editor;
  - emoji trigger `:`, ASCII emoticon/keyword search, 9 candidates/page, arrows/1–9;
  - fuzzer that generates realistic typing scenarios from Vietnamese words.
- **Limits:** GPL-3.0; Linux-only; the “100%” fuzz report is a small, self-selected
  corpus and should not be treated as proof of correctness.

---

## 2. Feature matrix

Legend: ✅ present and substantial · ◐ partial/experimental/platform-specific · — absent.

### 2.1 Input and composition

| Capability | AVIM | v7 | Funput | VKey | Bamboo | ViFastKey today |
|---|---|---|---|---|---|---|
| Telex | ✅ | ◐ hybrid/v7 | ✅ | ✅ | ✅ | ✅ |
| Simple Telex | — | — | — | ✅ | ✅ | ✅ |
| VNI | ✅ | ✅ (6/8-tone) | ✅ | ✅ | ✅ | ✅ |
| VIQR / VIQR* | ✅ | — | — | — | — | — |
| Auto method detection | ✅ tries methods sequentially | — | — | — | — | — |
| Combined Telex + VNI | — | ◐ hybrid | — | ✅ | — | — |
| Custom key map | — | — | — | ✅ | — | — |
| 8-tone analysis (stop-coda tones 6/7) | — | ✅ | — | ◐ via spell check | — | ◐ via coda rules |
| Extra onsets (`z f w j`) | ◐ informal `f/z/dz` | — | ✅ | ◐ spell exclusion | — | — |
| Coda `k` / place names | — | — | ✅ | ◐ | — | ✅ `allow_coda_k` |
| Preedit | — | — | ✅ all platforms | ✅ TSF | ✅ | ✅ simulated/web + engine |
| Replace / surrounding edit | ✅ DOM | — | ✅ desktop | ✅ hook/TSF | ✅ | ✅ |
| Marked text (mobile) | — | — | ✅ iOS | — | — | — |
| Commit undo / macro undo | — | — | ✅ | ✅ | — | — |
| Browser `contenteditable` | ✅ legacy | — | — | — | — | — |
| Native composition events | ◐ XUL key events | — | ✅ iOS/TSF | ✅ TSF | ◐ IBus preedit | — |
| Selection-aware replacement | ◐ collapsed only | — | ✅ | ✅ | ✅ | ◐ delete only |

### 2.2 Intelligence and text features

| Capability | AVIM | v7 | Funput | VKey | Bamboo | ViFastKey today |
|---|---|---|---|---|---|---|
| Spell check | ✅ rule engine | ◐ analyser | ✅ rules + corpus | ✅ rules + dict/exclusions | ✅ rules/dict | ✅ |
| English restore | — | — | ✅ | ✅ | ✅ | ✅ |
| Doubled-key restore | — | — | ✅ | ✅ | — | ◐ via restore |
| Macro / text expansion | — | — | ✅ smart case + undo | ✅ long/multiline + undo | ✅ case + hot reload | ✅ basic + digits |
| Macro across punctuation | — | — | ✅ | ✅ | ✅ | — |
| Macro boundary guard | — | — | ✅ | ✅ | ✅ | ◐ word end |
| Typo correction | — | — | ✅ mobile | ◐ closed advanced engine | — | — |
| Prefix suggestions | — | ◐ dictionary | ✅ personal + English | — | ◐ emoji only | — |
| Next-word prediction | — | ✅ GPT/dictionary | ✅ personal bigram | — | — | — |
| Personal learning | — | — | ✅ | ◐ smart switch | — | — |
| Emoji picker/search | — | — | ✅ | — | ✅ | — |
| Voice input | — | — | ◐ Android on-device (implemented, release pending) | — | — | — |
| Legacy charsets | ◐ VIQR | — | ✅ TCVN3/VNI-Windows/compound | ✅ multiple | ✅ TCVN3/VNI/VIQR | — |
| Text case / convert tool | — | — | ✅ | ✅ | ◐ charset GUI | — |

### 2.3 Platforms and UX

| Capability | AVIM | v7 | Funput | VKey | Bamboo | ViFastKey today |
|---|---|---|---|---|---|---|
| Windows | — | ✅ demo | ✅ | ✅ | — | ◐ CLI/FFI |
| macOS | — | ✅ demo | ✅ | — | — | ◐ CLI/FFI |
| Linux | — | — | ✅ | — | ✅ | ◐ CLI/FFI |
| iOS keyboard | — | — | ✅ | — | — | — |
| Android keyboard | — | — | ✅ | — | — | — |
| Browser extension | ✅ | — | — | ✅ companion | — | — |
| Per-app profiles | ◐ field blacklist | — | ✅ | ✅ | ✅ | ✅ |
| Per-domain profiles | — | — | — | ✅ | — | — |
| GUI settings | ✅ XUL | ◐ PyQt | ✅ SwiftUI/Compose/Slint/GTK | ✅ Sciter/Classic | ✅ GTK3 | ◐ web/CLI |
| Settings search | — | — | ✅ | ◐ | — | — |
| Onboarding | ◐ | — | ✅ | ✅ | — | — |
| Themes / contrast | — | — | ✅ | ✅ | ◐ | — |
| OSD / floating status | ◐ audio + popup | — | ◐ menu bar | ✅ floating V/E | ◐ tray | — |
| Hotkey capture | ◐ double Ctrl | — | ✅ | ✅ | ◐ | — |
| Config import/export | ◐ prefs sync | — | ✅ versioned JSON | ◐ lists | ◐ JSON/CLI | ◐ JSON |
| Accessibility | ◐ accesskeys/ARIA | — | ✅ labels/hints/tests | ◐ | ◐ | ◐ |
| Privacy by design | ◐ local | ✅ local | ✅ no-network keyboard | ✅ offline | ✅ local | ✅ local |

---

## 3. Techniques worth adopting

### 3.1 Browser and extension techniques

#### 3.1.1 Start with two adapters, not one

AVIM's `frame.js` splits the world into:

- **Plain text controls:** `<input type=text|search|password|url|email>` and
  `<textarea>`; use `selectionStart`/`selectionEnd`; only act when the selection is
  collapsed; honour `maxLength`; read the last word before the caret.
- **Rich editors:** `document.designMode === "on"` or `contentEditable === "true"`;
  operate on the selected text node, split the text node at the caret for a syllable
  break, and keep the editor's selection object in sync.

For ViFastKey this becomes:

```
TextControlAdapter  -> HTMLInputElement / HTMLTextAreaElement
EditableAdapter     -> contenteditable / designMode
EditorShim          -> last-resort site/editor-specific adapter
```

The unified contract should expose: `getContextBeforeCaret`, `getSelection`,
`replaceRange`, `insertText`, `deleteBackward`, `notifyInput`, `scrollCaretIntoView`,
`readValue`, `maxLength`, and `isComposing`. The engine never knows which adapter is in
use.

#### 3.1.2 Use `beforeinput` as the primary event, with `keydown` as a fallback

Modern browsers deliver text edits through `beforeinput`/`input`, and mobile virtual
keyboards often do **not** emit useful `keydown` events. A robust browser adapter
should:

1. Listen to `beforeinput` in the capture phase.
2. Handle at least: `insertText`, `insertCompositionText`, `insertFromPaste`,
   `insertFromDrop`, `deleteContentBackward`, `deleteContentForward`,
   `deleteWordBackward`, `historyUndo`, `historyRedo`.
3. If the event is part of an active composition (`isComposing`, `keyCode === 229`),
   stand down completely.
4. Prevent the browser's default only when the engine will write the result; otherwise
   let the browser edit and resynchronise.
5. After a programmatic edit, emit an `InputEvent` (or use the browser's own event from
   `execCommand`/`setRangeText`) so React/Vue/Svelte controlled inputs, autocomplete,
   in-page find and `oninput` handlers see the change. AVIM learned this the hard way:
   after `preventDefault()` most textboxes do not update their own state, so it
   synthesises an `input` event.

`keydown` remains useful for Backspace/Enter/Escape/Tab and for browsers that do not
give `beforeinput` target ranges, but it must not be the only path.

#### 3.1.3 Never rewrite the whole value

ViFastKey's playground currently does `pad.value = state.text` on every render. That
destroys the browser's native undo stack, can move the caret, and is exactly the wrong
primitive for a real extension. Use the smallest edit that expresses the action:

- For **replace**: select `[caret - deleteBefore, caret)` and insert the new text.
- For **preedit**: keep a marked/underlined range and update only its contents; do not
  touch text outside the range.
- For **commit**: replace the marked range and place the caret after it.
- For **pass**: do nothing; let the browser handle the key.

AVIM's `SpliceTxn` is the old proof of the pattern: replace a substring in one text
node, shift the selection by the length delta, merge transactions that touch the same
word, and undo by replacing the inserted text with the original. The modern version
should use a `Range` for contenteditable and `setRangeText`/`execCommand` for text
controls, with a fallback journal.

#### 3.1.4 Selection, caret, `maxLength` and scroll

- Do not process if the selection is non-collapsed **unless** the adapter has an
  explicit “replace selection” policy. AVIM simply returns when
  `selectionStart !== selectionEnd`; that is safe but leaves selected text untouched.
  A better UX is to delete the selection first, then process.
- Clamp the replacement to the field's `maxLength` so the engine cannot overflow a
  limit. AVIM computes `wordStart` and truncates the result to
  `maxLength - wordStart`.
- Keep the caret after the modified word; AVIM's transaction shifts the selection by
  `repl.length - len` and its `scrollToCaret` uses the editor's selection controller.
- For contenteditable, work on text nodes and preserve inline markup. Do not set
  `innerHTML`; it destroys spans, links, mentions and React reconciliation state.

#### 3.1.5 Word-level undo/redo

Browser-native undo cannot be trusted when the IME intercepts `beforeinput`. Maintain a
small journal:

```
{ beforeText, afterText, selectionBefore, selectionAfter, kind, at }
```

Rules that matter:

- Coalesce all edits within one word into one undo entry; flush at a word boundary,
  caret move, blur, paste, or mode change. AVIM's transactions merge only while they
  operate on the same word.
- Intercept `Ctrl/Cmd+Z` and `Ctrl/Cmd+Shift+Z`/`Ctrl+Y` before the browser; if the
  journal has an entry, apply it and update the engine state; otherwise let the browser
  handle it.
- A preedit that is still open should be committed or discarded before undo.
- If the document changed outside ViFastKey, invalidate the journal instead of
  replaying a stale edit. This is the browser equivalent of Funput's “echo mismatch:
  drop the word, never overwrite”.

#### 3.1.6 Handle hostile editors, but only as a last resort

AVIM's most valuable browser lesson is that some frameworks overwrite DOM mutations:

- React and Office Online (OWA) overwrite text inserted via DOM methods; AVIM
  synthesises key events for those cases.
- Google Docs (Kix) is an iframe-based editor with its own selection overlays; AVIM
  uses a `KixProxy` that emulates a textarea, with special cases for tables, objects and
  selections.
- Zoho Writer/Show, Ace, Ymacs, iCloud and Silverlight each get a shim.
- A site/field blacklist is honoured by `id`/`name`, plus CSS `ime-mode: disabled`.
- AVIM also detects and disables other embedded Vietnamese IME scripts (AVIM, Mudim,
  CHIM, VietUni, VietTyping, Vinova, Google Input Tools) to avoid double diacritics.

For ViFastKey, do **not** port this as a pile of host-specific hacks. Instead:

1. Make the generic adapters correct for standard inputs/contenteditable.
2. Add a `siteRules` registry keyed by hostname + optional selector, with a small,
   documented shim API.
3. Prefer the editor's public event/API. If none exists, use native value setters plus
   `input` events before considering synthetic key events.
4. Give users a per-site “off / replace / preedit” switch and a visible status.
5. Treat “another Vietnamese IME is present” as a conflict signal; disable ViFastKey
   on that page rather than fight it.

#### 3.1.7 Composition events and native IME coexistence

- Never handle `keydown` when `event.isComposing` or `keyCode === 229`.
- On `compositionstart`, flush the current ViFastKey word and stop intercepting.
- On `compositionend`, read the final text, reset the engine, and resume on the next
  trusted `beforeinput`.
- If a hardware/OS IME is active, the extension should stand down; two IMEs fighting
  over the same caret produce duplicated or reordered characters.
- For the web playground, add a “simulate native IME composition” test fixture so this
  path is exercised without a real OS IME.

#### 3.1.8 Per-domain and per-app profiles

VKey's browser extension is the best model:

- The native host returns only protocol version, browser executable, focused flag,
  route, session mode and hostname; no URLs, paths, query strings or keystrokes.
- Extension stores learned V/E state in `storage.session` (cleared on browser restart)
  and hard rules in `storage.local`.
- Hard English beats session mode; an application-level hard lock beats a domain
  preference; a 5-second seqlock expiry prevents a crashed browser from leaving stale
  state.
- The normal address bar is a known limitation: extension APIs cannot see text typed
  into the omnibox, so the route applies after navigation/focus, not while typing.
- Native messaging is optional; a web-only extension does not need it.

For a future ViFastKey browser extension, the minimal version is a content script with
a per-hostname `on/off/method` setting and a toolbar popup. Native messaging is only
needed if the extension must drive a desktop IME.

#### 3.1.9 Lifecycle

- Inject into all frames (`all_frames: true`) but keep a per-frame adapter; cross-origin
  iframes need their own content script instance.
- Use `focusin`/`focusout` and `selectionchange` to flush or reset the current word.
- Listen for SPA navigation (`popstate`, `hashchange`, `MutationObserver` on the app
  root) and re-evaluate whether the focused field is still the same.
- On `pagehide`/`visibilitychange`, commit any open preedit so text is not lost.
- In MV3, keep no authoritative state in the service worker; the content script owns
  the per-field adapter, and the worker only routes messages/storage.

### 3.2 Mobile keyboard behaviour

#### 3.2.1 Marked text vs. direct commit is a user setting

Funput's iOS 3.0 design proposal (drawing on azooKey) treats `setMarkedText` as
production-viable but not universally reliable, and keeps two document-writing paths,
selected by a setting rather than by per-host detection:

- **Marked:** `setMarkedText(text, selectedRange: NSRange(location: caret, length: 0))`
  then `unmarkText` on commit. `selectedRange.length` is 0: it is a caret inside the
  marked region, not a selection.
- **Direct:** `insertText` plus `replacementRange` for the changed tail; no underline.
  Before writing, read back the text at the expected range and verify it still equals
  what the keyboard last showed. If not, drop the word and replay the key at the new
  caret; never overwrite text the app changed.

Adopt the same two-path model for any future mobile adapter. Do not hardcode “marked
text everywhere” or “direct everywhere”.

#### 3.2.2 Touch ingest and ordering

Funput's iOS 3.0 design is worth copying conceptually:

- Every `touchesEnded` produces exactly one key intent; no ingest path may silently
  drop a touch.
- An append-only journal records `downTime`, `upTime`, key and sequence.
- No timer/deadline on the hot path; the old “wait 40 ms for the other finger” arbiter
  was removed because it added latency and did not solve the real lost-key cases.
- If a late intent belongs before an already-applied intent and the region is still
  uncommitted, recompute and write once; if it is committed, append instead of
  rewriting.
- Layout/theme changes must suspend contacts, not destroy them.

This is more relevant to a native mobile keyboard than to web, but the invariant —
“exactly one output per touch, preserve down-time order” — also applies to a web
mobile adapter.

#### 3.2.3 Touch evidence and typo correction

Funput records the touch point at `.began`, not at lift. Finger drift on release says
more about the slide than about the intended key. The correction model:

- scores candidate words by edit cost plus touch distance to neighbouring keys;
- only uses adjacent keys within ~1.2 key pitch, up to 3 candidates;
- treats “no evidence” as no correction, which cuts the search space;
- distinguishes ambiguous cases (`nhad` between `nhá` and `nhà`) and does not
  auto-apply when the top two are within a margin;
- offers the typed word and candidate in the suggestion bar; Space applies, tapping the
  typed word refuses, Backspace undoes;
- keeps a per-field policy: passwords, emails, URLs, numbers and `autocorrectionType
  == .no` never autocorrect;
- stores learned “keep” words locally and treats them as known.

This is a P2 feature for ViFastKey, because it only pays off with touch geometry and a
large word list. Do not add it to the desktop engine.

#### 3.2.4 Suggestions and personal learning

Funput's model is a useful template:

- Three suggestion slots; personal words first, English lexicon fills empty slots.
- Minimum prefix is 2 characters; an empty prefix can “predict” only when the leading
  follower dominates (default 40% of context occurrences).
- Personal store is capped (e.g. 5,000 words), with a trie and snapshot/journal.
- Bigram is stored inside each `WordRecord` as four follower slots; each slot has a
  generation tag so an evicted/reused index cannot silently point at another word.
- Slot loading is Misra–Gries-like: increment existing, fill empty/dead, otherwise
  decrement weakest and replace at zero. Aging halves all followers after 256
  occurrences.
- Rerank (prefix non-empty) may use a pair seen once; Predict (empty prefix) requires
  at least two sightings and dominance, so `cảm` → `ơn` appears but `của` → ? stays
  quiet.
- Context is the last completed token; it resets on punctuation, caret move, paste,
  focus change, and backspace-to-empty. Never re-read the whole document on each key.
- Accepting a suggestion is itself a learned token and can create a bigram.

For ViFastKey this suggests a separate `vfk-suggest` crate or a WASM-only module, not
code in `vfk-core`.

#### 3.2.5 Layout, hit testing and gestures

Funput/VKey mobile UX details worth reproducing:

- The alphabetic row owns both screen edges; there must be no dead gap between the
  toolbar and the first row.
- Touch targets have a minimum size; key frames are stable while typing, so a test
  driver can cache them.
- Double-tap Space inserts `. ` only when no composition is active.
- Hold Space and drag turns the keyboard into a trackpad; it must not type spaces even
  after the old repeat delay (Funput tests hold for 1.6 s).
- Swipe left on Backspace deletes a whole word.
- Holding the period key offers punctuation alternatives.
- VNI locks the number row on; Telex leaves it user-toggleable.
- Accessibility labels are semantic, not literal: VNI `1` is “Dấu sắc”, not “1”; Space
  carries the gesture hint.
- Theme height/size is user-selectable; test at 0.85× and 1.2× scale.

#### 3.2.6 Field policy and privacy

Funput's policy is a good checklist:

- Disable composition/suggestions/learning/voice for password, PIN, one-time code,
  email, URL and numeric fields.
- On iOS inspect `textContentType`, `keyboardType` (`.asciiCapable`,
  `.asciiCapableNumberPad`) and `autocorrectionType == .no`.
- On Android inspect `EditorInfo.inputType`, `TYPE_TEXT_FLAG_NO_SUGGESTIONS`,
  `IME_FLAG_NO_PERSONALIZED_LEARNING`, and password/email/URI flags.
- The keyboard extension contains no network code; the privacy manifest declares no
  tracking and no collected data.
- Personal suggestions and clipboard live in the App Group/private files; clearing
  them is an explicit destructive action with confirmation.
- Full Access is required for App Group config/suggestions/haptics; the app explains
  why and detects which onboarding step is incomplete.
- Do not log typed content. If a debug log is offered, warn that it captures raw keys
  and can contain passwords.

#### 3.2.7 Emoji, clipboard, voice

- Emoji: a dedicated panel plus search. Bamboo's `:` trigger, ASCII emoticon/keyword
  trie (`:')` → 😂, `grin` → 😀), 9 per page, arrows/1–9, Escape commits raw, `:` again
  closes. Funput separates emoji and kaomoji into two tabs.
- Clipboard: store only text the user actually pasted through the keyboard; never
  capture arbitrary clipboard changes; expiry + pinned items; clear-all.
- Voice: explicit mic tap, on-device recognizer only, no cloud fallback, 60-second
  session cap, preview in panel, insert final once, discard late results after field
  change/cancel, no audio/transcript logs.

### 3.3 AI and linguistic features

#### 3.3.1 The 8-tone model is useful even without AI

v7's analysis distinguishes six diacritics from eight phonological tones: tones 6 and 7
are the “entering/checked” acute and underdot after stop codas `p t c ch`. Examples:
`cấp`, `tiếc`, `thích` (tone 6); `nhập`, `phục`, `mạch` (tone 7). Its analyser also
normalises:

- `c k q` → consonant family `k`;
- `ng ngh` → `ng`; `g gh` → `g`; `gi` → `z`; no onset → `0`;
- `i y` → `i`; `iên yên` → `iên`;
- final `t c p ch` → `n ng m nh` for rhyme-family lookup.

This is excellent as a **test oracle / validation layer** for ViFastKey's spell checker
and coda rules, even if ViFastKey never exposes an 8-tone input method. It also gives a
compact way to detect “is this word Vietnamese?” without a dictionary.

#### 3.3.2 Constrained decoding

v7's AI mode is not “let the model write anything”. It:

1. Splits the raw input into words, each `(consonant, rhyme, tone)`.
2. Asks the model for the next token given prior committed context (or `SOS = "tôi"`).
3. Accepts only predictions whose `(consonant family, rhyme family, tone)` match the
   user's typed structure; `c/k/q` is handled as a family.
4. Returns up to 36 candidates; the UI shows 9 per page.
5. Keeps a context buffer of committed words; on Backspace it trims the last raw unit
   and re-predicts; on forward typing it predicts only the last term and concatenates
   the buffer.

If ViFastKey ever adds AI, this constraint layer is mandatory: it keeps the model from
producing non-Vietnamese words with the wrong tone or coda. The model can be tiny; the
phonology does the heavy filtering.

#### 3.3.3 Personal n-gram before neural

Funput's bigram is a better first investment:

- zero network, zero model file, fixed memory;
- deterministic and testable;
- learns the user's own phrases (`xin` → `chào`);
- can be disabled/reset without model licensing;
- prediction can be silent unless dominant, avoiding the “creepy/wrong” feeling.

A small model should only be added after the personal n-gram path is measured and after
the user explicitly opts in.

#### 3.3.4 Typo correction as an optional, reversible feature

Funput's measurements are a warning: at realistic touch noise, a weak corrector fixes
some words but changes intentionally typed slang/brand names. Its safeguards:

- valid Vietnamese syllables typed correctly are never changed (property test);
- only ambiguous when top candidates are close;
- typed word competes with candidates via an invalid-word cost;
- “keep” list and user dictionary suppress corrections;
- one Backspace undoes; a refused correction is remembered for the next occurrence;
- feature is off by default for existing users (or on only for new installs), and
  field policy can disable it.

Any ViFastKey typo corrector should follow the same “opt-in + undo + keep list +
never touch valid words” contract.

#### 3.3.5 Privacy constraints for AI

- Inference local only; no network calls from the keyboard/engine.
- Model checkpoint optional and lazily loaded; app works without it.
- No keystroke content in analytics/logs; metrics are counts and timings only.
- If an English/Vietnamese lexicon is bundled, ship its licence/notice and keep it
  separate from the personal store so “clear learned words” never deletes the
  dictionary.
- Do not train on user text or upload it. If a model is trained offline, document the
  dataset licence and provenance.

### 3.4 Modern UX

#### 3.4.1 The UniKey-style control panel is still the right information architecture

Bamboo's GUI and VKey's settings converge on:

- **Compact main panel:** charset, input method, tone style, default mode, on/off.
- **Advanced expander:** spell check, English restore, free typing, `đ` rules, macro,
  auto-cap, smart switch, exclusions.
- **Per-app tab:** list of apps, current mode, add/remove, import/export, reset.
- **Macro tab/dialog:** plain text `trigger: expansion` editor, multi-line, save/apply,
  sample rows.
- **About / diagnostics:** version, config paths, debug-log warning, fuzzer/test runner.
- **Bottom action bar:** Save & Apply, Reset defaults, About; apply should be explicit
  and show a toast, not silently write on every toggle.

VKey additionally has a compact/expanded responsive window and a “Classic” light UI,
which is useful for low-power machines.

#### 3.4.2 Settings search and quick settings

Funput's iOS settings are a good accessibility/UX reference:

- `searchable` settings with accent-free and English keywords (`rung`, `haptic`,
  `phim cach`, `telex`, `vni`).
- Six quick tiles for the most-used toggles; each edits a different setting; a locked
  tile explains why (VNI number row is “Luôn bật”).
- Toggles that need Full Access ask for it instead of silently failing; switching off
  never needs permission.
- Destructive actions are grouped in one “Data” card with confirmation dialogs and
  clear consequences.
- Theme editor warns about low contrast but still allows saving; warnings name the
  role pair and the light/dark mode.
- Onboarding detects the exact incomplete step (keyboard added, Full Access, etc.) and
  deep-links to the right Settings page.

#### 3.4.3 Macro UX

Combine Funput and VKey:

- Table of `trigger → expansion`, long/multiline values supported.
- Smart case: lowercase trigger matches lower/Title/UPPER and re-cases the expansion;
  a trigger containing uppercase is exact-match only; single capital `V` means
  “capitalize”, `VN` means “shout”.
- Longest key wins; a trigger inside a longer word never fires; punctuation inside a
  key (`a/b`) must remain reachable.
- Trigger keys configurable: Space, Enter, Tab, punctuation.
- **Undo:** immediately after expansion, one Backspace restores the exact pre-expansion
  text plus the boundary (`vn `), not the expansion minus one character. The refusal
  applies to the next occurrence only; typing another word spends it; a second
  Backspace deletes the boundary as usual.
- Import UniKey `ukmacro.txt`: UTF-8 BOM, `;` header ignored, first colon splits
  trigger/expansion, duplicate last wins, legacy charset detection, invalid lines
  dropped not fatal.
- Export/import a versioned JSON with merge semantics; unknown fields ignored; a newer
  version accepted best-effort with a notice.

#### 3.4.4 OSD, hotkeys and mode switching

- VKey's floating V/E icon solves the “which mode am I in?” problem in fullscreen and
  multi-monitor setups; tray icon colors and a status toast are cheaper alternatives.
- AVIM's double-Ctrl toggle with an optional audio cue and a 2-second status popup is a
  good low-vision/eyes-free pattern; make the shortcut and sound configurable.
- Bamboo's `Shift+~` switches input mode for the current window; VKey's hotkey capture
  dialog lets users record a combo and detects conflicts.
- A single-tap Alt “access-key mode” (Funput Windows) prevents Excel/Office menu
  accelerators from being turned into Vietnamese words; exit on Enter/Esc/Tab/arrow,
  another Alt, mouse click, focus change, or 3-second timeout.
- Game mode: forward the original key in the same event so WASD still reaches the game
  while Vietnamese composition works.

#### 3.4.5 Convert tool and themes

- Convert selection: charset + case transforms (UPPER, lower, no diacritics, sentence
  case, title case), auto-paste and re-select, sequential mode with a 5-second cycle
  and reset on new selection.
- Warn by **naming** characters that cannot be encoded in the target charset, not just
  counting them.
- Theme model: semantic roles (label, character key, special key, accent, background),
  light/dark/auto, image background, per-key geometry, pressed state; preview uses the
  real renderer; contrast warnings; never let a theme change hitboxes or key meanings.
- Keep a “system-like” layout option for users who do not want a new keyboard shape.

---

## 4. What ViFastKey currently lacks

| Area | Current state | Gap | Best source | Priority |
|---|---|---|---|---|
| Browser event model | `keydown` on one `<textarea>`; ignores `isComposing`/229 | No `beforeinput`, composition lifecycle, paste/undo/autofill handling, mobile virtual keyboard | AVIM, modern web platform | **P0** |
| Browser document writes | Whole `textarea.value` rewritten each key | Native undo destroyed; caret/selection fragile; no contenteditable | AVIM `SpliceTxn` | **P0** |
| Browser selection | Deletes selection for some keys | No general replace-selection policy; no target ranges | AVIM | **P0** |
| Browser undo | None | Word-level undo/redo, preedit commit/discard before undo | AVIM + Funput | **P0** |
| Macro matching | Lowercase lookup, word-end only | Exact/smart case, longest match, boundary guard, punctuation triggers | Funput, VKey | **P0/P1** |
| Macro undo | None | One Backspace restores trigger + boundary; refusal semantics | Funput, VKey | **P0** |
| Sentence caps | Tracks `. ! ?` + whitespace | `…`, closing quotes, newlines, `1.5`, abbreviations, numeric starts | Funput | **P0/P1** |
| Spell exclusions | Spell check only | User dictionary, per-app spell exclusions, extra onsets, place names | VKey, Funput | **P1** |
| Suggestions | None | Prefix suggestions, English lexicon, personal learning, bigram | Funput | **P1/P2** |
| Typo correction | None | Touch-evidence correction, ambiguity, undo/refuse, field policy | Funput | **P2** |
| Mobile | No keyboard | Marked/direct composition, touch journal, field policy, onboarding | Funput | **P2** (large) |
| Per-domain profiles | App profiles only | Browser hostname route, hard rules, session memory | VKey | **P1** |
| Config | Ad-hoc JSON | Versioned interchange, merge import, platform blocks, UniKey macro import | Funput | **P1** |
| Settings/onboarding | Web/CLI only | GUI, search, quick tiles, Full Access, themes, OSD | VKey, Funput, Bamboo | **P1/P2** |
| Emoji/voice/clipboard | None | Emoji search, on-device voice, clipboard policy | Funput, Bamboo | **P2** |
| Legacy charsets/convert | None | TCVN3/VNI-Windows/VISCII/CP1258 + case conversion | Funput, VKey | **P1/P2** |
| Preedit previous word | REPLACE only | `DeleteSurrounding`/replace-committed action for preedit | Funput roadmap | **P1** |
| Accessibility | Basic ARIA on toggle | Status live region, keyboard-only flow, labels/hints, contrast | Funput, AVIM | **P1** |

---

## 5. Concrete test cases

These are behavioural cases worth turning into ViFastKey fixtures. They are written
from observed behaviour, not copied test code. IDs are local to this report.

### 5.1 Browser/DOM adapter

| # | Source | Given / when | Expected |
|---|---|---|---|
| B1 | AVIM | `<textarea>` focused, type `Tieengs` | `Tiếng`; caret after the word; `input` event observed |
| B2 | AVIM | `contenteditable` with `<b>Ti</b>engs` | Text becomes `Tiếng`; the `<b>` wrapper survives; no `innerHTML` rewrite |
| B3 | AVIM | Select `chao` in `xin chao`, type `chaof` | Selection replaced; result `xin chào`; no duplication |
| B4 | AVIM | `maxLength=10`, type `TieengsVieejt` | No value exceeds 10 UTF-16 units; no exception; caret clamped |
| B5 | AVIM | React-controlled input whose DOM mutation is overwritten | Adapter still commits the Vietnamese text once, or safely falls back; no lost keystrokes |
| B6 | AVIM/OWA | Office Online/contenteditable model that rejects DOM `replaceData` | Adapter detects rejection and uses the documented fallback; text not duplicated |
| B7 | AVIM | Type `tiengs`, press Ctrl/Cmd+Z | One undo reverts the whole word, not one code point; engine reset |
| B8 | AVIM | `oninput` autocomplete field; type a Vietnamese word | Autocomplete/in-page find sees the updated value; no stale suggestion list |
| B9 | AVIM | Shift+Space in the middle of `tieng` | A syllable break is inserted (new word), caret stays; optional 1 s overlay appears |
| B10 | AVIM | Field has `ime-mode: disabled`, or id/name in the exclusion list | Adapter is completely inert; no `input` event, no preventDefault |
| B11 | AVIM | Password field, then email/URL/number field | IME disabled according to policy; no preedit/logging |
| B12 | modern | Mobile Safari soft keyboard inserts `a` then `s` via `beforeinput` only | `á` composed; no reliance on `keydown` |
| B13 | modern | Native Japanese IME composition starts in the field | ViFastKey flushes and stands down; `compositionend` resynchronises; no double handling |
| B14 | modern | Paste `https://example.com/tieng` into the field, then type | Paste is untouched, engine resets; following Vietnamese still composes |
| B15 | VKey | Domain `google.com` in V mode, `github.com` in E mode; switch tabs | Mode follows hostname; hard-English rule blocks the V/E hotkey |
| B16 | VKey | Browser crashes/restarts while a domain rule is active | Session mode clears; hard rule survives; no stale mode after 5 s |
| B17 | AVIM | Another Vietnamese IME object is present on the page | ViFastKey disables itself on that page (or shows conflict), no double diacritics |
| B18 | modern | SPA navigates from a form to a search box without reload | Old composition flushed; new field gets a fresh adapter; no text carried over |

### 5.2 Mobile keyboard

| # | Source | Given / when | Expected |
|---|---|---|---|
| M1 | Funput | Same key sequence in marked-text mode and direct-commit mode | Final document text identical; both modes can be undone |
| M2 | Funput | Host coalesces two expected edits into one echo | Ledger matches the chained edits; composition is not cleared incorrectly |
| M3 | Funput | Touch A down, touch B down, B up, A up | Output order follows down-time, not lift-time; exactly one key per touch |
| M4 | Funput | `d9u7o7ng2`, thumb lands slightly toward `h` | Repaired to `đường` when evidence is strong; `nhad` near midpoint stays or offers both |
| M5 | Funput | Tap a key at the extreme left/right edge of the alphabetic row | Key receives the touch; no dead gap at screen edge |
| M6 | Funput | Double-tap Space; then hold Space 1.6 s and drag; then swipe left on Backspace | First inserts `. `; hold/drag moves caret and inserts no spaces; swipe deletes whole word |
| M7 | Funput | Switch to VNI | Number row is on and locked; accessibility label is “Dấu sắc”, not “1”; Telex row is toggleable |
| M8 | Funput | Password/email/URL/number field, or iOS `.asciiCapable` | Suggestions, learning, autocorrect, voice disabled; normal typing still works |
| M9 | Funput | Full Access off, toggle haptics/sound | Toggle asks for Full Access and does not silently turn on; turning off never asks |
| M10 | Funput | Search settings for `rung` or `haptic` | Haptics row is found; opens correct page; no accent needed |
| M11 | Funput | Theme with low text/key contrast | Editor shows a named contrast warning for the current light/dark mode, but saving is allowed |
| M12 | Funput | Paste text through the keyboard, then inspect clipboard history | Only text pasted through Funput is stored; pinned items do not expire; clear-all confirms |
| M13 | Bamboo | Type `:')` then `grin` in emoji panel | `:')` offers 😂; `grin` offers 😀; 9/page; arrows and 1–9 select; Esc restores raw |
| M14 | Funput | Voice session active, then change field/cancel/typing | Late final is discarded; final inserted once; no spaces/punctuation added; no transcript log |

### 5.3 Linguistic and AI

| # | Source | Given / when | Expected |
|---|---|---|---|
| L1 | v7 | `cấp`, `tiếc`, `thích`; `nhập`, `phục`, `mạch` | Tone 6 for the first group; tone 7 for the second; ordinary acute/underdot elsewhere remains 1/5 |
| L2 | v7 | `tèt`, `bèc`, `tẽt` | Rejected/restored: stop coda + huyền/ngã is not a valid Vietnamese syllable |
| L3 | v7 | Analyse `c/k/q`, `ng/ngh`, `g/gh`, `gi`, no onset | Families `k`, `ng`, `g`, `z`, `0`; `qua` → `koa`, `quen` → `koen`, `quy` → `kuy` |
| L4 | v7 | `gin`, `giết`, `giêng` | `z+in`, `z+iên` (tone 6 after final `t`→`n` normalisation), `z+iêng`; `gi` is an onset, not a separate vowel |
| L5 | v7 | `cuốc` and `quốc` from the same triplet | Both are valid outputs; ordering follows frequency, not spelling uniqueness |
| L6 | v7 | `xi0chao2mo5ng2` | Candidate list begins `xin chào mộ người`, `xin chào mộ ngày`, `xin chào mọi người`, … |
| L7 | v7 | `b7` (wildcard rhyme); `ximg0ch2` | Wildcard returns frequency-desc words; malformed raw returns no prediction, never a partial guess |
| L8 | v7 | After `xin`, type a constrained raw for `chao` with wrong tone/coda | Only candidates matching consonant family + rhyme family + tone survive; context reranks |
| L9 | Funput | Type `cảm `; then type `của ` | `cảm` predicts `ơn`; `của` stays silent because no follower dominates |
| L10 | Funput | Complete a word with `.`, newline, caret move, paste | Bigram context resets; no pair is learned across a sentence boundary |
| L11 | Funput | Type every valid Vietnamese syllable correctly with typo correction on | Output is byte-identical to correction off (property test) |
| L12 | Funput | `nhad` with touch slightly left/right/centre | Left may offer `nhá`, right `nhà`, centre no auto-apply; Backspace restores typed form |
| L13 | Funput/VKey | `password`, `correct`, `office`, `coffee`, `issue`, `seemed` | English words survive with doubled letters intact |
| L14 | Funput/VKey | `tesst`, `mixx`, `ass`, `offf` | Deliberate doubled-key habit still yields `test`, `mix`, `as`, `off` (not “repaired” into something else) |
| L15 | Funput | `Đắk Lắk`, `Krông Pắc`, `Chư Păh`, `Ia Kdăm` | Accepted with place-name/extra-onset rules; English `draw`, `cash`, `cool` still restore |
| L16 | Funput | `zoo`, `fair`, `jowf` with extra onsets; `food`, `wood`, `zebra` | `zô`, `fải`, `jờ`; English words with non-Vietnamese rhymes remain unchanged |
| L17 | Funput | `Hello. ` / `Hello.` / `1.5` / `3 con mèo` / `nói "Xin chào." rồi` / `v.v. ` | Cap after sentence + space; no cap after bare `.` or decimal; no cap after leading digit; closing quote is transparent; abbreviation guard suppresses `v.v. ` |
| L18 | Funput | `thuowng`, `thuowwng`, `thuow`, `huow`, `huoww`, `huowww` | `thương`, `thương`, `thuơ`, `huơ`, `hươ`, `huow` (3-state h/th/kh cycle) |
| L19 | Funput | `ass`, `ngs`, `aaa`, `ddd`, `WW` | `as`, `ngs`, `aa`, `dd`, `W`; raw keys preserved for Flip |
| L20 | Funput | `chana`, `homo`, then Flip, then Space | `chân`; Flip restores `homo`; Flip again restores `hôm`; Space clears composition |
| L21 | Funput | NFD input with combining marks at word boundary; delete a word | Combining marks are not split from their base; deletion count is UTF-16/code-point correct |

### 5.4 Macro, config and UX

| # | Source | Given / when | Expected |
|---|---|---|---|
| U1 | Bamboo/Funput | Macro `vn → Việt Nam`; type `vn `, `vn.`, `vn, ` | `Việt Nam `, `Việt Nam.`, `Việt Nam, `; punctuation preserved |
| U2 | VKey | Macro `bbh → bún bò Huế`; type `bbh`, `Bbh`, `BBH`; enable title case | Lower/title/UPPER variants; `Bbh` → `Bún bò Huế` with title-case option, `BBH` → `BÚN BÒ HUẾ` |
| U3 | VKey | Macro key `iOS → iPhone OS`; macro key `vn` | `iOS` only matches exactly; `VN`/`Vn`/`vn` all match the lowercase key |
| U4 | VKey | Macros `a b` and `b`; macro `a/b`; text `AnMa` | Longest candidate wins; punctuation inside key works; no match inside a longer word |
| U5 | Funput/VKey | Expand `vn `, press Backspace once | `vn ` restored (space kept); second Backspace deletes space; typing another key spends undo |
| U6 | Funput | Expand, undo, retype same trigger; then type another macro; then another word | First retype is refused (stays `vn `); refusal spent by next word; later `vn ` expands again |
| U7 | Funput | Expand `đc` from `ddc `, undo | `đc ` (shown form), not `ddc ` (raw keys) |
| U8 | VKey | Press Ctrl/Alt/Win+Backspace; tap modifier alone; press Esc after a partial word | No word deletion; composition preserved on modifier-only tap; Esc restores raw keys |
| U9 | AVIM | Double-tap Ctrl within 1 s; single Ctrl; Ctrl+C | Double tap toggles V/E with cue; single Ctrl does not; Ctrl+C passes through |
| U10 | VKey/Bamboo | App profile: Chrome → surrounding; terminal → preedit/forward; Minecraft excluded | Correct mode per app; terminal commits atomically; Minecraft receives no Vietnamese processing |
| U11 | VKey | App in excluded list and forced-V list simultaneously | English/excluded wins; never lock the user out of English |
| U12 | VKey | CJK layout activated in a forced-V app; then return to US | No E flip while forced-V; on return, saved mode is restored; composing word committed |
| U13 | Funput | Import config v1 with existing shortcuts; incoming duplicate trigger | Existing rows kept, duplicate trigger overwritten, new rows appended; unknown platform block ignored |
| U14 | Funput | Import UniKey macro file with BOM, `;` header, `url:https://…`, duplicate trigger | Header skipped; first colon only; duplicate last wins; invalid lines dropped; legacy charset detected |
| U15 | VKey | Select `Xin chào`; run convert sequential with UPPER + no-diacritic | First press `XIN CHÀO`, second `Xin chao`, third original; cycle resets after 5 s or new selection |
| U16 | Bamboo | CLI `status`, `set-method VNI`, `list-presets`, `set-app code 2`, `restart` | Same state as GUI; invalid values print choices; restart failure is reported but config persists |
| U17 | Funput | `Từ đã học`, clipboard, reset settings | All destructive actions confirm; clearing learned words keeps English lexicon; reset restores defaults |
| U18 | AVIM | Field id/name in blacklist; `ime-mode: disabled`; passwords toggle off | No processing and no events in those fields; changing the toggle affects new focus only |

### 5.5 Core regression cases

These should remain in the oracle/conformance suite when adapters or intelligence are
added.

| # | Case | Expected |
|---|---|---|
| C1 | `aa`, `ee`, `oo`, `aw`, `ow`, `uw`, `dd` | `â ê ô ă ơ ư đ` |
| C2 | `quatw`, `duowngfd`, `qusa`, `khuya`, `khuyur`, `queo` | `quăt`, `đường`, `quá`, `khuya`, `khuỷu`, `quẹo` |
| C3 | `truowng`, `nguoiwf`, `thuowr`, `huowu`, `ruowuj` | `trương`, `người`, `thuở`, `hươu`, `rượu` |
| C4 | `xoong`, `boong`, `quyts`, `buyts`, `huychs` | `xoong`, `boong`, `quýt`, `buýt`, `huých` |
| C5 | `batf`, `bàt`, `tèt`, `bèc`, `tẽt` | Rejected/restored where stop-coda tone rules forbid |
| C6 | `this`, `data`, `text`, `pass`, `password` | `this` stays Vietnamese `thí`; `data`/`text`/`password` restore English |
| C7 | `Đắk`, `Lắk`, `Búk`, `Krông`, `Plông` | Accepted with coda-k/place-name options; wrong tone on stop coda still rejected |
| C8 | `mp3`, `k8s`, `win10`, `bin10` | Macro digits work only when enabled; VNI digits still act as tone keys mid-word |
| C9 | `Chào` + Space + Backspace + `f` | `Chaof` (soft reset); no soft reset after a macro or English restore |
| C10 | `tiếng` + Backspace | `tiến`; repeated Backspace reaches empty; display-character deletion, not raw-key deletion |
| C11 | NFC/NFD output, combining marks, surrogate pairs | Delete counts and caret positions remain correct; no split grapheme |
| C12 | Raw length > 24 or URL/paste | Literal passthrough; no partial transformation |
| C13 | `Escape` after partial word | Raw keystrokes restored exactly |
| C14 | `[`/`]` in Simple Telex vs. full Telex | Simple Telex literal; full Telex `ơ`/`ư` with doubled-key escape |
| C15 | `w` at word start vs. after onset | Full Telex `ư`; ordinary Telex preserves its documented deferred-`w` behaviour |
| C16 | Auto-cap at document start/newline/after `…`/after quote | Correct; `1.5`, `3 con mèo`, bare `Hello.` do not cap |

---

## 6. Prioritized recommendations for ViFastKey

The ordering optimises user-visible value per unit of risk. P0 keeps the engine pure and
fixes the browser adapter; P1 adds text intelligence and product surface; P2 is
optional/mobile/AI.

### P0 — do first

#### P0.1 Engine: macro contract v2 with undo (highest value / low risk)

**Why:** macros already exist, but the most common complaint in the surveyed projects
is “I expanded the wrong thing and have to delete it character by character”. Funput
and VKey both solved it with a one-Backspace undo, and both found the same edge cases.

**Engine work:**

- Matching order: exact key first, then smart-case only for a lowercase stored trigger;
  a trigger containing uppercase is exact-match only.
- Longest match at the word boundary; boundary guard so `AnMa` never matches `nMa`.
- Trigger keys configurable: Space, Enter, Tab, punctuation; punctuation inside a key
  remains reachable (`a/b`).
- Store the pre-expansion shown text, the trigger, and the exact written expansion
  including the boundary.
- API: `has_shortcut_undo`, `shortcut_undo_text`, `shortcut_undo_written`,
  `undo_shortcut`, `dismiss_shortcut_undo`.
- Undo restores the trigger + boundary; a second Backspace deletes the boundary
  normally. Undo is valid until the next key, focus/caret change, or document mismatch;
  a refused trigger applies to the next occurrence only.
- Works in REPLACE and PREEDIT; in PREEDIT, undo must replace the committed expansion
  only if the adapter can verify the document still ends with it.
- A `shortcut_undo` is not consumed by a failed document check; fail safe by deleting
  one character as usual.

**Acceptance:** U1–U8; differential test: macro undo off/on, no macro table present,
English mode, punctuation/Enter/Tab, NFD output.

#### P0.2 Engine: shared sentence scanner and auto-cap policy

**Why:** the current `. ! ? + whitespace` rule is too narrow and produces wrong caps in
real text (`…`, quotes, decimals, abbreviations, line starts).

**Engine work:**

- One pure scanner used by auto-cap, text-case and conversion:
  - sentence end: `.` `?` `!` `…`; closing quotes/brackets are transparent;
  - newline is a boundary even without punctuation;
  - bare punctuation at the caret is not yet a boundary until whitespace follows;
  - `1.5` is not a boundary; a leading digit means the sentence has started (do not
    cap the following word);
  - typing-mode abbreviation guard: `v.v. `, `U.S. `, `a.m. ` do not cap; `TS. ` and
    `TP. ` remain known limitations unless a list is added.
- Field policy: password/email/URL/number/one-time-code never auto-cap; “all
  characters” field mode wins over the user toggle; if the user toggle is off, no
  sentence cap.
- Adapter reports focus/caret moves so the scanner can arm/disarm; don't assume
  document start after a click or app switch.

**Acceptance:** L17 and C16, plus scanner unit tests for `…`, `»”’›`, `\n`,
`Hello.` vs. `Hello. `, `1.5`, `3 con mèo`, `v.v. `.

#### P0.3 Web: replace the keydown-only playground adapter

**Why:** the current playground cannot represent real browser/mobile behaviour, and it
destroys native undo by assigning `textarea.value`. It is also the best place to prove
the adapter design before building an extension.

**Web work:**

- Introduce `web/adapter.js` with `TextControlAdapter` and `EditableAdapter`.
- Event pipeline: `beforeinput` primary, `compositionstart/update/end` guards,
  `keydown` fallback, `input` resync; ignore `isComposing`/229.
- Never rewrite the whole value. Apply the smallest edit:
  - text controls: select range + `setRangeText`/`execCommand`; preserve undo where
    possible;
  - contenteditable: `Range` on text nodes, never `innerHTML`.
- Selection: replace non-collapsed selection on a character key; otherwise pass.
- Undo journal: word-level coalescing, Ctrl/Cmd+Z / Shift+Z / Ctrl+Y, invalidate on
  external changes.
- `maxLength` clamp; caret/scroll preservation; dispatch an `InputEvent` after
  programmatic writes.
- Add a second demo: `contenteditable` article with bold/italic spans, plus a mobile
  “soft keyboard” simulation that emits only `beforeinput`.
- Add a visible status live region (`aria-live="polite"`) for “Tiếng Việt / English /
  composing / word restored”.
- Keep the current REPLACE/PREEDIT engine modes; the adapter is separate.

**Acceptance:** B1–B14, B18; Playwright (Chromium + Firefox + WebKit) and a jsdom unit
suite; native undo works for at least one word; no input duplication in a controlled
React fixture.

#### P0.4 Web/docs: publish the adapter contract and test matrix

- `docs/browser-adapter.md`: event order, input types, composition, selection, undo,
  contenteditable, mobile, host-specific shim policy, per-domain policy.
- `docs/undo.md`: journal format, coalescing, invalidation, relation to macro undo.
- `docs/macro.md`: matching, triggers, case rules, undo/refusal, import/export.
- Add the cases in §5.1 and §5.4 to `web/tests` or a shared fixture file.
- Add a “browser support” table with known limitations; do not promise address-bar
  interception.

### P1 — next

#### P1.1 Engine: preedit edit-previous-word and generalized commit undo

- Add an action that can replace a committed range (`DeleteSurrounding`/`ReplaceRange`)
  so PREEDIT can edit the word just committed, not only the current composition.
- Generalize undo state for macro and future correction: same “one Backspace restores”
  contract, with a reason tag (`macro`, `correction`, `restore`).
- Acceptance: type `tiêng`, press `s` → `tiếng` in PREEDIT; type a macro, Backspace
  restores; an external document mismatch refuses the undo safely.

#### P1.2 Engine: spelling policy and place names

- Add a user spell-exclusion list (case-insensitive, local, importable/exportable).
- Add optional extra onsets `z f w j` (bitmask, off by default) with English differential:
  `zoo` → `zô`, `fair` → `fải`, but `food`/`wood`/`zebra` unchanged.
- Add place-name/ethnic-minority rules (shared/distinct onset clusters, final `k`,
  `h/l/r` only where the method can distinguish them). Keep English restore as the
  guard: a word that used to restore must still restore.
- Acceptance: L13–L16, C7; on/off differential over the English dictionary and the
  Vietnamese corpus.

#### P1.3 Engine/product: suggestion port (separate crate, not core)

- `vfk-suggest` (or a feature-gated module) with:
  - personal word store (bounded, local, reset button);
  - top-3 prefix suggestions, personal first, optional English lexicon fills slots;
  - minimum prefix 2; empty-prefix predict only with follower dominance;
  - no I/O in the key path; 0 allocations warm; p99 < 2 ms.
- Keep it out of `vi-syllable`/`vfk-core`; the engine only gets a port/callback.
- Lexicon data must ship with its own notice and be separate from the personal store.
- Acceptance: `cảm` → `ơn`, `của` silent; suggestion acceptance learns a token; clearing
  learned words keeps the bundled lexicon; typing latency unchanged when disabled.

#### P1.4 Web/product: per-domain profiles and a small extension scaffold

- Add hostname profiles (on/off/method) and a precedence rule:
  site hard English > app hard English > session memory > global.
- Start with a content-script + popup; native messaging is optional and should follow
  VKey's minimum-data protocol if added.
- Store learned state in session storage and hard rules in local storage; clear session
  state on browser restart.
- Acceptance: B15–B17; no URL/path/keystroke crosses the extension boundary; an
  address-bar limitation is documented.

#### P1.5 Product: config interchange and UniKey import

- Versioned JSON with `schema`, `version`, `preferences`, `shortcuts`, `platform`.
- Merge import (never delete); unknown keys ignored; newer version accepted best-effort
  with a notice; platform blocks applied only on that platform.
- UniKey macro import: BOM, `;` header, first colon, duplicate last wins, encoding
  detection, invalid lines dropped.
- Atomic write: temp file + fsync + rename; keep a backup; never leave a half-written
  config.
- Acceptance: U13–U14; round-trip across Python/Rust/WASM; invalid file leaves current
  config untouched.

#### P1.6 Web/docs: settings, accessibility and convert surface

- Settings panel with sections, search keywords (accent-free + English), quick toggles,
  reset defaults, macro editor, import/export.
- Accessibility: labels/hints, focus order, live status, contrast warnings, keyboard-only
  path, large-font/narrow layout tests.
- Convert panel: charset + case transforms, sequential mode with timeout, warning that
  names lost characters.
- Docs: `docs/settings.md`, `docs/accessibility.md`, `docs/config-format.md`.

### P2 — optional / larger bets

#### P2.1 Mobile adapter/keyboard

- Only after the web adapter proves the action model; use Funput's marked/direct
  switch, touch journal, field policy, and local suggestions.
- Start with an iOS or Android prototype, not both; reuse the C ABI/WASM core.
- Acceptance: M1–M10; no network code; Full Access/privacy manifest; 20-minute typing
  session with no lost keys.

#### P2.2 Personal bigram / “predict only when sure”

- Add four follower slots, generation-tagged indices, Misra–Gries loading, aging 256,
  rerank threshold 1, predict threshold 2 + dominance 40%.
- Context resets on punctuation, caret move, paste, focus change, backspace-to-empty.
- Never read the document on each key.
- Acceptance: L9–L10; memory bounded (measure bytes/word); query 0 alloc warm; UI only
  updates when the list changes.

#### P2.3 Constrained neural next-word (v7-style)

- Optional, lazy-loaded, local-only; separate from core and from the default WASM
  bundle.
- Model proposes; phonology filters `(consonant, rhyme, tone)`; candidates 9/page;
  context from committed words only; fallback to personal n-gram/dictionary.
- Do not ship a checkpoint until model/dataset licences are verified.
- Acceptance: L6–L8; p99 latency budget; no network; works with model absent.

#### P2.4 Typo correction (mobile-first)

- Only with touch evidence and a large known-word filter; off by default for existing
  users; per-field policy; one-Backspace undo; “keep” list; refusal next occurrence.
- Never change a valid Vietnamese syllable; never change a word the user typed correctly.
- Acceptance: L11–L12; measured false-fix rate; no impact on typing latency when off.

#### P2.5 Emoji, voice, clipboard, themes, legacy charsets

- Emoji: Bamboo's `:`/keyword trie + 9/page; Funput's emoji/kaomoji split.
- Voice: on-device only, explicit start, 60 s, no cloud/logging.
- Clipboard: only what the user pasted through ViFastKey; expiry/pin/clear.
- Themes: semantic roles, contrast warnings, real renderer preview, no hitbox changes.
- Legacy charsets/convert: TCVN3, VNI-Windows, VISCII, CP1258, with round-trip and
  loss-by-name warnings.

### Acceptance gates across all work

1. **Differential:** every new option off must be byte-identical to the current
   oracle; on may change only the documented cases.
2. **Property tests:** valid syllables never altered by correction; preedit and replace
   produce the same final text; undo restores exactly; diff(old,new) applies to new.
3. **Conformance:** Python and Rust outputs equal on the expanded fixture set.
4. **Performance:** no new allocation in the key path; suggestions p99 < 2 ms; typing
   path latency unchanged when features are disabled.
5. **Privacy:** no network in engine/keyboard; no content logs; metrics are counts.
6. **Accessibility:** keyboard-only flow, labels/hints, live status, contrast checks.
7. **Licensing:** every borrowed data file/model/asset has a notice and a compatible
   licence; no GPL/AGPL code in the MIT tree.

---

## 7. Licensing constraints

### 7.1 Project licences

| Project | Licence | Code reuse in MIT ViFastKey | Notes |
|---|---|---|---|
| **1ec5_avim** | MIT | **Allowed** with copyright/permission notice | Old code but legally compatible; prefer reimplementation of behaviour; tests may also be MIT |
| **Funput** | MIT (workspace/crates) | **Allowed** with notice | Some data/assets have separate licences; see below |
| **ducngg_v7** | Apache-2.0 | **Allowed** with NOTICE and patent clause | Model checkpoint, datasets and PyTorch dependencies are separate; verify before bundling |
| **phatMT97_VKey** | GPL-3.0 | **Not allowed** | Behaviour, UX, test cases only; no code, no linking, no copied test implementation. The “advanced” Rust engine is closed-source and unavailable anyway |
| **ngkhhuy_bamboo-viet** | GPL-3.0 (wrapper); `bamboo-core` MIT | **Wrapper: no. `bamboo-core`: MIT-compatible** | Do not copy wrapper/UI/engine code; the MIT core could be used with notice, but ViFastKey already has its own core, so behaviour-only is simpler |

### 7.2 Data, models and assets

These are **not** covered by the repository's code licence and must be audited
separately:

- **Funput English lexicon:** SCOWL/ESDB (MIT-like, keep copyright), Google Books Ngram
  v3 (CC BY 3.0), LDNOOBW (CC BY 4.0), a project-authored supplement (MIT), and a
  project-authored blocklist. If ViFastKey ships a similar lexicon, keep a
  `THIRD_PARTY_NOTICES` file and preserve attribution; do not copy the built `.lex`
  without its notices.
- **Funput fonts/themes/emoji:** Be Vietnam Pro is OFL; theme images/fonts may have
  their own terms; do not assume the MIT code licence covers artwork.
- **v7 model/dataset:** the code is Apache-2.0, but the checkpoint is a separate
  download and the training dataset (`5CD-AI/Vietnamese-alpaca-gpt4-gg-translated`) has
  its own terms. Do not bundle weights or training data until the licences are checked.
- **Vietnamese dictionaries:** Viet74K, news corpora, `vietnamese_curated_dataset`,
  hunspell-vi, etc. each have different terms. A dictionary's licence is not implied by
  the IME's code licence.
- **Bamboo emoji data:** EmojiOne/emoji JSON may be CC-BY or another data licence;
  verify before copying the trie or keyword list.
- **AVIM audio cues/icons:** MIT code, but check any bundled sound/icon assets.

### 7.3 Safe process for GPL/AGPL projects

1. Read the GPL code to understand behaviour, then close the file.
2. Write a behavioural specification in your own words (this report is an example).
3. Implement from the specification and from public standards (Unicode, Vietnamese
   orthography), not from the GPL source structure.
4. Re-express test cases in ViFastKey's fixture format; do not copy test code, helper
   classes, comments or identifiers.
5. Record provenance in `docs/research/` and `THIRD_PARTY_NOTICES.md`.
6. Run a licence scan in CI (e.g. `cargo-deny`, `pip-licenses`, `reuse`) and keep SPDX
   headers.
7. Do not link GPL libraries into the MIT engine, even dynamically, if the resulting
   distribution could be considered a derivative. When in doubt, ask a lawyer.

### 7.4 Inbound contribution policy

- ViFastKey is MIT; contributions must be MIT-compatible.
- Require a DCO/sign-off or a short CLA.
- Reject patches that copy GPL/AGPL code or data without a compatible licence.
- Keep a `docs/provenance.md` table: feature, source project, licence, whether code or
  behaviour was used, reviewer.

---

## 8. Source map (most useful files)

| Topic | Files |
|---|---|
| AVIM browser adapter | `1ec5_avim/content/frame.js`, `content/avim.js`, `components/transformer.js`, `content/kix.js`, `content/disabler.js` |
| AVIM tests | `1ec5_avim/tests/test.js`, `tests/corpus.js`, `tests/transformer.js` |
| AVIM settings/blacklist | `1ec5_avim/content/options.xul`, `content/blacklist.xul`, `defaults/preferences/avim.js` |
| v7 phonology | `ducngg_v7/utils/vietnamese.py`, `utils/dictionary.py` |
| v7 prediction/UI | `ducngg_v7/imethod/v7.py`, `imethod/v7ai.py`, `app/app.py`, `ai/configs.py`, `ai/tokenizer.py` |
| Funput mobile pipeline | `Funput_Funput/platforms/ios/docs/INPUT_PIPELINE_3_ARCHITECTURE.md`, `platforms/ios/docs/ARCHITECTURE.md` |
| Funput direct composition | `Funput_Funput/docs/features/macos-direct-composition.md` |
| Funput suggestions/bigram | `Funput_Funput/docs/features/context-suggestion.md`, `docs/features/english-lexicon-suggestion.md` |
| Funput typo correction | `Funput_Funput/docs/features/typo-correction.md` |
| Funput macro/undo/casing | `Funput_Funput/docs/features/shortcut-undo.md`, `crates/funput-engine/tests/words/shortcut*.rs` |
| Funput config/import | `Funput_Funput/platforms/CONFIG_FORMAT.md`, `crates/funput-config/src/unikey.rs` |
| Funput auto-cap/sentence | `Funput_Funput/docs/features/auto-capitalize.md`, `crates/funput-core/src/sentence` |
| VKey macro/casing | `phatMT97_VKey/docs/MACRO_CASING.md`, `tests/MacroCaseTest.cpp`, `tests/MacroContextMatchTest.cpp` |
| VKey auto-cap/per-app/CJK | `phatMT97_VKey/tests/AutoCapDecisionTest.cpp`, `tests/PerAppModeDecisionTest.cpp`, `tests/CjkSwitchDecisionTest.cpp` |
| VKey browser extension | `phatMT97_VKey/docs/BROWSER_EXTENSION.md` |
| VKey convert/hotkeys/UI | `phatMT97_VKey/docs/GUIDE.md`, `src/app/classic/ClassicHotkeysDialog.cpp`, `src/app/system/QuickConvert.cpp` |
| Bamboo presets/modes/CLI | `ngkhhuy_bamboo-viet/config/config.go`, `config/flags.go`, `cmd/config_tool/main.go` |
| Bamboo GUI/macro/emoji | `ngkhhuy_bamboo-viet/cmd/gui/bamboo_viet_gui.py`, `mactab.go`, `emoji.go`, `engine_emoji.go` |
| Bamboo fuzzer | `ngkhhuy_bamboo-viet/cmd/fuzzer/*`, `docs/fuzz-report.md` |
| ViFastKey current web | `ViFastKey/web/app.js`, `web/index.html`, `web/README.md` |
| ViFastKey current engine | `ViFastKey/python/vfk_engine/session.py`, `python/vfk_core`, `python/vi_syllable`, `python/EDGE_CASES.md` |

---

## 9. Final judgement

ViFastKey's core is already stronger than most surveyed engines in clean separation,
conformance testing and cross-language portability. The next leverage is **not** more
Vietnamese rules; it is the **adapter layer** and the **reversible editing UX** around
the engine.

Do P0 in this order:

1. macro undo + smart matching (engine, small, immediately useful);
2. sentence scanner + auto-cap policy (engine, small);
3. browser `beforeinput`/composition/contenteditable/undo adapter (web, largest visible
   gap);
4. adapter/undo/macro docs and fixtures (docs, keeps the design honest).

Then P1 adds suggestions, config interchange, per-domain profiles and settings. P2 is
where mobile, typo correction, AI and voice belong — after the data/model/licence work
and only behind explicit opt-ins.
