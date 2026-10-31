> **Báo cáo nghiên cứu nội bộ.** Tài liệu này chỉ ghi nhận hành vi, kiến trúc, UX và
> test fact của các dự án được khảo sát. Không có mã nguồn, bảng dữ liệu, từ điển hay test
> corpus GPL/AGPL nào được sao chép vào ViFastKey. Xem `THIRD_PARTY_NOTICES.md` và
> `docs/IME_FEATURE_RESEARCH.md`.

# Desktop Vietnamese IME review — OpenKey, XKey, VKey, Funput, GõViệt

**Date:** 2026-10-29  
**Purpose:** extract behavior, UX, architecture patterns and test cases for ViFastKey without taking code from copyleft projects.  
**Inputs:** local clones under `thư mục clone tạm/`; READMEs, docs, settings schemas, engine/session code, platform integration code, test corpora, changelogs.  
**ViFastKey baseline reviewed:** `README.md`, `ROADMAP.md`, `python/README.md`, `python/PORTING.md`, `python/EDGE_CASES.md`, `python/vfk_engine/*`, `rust/README.md`, `rust/crates/vfk-engine/src/ports.rs`, `rust/crates/vfk-ffi/include/vifastkey.h`.

> **Legal note up front:** code was read only to understand behavior and architecture. No code, tables, word lists or UI strings from GPL/AGPL projects should be copied into ViFastKey. XKey is especially risky: its `LICENSE` says MIT, but the engine headers say “Ported from OpenKey” / “Direct port from OpenKey”, and its spell dictionary is GPL. Treat XKey's engine as GPL-derived, not MIT. See §7.

---

## 1. Project inventory

| Project | Version / last activity | License | Platforms | Stack / architecture | Size (source, excl. extern) | Distribution | Best at | Main risk |
|---|---|---|---|---|---|---|---|---|
| **OpenKey** (`tuyenvm_OpenKey`) | 2.0.3; commits through 2026-06 | **GPL-3.0** | macOS (main), Windows, Linux (WIP) | C++ engine + Objective-C/AppKit macOS shell; Win32 native; CGEventTap + synthetic Backspace/text | ~10.6k LOC | DMG, Homebrew cask | Proven event-tap IME; smart switch; macro table; charset converter; per-app code table | Old UI, no secure-input handling, GPL |
| **XKey** (`xmannv_xkey`) | 1.2.25; 2026-10 | **MIT header, but OpenKey-derived engine; GPL dictionary** | macOS 12+ | Swift/SwiftUI; dual CGEventTap and IMK (`XKeyIM`); Sparkle; iCloud KVS | ~66.5k LOC | DMG, Homebrew, Sparkle | Richest macOS UX: 11 settings panes, window-title rules, injection methods, secure input, debug window, translation, backup/iCloud | License provenance; huge surface; IMK + tap dual path complexity |
| **VKey** (`phatMT97_VKey`) | 4.4.0; 2026-10-29 | **GPL-3.0** (optional engine proprietary/CC BY-NC) | Windows 10/11 | C++20 engine, Win32 hook + TSF DLL, Sciter.JS UI, watchdog, custom update signature | ~102k LOC | ZIP, WinGet, website (Classic + Sciter) | Deep Windows integration: hard/soft app locks, game mode, TSF, per-app send methods, macro UX, update security | GPL; optional closed engine; AI attribution policy; Windows-only |
| **Funput** (`Funput_Funput`) | 0.1.0; active 2026-10 | **MIT** (data notices: SCOWL, CC BY 3.0/4.0) | iOS, Android, macOS, Windows, Linux | Rust core (`funput-core`, `funput-engine`), C ABI/JNI, SwiftUI/Slint/GTK shells, IMK direct/marked, Fcitx5/IBus | ~134k LOC | App Store, Play Store, MS Store, DMG, distro packages | Best engine behavior catalog: doubled-key restore, typo correction, extra onsets, place names, advanced Telex; documented config contract; direct composition | Very large; some features mobile-only; data licenses need notices |
| **GõViệt** (`nguyenrot_goviet`) | 0.2.10; 2026-09 | **MIT** | macOS | Rust engine + Swift shell; CGEventTap; process-pinned injection; stable signing | ~4.9k LOC | DMG (not notarized) | Clean, small reference for macOS reliability: tap watchdog, PID pinning, slow queue, secure input, TSV corpora, config migration | Minimal UI; no auto-update; no charsets |
| **ViFastKey** (this repo) | 0.4.0 | **MIT** | Engine only (C ABI/WASM/CLI/web) | Python oracle + Rust port; pure `ImeSession`; ports for config/app/surrounding/dictionary | ~8.7k LOC (Python + Rust + web) | source/library | Core conformance, NFC/NFD, preedit/replace, surrounding edit, soft reset, macro basics, smart caps | No real desktop adapter, no charset/convert, limited macro/context features |

### 1.1 What each project is actually good for

- **OpenKey** — the de-facto “UniKey on macOS” event-tap design. Its value is the long bug history: dozens of concrete Vietnamese regressions, the Chromium/Excel workarounds, smart switch, and a UniKey-compatible macro file format.
- **XKey** — the most complete *macOS product UX*. Its context model (excluded apps → window-title rules → smart switch) and injection-method matrix are the best available blueprint. Its code is not safe to reuse.
- **VKey** — the most complete *Windows product model*. Hard/soft app locks, game mode, hook+TSF parallel architecture, commit-undo boundary rules, macro editor, update threat model.
- **Funput** — the best *engine/specification* source. Its docs describe behavior precisely (including failures and known limits), and its five-platform Rust architecture is the closest analogue to ViFastKey's intended shape.
- **GõViệt** — the best *small macOS shell* reference: a clean, testable event tap + injector with explicit invariants and a real per-app test matrix.

---

## 2. Feature inventory matrix

Legend: **✅** present and user-facing or fully implemented; **◐** partial, legacy, experimental, engine-only, or platform-limited; **✗** absent; **—** not applicable.

### 2.1 Input methods and composition

