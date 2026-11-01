> **Báo cáo nghiên cứu nội bộ.** Tài liệu này chỉ ghi nhận hành vi, kiến trúc, UX và
> test fact của các dự án được khảo sát. Không có mã nguồn, bảng dữ liệu, từ điển hay test
> corpus GPL/AGPL nào được sao chép vào ViFastKey. Xem `THIRD_PARTY_NOTICES.md` và
> `docs/IME_FEATURE_RESEARCH.md`.

# Analysis 04 — Public Linux Vietnamese IMEs, extracted for ViFastKey

**Date:** 2026-10-29  
**Scope:** six locally cloned projects under `thư mục clone tạm/`:
`BambooEngine/ibus-bamboo`, `LotusInputMethod/fcitx5-lotus`, `ngkhhuy/bamboo-viet`,
`vn-input/ibus-unikey`, `fcitx/fcitx5-unikey`, `sonnam0904/telebit`.
**Method:** README/docs, source, config schemas, test code, golden corpora, and issue-oriented comments.
A small differential probe of Telebit's MIT core was run against the current ViFastKey Python
engine to confirm the concrete typing differences in §4.1.

**No GPL code was copied.** This report extracts behavior, UX, adapter patterns, schemas and
test ideas only. Licensing rules are in §7.

---

## 0. TL;DR

1. Linux IMEs converge on a **preedit-first UX with per-app fallbacks**: preedit for most apps,
   surrounding-text/delete for browsers and office suites, synthetic keys/uinput for the rest,
   and a complete exclusion path for games/password fields.
2. **Lotus is the richest reference** (7 modes, per-app rules with three delay knobs, ACL-based
   uinput server, emoji history, custom dictionary/keymap, very strong tests). It is GPLv3, so
   use it as a behavior/test reference only.
3. **Telebit is the best-licensed modern reference** (MIT): direct-commit rollback, browser/PWA
   force-preedit detection, English-restore escape rules, app auto-registration, CLI diagnostics.
   Its core test corpus is a good independent oracle for ViFastKey's English/shaped-vowel cases.
4. ViFastKey's core is already stronger than most Linux engines in raw typing features
   (Simple Telex, quick Telex, coda k, macro digits, smart caps, soft reset, NFC/NFD, FFI/WASM).
   Its biggest gaps are **host integration** and a small set of **core edge cases**.
5. Two verified ViFastKey core bugs are high-impact:
   - common English words lose letters (`coffee → cofee`, `mission → mision`,
     `daddy → dady`, `database → datbase`, `cheese → chée`);
   - standalone shaped vowels revert at word end (`aw ␠ → aw ␠`, `aws ␠ → aws ␠`,
     VNI `a8 ␠ → a8 ␠`), and auto-capitalize therefore produces `Aw` instead of `Ă`.
6. No analyzed project implements **libei or the RemoteDesktop portal**. Wayland injection is
   either fcitx5 surrounding-text/preedit, or Lotus's kernel uinput helper. This is an open
   opportunity, not a solved pattern.

---

## 1. Project inventory

Approximate code size excludes `.git`, vendored dependencies, and most generated data.

| Project | Version / last commit | Stack / integration | License | Size | Status | Why it matters |
|---|---|---|---|---|---|---|
| **BambooEngine/ibus-bamboo** | 0.8.5, 2026-05 | Go + cgo, IBus D-Bus engine; X11 introspection/XTest; wlr foreign-toplevel + GNOME Shell Eval | **GPL-3.0**; vendored `bamboo-core` is **MIT**; dictionary GPLv3; emoji JSON MIT | ~5.9k Go | Stalled; README points users to fcitx5-unikey | The classic Linux Vietnamese IME feature set: 7 output modes, many charsets, per-app mode menu, macro file, emoji, Unicode hex |
| **LotusInputMethod/fcitx5-lotus** | 6.0.3, 2026-10-28 | C++17 fcitx5 addon + separate uinput server + Python/Qt settings GUI | **GPL-3.0-or-later** (including `lotus/`, despite MIT core notice); dictionary GPLv3 | ~26.9k C/C++/Python | Active | Most sophisticated Linux engine: Smooth/SuperSmooth/Select uinput modes, app rules + delay overrides, emoji history, custom keymap, 29k-line golden corpus |
| **ngkhhuy/bamboo-viet** | 1.0.1, 2026-08-25 | Go IBus engine + C shared lib (`libvicore`) + C++ fcitx5 addon + GTK GUI/CLI | **GPL-3.0**; core MIT | ~8.3k Go/C++ | Active fork, ambitious but uneven | Built-in app presets, CLI/GUI control panel, C ABI, `docs/known-issues.md` bug matrix; its fcitx5 addon is incomplete |
| **vn-input/ibus-unikey** | 0.7.0, 2025-07 | C++/C IBus engine + LGPL `ukengine` | **GPL-3.0**; `ukengine` **LGPL-2.1** | ~9.4k C/C++ | Explicitly unsupported | Minimal, stable preedit-only IME; GSettings config; macro/keymap editors; useful for UniKey semantics |
| **fcitx/fcitx5-unikey** | 5.1.12, 2026-09-05 | C++ fcitx5 addon + `ukengine` + Qt macro/keymap editors | **GPL-2.0-or-later**; `ukengine` LGPL | ~13.5k C++ | Active | Preedit-only but with surrounding-text rebuild/modify, preedit styles, auto-commit of stable consonants, 1,721-case Telex corpus |
| **sonnam0904/telebit** | 2.15.1, 2026-09-15 | Standalone C++ core + fcitx5 addon + GTK4 setup + CLI doctor | **MIT** | ~14k C++ | Active | Modern, MIT, direct-commit rollback, browser/PWA force-preedit, English escape rules, app auto-registration, AI assistant, strong tests |
| **ViFastKey (current)** | repo at `repo ViFastKey` | Python oracle + Rust crates + C ABI + WASM | **MIT** | ~7.4k Python/Rust | Core/conformance strong; no real Linux adapter | Already covers most engine features; needs adapter, charsets, emoji, app-rule enforcement, and a few core fixes |

### 1.1 Architecture notes

- **Bamboo** is an IBus engine with no separate helper. It injects keys through IBus
  `ForwardKeyEvent`, XTest, or `DeleteSurroundingText`; it sleeps in the key path and queues
  keystrokes to compensate. App identity is obtained from IBus `WM_CLASS`, X11 focus window,
  GNOME Shell `Eval`, or `wlr-foreign-toplevel` (wlroots only).
- **Lotus** is a fcitx5 addon plus a separate `fcitx5-lotus-server` process. The server owns
  `/dev/uinput`, creates a virtual keyboard, and accepts a small binary protocol over per-user
  abstract Unix sockets. The addon talks to it for backspace/select bursts; the server paces
  injected keys and handles physical Shift state. Packaging supplies a udev ACL and a systemd
  unit, so the addon itself does not need root.
- **Bamboo Viet** keeps the IBus engine but exports a C ABI (`libvicore`) and adds a C++ fcitx5
  addon. The fcitx5 addon currently just forwards a commit/preedit pair and ignores the
  backspace/diff information returned by the C ABI; it is a demo, not parity with the IBus path.
- **ibus-unikey** is a thin IBus wrapper over `UkEngine`; it uses only client preedit plus
  `DeleteSurroundingText` for the optional `ModifySurroundingText` experiment.
- **fcitx5-unikey** is a thin fcitx5 wrapper over `UkEngine`; no key injection. It can rebuild
  the engine state from surrounding text and, experimentally, pull the previous word into
  preedit and rewrite it.
- **Telebit** keeps a pure C++ word engine independent of fcitx5. The addon chooses preedit or
  direct commit + rollback, and uses fcitx5 `deleteSurroundingText`/`commitString` rather than
  synthetic keys.
- **ViFastKey** already has the cleanest split: pure `vi-syllable`/`vfk-core`, an `ImeSession`
  that returns `PassThrough | Preedit | Replace | Commit`, and a C ABI that expresses the same
  actions. It has no Linux adapter yet, which is exactly why the Linux patterns in this report
  matter.

---

## 2. Feature matrix

Legend: **●** full / first-class, **◐** partial, optional, or inherited with caveats, **—** absent.
`Bamboo Viet` means the ngkhhuy fork; its IBus path inherits Bamboo, its fcitx5 path is partial.

### 2.1 Typing and engine features

| Feature | Bamboo | Lotus | Bamboo Viet | ibus-unikey | fcitx5-unikey | Telebit | ViFastKey |
|---|---|---|---|---|---|---|---|
| Telex | ● | ● | ● | ● | ● | ● | ● |
| Simple Telex | — | — | ◐ advertised in GUI, **not supported by core** | ● | ● (`Simple Telex`, `Simple Telex2`) | — | ● |
| VNI | ● | ● | ● | ● | ● | ● | ● |
| VIQR | ● | ● | ● | ◐ engine supports, UI does not expose | ● | — | — |
| Microsoft layout / hybrid / custom | ● (MS, Telex+VNI+VIQR, custom) | ● (8 methods incl. custom) | ◐ IBus path; fcitx5 addon hardcodes Telex | ◐ `ukengine` supports, UI limited | ● (MS, UserIM) | — | — |
| Old/new tone style | ● | ● | ● | ● | ● | ● | ● |
| Free tone marking | ● | ● | ● | ● | ● | ● (always, no toggle) | ● (always, no toggle) |
| Spell check by rules | ● | ● | ● | ● | ● | ● | ● |
| Spell check by dictionary | ● optional | ● optional custom dict | ◐ inherits | — | ◐ engine dictionary | — | ● port, small data |
| Auto-restore invalid/non-VN word | ● | ● | ● | ● | ● | ● | ● |
| `dd` free style | ● | ● | ◐ inherits | — | — | — | ◐ core behavior, no toggle |
| `w` → `ư` granularity | ◐ Telex W / Telex 2 variants | ● Disabled / NonStart / Everywhere | ◐ inherits | ● `standalone-w-as-uw` bool | ● `process_w_at_begin` bool | ◐ context only; `w` at start stays `w` | ◐ bool only (`w_as_u`, everywhere) |
| Bracket `[`/`]` → `ơ`/`ư` | ● via Telex 2/MS layouts | ● Disabled / NonStart / Everywhere | ◐ inherits | — | — | — | ● bool only |
| Quick Telex (`cc=ch`, `gg=gi`, …) | — | — | — | — | — | — | ● |
| `allow_coda_k` (Đắk Lắk) | — | — | — | — | — | — | ● |
| Macro basic | ● | ● | ● | ● | ● | ● | ● |
| Macro case preservation | ● | ● | ● | ◐ | ◐ | — | ● |
| Macro digits (`mp3`, `k8s`) | ◐ only if macro prefix contains digits | ● | ◐ inherits | — | — | — | ● option |
| Macro on Tab / `$TIME` / `$DATE` | ◐ Tab, no time/date | ● Tab + time/date | ◐ Tab | — | — | — | — |
| Macro skip modifier for next word | — | ● Shift/Ctrl/Alt | — | — | — | — | — |
| Emoji picker | ● 2,666 emojis, 9/page; shortcut opt-in (default disabled) | ● fcitx5 emoji addon, history 9, 9/page | — | — | — | — | — |
| Auto-capitalize sentence | — (macro auto-cap only) | ● | — | — | — | ● ASCII-only | ● |
| Smart Caps (`JSON`, `HTTP`, not `DD`) | — | — | — | — | — | — | ● |
| Double space → `. ` / double hyphen → em dash | — | ● | — | — | — | — | — |
| Legacy output charsets | ● 10 | ● 17 | ◐ inherits | ● 8 | ● 8 | — | — |
| NFC/NFD output choice | — | ◐ “Unicode tổ hợp” charset, no normalisation option | — | — | — | — | ● |
| Custom keymap / custom IME | ● custom input-method definitions | ● custom keymap + `Custom` method | ◐ inherits + C ABI | ● keymap editor | ● keymap editor | — | ◐ data-driven `InputMethodSpec`, no import/editor |
| Custom dictionary editor | — | ● | — | — | — | — | ◐ port only, no editor |
| Unicode hex input (`Ctrl+Shift+U`) | ● | — | ◐ inherits | — | — | — | — |
| English dictionary restore | ◐ rule-based auto-NVN, no English list | ◐ rule-based auto-NVN | ◐ inherits | ◐ rule-based | ◐ rule-based | ● syllable-gate restore (not dictionary) | ● dictionary port, small list |

Notes:
- **Free marking** in Bamboo/Lotus/Unikey is a configurable engine flag. ViFastKey and Telebit
  effectively always allow arbitrary tone-key placement because the tone is stored separately
  and rendered at the end; there is no “strict” toggle.
- **Legacy charsets** are output-only. Lotus lists 17: Unicode, BKHCM 1/2, NCR Decimal/Hex,
  TCVN3 (ABC), UTF-8, Unicode C-string Decimal/Hex, Unicode combining, VIQR, VISCII,
  VNI Windows, VPS, Vietware Full/X, Windows-1258. Bamboo has 10: Unicode, TCVN3, VNI Windows,
  VIQR, VISCII, VPS, BKHCM 1/2, NCR Decimal/Hex. fcitx5-unikey/ibus-unikey have 8.
- **Bamboo Viet's GUI bug:** `cmd/gui/bamboo_viet_gui.py` offers `Simple Telex` and
  `Unicode Tổ Hợp`, but `bamboo-core` has neither an input method by that name nor that
  charset. Selecting them writes an unsupported value and the engine falls back (empty input
  method / Unicode). The CLI correctly limits to `Telex|VNI`.
- **Lotus macro-key bug:** `MacroTable::set()` stores keys verbatim, while `get()` lowercases the
  query. A macro key containing uppercase letters can never match. The editor/import path should
  lower-case keys or document lower-case-only keys.
- **Lotus mode-order bug:** the settings GUI's reset default is
  `Smooth,Uinput,Select,SurroundingText,...`, but `kModeNames` expects `Smooth,Select,...`.
  `Uinput` is silently ignored; the resulting menu order differs from the label.
- **Macro reload:** Bamboo polls its macro text file every 3 seconds and reloads on mtime change;
  Lotus reloads the macro table on config apply (and the dictionary via `dict-reload.cpp`);
  Telebit rebuilds the macro index on `setConfig`/`reloadConfig`. A file watcher is nicer than a
  poll and is a small ViFastKey adapter win.
- **Mode names to keep straight**:
  - Bamboo: *Pre-edit*, *Surrounding Text*, *ForwardKeyEvent I* (Backspace forwarding),
    *ForwardKeyEvent II* (Shift+Left), *Forward as commit*, *XTestFakeKeyEvent*, and *exclude
    this app* (`UsIM`).
  - Lotus: *Off*, *Uinput (Smooth)*, *Uinput (Super Smooth)*, *Uinput (Select)*,
    *Surrounding Text*, *Preedit*, *Emoji Picker*.
  - fcitx5-unikey: preedit only; *Modify surrounding text* is a separate experimental option,
    not a mode.
  - Telebit: preedit vs *Direct commit + rollback*; force-preedit is per app, not a global mode.

### 2.2 Adapter / output / UX features

| Feature | Bamboo | Lotus | Bamboo Viet | ibus-unikey | fcitx5-unikey | Telebit | ViFastKey |
|---|---|---|---|---|---|---|---|
| Preedit | ● | ● | ● | ● | ● | ● | ● |
| Surrounding-text edit / previous word | ● mode | ● mode | ● improved selection-aware `SetSurroundingText` | — | ● rebuild + experimental modify | — (delete only) | ● REPLACE + adopt, no selection |
| Replace / backspace stream | ● | ● uinput | ● | — | — | ● direct commit rollback | ● `Replace` action |
| ForwardKeyEvent (IBus) | ● 2 variants + forward-as-commit | — | ◐ inherits | — | — | — | — |
| XTest | ● X11 only | — | ◐ inherits | — | — | — | — |
| uinput helper | — | ● separate server | — | — | — | — | — |
| libei / portal injection | — | — | — | — | — | — | — |
| Direct commit + rollback | ● backspace modes | ● Smooth/SuperSmooth | ● | — | — | ● | ● REPLACE |
| Per-app output mode | ● 7-mode mapping | ● mode + delays + `ctx_` fallback | ● presets + mapping | — | — | ◐ force-preedit list only | ◐ profiles/hints, not enforced |
| Per-app Vietnamese/English/exclusion | ◐ UsIM/exclusion; English toggle global | ◐ Off mode | ◐ presets/exclusion | — | — | ◐ global toggle, not per-app | ● `app_modes`, `excluded_apps` |
| Per-app delays / ACK | — | ● commit/backspace/post + echo state machine | — | — | — | — | ◐ profile hints only |
| Mode menu / cycle hotkey | ● Shift+~ + lookup table | ● grave/cycle + per-mode keys | ● Shift+~ | — | — | — | — |
| App presets | — | ◐ user rules, no built-in | ● browser/editor/chat/terminal | — | — | ● browser/PWA auto-list | ◐ small default map |
| Password/field-kind handling | — | ● `CapabilityFlag::Password` | — | ◐ fcitx/IBus may handle | ◐ core may handle | — | ● `FieldKind` |
| Selection / anchor awareness | ◐ old version ignores anchor; fork fixes | ● `isAutofillCertain`, anchor/cursor | ● selection-aware word extraction | — | ◐ checks selected text empty | — | — |
| Focus/lifecycle commit | ◐ | ● focus bounce/deferral/cancel | ◐ | ◐ reset only | ◐ reset only | ◐ reset clears preedit | ● actions, adapter decides |
| Mouse/cursor reset | — (focus events only) | ● libinput mouse socket | ◐ XRecord code present, partly unwired | — | — | — | ● `cursor_moved` adapter port |
| Desktop GUI | ● GTK3 | ● PySide6/Qt | ● GTK3 + CLI | ● GTK3 setup | ● fcitx config + Qt editors | ● GTK4 setup | — (web playground/CLI only) |
| CLI / doctor | — | — | ● `bamboo-viet-config` | — | — | ● `telebit doctor` | ◐ CLI demo, no doctor |
| Tests / golden corpus | ● Go tests | ● 16 unit groups + ~30 integration + 29k golden lines | ● unit + 1,476 fuzz scenarios | — | ● 1,721 Telex pairs + surrounding tests | ● 766 core + CLI/GUI tests | ● 442 Python, 13k/3k conformance, Rust |
| FFI / WASM | — | — | ● C ABI (`libvicore`) | — | — | — | ● C ABI + WASM + Python FFI |

### 2.3 Defaults that cause surprise

| Default | Bamboo | Lotus | Bamboo Viet | ibus-unikey | fcitx5-unikey | Telebit | ViFastKey |
|---|---|---|---|---|---|---|---|
| Start mode | Preedit | Preedit | Preedit | Preedit | Preedit | Direct commit + rollback | Adapter-chosen |
| Tone style | Old (standard) | **New** | Old | Old | Old | Old | Old |
| `w` at word start | Default Telex: w; **Telex W/Telex 2: ư** | **NonStart: stays w** | inherits | ư (option true) | ư (option true) | **w** (needs a vowel context) | ư |
| Auto-cap sentence | — | **Off** | — | — | — | **On** | **Off** |
| Macro enabled | Off | On | Off | Off | On | Empty table | On (empty) |
| Spell dictionary | Off | Off | Off | Off | On (rules) | Syllable gate | On (small dict) |
| Preedit underline | Hidden by default | No underline | Hidden | Underline | Underline | Underline | Adapter-chosen |

These defaults explain many “why does it behave differently?” reports. ViFastKey should publish
its defaults explicitly and provide one-command migration examples from UniKey/Lotus.

### 2.4 What each project uniquely contributes

- **Bamboo**: the 7-mode per-app lookup table (`Shift+~`, including “exclude this app”), a
  user-editable macro file (`key:value`, `#`/`;` comments, hot reload every 3 s), emoji search by
  ASCII/name aliases, and Unicode hex input. Its output-mode taxonomy is still the clearest
  explanation of *why* Linux needs several backends.
- **Lotus**: the most complete Wayland-safe replacement backend (uinput server + anchor/ACK +
  per-app delays), a real app-rule schema with three delay overrides, emoji history/search,
  custom dictionary/keymap editors, and integration tests for nearly every race. Also the best
  demonstration that “mode” must be per app, not global.
- **Bamboo Viet**: built-in app presets with user-override priority, a CLI control panel, a
  C ABI usable from C++/Rust, and a candid `docs/known-issues.md` bug matrix (even where the
  implementation lags the README).
- **ibus-unikey**: the minimal preedit-only baseline; GSettings schema; UniKey macro format
  (version header, `key:text`, limits); Shift+Space and double-Shift restore-key-strokes
  semantics.
- **fcitx5-unikey**: preedit styles, stable-consonant auto-commit, surrounding-text rebuild and
  the experimental modify-previous-word path; Shift+Space and double-Shift restore-key-strokes;
  a large Telex conformance table and realistic fcitx5 test-frontend harness.
- **Telebit**: direct-commit rollback as a first-class mode, browser/PWA force-preedit with
  app auto-registration, MIT-licensed escape rules for English, a CLI doctor, and an AI prompt
  flow with careful async/cancellation/privacy design.
- **ViFastKey**: the cleanest pure engine/action split, 13k+3k cross-language conformance,
  Simple Telex/quick Telex/coda k/macro digits/smart caps/soft reset/NFC–NFD already present.
  The missing piece is the host side, not the syllable model.

---

## 3. Adapter and output-mode design patterns

### 3.1 Preedit: the default, but lifecycle details decide quality

All six IMEs expose preedit; Bamboo, Lotus, ibus-unikey and fcitx5-unikey default to it, while
Telebit defaults to direct-commit rollback and falls back to preedit. The important differences
are not the widget but the **commit ordering and reset semantics**:

- **Commit before hiding** (Bamboo Viet's fix for the chat duplicate-word bug): on Enter/word
  break, send `CommitText(text)` first, then hide/clear preedit. The original Bamboo only does
  this when `IBworkaroundForFBMessenger` is set or the key is a word-break rune; otherwise it
  hides first and commits second. In apps whose editor commits an empty-preedit event into its
  own buffer, that order duplicates the last word. The fork made commit-before-hide the default.
- **Preedit lives on the client when supported**: Lotus and fcitx5-unikey call
  `setClientPreedit` when `CapabilityFlag::Preedit` is present, otherwise `setPreedit` on the
  input panel. This matters for inline composition in Qt/GTK/Electron.
- **Backspace inside preedit edits the composition, not the document**: Lotus's test
  `preedit-backspace.cpp` asserts that `lo` + Backspace yields preedit `l` with no commit, and a
  second Backspace ends composition with no commit. The engine must receive the keysym
  `FcitxKey_BackSpace`, not Unicode `0x08`; otherwise the engine treats it as an unknown key
  and commits the whole preedit.
- **Focus-out commits once**: `preedit-lifecycle.cpp` requires an active composition to be
  committed exactly once on focus loss/deactivate, and the client preedit buffer to be clear on
  focus regain. A reset that just drops the buffer loses text.
- **Client reset after Enter is not a context switch**: GTK frontends send a reset when Enter
  inserts a newline. Lotus keeps the sentence-capitalization state across that reset and clears
  it only on real focus-out (`auto-capitalize-reset.cpp`). ViFastKey's adapter should distinguish
  `InputContextReset` from `FocusOut`.
- **Preedit styles**: fcitx5-unikey exposes `None / Underline / Highlight`; Highlight sets the
  cursor to 0 instead of the end. ViFastKey's `Preedit` action only has text + cursor at end;
  a style hint would be a useful extension.
- **Stable-prefix preedit**: fcitx5-unikey auto-commits leading consonants when macro is off and
  surrounding modification is off, so preedit starts at the first vowel. This reduces flicker
  and the amount of text that must be rewritten. It is a UX optimization, not a correctness
  requirement; ViFastKey can defer it.
- **Escape policy is not universal**: ViFastKey restores the raw keystrokes on Esc; Telebit
  clears the buffer without committing; fcitx5-unikey treats Esc as an ignored key and commits
  the current preedit. Pick one behavior per platform, document it, and test against the app’s
  own Esc handling (dialogs/terminal).

### 3.2 Surrounding text: read, validate, diff, delete, commit

The robust shape is:

1. Adapter supplies **text before cursor**, cursor/anchor, selection, and validity.
2. Engine reconstructs or adopts the last word only if the text is trustworthy.
3. Engine computes `deleted`/`added` (longest common prefix).
4. Adapter deletes exactly `deleted` characters and commits `added`.
5. Never mix synchronous deletes and commits without respecting the app's event loop.

Concrete strategies seen:

- **Bamboo SurroundingText mode**: on `SetSurroundingText`, rebuild the engine state from the
  characters before the cursor in reverse order using English/full-text mode, replacing
  punctuation with spaces for spell-check purposes. On each key, compute the new word and send
  `DeleteSurroundingText` + commit. The old version ignores `anchor`; the ngkhhuy fork fixes it
  by taking `min(cursor, anchor)` and only extracting the word before the selection, so browser
  autocomplete suffixes are not included.
- **fcitx5-unikey `rebuildFromSurroundingText`**: only when at word beginning, last character is
  an auto-committable consonant (not a digit), the preceding text is not a larger Vietnamese
  word, and the word is ≤ 7 characters. It rebuilds `uic_` with English mode and sets
  `autoCommit_`. This lets the user continue a word after a focus change.
- **fcitx5-unikey `rebuildPreedit` / `ModifySurroundingText`**: if enabled and UTF-8, at word
  beginning with valid surrounding text and no selection, take the last ≤ 7 Vietnamese
  characters, rebuild them as preedit, `deleteSurroundingText(-length, length)`, and show
  preedit. The user can then add a tone to the previous word (`nga` + `r → ngả`). It is
  experimental because it rewrites the previous word on every key and conflicts with
  auto-commit.
- **Lotus `rebuildFromText`**: rebuild the internal composition from a Vietnamese text, then
  process the new key and diff the new word. This is cleaner than fcitx5-unikey's repeated
  delete/preedit dance, and is the model ViFastKey's `rebuild()` already resembles.
- **Telebit direct rollback**: no read-back at all. It keeps the raw ASCII buffer and the last
  displayed text, computes the common prefix in bytes (backing up to UTF-8 boundaries), deletes
  the differing suffix with `deleteSurroundingText`, and commits the new suffix. This is simple
  and fast, but has no ACK, so fast typing can race the app. It falls back to preedit when the
  client lacks SurroundingText, when `program()` is empty (raw Wayland text-input-v3), or when
  the app is in the force-preedit list.

**Guardrails that all implementations need:**

- Do not edit surrounding text when the selection is non-empty (autocomplete, browser address
  bar, AI ghost text), unless the strategy is explicitly select-then-delete.
- Do not trust surrounding text that is shorter than the currently displayed word (Firefox's
  async updates); reject and start a new word or fall back to preedit.
- Bound the scan (Lotus uses 15 code points; fcitx5-unikey uses 7 characters; ViFastKey uses
  `MAX_RAW // 2` for adoption). Do not decode an entire document per key.
- Normalize/compare in the same form as output. ViFastKey already has the important guard:
  only adopt a previous word when `self._out(on_screen) == on_screen`; otherwise NFD/NFD
  mismatches delete the wrong number of characters.
- Never delete more than the app actually has before the cursor; clamp to the validated prefix.

### 3.3 Synthetic-key injection: four distinct mechanisms

| Mechanism | How it works | Works on | Main risks | Seen in |
|---|---|---|---|---|
| IBus `ForwardKeyEvent` | IBus forwards a synthetic key event through the client's IBus connection | X11 and some Wayland IBus clients | App may ignore synthetic/Unicode keys; ordering vs commits; no root | Bamboo `BackspaceForwardingIM`, `ShiftLeftForwardingIM`, `ForwardAsCommitIM` |
| XTest | X server injects key events | X11/XWayland only | No native Wayland; focus/race; global side effects; needs X11 | Bamboo `XTestFakeKeyEventIM` |
| uinput | Kernel virtual keyboard; compositor sees normal key events | Linux kernel, X11 + Wayland | Privileges/udev/systemd; device identity; pacing; cleanup; security | Lotus Smooth/SuperSmooth/Select |
| fcitx5 edit API | `deleteSurroundingText` + `commitString` (not synthetic keys) | Apps exposing surrounding text | Apps that lie about support; no echo/ACK; browser autocomplete | Lotus SurroundingText, fcitx5-unikey, Telebit |

Bamboo's `ForwardAsCommit` is a fourth variant: it sends each replacement character as a
synthetic key event rather than a commit. It can keep terminals happy where preedit is ignored,
but it is slow and depends on the app accepting synthetic/Unicode keys.

**Pattern: choose per app, not globally.** Bamboo's mode menu stores a mode per `WM_CLASS`; Lotus
stores a mode + three delays per app; Telebit force-preedit is a per-app list; ViFastKey has
`app_profiles`/`excluded_apps` but no enforcement yet.

**Pattern: never block the key handler.** Bamboo sleeps 10–50 ms in `SendBackSpace` and queues
keys in a channel; this causes dropped/mixed keys under fast typing. Lotus moves the burst to a
separate server process and uses ACK/anchor plus a bounded key buffer. Telebit's direct rollback
is synchronous but simple. ViFastKey should make injection asynchronous/queued at the adapter,
not in the engine.

### 3.4 Lotus uinput server: the most complete local injection protocol

This is the best reference for a Wayland-capable replace backend, independent of GPL code.

**Process separation**

- `fcitx5-lotus-server` runs as a dedicated user (or the target user with `-u`), not inside the
  fcitx5 process. A udev rule grants `rw` on `/dev/uinput` to that proxy user/group; systemd
  starts one instance per user.
- The addon connects to **abstract Unix sockets** named
  `lotussocket-<namespace>-kbSocket` and `-mouseSocket`. The namespace defaults to the user name
  and can be overridden by `LOTUS_SOCKET_NAMESPACE` for tests.
- The server authorizes peers with `SO_PEERCRED` UID plus `/proc/<pid>/exe` allowlist; it rejects
  other users/executables. It boosts priority and pins to performance cores.
- The virtual keyboard exposes exactly `KEY_BACKSPACE`, `KEY_LEFT`, `KEY_LEFTSHIFT`,
  `KEY_DELETE`, `KEY_PAUSE`. `KEY_PAUSE` is the **anchor**: a real key event ordered after the
  burst that no app binds; the addon swallows it in the normal path, and a stray anchor is a
  no-op. This solves "how do I know the burst is done?" without guessing delays.
- The mouse socket uses libinput pointer-button events to notify the addon that the user clicked,
  so stale composition can be committed/reset. A separate physical-Shift tracker avoids
  selecting with a synthetic Shift while the user is holding Shift.

**Wire protocol (`KbMsg`)**

```
op          Backspace | Select | Cancel
count       characters to delete/select (autofill included)
preDelay    ms before any key (commit cooldown since last commit)
interval    ms between injected keys (backspace pacing)
postDelay   ms before the final trigger key (anchor/Delete)
```

- **Backspace**: send the first Backspace immediately, queue `count-1` at `interval`, then wait
  `postDelay` and send `KEY_PAUSE`. Intermediate backspaces are allowed to reach the app so it
  can clear autocomplete/old text.
- **Select**: press Shift, send `count` Left arrows at `interval` (so the app extends the
  selection), release Shift, send Delete, then `postDelay` + Pause. Used for Chromium address
  bars and Select mode. If the physical Shift is held or unknown, fall back to Backspace.
- **Cancel**: finish an in-flight selection (complete Lefts, release Shift, Delete) so the old
  text is fully removed, then let the addon stash the replacement text for a later commit.

**Why the anchor matters:** the addon counts echoed Backspace/Left events; if some are lost, the
Pause still closes the burst. `select-anchor-completion.cpp` verifies a missing echo still
commits on the anchor and a stray anchor does not commit twice.

**Pacing/ack state machine:**

- `expectedBackspaces = deletedPart.size()`, plus one extra in non-SuperSmooth if autofill is
  detected.
- Every echo increments `currentBackspaceCount`; the last one (or the Pause) finishes.
- `replacementDeadlineMs` = preDelay + count×interval + postDelay + 200 ms slack; each echo
  re-arms the grace. On timeout, cancel the burst, commit pending text once, replay buffered
  keys.
- Keys typed during deletion are buffered (cap 50, drop overflow) and replayed after commit.
  In Select mode, synthetic Shift is stripped at replay time unless physical Shift is still
  held. This is a subtle but excellent test case (`select-modifier-replay.cpp`).
- Focus-out during a replacement cancels it, stashes the replacement text, and commits it on
  refocus if within 2 s; longer means a real app switch, so the stash is dropped.
- DBus frontends receive the commit via a zero-delay event-loop timer, not inside the consumed
  key event; text-input-v3 clients get one merged commit for the replacement + buffered keys.

### 3.5 Telebit direct-commit rollback: the simplest no-preedit path

- Keeps `rollbackRawAscii` and `rollbackDisplay`.
- On each letter, append raw, convert, compute common UTF-8 prefix, delete the differing suffix,
  commit the new suffix. The original key is swallowed (`filterAndAccept`).
- On Backspace, pop raw, recompute display, delete the differing suffix, commit the new suffix;
  if raw is empty, let the app handle Backspace.
- On a non-letter word boundary (space/Enter/Tab/punctuation/digit), expand a macro if the raw
  buffer matches, clear, and **do not consume** the boundary key; the app inserts it. This is why
  Enter sends immediately and does not duplicate the last word.
- Fallback to preedit when:
  - the client lacks `CapabilityFlag::SurroundingText`;
  - `ic->program()` is empty (raw Wayland text-input-v3: browsers/Electron arrive without an
    app id);
  - the program is in the force-preedit list (`firefox`, `chrome`, `chromium`, …);
  - the program is a PWA/app-mode browser window (`ffpwa-`, `chrome-`, `msedge-`, `brave-`,
    `vivaldi-`, `crx_` prefixes).
- It does **not** verify that `deleteSurroundingText` is reliable; LibreOffice/old Electron apps
  may still break. That is why ViFastKey needs app profiles for reliability, not only capability
  flags.

### 3.6 Focus, lifecycle and cross-context state

Recommended adapter rules, synthesized from Lotus/Telebit/Bamboo tests:

1. **Focus-in**: clear preedit only if this is a real context change; keep the word across a
   focus bounce (focus-out → focus-in within ~100 ms). Brave/Chromium on Wayland can report a
   bounce on every key.
2. **Focus-out**: commit the preedit exactly once, or cancel an in-flight replacement and stash
   the replacement text. Do not leave a “deleting” state that swallows all future keys.
3. **Client reset** (GTK Enter/newline): flush the word but preserve sentence-capitalization
   state; it is not a real context switch.
4. **Reset while deleting**: ignore/reschedule. Reset should not drop the remaining burst before
   the anchor; if it must, cancel and reconcile.
5. **Stale abort**: if an aborted replacement returns after ~2 s, drop its stash; it was a real
   app switch.
6. **Cross-context isolation**: all replacement/deletion/echo state must be per input context.
   Lotus uses some process-global atomics (`g_isDeleting`, `g_realTextLen`), which is an
   architectural risk; its focus-bounce test checks isolation despite that. ViFastKey should
   keep all of this inside `ImeSession`.
7. **Cursor movement/click**: `cursor_moved` should flush or commit the current word and clear
   soft-reset state. X11 can observe mouse clicks via XRecord; Wayland can use libinput or the
   compositor's input-context focus events.

### 3.7 App identity, presets and matching

- **Bamboo (original)**:
  - X11: `WM_CLASS` of the focused window, `instance:class`.
  - Wayland/GNOME: `org.gnome.Shell.Eval` (often disabled in modern GNOME) or
    `wlr-foreign-toplevel` (wlroots only, not Mutter).
  - Mode mapping is exact `WM_CLASS`; per-app modes persist in JSON.
- **Bamboo Viet**:
  - `ResolvePresetInputMode` tries exact lower-case, then splits on `: . space`, then substring.
  - Priority: user `InputModeMapping` > built-in preset > global default. Good policy.
  - Substring fallback iterates an unordered map; overlapping keys can be nondeterministic.
- **Lotus**:
  - `ic->program()`; if empty/`wayland`/`x11`, synthesize `ctx_<pointer>` so each window has a
    separate rule slot. App rules can be keyed by real program or context pointer; `ctx_` rules
    are not persisted.
  - App rule file `lotus-app-rules.conf`: `app=mode,commit_ms,backspace_ms,post_ms`, comments
    with `#`, omitted fields = defaults, malformed lines skipped, legacy mode 2 = Smooth.
  - UI auto-scans `.desktop` files for icons/labels, but matching remains `program`.
- **Telebit**:
  - On focus-in, auto-registers every program in the force-preedit config list; known
    browser/PWA names are enabled by default, others disabled. Labels are backfilled from
    `.desktop` `Name=` (exact `Name`, not localized), so the list is human-readable.
  - Empty program => safe preedit. This is the correct default for raw Wayland text-input.
  - PWA prefixes are a prefix test, not a fixed list.
- **ViFastKey**:
  - `resolve_profile` strips path/`.exe`/`.app` and lowercases, then exact lookup. It does not
    split `WM_CLASS` `instance:class`, does not strip `.desktop`, does not handle PWA prefixes,
    and its default map lacks common Linux names (`google-chrome`, `microsoft-edge`,
    `brave-browser`, `soffice.bin`, `gnome-terminal-server`, `zalo`, `telegram-desktop`, etc.).
  - `excluded_apps` and `app_modes` use the same raw string; exact-match only.
  - `AppProfile.output_mode` is returned by `vfk_app_profile_json` but the session mode is fixed
    at `vfk_session_new`; the engine never switches modes based on it.

**Recommended normalization for ViFastKey:**

```
raw app id -> lowercase
           -> take basename after / or \
           -> strip .exe/.app/.desktop
           -> split "instance:class" and test both parts
           -> if it starts with ffpwa-/chrome-/msedge-/brave-/vivaldi-/crx_ => browser/PWA
           -> aliases: google-chrome-stable/google-chrome/chrome, msedge/microsoft-edge,
              code/code-url-handler, soffice.bin/libreoffice, gnome-terminal-server/gnome-terminal
           -> exact user override wins over alias; alias wins over built-in default
```

**App-workaround matrix** (what the adapter should try first):

| App class | Typical symptom | Best first workaround | Secondary / notes | Source |
|---|---|---|---|---|
| Chromium/Chrome/Edge/Brave address bar | Inline autocomplete selection; first Backspace only collapses it; dropped/duplicated chars | **Preedit**, or Select-then-Delete (`Shift+Left × N`, Delete, insert) | Extra autofill backspace in non-SuperSmooth; detect `CapabilityFlag::Url`; preedit is safest | Bamboo `shouldAppendDeadKey`; Lotus `chromium-address-bar.cpp`; Telebit browser force-preedit |
| Chromium/Electron content areas (Chrome, VSCode, Slack, Discord, Zalo) | Backspaces/commits race; dropped first char; wrong tone | Surrounding text + **delay/ACK**; per-app pacing | If surrounding text unreliable, force preedit; do not use XTest | Lotus Smooth/Select; Bamboo Viet presets; ViFastKey `code`/`slack` profiles |
| Firefox / LibreWolf address/search | Stale or asynchronous surrounding text; autocomplete | Preedit; **disable previous-word adopt** | Reject surrounding text when shorter than displayed word or prefix mismatches | fcitx5-unikey `surroundingText`; ViFastKey Firefox profile |
| LibreOffice Writer/Calc | Cursor jumps, lost text with `deleteSurroundingText` timing | `deleteSurroundingText` + commit (Lotus issue #162 path), preedit fallback | Avoid uinput backspace burst; no blocking sleeps | Lotus `isLibreoffice_`; fcitx5-unikey `soffice.bin` preset |
| Terminals (kitty, alacritty, wezterm, gnome-terminal, foot, ghostty) | Preedit ignored, cursor weirdness, raw mode eats Enter | **Preedit/atomic commit** or forward-as-commit; no previous-word edit | Different terminals need different presets; test vim/Claude Code | Bamboo `ForwardAsCommit`; Bamboo Viet presets; ViFastKey `FieldKind.TERMINAL` |
| Facebook Messenger, Slack, Telegram, Zalo | Duplicate last word when Enter/Space hides preedit | **Commit before hiding preedit**; then forward Enter | Test exactly-once commit; Telebit Enter path is a good model | Bamboo `IBworkaroundForFBMessenger`; Bamboo Viet BUG-01 |
| WPS Office | Inline preedit renders badly | Show composition in **auxiliary text** | Only for `wpsoffice:wpsoffice`; optional workaround | Bamboo `enabledAuxiliaryTextList` |
| GNOME Wayland (Mutter) | Empty app id; app rules don’t match; IBus/text-input-v3 double buffering; focus bounce on every key; uinput echo lost | Unknown app → **preedit**; merge replacement + buffered keys into one commit; defer commit out of key event; tolerate 100 ms bounce | `org.gnome.Shell.Eval` often disabled; extension/kdotool fragile; fcitx5 `ibus` frontend | Lotus dbus/merged commit/focus-bounce tests; Telebit empty-program fallback |
| KDE Wayland | Generally fcitx5-native; panel icon theme mismatch; app id usually present | Native fcitx5 preedit/surrounding; uinput fallback only if needed | Panel theme detection is cosmetic | Lotus `isKdePlasmaSession`/`isDarkMode`; fcitx5-unikey |
| Flatpak/Snap apps | IME not reachable or GTK/Qt module missing | Ensure fcitx5 portal frontend + host/sandbox modules; use `vfk doctor`-style diagnostics | Snap runtimes often ship fcitx4 GTK module; override `GTK_IM_MODULE=fcitx` may be needed | Telebit `doctor` |
| Games / raw-input apps | Keystrokes swallowed or camera moves | **Exclude/off** by app ID; pass everything | `minecraft` is already a ViFastKey profile | Lotus Off mode; Bamboo exclusion; ViFastKey `excluded_apps` |
| Password / number / email / URL fields | Preedit leaks secrets or blocks input | **Disable** IME; no logging of raw keys | URL can be a search box; consider per-field policy rather than blanket URL off | Lotus password capability; ViFastKey `FieldKind` |
| Numpad / shortcuts | Numpad digits treated as VNI modifiers; Ctrl/Alt shortcuts typed as letters | Pass numpad and any Ctrl/Alt/Meta/Super chord through; normalize keysyms | Bamboo issue #561; Lotus `handleAppModeMenu` reserves shortcuts | Bamboo `isValidState`; Lotus `handleKeyEventGuards`; ViFastKey modifier handling |

### 3.8 Delays, ACK and buffering

- **Lotus** exposes global defaults and per-app overrides:
  - `DefaultCommitInterval` = 20 ms, cooldown since the last commit before starting a new
    replacement;
  - `DefaultBackspaceInterval` = 5 ms, between injected keys;
  - `DefaultPostDelay` = 10 ms, before the final trigger key;
  - clamp 1–1000 ms; 0 in a per-app rule means “use global”.
- **Bamboo** hardcodes 10–50 ms sleeps and adds 20–30 ms per key. This is fragile and blocks the
  engine thread; it tries to compensate with a 100-entry key channel.
- **Telebit** has no ACK or delay; fast typing can race `deleteSurroundingText`.
- **fcitx5-unikey** uses synchronous `deleteSurroundingText`, no delay.
- **ViFastKey** has `AppProfile.backspace_delay_ms`, `delay_per_char_ms`, `wait_ack`, but these
  are only hints in `app_profile_json`; `Replace` has no timing/ACK fields and the session never
  uses them.

**Recommended `Replace` metadata** (or a separate adapter policy object):

```
delete_before, insert, pass_key,
policy: { pre_delay_ms, interval_ms, post_delay_ms, wait_ack, max_retries,
          mode: surrounding | backspace | select | forward }
```

This lets the adapter implement Lotus-style pacing without polluting the engine, while still
making the intent auditable and testable.

### 3.9 Mode switching and config UX

- **Bamboo**: `Shift+~` opens a lookup table with 7 modes; the same shortcut twice types `~`.
  Mode is persisted per `WM_CLASS`; selecting “Add to exclusion list” disables Vietnamese in that
  app. A dedicated shortcut editor exists.
- **Lotus**: `grave` opens the app-mode menu; optional cycle hotkey; per-mode shortcut keys
  (`1`, `a`, `2`, `4`, `q`, `w`, `e`, `r` by default); mode order configurable; the menu includes
  a “Type [hotkey]” item so a mode hotkey that is also a printable character can still be typed.
  Mode change shows an 800 ms notification and closes on mouse click.
- **Telebit**: one global `Ctrl+Shift+Z` Vietnamese toggle; no per-app persistence; force-preedit
  is per app. Simpler but less capable.
- **ViFastKey**: `toggle_vietnamese()` flushes and remembers per app, but there is no mode menu,
  cycle hotkey, or UI. Adding a configurable toggle is cheap; a mode menu is a larger UI task.
- **Config GUI patterns**:
  - Lotus: General / Typing / Applications / Macros / Dictionary / Keymap / Shortcuts /
    Appearance / Backup / About; dynamic settings built from fcitx config metadata; app rule
    editor with search, icons, mode cards and delay spinboxes; macro/dictionary/keymap editors.
  - Bamboo Viet: UniKey-style GTK control panel + CLI `status`, `set-method`, `set-mode`,
    `list-presets`, `set-app`, `restart`.
  - Telebit: GTK4 setup with Setup/Apps/Macros/Status pages, plus `telebit doctor`.
  - fcitx5-unikey: fcitx config + separate Qt macro/keymap editors.
- **Storage locations observed**:
  - Bamboo: `~/.config/ibus-bamboo/ibus-bamboo.config.json` and
    `~/.config/ibus-bamboo/ibus-bamboo.macro.text`; the whole input-method-definition map is
    embedded in the JSON.
  - Lotus: `~/.config/fcitx5/conf/lotus.conf`, `lotus-app-rules.conf`,
    `lotus-macro-table.conf`, `lotus-custom-keymap.conf`, `lotus-emoji-history.conf`;
    dictionary `$XDG_DATA_HOME/fcitx5/lotus/vietnamese.cm.dict`.
  - fcitx5-unikey: `conf/unikey.conf`, `unikey/macro`, `unikey/keymap.txt` under the fcitx
    config/data roots.
  - ibus-unikey: GSettings schema `org.freedesktop.ibus.engine.unikey`.
  - Telebit: `~/.config/fcitx5/conf/telebit-fcitx5.conf`; API keys in environment; skills in
    `~/.config/telebit/skills/`.
  - ViFastKey: adapter-defined JSON; there is no installed path convention yet.

#### 3.9.1 Suggested ViFastKey config shape (v2)

Do not invent a new format for each adapter. A single versioned JSON (or TOML for hand editing)
with explicit enums is enough; adapters read it through the existing `Config`/FFI JSON. Sketch:

```json
{
  "config_version": 2,
  "input_method": "telex",
  "tone_style": "old",
  "spell_check": true,
  "w_as_u": "non_start",
  "bracket_keys": "disabled",
  "free_marking": true,
  "dd_free_style": true,
  "quick_telex": false,
  "allow_coda_k": false,
  "macro_enabled": true,
  "macro_digits": false,
  "macros": { "vn": "Việt Nam" },
  "auto_capitalize": false,
  "smart_caps": false,
  "unicode_form": "NFC",
  "output_charset": "Unicode",
  "app_rules": [
    { "app": "google-chrome", "mode": "preedit", "commit_ms": 20,
      "backspace_ms": 5, "post_ms": 10, "vietnamese": true }
  ],
  "excluded_apps": ["minecraft"],
  "app_modes": { "firefox": true }
}
```

Rules: unknown keys are ignored for forward compatibility; bools `w_as_u`/`bracket_keys` are
migrated to enums on load; `0` delays mean “use global”; app matching normalizes aliases; all
paths are UTF-8. This mirrors the useful parts of Lotus (app rules + delays) and Telebit
(auto-registered apps) without copying their config files.

### 3.10 Diagnostics and security

**Telebit doctor** is the best support pattern:

- Four sections: session/env, fcitx5, host apps, sandboxed apps.
- Checks fcitx5 process/version, addon path, active frontends (`native`, `portal`, `fcitx4`,
  `ibus`), GTK3/GTK4/Qt5/Qt6 client modules, GTK3 `immodules.cache` contents, Flatpak/Snap
  runtimes, and session IM env vars.
- Uses positive assertions only: “app X uses GTK4” is actionable; “no app uses GTK4” is not
  proven if some binaries could not be inspected. Exit codes distinguish broken vs warning.
- One `ldd` pass over all app binaries, capped list, says how many were unreadable.
- This is a good blueprint for a future `vfk doctor` on Linux (and macOS/Windows equivalents).

**Security/privacy lessons:**

- Bamboo logs every key value/rune and every commit to stderr/journal; `SetSurroundingText`
  prints the surrounding text. In a password field this can leak secrets. Avoid raw-key logging
  by default; redact field kinds.
- Lotus logs only via a disabled-by-default category, uses a separate least-privilege uinput
  process with peer auth, and notifies (rate-limited) when the server is missing.
- Telebit keeps AI keys in environment variables, not the plain-text fcitx config; AI debug
  logging is off unless an env var is set; clipboard reads run on a worker with a timeout to
  avoid deadlocking the app that owns the selection.
- ViFastKey should log actions/metadata, never raw keys or committed text, and should make
  password/number/email/URL field policy explicit in the adapter contract.

### 3.11 Adapter decision tree

A practical first Linux adapter policy:

```
if field is password/number/email (or URL policy says disable), or app is excluded:
    PassThrough / disable
elif app profile says OFF or game:
    PassThrough
elif terminal or profile.output_mode == preedit:
    PREEDIT; commit atomically before Enter; do not edit previous word
elif client has reliable SurroundingText and app is not in force-preedit:
    REPLACE via surrounding diff; use app delay/ACK profile
elif app is Chromium address bar / URL:
    PREEDIT, or Select-then-Delete (Shift+Left × N, Delete, insert)
elif client has Preedit:
    PREEDIT
elif X11:
    XTest/ForwardKeyEvent fallback
elif Wayland and uinput helper is available:
    uinput backspace/select fallback
else:
    disable/exclude, show a diagnostic
```

Do not make “replace” the global default on Wayland. Prefer preedit/surrounding; use uinput only
where the app cannot be edited reliably.

---

## 4. Concrete bugs and edge cases

### 4.1 Verified ViFastKey typing differences (differential probe)

The “current” column is ViFastKey's `transform(raw, final=True)` with default options and
`spell_check=True`; the “expected” column is Telebit's MIT core (unless noted) and matches
UniKey/fcitx5-unikey behavior. Session-level dictionary restore can rescue a subset; it cannot
fix the letter loss at the core.

| Input | Current ViFastKey | Expected | Why / source |
|---|---|---|---|
| `coffee` | `cofee` | `coffee` | `ff` is treated as a tone-undo; Telebit's escape gate declines because the whole word is not a syllable |
| `office` | `ofice` | `office` | same; session may restore if “office” is in the dictionary |
| `mission` | `mision` | `mission` | `ss` undo loses an `s` |
| `session` | `sesion` | `session` | same |
| `possible` | `posible` | `possible` | same |
| `daddy` | `dady` | `daddy` | second `d` triggers the `đ`/undo path |
| `necessary` | `nêcsary` | `necessary` | repeated `s` + invalid shape; core does not restore because `undone` is set |
| `voodoo` | `vodoo` | `voodoo` | repeated `o` / undo path |
| `mongoose` | `mongose` | `mongoose` | same |
| `mongoos` | `mongos` | `mongoos` | same |
| `database` | `datbase` | `database` | repeated `a`/hat interaction |
| `official` | `oficial` | `official` | repeated `f` |
| `different` | `diferent` | `different` | repeated `f` |
| `difficult` | `dificult` | `difficult` | repeated `f` |
| `cheese` | `chée` | `cheese` | `ee` is treated as a hat undo; status becomes INVALID but `undone` suppresses restore |
| `toolbox` | `tolbox` | `toolbox` | repeated `o` |
| `lessee` | `lesee` | `lesse` | trail-hat/undo escalation differs (minor, but pin it) |
| `lesseee` | `leseee` | `lessee` | same |
| `aw` (at word end) | `aw` | `ă` | standalone `ă` is PARTIAL, so final restore throws away the shape |
| `aws` (at word end) | `aws` | `ắ` | same |
| `aa` (at word end) | `aa` | `â` | standalone `â` is PARTIAL |
| `aas` (at word end) | `aas` | `ấ` | same |
| VNI `a8` | `a8` | `ă` | same, `a` + breve |
| VNI `a81` | `a81` | `ắ` | same |
| VNI `a85` | `a85` | `ặ` | same |
| `huwow` | `huwow` | `huơ` | `uwow` order not normalized to `uow` |
| `nguwow` | `nguwow` | `nguơ` | same |
| `law` | `law` | `law` | correct; this is why standalone `ă` cannot simply be added to OPEN_CLUSTERS |
| `mongo` | `mông` | `mông` | pass |
| `mongoo` | `mongo` | `mongo` | pass |
| `mongooo` | `mongoo` | `mongoo` | pass |
| `tepe` / `tepee` / `tepeee` | `têp` / `tepe` / `tepee` | same | pass |
| `data` (word end) | `data` | `data` | `check_final`: âm cuối tắc cần sắc/nặng |
| `mix` | `mĩ` | `mix` via Telex escape `mixx` | từ Việt hợp lệ luôn thắng |
| `boss` with bundled dictionary | `boss` | `boss` | pass; without dictionary `bos` (Telebit also `bos`) |
| `kiss` without dictionary | `kis` | `kis` in Telebit | known limitation, not a regression |
| `hello. aw␠` with auto-cap | `hello. Aw␠` | `hello. Ă␠` | auto-cap uppercases the raw `a`, but final restore turns `Ă` back to `Aw` |
| `awn␠` with auto-cap | `Ăn␠` | `Ăn␠` | pass; coda makes the syllable valid |

**Root causes in ViFastKey:**

1. `vi_syllable.OPEN_CLUSTERS` contains `a, e, ê, i, o, ô, ơ, u, ư, y` but not `ă, â`.
   A single shaped vowel is therefore `PARTIAL`; `_result()` restores raw on final when
   `status == PARTIAL and end > start`. Adding `ă/â` unconditionally would make `law → lă`,
   so the fix must be narrower: treat `ă/â` (and their toned forms) as valid **only when there
   is no onset and no coda**, or make `final` restore conditional on onset/coda.
2. `_step()` sets `st.undone = True` and `st.latched = True` for *any* repeated tone key, and
   `st.undone` for a repeated mark key. `_result()` then suppresses the raw restore even when
   the final status is `INVALID`. Telebit's rule is better: a doubled tone key is an escape only
   when (a) there is a vowel before it, (b) the prefix through the first key would convert, and
   (c) with spell-check restore on, the whole word would convert. Otherwise the letters are
   literal and the invalid word falls back to raw.
3. `uow`/`uwow` equivalence is only handled when a coda/glide follows; bare `huwow`/`nguwow`
   remain partial. Telebit normalizes the finished rime (`UQ` → `uQ`) regardless of key order.

**Test harness lesson:** Telebit's `tests.cpp` begins with `#undef NDEBUG` because Release
builds define `NDEBUG`, which compiles `assert()` away. ViFastKey's Rust tests are fine, but any
C/C++ adapter test using `assert` must do the same or use a test macro.

### 4.2 Concrete app/adapter test cases from the analyzed projects

These are behavior-level test cases worth porting to ViFastKey's fake host/adapter tests.

| Area | Scenario | Expected | Source |
|---|---|---|---|
| Chromium address bar | Surrounding `docs.google.com`, type `d`,`d` with inline autocomplete selection | Use Select `Shift+Left` path, exactly one commit `đ`; Delete echo reaches app; buffered key replay strips synthetic Shift | Lotus `chromium-address-bar.cpp` |
| Address bar selection | full text `https://vietnamnet.vn`, cursor 21, anchor 12 | Extract only the typed prefix `viet`, not the selected autocomplete suffix | Bamboo Viet `SetSurroundingText` tests |
| Firefox search | Surrounding text stale/shorter than displayed word | Do not adopt/delete; keep typed word and forward the new key | Lotus autofill heuristic, ViFastKey Firefox profile |
| LibreOffice | `nga` + `r` with surrounding text | `ngả`, no cursor jump; prefer `deleteSurroundingText` + commit, not uinput | Lotus `performReplacement` LibreOffice branch; fcitx5-unikey tests |
| WPS Office | Inline preedit renders badly or is ignored | Show the composing text in **auxiliary text** for `wpsoffice:wpsoffice` when the WPS workaround is enabled, not inline preedit | Bamboo `enabledAuxiliaryTextList` + `IBworkaroundForWPS` |
| Terminal | `vieejt` + Enter | Commit `việt` exactly once before Enter; no split delete/commit; no raw key injection | ViFastKey `FieldKind.TERMINAL`; Telebit warning; Bamboo forward-as-commit |
| Chat/Enter | `tiếng` + Enter | One `tiếng`, then Enter; never `tiếng tiếng` | Bamboo Viet BUG-01; Telebit `EngineVietCpp` Enter path |
| Facebook Messenger/WPS | `tiếng` + Space/Enter in preedit | Commit text before hiding preedit (FB Messenger flag/word break); WPS uses auxiliary text | Bamboo `commitPreeditAndResetForWBS`, `IBworkaroundForFBMessenger` |
| Focus bounce | Type `t`, bounce focus, `u`, bounce, `w` | Final `ư`; word survives each bounce | Lotus `focus-bounce-word-survival.cpp` |
| Focus-out mid-replacement | Trigger `ang` + `f` (3 backspaces), only one echo, lose/gain focus, wait 600 ms | Commit `àngq` once; queued `q` replayed; no stuck deleting state | Lotus `focus-out-mid-replacement.cpp` |
| Buffered key replay | `a`,`s` starts replacement; type `x` during burst | Replay produces commits `á`, `ã`; no immediate commit during deletion | Lotus `smooth-buffered-key-replay.cpp` |
| DBus deferred commit | Same replacement through `dbus` frontend | No commit inside the consumed anchor-backspace key event; one deferred commit after event loop | Lotus `dbus-deferred-commit.cpp` |
| Merged commit | `o`,`o` replacement; type `i` during deletion | One commit `ôi`; text-input-v3 double buffering cannot drop the first commit | Lotus `merged-replacement-commit.cpp` |
| Select cancel | `at` + `s`, focus out during Select | Send Cancel; no commit while unfocused; stashed `át` committed on refocus/Esc; stale >2 s dropped | Lotus `select-cancel*.cpp` |
| Select modifier replay | Select replacement, type `N` while synthetic Shift held; then with physical Shift held | Replayed `n` lowercase in first case, `N` uppercase in second | Lotus `select-modifier-replay.cpp` |
| Anchor completion | One Left echo lost, then Pause | Commit once on Pause; stray Pause does not commit again | Lotus `select-anchor-completion.cpp` |
| Auto-cap reset | `t`, Enter, client reset, `t` | Second `t` becomes `T`; focus-out clears it | Lotus `auto-capitalize-reset.cpp` |
| Auto-cap shortcuts | Ctrl+`.` then Space then `a` | `a` stays lowercase; Ctrl+`.` must not arm sentence state | Lotus `auto-capitalize-shortcut.cpp` |
| Numeric macro case | `123` + Tab, macro `123=Mixed Case`, auto-cap on | `Mixed Case`, not lowercased | Lotus `numeric-macro-case-preservation.cpp` |
| App rule reset | Focused app has rule `Off`, global `Preedit`, reload config | Rule still `Off`; `setEngine()` must not reset to global | Lotus `app-rule-reset.cpp` |
| App rule delays | Rule `test=1,0,20,25`, defaults 17/33 | Request interval 20, postDelay 25 | Lotus `app-rule-delay-wire.cpp` |
| Emoji history | 12 history lines | Keep newest 9, dedup first occurrence, malformed lines skipped, atomic save, failed write preserves old file | Lotus `emoji-history-unit.cpp` |
| Emoji search | exact/prefix/word-boundary/fuzzy | Tier order exact > prefix > boundary > fuzzy; case-insensitive; top-K deterministic | Lotus `emoji-search-history.cpp` |
| Emoji navigation | Up on first of 9 history page | Wrap to last on same page, not previous page | Lotus `emoji-history-page-navigation.cpp` |
| Key buffer | Push 60 keys, cap 50 | First 50 retained FIFO; overflow dropped; no crash | Lotus `key-buffer.cpp` |
| uinput device | Build virtual keyboard description | Exact keys `BACKSPACE, LEFT, LEFTSHIFT, DELETE, PAUSE`; no `/dev/uinput` needed for the test | Lotus `uinput-device.cpp` |
| Server socket | bind duplicate / listen invalid | `EADDRINUSE` / `EBADF` reported, never used deaf | Lotus `server-socket-init.cpp` |
| Preedit backspace | `lo` + Backspace ×2 | First removes one preedit rune, no commit; second empties, no commit; with empty preedit Backspace goes to app | Lotus `preedit-backspace.cpp` |
| Escape policy | `vieejt` + Esc in PREEDIT/REPLACE | Clear/replace composition and **do not forward Esc** (or make forwarding explicit); never both commit raw text and close the app dialog | ViFastKey current `_escape()` returns `pass_key=True`; compare Telebit/Unikey |
| Surrounding modifier | Ctrl_L press with selected surrounding text | No deletes/commits/forwarded; modifier alone must bypass surrounding processing | Lotus `modifier-key-bypasses-surrounding-text.cpp` |
| Simple Telex | `w`, `tw`, `tuw` | `w`, `tw`, `tư` | ViFastKey golden; fcitx5-unikey `UkSimpleTelex` |
| Unikey escapes | `aaa→aa`, `aww→aw`, `ddd→dd`, `eee→ee`, `ooo→oo`, `ww→w` | Escape table | fcitx5-unikey testunikey |
| Unikey VNI | `vie65t`, `viet65`, `tua6n`, `toi6`, `tu7o7ng`, `a8n`, `d9i`, `di9`, `nguye64n` | Correct Vietnamese | fcitx5-unikey testunikey |
| Unikey VNI literals | `toa10n→toan`, `ba10→ba10`, `a11→a1`, `nam2024`, `2024`, `box1` | Tone clear/literal number behavior | fcitx5-unikey testunikey |
| Surround rebuild | `q` + `u`,`e`,`r` + Enter | `uẻ` (preedit path), not a lost first key | fcitx5-unikey testBasic |
| Modify previous word | `nga` + `r`; `ngả nghieng` + `e`; cursor 3 + `s`; `application` + `s` | `ngả`; `nghiêng`; `ngá`; `s` | fcitx5-unikey testBasic |
| Macro longest prefix | `csao`, `csao2` | `csao2` does not expand early; Enter expands `✬` | Bamboo `engine_test.go` |
| Macro punctuation | `vn.`, `vn,` | `việt nam.`; `việt nam,` then space | Bamboo `engine_test.go` |
| Macro symbol key | `->` + `.` or Enter | `arrow.` / `arrow` | Bamboo `engine_test.go` |
| Spreadsheet numbers | `1`,`2`, Enter with macro off | Keys pass through, no preedit; no Vietnamese processing | Bamboo `workaround_spreadsheet_number_enter` |
| Ctrl+A in buffer | Ctrl+A after typed word | Commit/flush, no preedit; do not turn Ctrl+A into `a` input | Bamboo `control_a` tests |
| App preset priority | user mapping > built-in preset > global default | `code:Code` override, Chrome UsIM, Zalo preset, Kitty preset, unknown global | Bamboo Viet `TestGetInputModePriority` |
| Telebit escapes | `aaa→aa`, `dataa→data`, `mongoo→mongo`, `tepeee→tepee`, `coffee→coffee`, `daddy→daddy`, `huwow→huơ` | See §4.1 | Telebit `tests.cpp` |
| Telebit VNI | `ba1`, `cha2o`, `vie65t`, `toa10n`, `ba10`, `a11`, `person` | Correct VNI and literal behavior | Telebit `test_vni_mode` |
| Telebit auto-cap | sentence punctuation + Space/Enter | Uppercase ASCII first letter; shaped-vowel cases still need work | Telebit source + docs |

### 4.3 Bugs and anti-patterns in the analyzed projects

| Project | Issue | Impact | Lesson for ViFastKey |
|---|---|---|---|
| Bamboo | Logs every key value/rune and every committed text to stderr | Password/privacy leak; journal noise | Never log raw keys; redact field kinds |
| Bamboo | Blocking 10–50 ms sleeps in key path; key channel cap 100 | Fast typing drops/mixes keys; UI stalls | Async paced injection; bounded queue with explicit overflow policy |
| Bamboo | `SetSurroundingText` ignores `anchor` in original version | Browser autocomplete selection can be included/deleted | Pass cursor + anchor + selection to engine |
| Bamboo | Commit-before-hide only for FB Messenger/WBS by default | Chat apps can duplicate the last word on Enter | Make commit-before-hide the default for preedit word breaks |
| Bamboo | `Simple Telex` / `Unicode Tổ Hợp` advertised in fork GUI but absent from core | Selecting breaks input or silently falls back | Do not advertise unsupported config values; validate enums |
| Bamboo | Per-app mode exact `WM_CLASS`; GNOME Wayland app-id often empty | Mode mapping silently stops working | Normalize app IDs; unknown -> safe preedit |
| Lotus | `g_isDeleting`, `g_realTextLen` process-global across contexts | One context's replacement can block/affect another | Keep deletion/echo state per `ImeSession` |
| Lotus | `Smooth` vs `SuperSmooth` undocumented; only autofill extra-backspace differs | Users cannot choose correctly | Name/describe modes by behavior, not “smoothness” |
| Lotus | Mode-order GUI default uses `Uinput` token not in `kModeNames` | Reset order differs; token silently ignored | Validate config tokens against one enum source |
| Lotus | Macro keys stored verbatim but queried lowercased | Uppercase macro keys never match | Normalize both sides or enforce lowercase in editor |
| Lotus | `grave` default mode-menu hotkey consumes a normal character | Backtick requires the “Type” escape item | Mode hotkey must be escapable and documented |
| Bamboo Viet | fcitx5 addon ignores `bsCount`/replace; only commits preedit | “Fcitx5 support” is not parity; no Wayland replace | Treat adapter parity as a testable contract |
| Bamboo Viet | Built-in preset substring loop over unordered map | Overlapping names can resolve nondeterministically | Deterministic longest/priority match |
| Bamboo Viet | `simple_telex` GUI value unsupported by `bamboo-core` | Silent empty input method | Same enum/validation issue |
| ibus-unikey | No surrounding-text output, only preedit; no tests | Word editing/focus behavior fragile | Preedit-only is not enough for browsers/offices |
| fcitx5-unikey | `deactivate` commits only on IM switch, then `reset()` drops preedit | Possible loss on focus-out | Commit on every real focus-out; test |
| fcitx5-unikey | `ModifySurroundingText` rewrites previous word every key, UTF-8 only, ≤7 chars | Experimental/flicker; conflicts with auto-commit | Prefer one-shot adopt/rebuild; bound and guard |
| fcitx5-unikey | Turning macro on disables auto-commit of stable consonants | Full preedit flicker returns | Keep stable-prefix optimization independent of macro |
| fcitx5-unikey | Macro table 16-char key, 1024-char text, 1024 entries, 128 KB | Silent limits | Document limits; validate/import diagnostics |
| Telebit | `vietnameseEnabled_` is global, not per-app | Toggle in one app disables all; not persisted | ViFastKey's per-app `app_modes` is better |
| Telebit | Auto-cap only ASCII; shaped vowel starting a sentence may not capitalize | `ă/â/ê/ô/ơ/ư` cases | Capitalize raw key or final displayed letter, not just ASCII |
| Telebit | `reset()` clears preedit without commit | Possible focus-out loss | Commit preedit on focus-out; reset only on client reset |
| Telebit | Direct rollback has no ACK/delay | Fast typing races app; LibreOffice/Electron can break | Add app delay/ACK metadata |
| Telebit | Force-preedit list limited to browsers/PWAs | Old Electron/LibreOffice still direct-commit | Per-app output mode/reliability profile |

### 4.4 Test-harness patterns worth copying (behaviorally)

- **Lotus**:
  - One shared `TestInputContext` recording `commits`, `deletes`, `forwarded`, `preedit`,
    `surroundingText`, focus state.
  - A mock `KbSocketListener` that receives `KbMsg` and lets the test echo backspaces/lefts,
    drop echoes, or assert a Cancel. Tests are deterministic and headless.
  - CTest labels (`unit`, `integration`, `smooth`, `select`, `preedit`, `emoji`, …), per-test
    timeouts, and a `RESOURCE_LOCK` around the socket namespace.
  - Golden TSV corpora generated from the Go core, replayed 1:1, plus property/regression tests.
- **fcitx5-unikey**:
  - fcitx5 `testfrontend` + `pushCommitExpectation` + surrounding-text fixtures; tests both
    conversion and lifecycle in one process.
  - A 1,721-pair expected map is useful as a broad Telex oracle, but do not copy it wholesale
    from GPL code; generate independent cases or use a permissive oracle.
- **Telebit**:
  - Table-driven core tests with `TelexOptions` variants; `#undef NDEBUG`; explicit comments for
    each regression; MIT license allows reuse with attribution.
  - Doctor tests for probing/verdict logic: pure functions over captured command output.
- **Bamboo**:
  - Fake engine recording preedit/commit, and test cases expressed as key sequences with expected
    intermediate preedit; good for macro/word-break behavior.
- **ViFastKey** should add an adapter contract suite:
  - simulate preedit/replace/commit/OS echo; assert screen == `session.display`;
  - simulate focus bounce/out and ACK loss; assert exactly-once commit and no stuck state;
  - simulate app profiles and field kinds; assert actions/order;
  - test NFD/UTF-16 delete counts;
  - test no raw-key logging on password fields;
  - fuzz key sequences, selections, and focus transitions.

---

## 5. What ViFastKey currently lacks

### 5.1 Product/adapter gaps

- **No real Linux adapter.** No fcitx5/IBus addon, no uinput/XTest/surrounding-text bridge.
  The engine can return actions, but nothing applies them to a Linux input context.
- **No output-mode switching per app.** `AppProfile.output_mode` is only a hint; the session's
  `OutputMode` is fixed at creation. An adapter must destroy/recreate sessions or ignore the
  profile.
- **No ACK/delay metadata on `Replace`.** `backspace_delay_ms`, `delay_per_char_ms`, `wait_ack`
  exist in `AppProfile` but are not used by the session or action; an adapter has to infer.
- **No selection/anchor support.** `SurroundingText` exposes only `before_cursor(n)`, and
  `vfk_set_surrounding_text` takes only a string. Browser autocomplete/selection cannot be
  handled safely.
- **No select-then-delete action.** Chromium address-bar workaround needs select/delete/insert;
  ViFastKey only has `delete_before`.
- **No uinput / XTest / forward-key backend.** No Wayland replacement fallback for apps without
  surrounding text.
- **No libei/portal path.** None of the analyzed projects has one either; a future differentiator.
- **No mode menu/cycle hotkey**, no per-mode shortcut, no “type literal hotkey” escape.
- **No per-app rule file with delay overrides** (only in-memory/persisted config map).
- **App matching is too exact**: no `instance:class`, `.desktop`, PWA prefixes, Flatpak/Snap IDs,
  or common Linux aliases; defaults miss Chrome/Edge/Brave/LibreOffice/terminal variants.
- **No `FieldKind.TERMINAL` behavior**: setting it only flushes; the session still uses whatever
  mode it was created with.
- **No compositor/field auto-detection**: URL vs search box, password vs text, terminal vs
  editor is adapter responsibility without an explicit contract.
- **No full English/Vietnamese dictionaries**; the bundled lists are small and determine whether
  English restore works.
- **No large-scale app compatibility matrix** or diagnostics tool.

### 5.2 Engine gaps

- English cluster letter loss and `undone` over-suppression (§4.1).
- Standalone `ă/â` + tones revert at word end, including VNI; auto-cap produces `Aw`/`Aas`
  instead of `Ă/Ấ`.
- `uwow`/`huwow`/`nguwow` order not equivalent to `uow`.
- No `w_as_u` granularity (Disabled/NonStart/Everywhere).
- No bracket-key granularity (Disabled/NonStart/Everywhere).
- No `free_marking` strict toggle; no `dd_free_style` toggle.
- No legacy charsets.
- No emoji.
- No macro Tab trigger, `$TIME`/`$DATE`, macro-skip modifier, or file/editor/import-export.
- Preedit cannot edit a previous word (ROADMAP limitation); only REPLACE adopts.
- `Escape` currently restores raw text with `pass_key=True`; an adapter may commit the raw
  keystrokes **and** forward Esc (closing a dialog). Decide cancel-vs-restore and make `pass_key`
  false for the cancel path.
- No preedit styles/stable prefix.
- No double-space-to-period or double-hyphen-to-em-dash.
- No hex Unicode input.
- No `w` at word start option; current `w_as_u` means `w → ư` everywhere, unlike Lotus's
  default NonStart.
- `MAX_RAW` 24 and O(n²) transform are acceptable now; optimize after correctness.

### 5.3 UX/config gaps

- Desktop GUI ✅ `vifastkey-settings` (egui, Windows/Linux); fcitx config page chưa có.
- CLI `doctor/status/set-method/set-mode/set-app/list-presets` vẫn còn thiếu.
- Config `schema_version` ✅; migration tooling vẫn mỏng (đọc best-effort).
- Config backup/import/export ✅ (`vfk-config` + `vifastkey-settings`).
- App profile editor + delay/ACK/charset ✅ trong `vifastkey-settings`; chưa có search/icon.
- Macro editor ✅ (validate trùng/rỗng); dictionary/keymap editor chưa.
- Status icon/tray chưa có; mode toggle qua menu/hotkey.
- i18n: **không làm** theo quyết định sản phẩm; UI tiếng Việt.
- Fcitx5 addon build/runtime ✅; Flatpak/Snap packaging chưa.

---

## 6. Prioritized recommendations

Tags: `[E]` engine/core, `[A]` adapter/Linux integration, `[D]` docs/schema/tooling.
Priority means “do before calling Linux support usable,” not “large vs small.”

### P0 — correctness and data loss

1. **`[E]` Fix English cluster handling and doubled-key undo.**
   Implement Telebit-style gating:
   - a doubled tone key is an escape only if a vowel precedes it, the prefix through the first
     key would convert, and (with spell-check restore on) the whole word would convert;
   - `ww` remains an unconditional escape; `aaa/eee/ooo/ddd` collapse rules stay;
   - trailing `aa/ee/oo` hat escape only when the prefix already produced a hat and the double
     is trailing.
   Add tests for `coffee`, `office`, `mission`, `session`, `possible`, `daddy`, `necessary`,
   `voodoo`, `mongoose`, `mongoos`, `database`, `official`, `different`, `difficult`, `cheese`,
   `toolbox`, `toffee`, `dataa`, `mongoo`, `tepeee`, `lessee/lesseee`.
2. **`[E]` Fix standalone shaped vowels at word end.**
   Make `ă/â` + tones valid when there is no onset/coda (so `aw→ă`, `aws→ắ`, `aa→â`,
   `aas→ấ`, VNI `a8/a81/a85`), without making `law→lă`. Add auto-cap tests `hello. aw␠` and
   `hello. aas␠`.
3. **`[A]` Define and implement a Linux adapter contract before shipping “Linux support.”**
   Start with fcitx5: preedit + surrounding/replace, field kinds, focus/commit ordering,
   exactly-once commit, password/terminal policy. The engine is not a Linux IME by itself.
4. **`[A]` Make surrounding-text edits selection- and staleness-safe.**
   Extend the port with `{before, cursor, anchor, selected_text, valid, sequence}`; never delete
   when selection is non-empty or the prefix is shorter than the displayed word; normalize in
   the output form; clamp to the validated word.
5. **`[A]` Enforce app profiles and normalize Linux app IDs.**
   Normalize `WM_CLASS`/`instance:class`, `.desktop`, basename, PWA prefixes, Flatpak/Snap IDs;
   add Chrome/Edge/Brave/LibreOffice/terminal/Zalo/Telegram aliases; unknown/empty app =>
   safe preedit. Add per-app output mode, excluded/OFF, and Vi/En persistence tests.
6. **`[A]` Add ACK/delay policy to replacement.**
   At minimum expose `pre_delay`, `interval`, `post_delay`, `wait_ack` and a `mode`
   (surrounding/backspace/select) in the action or profile; add echo-count/anchor/timeout and
   buffered-key replay tests. Do not block the key handler.
7. **`[D/A]` Privacy/security defaults.**
   No raw-key/commit logging; disable password/number/email by default; make URL a deliberate
   per-field policy (it may be a search box). If adding uinput, use a separate least-privilege
   helper with peer auth; never run root in the addon.

Implementation map for P0 engine work:
`python/vfk_core/transform.py` (`_step`, `_result`) and `rust/crates/vfk-core/src/transform.rs`;
`python/vi_syllable/__init__.py` (`OPEN_CLUSTERS`, `check`) and
`rust/crates/vi-syllable/src/lib.rs`; adapter/profile work in `python/vfk_engine/ports.py` +
`session.py` and the Rust `ports.rs`/`session.rs`; ABI changes in `rust/crates/vfk-ffi` and
`rust/crates/vfk-ffi/include/vifastkey.h`.

### P1 — parity and usability

1. **`[E]` Add `w_as_u` modes** (`Disabled`, `NonStart`, `Everywhere`) and a migration path from
   the existing bool; default NonStart or Everywhere deliberately, document the difference.
2. **`[E]` Add bracket modes** (`Disabled`, `NonStart`, `Everywhere`) and keep the current bool
   as `Everywhere`.
3. **`[E]` Add `free_marking` and `dd_free_style` toggles** after defining their exact semantics
   against UniKey/Lotus; do not regress ViFastKey's always-free typing.
4. **`[E]` Fix Escape action semantics.** Choose one of: cancel composition without commit and
   consume Esc; or restore raw and consume Esc. Do not return `pass_key=True` for the normal
   cancel/restore path; add tests for PREEDIT/REPLACE and dialogs/terminals.
5. **`[E]` Legacy charsets.**
   Add TCVN3, VNI-Windows, VISCII, VIQR, VPS, BKHCM 1/2, NCR Decimal/Hex, Unicode combining,
   CP1258, Unicode C-string. Prefer tables from MIT `bamboo-core` with attribution; keep the
   internal representation Unicode and encode only at commit. Add byte-for-byte round-trip tests.
6. **`[E]` Macro v2.**
   File format with version header, UTF-8, comments, import/export, duplicate detection, length
   limits; Tab trigger; `$TIME`/`$DATE`; optional case preservation; macro-skip modifier; hot
   reload; per-app macro sets later.
7. **`[E/A]` Preedit quality.**
   Add preedit style hints (None/Underline/Highlight), cursor style, and optionally stable-prefix
   auto-commit for consonants; keep terminal atomic commit.
8. **`[A]` App-specific workarounds/presets.**
   Chromium address bar (preedit or select-delete), Firefox (no previous-word adopt), LibreOffice
   (delete-surrounding path), Electron/Chromium (delay/ACK), terminals (atomic preedit/forward),
   chat apps (commit-before-hide), games/exclusions, password/URL fields.
9. **`[E]` Emoji picker** with search tiers, history cap 9, 9-per-page navigation, atomic history
   file; or explicitly defer with docs.
10. **`[D]` Config schema v1.**
    Add a schema version, enum names, migration from bools, app-rule list, profile list,
    dictionary path, macro file path, and a generated JSON schema/example. Test unknown keys and
    forward compatibility.
11. **`[D/A]` Diagnostics CLI** (`vfk doctor`) modeled on Telebit: frontends, GTK/Qt modules,
    Flatpak/Snap, uinput permission, environment, app-specific hints; positive assertions only.
12. **`[E]` Full dictionaries.** Bundle/allow-load a permissively licensed Vietnamese syllable
    list (~7k) and English list (~20k); add `Dictionary` reload from path and tests for BOM/CRLF,
    lowercase, duplicates, and English/Vietnamese ambiguity (`this`, `as`, `data`).

### P2 — differentiators and long-term

1. **`[A]` uinput helper** for Wayland replace fallback, modeled on Lotus's protocol but
   independently implemented: separate process, udev ACL/systemd, abstract socket, peer auth,
   `Backspace | Select | Cancel`, pre/interval/post delays, Pause anchor, physical-Shift check,
   focus-out cancel/stash. Package it properly; make it optional.
2. **`[A]` libei/portal research spike.** No analyzed project implements it. Investigate
   `org.freedesktop.portal.RemoteDesktop`/libei for a future Wayland injection path without
   uinput privileges; document compositor support and consent UX.
3. **`[D/A]` Desktop config GUI** (fcitx5 config page first): General/Typing/Applications/
   Macros/Dictionary/Keymap/Shortcuts/Backup; app rule editor with mode cards and delay
   spinboxes; reuse the JSON schema.
4. **`[E]` Custom keymap/IME import/export.** `InputMethodSpec` is already data-driven; add
   validated JSON/TOML import, conflict detection, and a minimal editor. This matches Lotus's
   custom keymap and Bamboo's custom definitions.
5. **`[E]` Double-space-to-period and double-hyphen-to-em-dash** as optional engine actions
   (with preedit/replace/surrounding behavior), disabled by default.
6. **`[E]` Hex Unicode input** (`Ctrl+Shift+U`) as an optional action; not important for most
   users but cheap parity with Bamboo.
7. **`[D]` Docs set**:
   - `docs/linux-adapter.md`: action contract, decision tree, focus/ACK, delete units, app IDs;
   - `docs/compatibility.md`: app × backend × mode × known workaround × status;
   - `docs/config-schema.md` + migration guide;
   - `docs/troubleshooting.md`: Chromium/Firefox/LibreOffice/terminals/GNOME Wayland/Flatpak/
     uinput permission/duplicate commit;
   - `docs/security.md`: password/no logging/uinput/peer auth;
   - `docs/testing.md`: fake host/ACK/focus contract, golden/conformance policy.
8. **`[E]` Performance/quality:** ArrayVec/bitset/caching after correctness; differential fuzzing
   against a permissive oracle (Telebit MIT) for Telex/VNI; property tests for no lost text,
   exactly-once commit, delete counts, NFC/NFD, and session isolation.
9. **`[D]` Packaging:** fcitx5 addon package, config install path, optional uinput service/udev,
   Flatpak/Snap notes; document how to verify the addon and frontends.
10. **`[E/A]` Optional AI assistant?** Telebit's AI prompt/clipboard/skills/async/cancel design is
    interesting but not core IME. If pursued, keep keys in env, log nothing by default, cap
    clipboard, and isolate network work from the key path. Do not let it delay P0/P1.

---

## 7. Licensing constraints

**Hard rule: ViFastKey is MIT. Do not paste GPL code, GPL data, or GPL test tables into it.**
Behavior can be reimplemented clean-room; short factual outputs/test vectors are fine; wholesale
copies of source or large test files are not.

| Project | Code license | Data / test / third-party | What ViFastKey may reuse |
|---|---|---|---|
| **BambooEngine/ibus-bamboo** | GPL-3.0 | `bamboo-core` vendored: **MIT**; `data/vietnamese.cm.dict`: **GPLv3** (upstream GPL + CC-BY-SA); `emojione.json`: MIT; Go tests: GPL | No GPL code/data/tests. The MIT `bamboo-core` may be reused/vendored with its MIT notice; verify each table's provenance. |
| **LotusInputMethod/fcitx5-lotus** | GPL-3.0-or-later (even `lotus/`, though it carries `LICENSE.bamboo-core` MIT notice) | dictionary GPLv3; golden corpora GPL; settings GUI GPL | Behavior/test ideas only. Do not copy `lotus/`, server, GUI, tests, dictionary, or golden files. |
| **ngkhhuy/bamboo-viet** | GPL-3.0 | docs GPL; inherited Bamboo data | Behavior only. Its `docs/known-issues.md` is useful as a bug list; rewrite, don't paste. |
| **vn-input/ibus-unikey** | GPL-3.0 | `ukengine` **LGPL-2.1**; no tests | Behavior only. Linking LGPL `ukengine` into an MIT product creates LGPL obligations; avoid for the MIT core. |
| **fcitx/fcitx5-unikey** | GPL-2.0-or-later | `ukengine` LGPL-2.1; 1,721-pair test table GPL | Behavior only. Do not copy the test table wholesale; generate independent cases or use a permissive oracle. |
| **sonnam0904/telebit** | **MIT** | English wordlist copied from Debian `wamerican`/SCOWL, **test-only**, not shipped; Rime-derived table needs provenance check; AI code MIT | Code/data may be reused with the MIT copyright notice. If using the wordlist, include the SCOWL/Debian notice and check redistribution terms. If the rime table is derived from Rime data, add the upstream attribution (Rime is BSD-3). |
| **ViFastKey** | **MIT** | own data; fonts OFL; check any imported wordlists | Keep MIT; add `NOTICE`/data provenance files for every imported table. |

Additional rules:

1. **Clean-room behavior extraction.** Reading a GPL project to understand behavior and writing
   an independent implementation is acceptable; do not keep GPL code in the tree, even commented
   out or behind a feature flag.
2. **Dictionaries are the highest-risk data.** The Bamboo/Lotus `vietnamese.cm.dict` is GPLv3 and
   also derives from GPL/CC-BY-SA sources. Do not bundle it with MIT ViFastKey. Use a
   permissively licensed Vietnamese word list, build one from public-domain sources, or make it
   a separately licensed optional data package with correct notices.
3. **Test data.** Functional input/output pairs are generally facts, but a large curated test
   file can be a copyrightable compilation. Do not copy the fcitx5-unikey 1,721-pair map or
   Lotus golden TSVs; write independent cases from standards/behavior, or use MIT Telebit tests
   with attribution.
4. **Charsets.** The MIT `bamboo-core` charset tables are the safest source; otherwise use
   public Unicode/legacy mapping data and add attribution. Avoid copying GPL `ukengine` tables.
5. **Emoji.** Bamboo's EmojiOne JSON is MIT; Lotus reads fcitx5's emoji addon instead of shipping
   data. If ViFastKey adds emoji, use an MIT/Unicode-licensed source and include notices.
6. **AI.** Telebit's AI client is MIT, but API keys and prompt/context are sensitive. Keep keys
   out of config files, disable debug logging by default, and document that prompts/clipboard go
   to an external service.
7. **Linking.** Do not link ViFastKey to GPL `fcitx5-unikey`/`ibus-unikey` code. Writing an
   adapter against fcitx5/IBus public APIs is fine; the addon can be MIT if it does not include
   GPL code.
8. **Distribution.** If ViFastKey ever ships a GPL-derived optional module, keep it a separate
   process/package with clear licensing; do not relicense the MIT core.

---

## 8. Appendix: high-value test vectors to add

### 8.1 English / escape rules

```
coffee      -> coffee
office      -> office
mission     -> mission
session     -> session
possible    -> possible
daddy       -> daddy
necessary   -> necessary
voodoo      -> voodoo
mongoose    -> mongoose
mongoos     -> mongoos
database    -> database
official    -> official
different   -> different
difficult   -> difficult
cheese      -> cheese
toolbox     -> toolbox
toffee      -> toffee
dataa       -> data
mongoo      -> mongo
mongooo     -> mongoo
tepee       -> tepe
tepeee      -> tepee
lessee      -> lesse
lesseee     -> lessee
sofa        -> sòa
soffa       -> sofa
sofaa       -> sofaa
javaa       -> javaa
pandaa      -> pandaa
```

### 8.2 Shaped vowels / `w` / `uơ`

```
aw          -> ă
aws         -> ắ
aa          -> â
aas         -> ấ
awn         -> ăn
aan         -> ân
VNI a8      -> ă
VNI a81     -> ắ
VNI a85     -> ặ
uow         -> uơ
uowr        -> uở
uowi        -> ươi
nguowif     -> người
ruowuj      -> rượu
thuowngf    -> thường
huwow       -> huơ   (= huow)
nguwow      -> nguơ  (= nguow)
aw␠         -> ă␠   (must not revert to "aw ")
```

### 8.3 Adapter invariants

- Screen text after every key == `session.display` (PREEDIT and REPLACE).
- A word followed by Enter/Space commits exactly once; no duplicated word in chat apps.
- Focus-out commits once and clears preedit; focus-in has no ghost text.
- Focus bounce (<100 ms) keeps the word; real focus move (>100 ms) clears it.
- Replacement with lost echoes still commits once on the anchor/deadline; no stuck state.
- Keys typed during replacement are replayed in FIFO order; synthetic Shift is stripped unless
  physical Shift is held.
- Delete counts match the displayed text in NFC/NFD and UTF-16/UTF-8 adapters.
- Password/number/email fields (and URL when the field policy disables it) produce no
  preedit/commit and no raw-key logs.
- App profile switch changes mode/delays as expected; unknown app defaults to preedit.
- Every action is idempotent under repeated reset/focus events; no cross-session state leaks.

---

## 9. Final note

The Linux ecosystem has already solved most *policy* questions: preedit is the safe default,
surrounding text is preferred where reliable, per-app modes are necessary, and paced/ACK-based
injection is the only robust replacement path. ViFastKey's engine is in good shape; the next
large win is an explicit adapter contract plus a reference fcitx5 adapter, backed by the
behavior tests above. The two core fixes in §4.1 are small in code but high in user impact and
should not wait for the adapter.