| Feature | OpenKey | XKey | VKey | Funput | GõViệt | ViFastKey |
|---|---|---|---|---|---|---|
| Telex | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Simple Telex | ✅ (2 variants) | ✅ (2 variants) | ✅ | ✗ | ✅ | ✅ |
| VNI | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Telex + VNI combined in one word | ✗ | ◐ (auto-detect) | ✅ (`Combined`) | ✗ | ✗ | ✗ |
| Auto-detect Telex/VNI | ✗ | ✅ | ✗ | ✗ | ✗ | ✗ |
| User-defined keymap | ✗ | ◐ (custom consonants) | ✅ | ✗ | ✗ | ✗ |
| Full/advanced Telex (`[`→ơ, `]`→ư, leading `w`→ư) | ◐ (brackets only) | ◐ | ◐ (brackets, W behavior) | ✅ (`telex_advanced`) | ✗ | ◐ (`bracket_keys`, `w_as_u`; no leading-`w`/`][` semantics) |
| Extra initial consonants `z f w j` | ✅ (legacy flag) | ✅ (custom consonants) | ✅ (`allowZwjf`) | ✅ (per-letter `ExtraOnsets`) | ✗ | ◐ (`w_as_u` only) |
| Tây Nguyên / ethnic place names | ✗ | ✗ | ✗ | ✅ (`Đắk Lắk`, `Krông`, `Chư Păh`, coda h/l/r) | ✗ | ◐ (`allow_coda_k` only) |
| VIQR / VISCII input | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Free tone placement (any position) | ✅/legacy | ✅ | ✅ | ✅ | ✅ | ✅ (all orders) |
| Double-key revert (`aa`→`â`→`aa`, `ww`→`w`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `z` remove tone / shape | ✅ | ✅ | ✅ | ✅ | ◐ | ✅ |
| Tone after `qu`, `gi` handling | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

### 2.2 Charsets, output and text tools

| Feature | OpenKey | XKey | VKey | Funput | GõViệt | ViFastKey |
|---|---|---|---|---|---|---|
| Unicode NFC | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Unicode NFD / “Unicode tổ hợp” | ✅ | ◐ (hidden/experimental) | ✅ | ✅ | ✗ | ✅ (NFD output) |
| TCVN3 (ABC) | ✅ | ✅ | ✅ | ✅ (convert only) | ✗ | ✗ |
| VNI-Windows | ✅ | ✅ | ✅ | ✅ (convert only) | ✗ | ✗ |
| Vietnamese locale CP 1258 | ✅ | ◐ (hidden) | ✅ | ✗ | ✗ | ✗ |
| VISCII | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Per-app output charset | ✅ (remember table code) | ✗ | ✅ | ✗ | ✗ | ✗ |
| Charset converter (clipboard / selection / file) | ✅ | ✅ | ✅ | ✅ | ✗ | ✗ |
| UPPER/lower/Title/remove accents | ✅ | ✅ | ✅ | ✅ | ✗ | ✗ |
| Sequential conversion cycle | ✗ | ✗ | ✅ | ✗ | ✗ | ✗ |
| Batch file conversion | ✗ | ✗ | ✗ | ✅ | ✗ | ✗ |
| Preserve clipboard non-text types | ◐ (HTML + text only) | ◐ | ✗ (warns images lost) | planned (V3) | ✗ | ✗ |

### 2.3 Restore, spell check and macro

| Feature | OpenKey | XKey | VKey | Funput | GõViệt | ViFastKey |
|---|---|---|---|---|---|---|
| Phonotactic spell check | ✅ | ✅ | ✅ (Off/Standard/Advanced) | ✅ | ◐ (validity gate only) | ✅ |
| Dictionary-based English restore | ◐ (spell validity) | ✅ | ✅ | ✅ | ✅ | ✅ |
| Eager restore while typing | ◐ | ✅ (`instantRestore`) | ✅ | ✅ (`eagerRestore`) | ◐ | ✅ (core invalid → raw) |
| Doubled-key restore (`coffee`, `seemed`, `correct`) | ✗ | ✗ | ✗ | ✅ | ✗ | ✗ |
| Abbreviation protection (`đc`, `đn`, `òh`) | ◐ | ◐ | ✅ (exclusions) | ✅ | ✅ | ◐ (macro only) |
| ESC restores raw keys | ✗ | ✅ (IMK + option) | ✅ (`escRestoreRaw`) | ✅ (Flip) | ✅ | ✅ |
| Personal dictionary | ✗ | ✅ | ✅ | ✗ | ✗ | ✗ (only load lists) |
| Spell exclusion prefixes | ✗ | ✗ | ✅ | ✗ | ✗ | ✗ |
| Free typing / English bypass | ◐ (legacy flag) | ✅ (`freeMark`, custom consonants) | ✅ (`allowEnglishBypass`) | ✗ | ✗ | ◐ (free tone already) |
| Typo/autocorrect on word end | ✗ | ✗ | ◐ (advanced engine) | ✅ (mobile; desktop off) | ✗ | ✗ |
| Basic macro | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Macro case mapping (`bc`/`Bc`/`BC`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Macro in English mode | ✅ | ✅ | ✅ | ✅ | ◐ | ✗ |
| Configurable macro triggers (Space/Enter/Tab/arrows) | ◐ | ◐ | ✅ | ◐ (space/punct) | ◐ | ◐ (word boundary only) |
| Macro undo with Backspace | ✗ | ✗ | ✅ (4 s) | ◐ (mobile only) | ✗ | ✗ |
| Multi-line / long macro | ◐ | ✅ | ✅ (20 000 chars) | ✅ | ✅ | ◐ (string field only) |
| Macro import/export | ✅ (UniKey `trigger:content`) | ✅ (txt) | ✅ (txt) | ✅ (JSON + UniKey import) | ✅ (JSON) | ✗ |
| Per-app macros | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |

### 2.4 Context and application integration

| Feature | OpenKey | XKey | VKey | Funput | GõViệt | ViFastKey |
|---|---|---|---|---|---|---|
| Smart switch (remember VI/EN per app) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Hard app exclusion (always English, toggle blocked) | ✗ | ✅ | ✅ | ✗ | ✅ | ✅ (`excluded_apps`) |
| Soft app lock (reset to EN on entry, toggle allowed) | ✗ | ✗ | ✅ | ✗ | ✗ | ✗ |
| Per-app input method | ✗ | ✗ | ✅ | ✗ | ✗ | ✗ |
| Per-app charset | ✅ | ✗ | ✅ | ✗ | ✗ | ✗ |
| Per-app send/injection strategy | ✗ | ✅ | ✅ | ◐ (built-in profiles) | ✅ | ◐ (profile hints only) |
| Window-title / AX context rules | ✗ | ✅ (regex, roles, DOM classes) | ✗ | ✗ | ✗ | ✗ |
| Per-domain browser rules | ✗ | ◐ | ✅ (extension) | ✗ | ✗ | ✗ |
| Foreign/CJK layout auto-suppress | ✅ (`vOtherLanguage`) | ✗ | ✅ | ✅ (Windows) | ✗ | ✗ |
| Secure input / password detection | ✗ | ✅ (+ overlay) | ✗ | ◐ (IMK host) | ✅ (+ 🔒) | ◐ (engine `FieldKind`, no adapter) |
| Remote-desktop mode | ✗ | ✅ | ✗ | ✗ | ✗ | ✗ |
| Game mode / send raw keys | ✗ | ✅ (excluded) | ✅ (+ toggle hotkey) | ✗ | ✅ (excluded) | ✗ (excluded only) |
| TSF mode | ✗ | ✗ | ✅ | ◐ (RFC only) | ✗ | ✗ |
| IMK mode | ✗ | ✅ | ✗ | ✅ | ✗ | ✗ |
| Fcitx5/IBus | ◐ (Linux WIP) | ✗ | ✗ | ✅ | ✗ | ✗ |
| Preedit model | ✗ (tap replace) | ✅ (IMK marked) | ✅ (TSF) | ✅ (marked + direct) | ✗ | ✅ (engine action) |
| Replace/backspace model | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

### 2.5 UX, operations and configuration

| Feature | OpenKey | XKey | VKey | Funput | GõViệt | ViFastKey |
|---|---|---|---|---|---|---|
| Menu bar / tray | ✅ | ✅ | ✅ | ✅ | ✅ | ✗ |
| Floating OSD / V-E indicator | ◐ (icon only) | ✅ (floating toolbar) | ✅ (floating icon, TSF “T”) | ✗ | ◐ (🔒 status) | ✗ |
| Toast / transient status | ✗ | ◐ | ✅ | ✗ | ✗ | ✗ |
| Debug window / live trace | ✗ | ✅ | ◐ (debug log) | ✗ | ◐ (logs) | 💻 CLI trace |
| Onboarding / permission walkthrough | ◐ | ◐ | ✅ | ✅ (4 steps) | ◐ | ✗ |
| Settings import/export | ◐ (macro only) | ✅ (plist) | ◐ (lists/macros) | ✅ (JSON merge) | ◐ (macros JSON) | ✗ |
| Backup / iCloud sync | ✗ | ✅ | ✗ | ✗ | ✗ | ✗ |
| Config schema version + migration | ✗ | ◐ | ✅ | ✅ | ✅ | ✗ |
| Reset to defaults | ✅ | ✅ | ✅ | ✅ | ✗ | ✗ |
| Hotkey recorder + presets | ◐ | ✅ | ✅ | ✅ | ◐ | ✗ |
| Bilingual UI | ◐ | ✅ (vi/en/zh) | ✅ (vi/en) | ✅ | ✗ | ✗ |
| Auto-update | ✅ (version.json) | ✅ (Sparkle) | ✅ (signed package) | ✅ (Store/Sparkle) | ✗ | ✗ |
| Update signature / notarization | ◐ | ✅ (Sparkle) | ◐ (package sig; unsigned Authenticode) | ✅ (Store/MSIX) | ◐ (stable dev cert, not notarized) | ✗ |
| Watchdog / self-healing | ✗ | ✗ (tap re-enable only) | ✅ (`VKeyWatchdog.exe`) | ✗ | ✅ | ✗ |
| Accessibility permission guidance | ✅ | ✅ | ◐ | ✅ | ✅ | ✗ |

### 2.6 Extras: emoji, suggestions, translation

| Feature | OpenKey | XKey | VKey | Funput | GõViệt | ViFastKey |
|---|---|---|---|---|---|---|
| Emoji picker / emoji suggestions | ✗ | ✗ | ✗ | ✅ (iOS/Android keyboard) | ✗ | ✗ |
| Vietnamese word suggestions / candidate bar | ✗ | ✗ | ✗ | ✅ (mobile) | ✗ | ✗ |
| Personal learning / bigram context suggestions | ✗ | ✗ | ✗ | ✅ (mobile) | ✗ | ✗ |
| Translation overlay (30+ languages) | ✗ | ✅ | ✗ | ✗ | ✗ | ✗ |
| Text replacement integration with the OS | ✗ | ◐ (yield to macOS replacements) | ✗ | ✗ | ✗ | ✗ |

Emoji and candidate UI are deliberately mobile concerns for this review: on desktop, an inline candidate window competes with the host app's autocomplete and is usually the wrong default. The useful desktop lesson is Funput's **grapheme-safe text handling** and **suggestion data model**, not the candidate bar itself.

---

## 3. Platform and UX patterns worth adopting

### 3.1 macOS event-tap pipeline (must-have for any replace-mode adapter)

The strongest pattern is GõViệt's small, explicit pipeline, reinforced by OpenKey's app-specific workarounds:

1. **Tap on its own thread** with `.userInteractive` QoS, not on the main run loop. Keep the callback tiny.
2. **Self-event marker.** Every synthesized event carries a magic `eventSourceUserData` (`"GVIT"`). The tap ignores its own events. This is the anti-feedback loop invariant; without it, backspace/text injection re-enters the engine.
3. **Tap watchdog.** A non-nil `CFMachPort` is not a healthy tap. Poll `CGEvent.tapIsEnabled` every ~2 s in addition to handling `tapDisabledByTimeout` / `tapDisabledByUserInput`. On recovery, **clear the engine word** — a disabled tap may have missed arbitrary edits, and the next replacement must not delete text the engine never saw.
4. **Target-process pinning.** Compute the target PID from the event (`eventTargetUnixProcessID`) and post slow replacements to that PID. If focus changes while a queued replacement is pending, the old text must not land in the new app. GõViệt's `EventRouting` test matrix and “slow queue + defer later physical events” design is the model.
5. **Slow path without blocking the tap.** Keep fast injection at the tap point. For terminals/Spotlight/Electron, run paced injection on a serial worker and defer subsequent physical events (with the marker) behind it, preserving order without a tap timeout. Cap total pacing time (GõViệt uses 2 s).
6. **Injection strategies, not one-size-fits-all.**
   - `fast`: burst backspaces + chunked Unicode text.
   - `slow`: paced backspaces/text, for terminals, VS Code/Cursor, Spotlight, Raycast.
   - `selectAndRetype`: Shift+Left select the composed word then type replacement, for Chromium omnibox/autocomplete.
   - `autocomplete`: Forward Delete + backspace for overlays.
   - `axDirect`: set text through Accessibility API (Firefox content area).
   - `passthrough`: no processing.
   Presets should be visible and user-overridable per app.
7. **Secure input is a first-class state.** `IsSecureEventInputEnabled()` plus the console-user `kCGSSessionSecureInputPID` identifies the holder. While secure input is on, pass every key through and show a lock state in the menu bar/OSD. Do not display preedit or cached text.
8. **Autorepeat, modifier-only events, non-BMP keys.** Ignore key repeats; do not reset the word on a standalone Shift/Option; decode the complete UTF-16 event string and pass through multi-scalar events as a safe boundary. These are concrete bugs in the wild.
9. **Non-BMP / grapheme integrity.** Chunk text at grapheme boundaries, never split a surrogate pair or a combining sequence. Test emoji insertion in the middle of a word.
10. **TCC identity.** Accessibility permission is tied to the code signature. Sign with a stable certificate (Developer ID or stable Apple Development identity), enable hardened runtime, never ad-hoc sign production builds. `tccutil reset Accessibility <bundle-id>` is the recovery path. GõViệt's README/ATTRIBUTION is an excellent model for documenting this.

### 3.2 Windows: hook + TSF, app locks, game mode

VKey's model is worth copying at the product level:

- **Two backends, one engine.** Hook remains the default; TSF is an explicit per-app or global mode. Never run both consuming input in one context. TSF has composition/edit-session semantics; hook has Backspace replacement. The engine output must be abstract enough for both.
- **Per-app mode decision is a pure function.** `excluded` (hard English) wins over `forcedVietnamese` even if a corrupted config lists an app in both sets. This should be a unit-tested function, not scattered `if`s.
- **Hard vs soft exclusion.** Hard = transparent, hotkey blocked. Soft = reset to English on focus, but the user may toggle to Vietnamese (games, fullscreen). Both are needed.
- **Game mode is a send strategy, not merely “disable IME”.** Re-send the original physical key so WASD still works while Telex composition is active. Provide a global hotkey to add/remove the foreground app, because a fullscreen game cannot reach Settings.
- **CJK / foreign layout detection.** Query the foreground *focus thread* `HKL` (`GetGUIThreadInfo → GetKeyboardLayout`), not the top-level window thread. Suspend Vietnamese for IME/CJK/non-Latin layouts; restore on the Latin layout. Keep the suspension in session state, not in persisted `enabled`.
- **Commit-undo boundaries.** VKey's `DecideCommitUndoArm` is a rich source of edge cases: Space/punctuation/digits arm; Enter/Tab clear; arrows/Home/End/Esc/Insert/Delete skip; punctuation after a space cancels a pending undo. Copy the *decision table*, not the code.
- **Clipboard fallback is a last resort.** It works for hook-hostile apps but destroys rich clipboard contents. Warn precisely (“images will be lost”) and restore plain text only where possible.
- **Elevated apps.** A non-elevated hook cannot type into an elevated process. State this in onboarding and provide “run as administrator” if the product supports it.
- **Update security.** VKey's custom package signature is a reaction to being unable to Authenticode-sign a Sciter build. ViFastKey should prefer standard platform signing; if that is impossible, sign the package manifest and verify before install, never silently auto-update an unsigned binary.

### 3.3 Linux: preedit first

Funput's Linux design is the right one:

- Fcitx5 and IBus addons share a small C++ composer that calls the Rust C ABI.
- Prefer preedit for terminals; commit atomically at word boundaries.
- On GNOME/Wayland, per-app identification is unreliable (`program()` may be `gnome-shell`), so do not expose a per-app list there.
- Settings process writes JSON; addon watches with inotify plus re-checks mtime on focus-in. The addon never writes user settings back.
- Document Wayland limitations honestly rather than promising per-app control.

### 3.4 Context precedence (adopt a single, tested order)

XKey's `AppPolicyRuntime.evaluate` has the clearest precedence:

1. **Hard exclusion** (transparent, no transform, toggle unaffected or blocked depending on product choice).
2. **Window-title / AX rule** (can force enable/disable and even switch input source).
3. **Per-app override** (method/charset/send strategy).
4. **Smart-switch remembered VI/EN**.
5. **Global default**.

Important details:

- Overlay apps (Spotlight, Raycast, Alfred) must not overwrite the remembered mode of the app underneath. XKey maps overlay windows to a synthetic bundle id for this reason.
- A context refinement (title change, click, focus within the same app) must not re-apply smart switch; otherwise changing VI/EN in place is immediately undone.
- `focus_changed` must flush the current word first. If preedit is active, commit it; if replace mode, leave the document as-is and clear state.
- Window-title rules should support `contains`, `prefix`, `suffix`, `exact`, `regex`, plus AX role/description/identifier and (optionally) DOM class. XKey's Google Docs/Sheets/Slides rules are concrete: “Google Tài liệu”, “Google Trang tính”, “Google Trang trình bày”.
- Rule order matters: XKey merges matching rules in sort order and the last matching value wins. Make that explicit and testable.

### 3.5 Injection presets / app presets

None of the projects ships a true “app preset library”, but their default profile maps are a good starting point. ViFastKey should expose presets rather than raw low-level fields:

| Preset | App examples | Behavior |
|---|---|---|
| Standard | TextEdit, Notes, most native apps | fast replace |
| Chromium / omnibox | Chrome, Edge, Brave, Arc, Cốc Cốc | select-and-retype; wait ack; per-char delay |
| Terminal / Electron | Terminal, iTerm2, kitty, WezTerm, VS Code, Cursor | slow paced replace or preedit; no previous-word edit |
| Overlay | Spotlight, Raycast, Alfred | autocomplete-safe, slow |
| Game | Minecraft and user-added games | passthrough or game send mode; no hotkeys |
| Legacy / remote | Photoshop, CAD, remote desktop | clipboard or legacy charset profile |
| Browser web-app | Google Docs, Notion, Figma | slower injection, no marked text, window-title rule |

Expose a per-app row with `Preset | Input method | Charset | Send method | Enabled/lock | Notes`, plus an “Advanced” escape hatch for delays and wait-ack.

### 3.6 Settings UX

Best patterns observed:

- **Sidebar with 4–7 top-level groups**, not 30 checkboxes on one screen. XKey: About, General, Quick Typing, Advanced, Input Sources, Excluded Apps, Macro, Translation, Convert, Appearance, Backup. VKey: Input, Macro, System, About, with sub-dialogs for app overrides, TSF apps, excluded apps, hotkeys, dictionary/exclusions, user-defined keymap, icon settings.
- **Cascade dependent options.** VKey disables spell sub-options when spell check is off; XKey disables restore sub-options when spell check/restore is off. Hide irrelevant controls rather than showing dead switches.
- **Hotkey recorder with presets.** XKey exposes modifier-only presets (Ctrl+Space, Fn, Ctrl+Shift, Option+Shift, Cmd+Shift) plus a free recorder and conflict warnings (e.g. macOS input-source shortcuts, fn emoji picker). VKey records modifiers + VK and supports F1–F24/OEM/numpad.
- **Macro editor.** VKey is the best model: two-column form + searchable list, multi-select/delete, character counter (20 480), “Enter for newline” hint, “long macros use clipboard and overwrite it” warning, trigger checkboxes (Space/Enter/Tab/arrows), import/export. XKey adds per-entry enable/disable and smart-case preview.
- **App picker.** XKey offers All/Running tabs, search, icons; VKey offers a running-app dropdown, manual `.exe`, refresh, and “pick window”. Adopt the picker + manual fallback + browse.
- **Dictionary editor.** XKey: personal words, add/edit/delete, import/export, explicit GPL download consent. VKey: personal dictionary + spell exclusion prefixes, add/edit/delete, load/export `.txt`, immediate apply, UTF-8/comment rules, invalid file must not destroy the last good list.
- **Backup/import/export.** Funput's config document is the best contract: portable `preferences`/`shortcuts` + `platform.<os>` blocks; stable lowercase enum strings; non-destructive merge; unknown keys ignored; newer version best-effort; macro merge by trigger; `appLanguageMemory` union with local wins. XKey offers full plist export/import + restart prompt + reset; VKey only list/macro exports. Adopt Funput's merge semantics, plus a backup copy before import.
- **Reset.** Separate “reset settings”, “clear macros”, “clear personal dictionary”, and “reset app overrides”. XKey's reset confirms and restarts; VKey resets individual lists. Never clear user macros/dictionary on a settings reset without explicit confirmation.
- **Onboarding.** Funput's 4 steps (welcome → enable input source/Accessibility → choose method → ready + launch at login) with a live status indicator is a good minimum. OpenKey/VKey also show permission instructions. Include a “test typing” field in the final step.
- **Debug view.** XKey's debug window logs every key, engine buffer, action, injection, secure-input changes and app context. For ViFastKey, a compact “event → engine action → injection” trace with redaction of text content (privacy) is enough; the CLI already has `--trace`.
- **Status surfaces.** Menu bar icon states: V/E, locked (🔒/hard), TSF “T”, paused, error. VKey's floating V/E icon solves fullscreen; XKey's floating toolbar solves temp-off. Do not overdo OSD; one compact indicator plus toasts is enough.
- **Language.** XKey ships vi/en/zh; VKey vi/en. ViFastKey chọn UI tiếng Việt và
  **không làm i18n**; vẫn nên gom string vào một chỗ nếu sau này đổi quyết định.
- **Accessibility.** Test keyboard-only navigation, VoiceOver/Narrator labels on every switch, Reduce Motion, high contrast, and large text. XKey/Funput document these manual checks; most IMEs fail them.

### 3.7 Data and text tools

- **Charset conversion belongs after output rendering.** Funput's “Atom” pivot (decode source → Unicode atom → encode target) is the right architecture: adding charset N costs two mappings, not N². Keep the engine internal representation in NFC and apply charset only at the adapter/output boundary.
- **Detect source charset by scoring decoded Vietnamese syllables**, not by byte heuristics. Return “unknown” on ties instead of guessing (VISCII vs TCVN3 is genuinely ambiguous).
- **Two counters**: `normalized` (safe re-encoding) vs `unmapped` (lost/guessed). Surface “3 characters cannot be represented” by name, not just a count.
- **TCVN3 only encodes lowercase**; uppercase accented vowels require the `.VnTimeH` font convention. Document this as a charset limitation, not a bug.
- **VNI-Windows is multi-unit**: one Vietnamese letter can be base byte + tone byte. Decode greedily (two-byte pair first); encode with the base/tone pair. Test round-trip over the whole character inventory.
- **NFD is not “tone-only combining”.** Real NFD puts `ạ` as `a + U+0323` and `ậ` as `a + U+0323 + U+0302` (dot below before circumflex). Accept any order on input; always emit the UniKey convention on output.
- **Case tools need explicit edge cases**: `TP. HCM`, `1.5`, `v.v.`, empty, punctuation-only, emoji, Japanese mixed in; `đ/Đ` preservation toggle; ALL-CAPS exception.
- **Clipboard conversion**: OpenKey preserves HTML + plain text and converts both; VKey warns that images are lost; Funput plans full clipboard backup. ViFastKey should support plain text first, preserve other types or refuse with a clear message.

### 3.8 Reliability and security

- Event tap watchdog and engine reset after discontinuity (GõViệt).
- Watchdog process for crash recovery (VKey: 90 s heartbeat, distinguishes user quit; starts via Task Scheduler).
- Stable code identity for TCC (GõViệt).
- Update verification before install (VKey custom signature; XKey Sparkle; Funput Store). Never auto-update from an unauthenticated JSON.
- Duplicate-instance guard at launch (OpenKey) to avoid two IMEs fighting.
- Per-app regression matrix after every injection change (GõViệt's `docs/TESTING.md` is a model: TextEdit, Notes, Safari, Chrome address bar, Google Docs, VS Code, iTerm2, Claude Code CLI, Spotlight, Excel, Messages, password fields, games).
- Log without leaking user text. XKey's debug window is useful; production logs should record keycodes/actions, not content.

---

## 4. Concrete test cases

Legend: `⌫` Backspace, `⎋` Esc, `⏎` Enter, `⇥` Tab. “Raw” means the exact ASCII keys typed. Sources: `[OK-CL]` OpenKey changelog, `[OK-SRC]` OpenKey engine, `[VK-T]` VKey tests, `[FP-DOC]` Funput docs, `[GV-C]` GõViệt TSV corpus, `[VF-G]` ViFastKey golden/property tests, `[UX]` observed product behavior.

### 4.1 Core composition regressions

| # | Keys | Expected | Source / why |
|---|---|---|---|
| C1 | `quởn` | `quởn` | `[OK-CL]` old OpenKey failed; `qu` onset + `ơ` + coda |
| C2 | `quét` with restore on | `quét` | `[OK-CL]` restore must not break valid `quét` |
| C3 | `tuyps` | `tuýp` (or valid `tuýp` form) | `[OK-CL]` `uy` + coda `p`; old engine dropped keys |
| C4 | `dieu96` (VNI) | `diệu`/`điểu`-class valid word, not literal | `[OK-CL]` VNI multi-key ordering |
| C5 | `dui9`, `duoi96` (VNI) | compose normally | `[OK-CL]` VNI backspace/D9 sequence |
| C6 | `oăc` vs `ooạc` (VNI) | `oăc`; no spurious `ooạc` | `[OK-CL]` VNI `o` + `ă` + `c` |
| C7 | `chú thòong`, `gòong` | preserve `thòong`, `gòong` | `[OK-CL]` `oo` must not collapse |
| C8 | `duocd` | `được` | `[OK-CL]` late `d` → `đ` |
| C9 | `duongd` | `đường` | `[OK-CL]` late `d` after coda |
| C10 | `quatw` | `quăt` | `[OK-CL]` `w` scans backward over coda |
| C11 | `gì` (old tone style) | `gì` | `[OK-CL]` old-style tone placement on `gi` |
| C12 | `uýt` | `uýt` | `[OK-CL]` `uy` + coda `t` |
| C13 | `thuơn`, `ưoi`, `ưom`, `ưoc` | valid words, not literal | `[OK-CL]` `ươ` family edge cases |
| C14 | `xooong`, `boong`, `goong`, `vooc` | `xoong`, `boong`, `goong`, `vooc` | `[VK-T]` double-`oo` + coda |
| C15 | `uow` / `uoww` | `ươ` / `uow` | `[VK-T]` direct transform then escape |
| C16 | `huow`, `huoww`, `huowww` | `huơ`, `hươ`, `huow` | `[VK-T]` second `w` upgrades, third escapes |
| C17 | `thuow`, `thuoww`, `thuowww` | `thuơ`, `thương`-class, `thuow` | `[VK-T]` / `[VF-G]` |
| C18 | `duow`, `duoww` | `dươ` / `duow` | `[VK-T]` |
| C19 | `truongwf` | `trường` | `[VK-T]` full coda then `w` then tone |
| C20 | `truwrow` | `trưở` | `[VK-T]` tone relocation after horn |
| C21 | `luoow` | `lươ` | `[VK-T]` circumflex then `w` |
| C22 | `loow` | `lơ` | `[VK-T]` `ô` → `ơ` |
| C23 | `giuaaw` | `giưa` | `[VK-T]` `gi` onset + `ưa` |
| C24 | `w`, `ww`, `W`, `sww`, `thww` | `ư`, `w`, `Ư`, `sw`, `thw` | `[VK-T]` / `[FP-DOC]` standalone `w` + revert |
| C25 | `m]a` | `mưa` | `[VF-G]` bracket keys |
| C26 | `t]`, `m[`, `tr][ngf` | `tư`, `mơ`, `trường` (advanced Telex) | `[FP-DOC]` |
| C27 | `[[`, `]]`, `m[[`, `t]]` | `[`, `]`, `m[`, `t]` | `[FP-DOC]` double bracket escape |
| C28 | `dd`, `did`, `dm` + `d`, + `d` | `đ`, `đi`, `đm`, `dmd` | `[VK-T]` first-character `đ` toggle |
| C29 | `HDDLDD`, `hddldd` | preserve abbreviation, no mangling | `[VK-T]` spell-on and spell-off |
| C30 | `hđlđ` | preserve mid-word `đ` | `[VK-T]` rule applies to first char only |
| C31 | `đcđt` | preserved if in exclusion list | `[VK-T]` spell exclusion |
| C32 | `batf` | `batf` (tone blocked on stop coda) | `[VF-G]` |
| C33 | `bats` | `bát` | `[VK-T]` |
| C34 | `looixfsx` | `lỗi` | `[VF-G]` repeated tone replacement |
| C35 | `ass`, `aaa`, `ddd` | `as`, `aa`, `dd` | `[VF-G]` double-key revert |
| C36 | `xooongj` | `xoọng` | `[VF-G]` tone undo semantics |
| C37 | `tuỳa` + `⌫` | `tùy` | `[OK-CL]` punctuation restore on delete |
| C38 | `vieejt⌫` / `vieejt⌫t` | `việ` / `việt` | `[VF-G]` backspace retone |
| C39 | `muowa⌫⌫wif` / `muowa⌫⌫⌫owif` | `mười` | `[VF-G]` backspace into invalid prefix then re-compose |
| C40 | `user⌫r` | `user` | `[VF-G]` invalid remainder stays inert |

> C3–C5 are historical OpenKey bug triggers; the exact expected word should be pinned by running a reference IME before adding them to conformance. They are listed because the *key sequence* is a known stress case (VNI multi-key ordering, `uy` + stop coda, late `đ`/circumflex).

### 4.2 English restore and doubled keys

| # | Keys | Expected | Source / why |
|---|---|---|---|
| E1 | `text`, `add`, `address`, `world`, `good`, `for`, `just`, `software`, `expression` | unchanged | `[GV-C]` |
| E2 | `this`, `mix`, `was`, `test`, `off` | `thí`, `mĩ`, `ứa`, `tét`, `of` (known limitations) | `[GV-C]` / `[FP-DOC]`; must be in a separate “known limitations” corpus |
| E3 | `password ` | `password ` | `[FP-DOC]`; ViFastKey currently passes |
| E4 | `coffee ` | `coffee ` | `[FP-DOC]`; **ViFastKey currently outputs `cofee`** |
| E5 | `seemed ` | `seemed ` | `[FP-DOC]`; **ViFastKey currently outputs `semed`** |
| E6 | `correct ` | `correct ` | `[FP-DOC]`; **ViFastKey currently outputs `corect`** |
| E7 | `office `, `pass `, `class `, `doses `, `casts `, `markers ` | full words | `[FP-DOC]` doubled-key table |
| E8 | `pass1 ` / `off1 ` | `pass1 ` / `of1 ` | `[FP-DOC]` digits trimmed before lookup; `off` not in table |
| E9 | `Password ` | `Password ` | `[FP-DOC]` case-preserving |
| E10 | `pềct ` (VKey) | `perfect` | `[VK-GUIDE]` auto-restore on Space |
| E11 | `asus` → `⎋` | `asus` | `[VK-GUIDE]` raw restore hotkey |
| E12 | `dduwowfnh ` / `tpoi ` / `khpong ` / `vieeyj ` / `minhg ` | `đường` / `tôi` / `không` / `việt` / `mình` (typo correction, optional) | `[FP-DOC]`; high-risk P2 |
| E13 | `ddc`, `ddn`, `ofh` | `đc`, `đn`, `òh`, not expanded/restored | `[GV-C]` abbreviation protection |
| E14 | `btw ` macro `btw`→`by the way` | macro wins over English restore | `[VF-G]` / `[FP-DOC]` order: shortcut before restore |

### 4.3 Macro

| # | Keys | Expected | Source / why |
|---|---|---|---|
| M1 | `vn ` / `Vn ` / `VN ` (macro `vn`→`Việt Nam`) | `Việt Nam ` / `Việt Nam ` / `VIỆT NAM ` | `[FP-DOC]` smart case |
| M2 | `vn.` | `Việt Nam.` | `[FP-DOC]` punctuation boundary retained |
| M3 | `bbh ` / `Bbh ` / `BBH ` (macro →`bún bò Huế`) | `bún bò Huế` / `Bún bò Huế` / `BÚN BÒ HUẾ` | `[VK-MACRO]` |
| M4 | `Bbh ` with title-case option | `Bún Bò Huế` | `[VK-MACRO]` |
| M5 | `bbh ` + `⌫` | `bbh ` (then second `⌫` removes space) | `[VK-MACRO]` / `[FP-DOC]` |
| M6 | `bbh` + `⇥` / `⏎` | expand according to trigger settings | `[VK-MACRO]` |
| M7 | `bbh` + `←` / `→` | does not expand after navigation | `[VK-MACRO]` |
| M8 | English mode, macro `sdt`→number | expands if “macro in English” on | `[FP-DOC]` |
| M9 | `btwx ` | no partial expansion (`btw` only matches full token) | `[VK-MACRO]` |
| M10 | macro trigger `pass`, typed `pass ` with doubled-key restore | trigger for typed keys wins | `[FP-DOC]` |
| M11 | 20 000-char macro with newlines | expands; clipboard path warns | `[VK-MACRO]` |
| M12 | import UniKey `ukmacro.txt` with `trigger:content` header | merge, duplicates not silently duplicate | `[OK-MACRO]` |
| M13 | uppercase-specific trigger `BTW` vs lowercase `btw` | exact uppercase key wins; lowercase key accepts any case | `[VK-MACRO]` |
| M14 | macro output at sentence start | auto-capitalized only first letter unless title-case option | `[VK-MACRO]` |

### 4.4 Auto-capitalization

| # | Keys | Expected | Source / why |
|---|---|---|---|
| A1 | `chaof. anh ` | `chào. Anh ` | `[GV-C]` |
| A2 | `a. ddi ` | `a. Đi ` | `[GV-C]` |
| A3 | `a. hello ` | `a. Hello ` | `[GV-C]` English restore + caps |
| A4 | `a. text ` | `a. Text ` | `[GV-C]` |
| A5 | `a. text⎋` | `a. text` | `[GV-C]` ESC wins over auto-caps |
| A6 | `google.com `, `3.14 `, `file.txt ` | no capitalization | `[XKEY-SET]` “only cap after punctuation + space” |
| A7 | `a? b ` / `a! b ` | `a? B ` / `a! B ` | `[VF-G]` |
| A8 | newline after `a.` | next first letter capitalized | `[XKEY-SET]` newline always caps |
| A9 | `TP. HCM` through case converter | not lowercased; ALL-CAPS exception | `[FP-DOC]` text-case tests |
| A10 | macro at sentence start `bbh` | `Bún bò Huế` (macro expansion, not title-case) | `[VK-MACRO]` |

### 4.5 App/context behaviors

| # | Scenario | Expected | Source / why |
|---|---|---|---|
| X1 | Hard-excluded app | every key passes; hotkey blocked; no preedit | `[VK-GUIDE]` / `[XKEY]` |
| X2 | Soft-excluded game | starts English; user can toggle Vietnamese | `[VK-GUIDE]` |
| X3 | Chrome EN, TextEdit VN, back to Chrome | Chrome returns EN | `[GV-TEST]` smart switch |
| X4 | Spotlight/Raycast overlay over an app | overlay uses its own policy; underlying app memory unchanged | `[XKEY]` overlay detection |
| X5 | Google Docs window in Vietnamese UI (“Google Tài liệu”) | window-title rule applies | `[XKEY]` |
| X6 | Chrome address bar vs page content | address bar uses select/retype or slow path; page content normal | `[XKEY]` / `[GV-TEST]` |
| X7 | CJK layout active | Vietnamese suspended; keys pass; icon shows EN | `[VK-GUIDE]` / `[OK-SRC]` |
| X8 | Return to US layout | Vietnamese restored automatically | `[VK-GUIDE]` |
| X9 | Password field / secure input | pass-through; no preedit; 🔒/lock status | `[XKEY]` / `[GV]` |
| X10 | Click in middle of preedit | commit preedit at old caret, start new word at click | `[VF-G]` / `[FP-DOC]` |
| X11 | Focus changes while slow macro is injecting | replacement and deferred keys stay in original PID | `[GV-TEST]` |
| X12 | App switch before first key | first key must not use previous app's strategy | `[GV]` target changed guard |
| X13 | TSF app | preedit underline, commit on focus/boundary; no hook double-input | `[VK-GUIDE]` |
| X14 | Browser domain `google.com` V, `github.com` E | tab switch applies per-domain state | `[VK-BROWSER]` |
| X15 | Remote desktop client | passthrough by default; optional clipboard injection | `[XKEY]` |
| X16 | Game WASD with Telex on | movement keys still reach game (game send mode) | `[VK-GUIDE]` |
| X17 | Corrupted config lists app in both hard-E and hard-V | hard-E wins; never lock user out of English | `[VK-T]` |
| X18 | Accessibility revoked at runtime | tap stops cleanly; status error; no stale replacement | `[XKEY]` / `[GV]` |

### 4.6 Charset and text conversion

| # | Scenario | Expected | Source / why |
|---|---|---|---|
| T1 | Unicode ↔ TCVN3 round-trip for lowercase inventory | exact | `[FP-CHARSET]` |
| T2 | Unicode → TCVN3 with `Ề` | lowercase `ề` + `unmapped=1`; never silent loss | `[FP-CHARSET]` |
| T3 | VNI `Ề` | representable; round-trip exact | `[FP-CHARSET]` |
| T4 | NFD `ậ` = `a + U+0323 + U+0302` | decode correctly; emit UniKey form | `[FP-CHARSET]` |
| T5 | `₫`, Japanese, emoji in converted text | pass through or count `unmapped`; no panic | `[FP-CHARSET]` |
| T6 | VISCII input detected as TCVN3 | known limitation; document; do not pretend | `[FP-CHARSET]` |
| T7 | `detect()` tie between two charsets | returns “unknown”, not a guess | `[FP-CHARSET]` |
| T8 | Batch file with mixed charsets | detect per file, not one charset for the folder | `[FP-CHARSET]` |
| T9 | UPPER/lower/Title/remove accents on `TP. HCM`, `1.5`, `v.v.` | documented exceptions, not naive word splitting | `[FP-TEXTCASE]` |
| T10 | Sequential convert: UPPER → remove accents → original | 5 s cycle, new selection resets | `[VK-GUIDE]` |
| T11 | Clipboard contains HTML + plain text | both converted; other types preserved or refused safely | `[OK-QUICK]` |
| T12 | OpenKey macro import from UniKey file | `trigger:content`, first header line skipped, duplicate not overwritten | `[OK-MACRO]` |

### 4.7 Config, settings and migration

| # | Scenario | Expected | Source / why |
|---|---|---|---|
| G1 | Old config missing new fields | fields get defaults; unknown keys ignored | `[FP-CONFIG]` |
| G2 | Enum value unknown/unsupported | keep current local preference | `[FP-CONFIG]` |
| G3 | Config `version` newer than reader | best-effort import, unknown ignored, warn | `[FP-CONFIG]` |
| G4 | Invalid JSON | quarantine as `config.invalid-<uuid>.json`, then defaults | `[GV-TEST]` |
| G5 | Import merge of macros by trigger | incoming overwrites same trigger; no deletions | `[FP-CONFIG]` |
| G6 | Import `appLanguageMemory` | union; existing local entries win | `[FP-CONFIG]` |
| G7 | Legacy `excludedApps` | migrated to English memory, then tombstoned | `[FP-CONFIG]` |
| G8 | Platform block from another OS | ignored; only `platform.<current>` applied | `[FP-CONFIG]` |
| G9 | Partial JSON config patch via FFI | missing fields keep current values, not defaults | `[VF-G]` / `[VK]` |
| G10 | Config version 2 seeded macros | remove only seeded pairs, keep user macros | `[GV-TEST]` |
| G11 | Reset settings | confirm; preserve user macros/dictionary unless explicitly selected | `[XKEY]` / `[VK]` |
| G12 | Import file with duplicate app entries | dedupe by normalized bundle/exe; case-insensitive | `[VK-T]` / `[XKEY]` |

### 4.8 Platform/runtime edge cases

| # | Scenario | Expected | Source / why |
|---|---|---|---|
| R1 | Chromium address bar burst `1 xius ` / `1 xisu ` | `1 xíu `, leading `x` preserved | `[GV-TEST]` OpenKey #319 class |
| R2 | Spotlight first character | not swallowed | `[GV-TEST]` |
| R3 | Terminal/iTerm2/vim/Claude Code CLI | atomic commit; no lost keys | `[GV-TEST]` |
| R4 | Long macro then immediate typing in slow app | macro completes before later keys; order preserved | `[GV-TEST]` |
| R5 | App switch while queue pending | no old text in new app | `[GV-TEST]` |
| R6 | Non-BMP emoji then `nam` | emoji intact; no wrong backspace | `[GV-TEST]` |
| R7 | Tap disabled (`killall -STOP` / timeout) | re-enabled within ~2 s; engine word cleared | `[GV-TEST]` |
| R8 | Rebuild + re-sign | Accessibility permission still granted | `[GV-TEST]` |
| R9 | Ctrl/Alt/Cmd + Backspace | must not delete extra text; does not trigger IME backspace | `[VK-RELEASE]` |
| R10 | Ctrl+Shift / fn toggle | toggles VI/EN without opening emoji/input-source panel; fn+arrow still works | `[GV-TEST]` |
| R11 | Numpad digits | pass through; no VNI tone/typo interference | `[VF-EDGE]` |
| R12 | Held key auto-repeat | no repeated composition/replacement | `[GV]` |
| R13 | Dead keys / non-US layout | physical key mapping or layout-compat conversion tested per app | `[OK-SRC]` |
| R14 | Second instance launched | first instance wins; second exits | `[OK-SRC]` |
| R15 | Elevated app on Windows | documented limitation or run-as-admin mode | `[VK-GUIDE]` |
| R16 | Sleep/wake, active space change | tap reinitialized; word reset | `[OK-SRC]` |

---

## 5. What ViFastKey currently lacks

### 5.1 Engine / behavior gaps

1. **Doubled-key English restore.** `coffee → cofee`, `seemed → semed`, `correct → corect`. This is the biggest visible English-typing regression versus Funput. ViFastKey has dictionary restore at word end but loses the second key before lookup.
2. **Macro v2.** No macro-in-English, no trigger configuration (Tab/Enter/arrows), no add-space option, no smart-case/title-case beyond first-letter/all-caps, no undo-on-Backspace, no import/export, no multi-line UI, no max length/counter. Only `dict[str,str]` plus word-boundary matching.
3. **Rich per-app model.** `AppProfile` only has `output_mode`, delays, `edit_previous_word`, `vietnamese`. No per-app input method, tone style, charset, macro table, or hard/soft lock semantics. No window-title or AX context. No app-preset library.
4. **Charsets / conversion.** No TCVN3, VNI-Windows, CP1258, VISCII, Unicode Compound, conversion tool, detection, or per-app charset. Roadmap mentions this but not implemented.
5. **Text tools.** No case conversion, strip accents, selection/clipboard conversion, sequential cycle, or batch.
6. **Advanced Telex / extra onsets / place names.** `bracket_keys` and `w_as_u` are partial. No leading `w`→`ư` outside vowel context, no `][`→`ươ`, no `z/f/j` onsets, no coda `h/l/r` place names (only `k`), no `Krông`-style clusters.
7. **Spell-check UX.** No personal dictionary CRUD, no exclusion prefixes, no free-typing/English-bypass toggle, no “instant restore” toggle (core invalid restore is always on), no dictionary download/licensing flow.
8. **Typo correction.** No autocorrect at word end. Optional/high-risk, but users coming from mobile Funput or VKey Advanced will miss it.
9. **Auto-caps nuance.** No “only after sentence punctuation + space” option; no explicit URL/decimal guard; macro interaction only partially specified.
10. **Config schema.** No version, no migration, no platform block, no merge-on-import, no quarantine of corrupt files, no config validation API.
11. **FFI surface.** No CRUD for macros/dictionary, no config export/import, no app-decision query, no secure-input/layout notification, no macro-table version/revision. Only whole/partial config JSON and dictionary load.
12. **Performance.** Rust still uses `Vec` in hot paths; roadmap's `ArrayVec`/bitset work not done. No fuzz/proptest cross-language; conformance is strong but fixed.

### 5.2 Adapter / product gaps

- No macOS, Windows, Linux, iOS or Android adapter at all. C ABI exists, but no event tap, IMK, TSF, Fcitx5/IBus, hook, or keyboard extension.
- No menu bar/tray, OSD, settings UI, onboarding, permission flow, hotkey handling, app detection, secure-input detection, injection strategy, watchdog, packaging, signing, notarization, or auto-update.
- No real-app regression matrix or “known limitations” corpus (GõViệt's TSV is a good format).
- No adapter author documentation: keycode→character mapping, preedit vs replace semantics, target-process pinning, event marker, focus/cursor/field lifecycle, error policy.

### 5.3 Documentation gaps

- No versioned config specification.
- No charset specification/provenance/test vectors.
- No context/precedence spec.
- No user-facing migration guide from UniKey/OpenKey (macro file, settings).
- No security/privacy model (what is logged, what is stored, clipboard behavior).

---

## 6. Prioritized recommendations

### P0 — before the first real desktop adapter

**P0-E1. English restore v2 (engine, Python + Rust).**  
Add `restore_doubled_keys` (default on, like Funput) and a compact English word table. Preserve the raw key string across a revert, decide at the earliest safe point, and at word boundary. Required cases: `coffee`, `seemed`, `correct`, `password`, `office`, `pass`, `class`, `doses`, `casts`, `markers`, `pass1`; keep `off → of` as a documented limitation. Do not let it break abbreviations (`đc`, `đn`, `òh`) or macros. Add a separate `known_limitations.tsv` corpus so “fixes” do not silently change UniKey-compatible behavior.

**P0-E2. Adapter context contract (engine + FFI + docs).**  
Introduce, without breaking the existing ABI, a richer context call (new `vfk_focus_changed_ex` / `vfk_set_context_json`):
`app_id` (bundle id / exe), `window_title`, `field_kind`, `secure_input`, `input_source`/`layout`, `cursor_moved`, `target_process_id`.
Define and test precedence for the fields available now: hard exclusion → per-app override → smart-switch memory → global default; reserve a slot above per-app override for future window-title rules. Add `vfk_set_secure_input`, `vfk_notify_layout`, `vfk_app_decision_json` (explain why the current mode/strategy was chosen). Document that `focus_changed` flushes the current word and commits preedit.

**P0-E3. Config schema v1 + import/export (engine + FFI + docs).**  
Port Funput's portable/platform split:
- `schema`, `version`, `exportedAt`, `source`
- `preferences`: method, tone, spell, restore, macro, auto-caps, extra onsets, shortcuts
- `shortcuts[]`: trigger/expansion
- `platform.<os>`: hotkeys, app memory, excluded apps, per-app overrides
- merge semantics: macros by trigger, app memory union with local wins, missing fields keep local, unknown keys ignored, newer version best-effort, corrupt file quarantined.
Add `vfk_config_export_json`, `vfk_config_import_json`, `vfk_config_validate_json`, `vfk_config_schema_version`. Test every rule in §4.7.

**P0-A1. macOS reference adapter (replace mode).**  
Minimum viable shell: CGEventTap on its own thread, event marker, watchdog, secure-input guard, target-PID pinning, app bundle + field kind + layout detection, fast/slow/select-and-retype strategies, flush on focus/cursor change, permission guidance. No full settings UI required; a menu bar toggle + debug trace is enough to validate the engine end-to-end.

**P0-D1. Adapter author guide + conformance harness.**  
Document and test:
- event mask and pass-through rules; autorepeat; modifiers; non-BMP;
- action application semantics for PREEDIT/REPLACE/COMMIT (including `pass_key`);
- UTF-16 delete/cursor units;
- focus/cursor/field lifecycle;
- target process pinning and self-event marker;
- failure policy (panic → PASS + reset);
- per-app test matrix from §4.8.
Use GõViệt's `docs/TESTING.md` structure as a model, not its code.

### P1 — competitive product features

**P1-E1. Charsets and text tools.**  
Add an output encoding layer after `_out()` (not inside `transform`): Unicode NFC/NFD, TCVN3, VNI-Windows, CP1258, eventually VISCII. Implement Atom-pivot conversion with `normalized`/`unmapped` counters, `detect()`, round-trip inventory tests, and a CLI/FFI converter. Add case tools (UPPER/lower/Title/remove accents) with the edge cases in §4.6.

**P1-E2. Macro v2.**  
Add trigger flags (space/enter/tab/arrows), smart case + title case, add-space, max length, multi-line, undo-on-Backspace (with 4 s/next-key expiry), macro-in-English, and CRUD/import/export FFI. Adopt OpenKey's `trigger:content` UniKey import and VKey's editor UX. Keep macro expansion before English restore and typo correction.

**P1-E3. Per-app overrides and locks.**  
Extend `AppProfile`/config with `input_method`, `tone_style`, `charset`, `send_method`, `vietnamese_lock` (`none | hard_en | hard_vn | soft_en`), `macro_set`. Add VKey-style hard/soft semantics and game mode. Add app presets from §3.5 and a running-app picker.

**P1-E4. Spell-check UX and dictionary.**  
Add personal dictionary CRUD, exclusion prefixes, free-typing/English-bypass, instant-restore toggle, and dictionary import/export with licensing. Use a large Vietnamese syllable list and an English list from permissive sources; do not copy GPL dictionaries or XKey's GPL download.

**P1-E5. Advanced Telex / extra onsets / place names.**  
Implement `[`/`]` semantics faithfully (`][`→`ươ`, `[[`/`]]` escape), leading `w`→`ư` after onset, optional `z/f/j` onsets, coda `h/l/r` place-name support, and the `Krông`/`Chư Păn` cluster rules. Keep these behind explicit options; they interact with English restore.

**P1-A1. Windows adapter.**  
Hook-first with TSF as an optional backend. Implement app locks, CJK layout detection, game send mode, clipboard fallback, commit-undo decision table. Start with a pure C++/Rust “decision core” and unit tests before COM.

**P1-A2. Linux adapter.**  
Fcitx5 and IBus addons sharing the C ABI; preedit-first; settings JSON shared with a GTK app; inotify/mtime reload. Document Wayland limits.

**P1-D1. Settings UI and data page.**  
Sidebar groups: Overview, Typing, Apps, Macro, Tools, System, About. Include onboarding, hotkey recorder with presets, app picker, macro editor, dictionary editor, import/export with merge preview, reset scopes, debug trace, V/E/lock/secure/T status. Follow accessibility checklist in §3.6.

### P2 — differentiators and hardening

- **P2-E1. Typo correction** at word end (off by default, undoable, frequency prior, never change a valid word). Treat as experimental; measure on a large corpus before enabling.
- **P2-A1. macOS direct composition** (no underline) with document verification, drop/replay, click detection, and Backspace delegated to the app. This is Funput's best UX differentiator but requires careful IMK work.
- **P2-A2. Window-title / AX rules and browser per-domain rules.** Start with simple bundle+title `contains/regex`; add Google Docs/Notion/Figma presets. Browser extension can come later.
- **P2-A3. Remote desktop mode.** Passthrough by default; optional clipboard injection; document security implications.
- **P2-U1. OSD, floating indicator, toasts, debug window.** Keep one compact status surface; do not duplicate XKey's entire floating toolbar.
- **P2-U2. Backup/iCloud sync.** Only after config schema v1 and conflict/merge semantics exist.
- **P2-O1. Packaging, signing, notarization, auto-update.** macOS: Developer ID + notarization + Sparkle or custom appcast; Windows: Authenticode or package signature + SmartScreen guidance; Linux: distro packages. Stable signing identity is a TCC requirement, not a polish item.
- **P2-Q1. Performance/fuzz.** `ArrayVec`/bitset hot paths, allocation budgets, cross-language fuzz with seeds, benchmark gates in CI.
- **P2-D1. Documentation site** with adapter guide, config schema, charset provenance, test matrix, migration from UniKey/OpenKey.

### Suggested sequencing

1. **Sprint 1:** P0-E1 (doubled-key restore) + P0-E3 (config schema) + golden/conformance tests.
2. **Sprint 2:** P0-E2 (context contract) + P0-D1 (adapter guide/tests) + P0-A1 (macOS reference adapter).
3. **Sprint 3:** P1-E2 (macro v2) + P1-E3 (app overrides/locks) + P1-D1 (settings shell).
4. **Sprint 4:** P1-E1 (charsets/tools) + P1-A1 (Windows) / P1-A2 (Linux).
5. **Sprint 5:** P1-E4/E5 and P2 differentiators as user feedback dictates.

---

## 7. Licensing constraints

### 7.1 Project-by-project

| Project | License reality | What ViFastKey may do |
|---|---|---|
| **OpenKey** | GPL-3.0. Engine, macOS shell, Windows/Linux ports all GPL. | Read behavior/issues; write independent tests from observed behavior. Do **not** copy C++/Objective-C code, tables, resources, or strings into MIT code. A GPL-compatible fork is possible only if the whole distributed work is GPL. |
| **XKey** | `LICENSE` says MIT, but `VNEngine.swift` says “Direct port from OpenKey”, `VietnameseData.swift` says “Ported from OpenKey Vietnamese.cpp”, `ConvertTool.swift` says “Ported from OpenKey ConvertTool.cpp”. The spell dictionary is downloaded separately under GPL. | Treat as **GPL-derived / license-laundered** for engine and data. Do not copy code or tables even though the file says MIT. UX/feature observation is fine. Do not bundle the GPL dictionary. |
| **VKey** | GPL-3.0. Optional `vkey_engine.dll` is separate, PolyForm Noncommercial, with CC BY-NC data. `THIRD_PARTY_NOTICES.txt` documents Sciter BSD-3. README has an “AI Reference Policy” requiring credit for architectural logic. | Do not copy GPL code or the proprietary engine. Behavior/test ideas are fine. If adopting distinctive designs (game mode, commit-undo FSM, hook/TSF split), credit VKey and document inspiration. |
| **Funput** | MIT for code; data notices: SCOWL/ESDB (permissive notice), Google Books Ngram ordering (CC BY 3.0), LDNOOBW (CC BY 4.0), Vietnamese syllable list hand-written MIT. | MIT code is compatible, but prefer independent implementation to keep provenance clean. If reusing data, carry the exact notices and comply with CC BY attribution. Do not assume all `data/` is MIT. |
| **GõViệt** | MIT, clean-room; `docs/ATTRIBUTION.md` explicitly says it only read OpenKey issues, not code. | Safest source for architecture/test ideas. Still write your own code; preserve MIT notice if any code is reused. |
| **UniKey** | Reference behavior; source licensing must be checked before any use. | Use only de-facto behavior and public documentation. Do not copy source/tables. |
| **Charset tables** | TCVN3/VNI/CP1258 mappings are factual but published tables may have arrangement copyright. | Build from public standards/vendor documentation, generate tables with scripts, keep provenance notes, test round-trip. Do not lift OpenKey/VKey/Funput tables wholesale. |

### 7.2 Practical rules for ViFastKey

1. **Behavior is not code.** You can implement “`coffee` should survive doubled-key restore” from observation and public docs; you cannot paste Funput's `doubled` module or OpenKey's engine.
2. **Keep a provenance log.** For each borrowed behavior, record the source URL/issue and write an independent test. This is also good engineering: it makes regressions traceable.
3. **Do not copy word lists.** XKey's GPL dictionary and VKey's CC BY-NC engine data are traps. Build or license a list with clear terms; keep notices in `THIRD_PARTY_NOTICES`.
4. **Do not copy UI strings.** Vietnamese UX text is copyrighted expression. Rewrite in ViFastKey's voice.
5. **Do not copy test corpora wholesale.** Test *cases* (key sequence → expected output) are factual, but a large curated TSV may be a copyrightable compilation. Recreate a smaller corpus, cite the source of each behavior, and expand independently.
6. **Respect VKey's AI policy.** If ViFastKey adopts game mode, hard/soft locks, commit-undo boundaries, or the TSF split, add a credit line in `ATTRIBUTION.md` and README. This is cheap and avoids an avoidable dispute.
7. **XKey is not an MIT safe harbor.** The MIT file does not cure the GPL provenance of a direct port. Treat the engine as GPL; do not use it in an MIT product.
8. **If ViFastKey ever wants GPL code**, the clean path is a separate GPL-compatible component with a clear boundary and lawyer review — not a copy into the MIT core.

---

## Appendix A — source map (for follow-up reading)

| Topic | File |
|---|---|
| OpenKey feature list / history | `tuyenvm_OpenKey/README.md`, `CHANGELOG.md` |
| OpenKey engine behavior | `tuyenvm_OpenKey/Sources/OpenKey/engine/Engine.cpp`, `DataType.h`, `Macro.cpp`, `ConvertTool.cpp` |
| OpenKey macOS tap / app switch | `.../macOS/ModernKey/OpenKey.mm`, `AppDelegate.m`, `OpenKeyManager.m` |
| XKey feature/UX | `xmannv_xkey/README.md`, `XKey/UI/PreferencesView.swift`, `XKey/UI/SettingsSections/*` |
| XKey context/injection | `XKey/Shared/AppBehaviorDetector.swift`, `Shared/InputSession/AppPolicyRuntime.swift`, `XKey/EventHandling/*` |
| VKey product/guide | `phatMT97_VKey/README.md`, `docs/GUIDE.md`, `docs/MACRO_CASING.md`, `docs/BROWSER_EXTENSION.md`, `docs/SECURITY.md` |
| VKey config/engine | `src/core/config/TypingConfig.h`, `src/core/PerAppModeDecision.h`, `src/core/pipeline/*`, `tests/*` |
| Funput behavior docs | `Funput_Funput/docs/features/*.md`, `platforms/CONFIG_FORMAT.md` |
| Funput config/data | `crates/funput-config/src/settings/*`, `crates/funput-suggestions/data/*/NOTICE.md` |
| GõViệt architecture/tests | `nguyenrot_goviet/CLAUDE.md`, `docs/TESTING.md`, `docs/ATTRIBUTION.md`, `rust/engine/tests/corpus/*.tsv` |
| ViFastKey baseline | `python/README.md`, `python/EDGE_CASES.md`, `python/vfk_engine/*`, `rust/crates/vfk-engine/src/ports.rs`, `rust/crates/vfk-ffi/include/vifastkey.h` |

---

*End of report.*
