> **Báo cáo nghiên cứu nội bộ.** ViFastTts là dự án TTS, không phải IME; báo cáo này tách
> rõ phần quan sát được từ code và phần suy luận cho adapter macOS. Không có model, dữ liệu
> huấn luyện, audio, từ điển hay asset thương hiệu nào được sao chép. Code ViFastTts là MIT,
> nhưng model/dataset có điều kiện riêng.

# ViFastTts → ViFastKey: transferable engineering and macOS adapter lessons

**Status:** research report, not a change proposal  
**Date:** 2026-10-29  
**Source:** `repo ViFastTts` at commit `8dbd73b` (the working tree also has uncommitted demo/docs changes; all paths below refer to the working tree as read)  
**Target:** `repo ViFastKey` at commit `f8abab2`  
**Scope:** repository/engineering practices, desktop/macOS architecture, macOS IME adapter design, diagnostics/UX, test strategy, licensing, and a prioritized implementation plan.

---

## 0. Executive summary

ViFastTts is a useful reference for **how to engineer a cross-language engine and ship it as a desktop/mobile product**. It is **not** a reference implementation of a macOS input method. Its “desktop” is a single-window `egui` TTS app written in Rust (`rust/crates/adapters/inbound/desktop/src/main.rs`), and the repository contains no IMK input controller, `CGEventTap`, Accessibility API, Input Monitoring, secure-input handling, `NSPasteboard` injection, menu-bar lifecycle, TCC onboarding, notarization, or universal build. Those must be designed fresh for ViFastKey.

The highest-value transferable lessons are:

1. **Contract-first porting with explicit gates.** ViFastTts writes the contract, golden corpus, and phase exit criteria before porting (`docs/RUST_PORT.md` §0, §5, §13). ViFastKey already has an excellent Python-oracle/Rust-conformance culture; extend it to the **adapter boundary**, not just the engine.
2. **Keep the engine pure and put the OS in adapters.** ViFastTts separates `text` / `audio` / `ports` / `engine` / `native` and verifies boundaries with `cargo tree` (`docs/RUST_PORT.md` §17). ViFastKey already has `vfk-core` / `vfk-engine` / `vfk-ffi`; the Swift layer must only translate `Action` into IMK/CGEvent calls.
3. **A headless command/event layer is testable; UI direct-binding is not.** `vifasttts-app` turns commands into events and owns cancellation/ETA (`rust/crates/core/application/app/src/session.rs`). ViFastKey should do the same for `AdapterCommand` / `AdapterEvent`, with fake IMK/AX/tap clients.
4. **IMK first, event tap second.** The default macOS adapter should be an `IMKInputController` using marked text/preedit. `CGEventTap` + synthetic events should be an explicit compatibility mode because it brings Input Monitoring/Accessibility TCC, secure-input failures, app-specific backspace bugs, and notarization/sandbox complications.
5. **Stable code signing is a prerequisite for permission testing.** ViFastTts ships an ad-hoc-signed, non-notarized `.app` and tells users to right-click Open (`desktop/README.md:42-43`, `docs/INSTALL.md:46-66`, `rust/crates/adapters/inbound/desktop/package-macos.sh:91-93`). That is acceptable for a TTS demo but unacceptable for an IME whose TCC grants are bound to signing identity.
6. **Package the IME as an input-method bundle, not as a drag-to-Applications app.** ViFastTts’s `.app` layout is a good model for bundle structure, `Info.plist`, icon generation, licence folder, and `ditto` packaging, but an IME must live in `~/Library/Input Methods` (or `/Library/Input Methods`), with IME-specific `Info.plist` keys and a `.pkg`/installer flow.
7. **Config must be typed, versioned, validated, atomic, and fail-safe.** ViFastTts’s `AppSettings`/`SettingsService` (`rust/crates/core/application/app/src/settings.rs`) is a good model for typed defaults, `serde(default)`, and validation, but its desktop save path is a non-atomic `fs::write` with ignored errors (`desktop/src/main.rs:264-272`). Do not copy that part.
8. **Diagnostics should be a first-class command, not a support conversation.** ViFastTts’s `--smoke` loads the model, synthesises, checks RIFF/non-empty WAV, and exits non-zero on failure (`desktop/src/main.rs:91-135`); CI runs it on the packaged artifact (`ci.yml:165-169`). ViFastKey needs an equivalent `--self-test` plus a richer `--doctor` for permissions, secure input, input source, signing, config, and tap state.
9. **Test in tiers: unit, golden/adapter conformance, property/fuzz, real-app matrix, packaged smoke.** ViFastTts documents this explicitly (`docs/RUST_PORT.md:693-716`) and uses scoped, named exceptions instead of loosening global thresholds (`docs/handoff/EXCEPTIONS.md`). ViFastKey already has 13,311 + 3,000 conformance cases in CI; add adapter fixtures and real-app tests rather than more engine-only tests.
10. **Borrow code/architecture patterns only, never TTS model/data/brand.** TTS code is MIT, but its sample model is research/non-commercial (dataset CC BY-NC 4.0) and brand assets are explicitly outside MIT (`README.md:145-147`, `docs/NGOC_HUYEN.md:3-9`, `resources/brand/README.md`). Do not copy `artifacts/`, `runs/`, `data/`, `.vfa/.vfts`, pronunciation dictionaries, demo audio, or logos.

The single most important adapter-specific finding from reading ViFastKey itself is that **app identity is currently process/path-oriented, not bundle-ID-oriented**. `resolve_profile` lowercases and takes the last path component, then strips `.app` (`rust/crates/vfk-engine/src/ports.rs:200-211`). Passing `com.google.Chrome` yields `com.google.chrome`, which does not match the built-in `chrome` profile. `excluded_apps` and `app_modes` also compare the raw string (`session.rs:368-403`). The macOS adapter must canonicalise to bundle IDs, and the engine must support bundle-ID aliases; otherwise per-app preedit/delay/exclusion silently does nothing.

---

## 1. Scope, evidence, and an important correction

### 1.1 What ViFastTts is

ViFastTts is a Vietnamese single-speaker TTS toolkit:

- Python frontend/model/training/export under `src/vifasttts/`.
- Rust inference workspace under `rust/` (25 crates) with `text`, `audio`, `ports`, `engine`, `native`, `browser`, `ffi`, `http-server`, `cli`, `desktop`, `tui`, and tools (`docs/RUST_PORT.md:901-945`).
- Desktop app is `egui` + `rodio`, local ONNX or remote HTTP (`desktop/README.md:3-13`).
- Mobile is Android Java + iOS SwiftUI, both using the C ABI.
- CI builds Rust on macOS/Ubuntu/Windows, builds web/extension, and packages desktop/CLI/mobile only on tags (`ci.yml:18-410`).

It is a mature example of:

- a Python oracle + Rust port with conformance/golden tests;
- a ports/adapters architecture with a headless application layer;
- C ABI and Swift/Kotlin/Java wrappers;
- tag-driven multi-platform packaging with smoke tests;
- model bundle integrity (manifest, SHA256, zip-bomb/path-traversal defence);
- extensive, honest documentation of limitations.

### 1.2 What ViFastTts is not

A repository-wide search found no implementation of:

- `IMKServer` / `IMKInputController` / `NSTextInputClient`;
- `CGEventTap`, `CGEventPostToPid`, synthetic event markers;
- `AXIsProcessTrusted`, `AXUIElement`, Accessibility/Input Monitoring TCC;
- `IsSecureEventInputEnabled` / secure input handling;
- `NSPasteboard` text injection or clipboard restore;
- `NSStatusItem` / menu-bar app lifecycle;
- macOS notarization, hardened runtime, universal2, Sparkle auto-update, crash reporting.

The desktop README says this directly: no system tray, no native menu, no auto-update, no signing/notarization (`desktop/README.md:101-102`). The macOS package is ad-hoc signed (`desktop/README.md:42-43`, `package-macos.sh:91-93`) and named by host architecture (`package-macos.sh:96`).

Therefore:

- Sections 2–3 describe **observed TTS practices**.
- Section 4 describes **platform lessons inferred for ViFastKey**, not TTS code to copy.
- The closest reusable Swift pattern is the iOS TTS app (`mobile/ios/ViFastTtsMobile/TtsEngine.swift`), which is a thin wrapper around the C ABI, not an IME.

### 1.3 Evidence labels

Where useful, claims are marked:

- **[Observed]** directly visible in TTS code/docs.
- **[Inference]** platform behaviour or a recommendation derived from the evidence.
- **[ViFastKey gap]** a current ViFastKey limitation found while mapping the lesson.

---

## 2. Repository and engineering practices worth copying

### 2.1 Contract-first porting, phase gates, and stop rules

ViFastTts’s strongest process document is `docs/RUST_PORT.md`. It starts with conclusions before implementation details (`:16-40`), defines what is in/out of scope (`:44-81`), then orders the work:

```text
P0 contract + golden corpus
  ├── P1 runtime (numeric risk)
  └── P2 frontend (semantic risk)
        └── P3 end-to-end
              └── P4 integration
                    └── P5 cutover
```

The document explicitly says: **do not start with the server/web; start with the contract and golden corpus** (`:23-24`), and each phase has an exit gate and a “stop if” condition (`:423-467`, `:470-511`, `:514-572`, `:576-598`, `:601-647`). The risk table has a general rule: if a gate fails for a reason that cannot be explained in one day, stop that branch and write a report; do not loosen the threshold to move on (`:740-761`).

The project-level `AGENTS.md` repeats the principle: **do not upgrade anything before there is a measurement, and verify the measurement on real data before using it to decide** (`AGENTS.md:18-19`). It also requires every change to run lint/tests and report negative results honestly (`AGENTS.md:76-93`).

**ViFastKey takeaway:** write `docs/MACOS_ADAPTER.md` with:

- exact scope: IMK only vs IMK + event-tap fallback;
- contract: `Action` → `IMKTextInput` calls, and `Action` → event-tap injection;
- golden corpus: key sequences + contexts + expected calls/deletions;
- gates: compile, unit, fixture parity, real-app matrix, packaged self-test, signing/notarization;
- stop rules: any lost/duplicated character, any processing in secure input, any TCC reset after update, any unexplained fixture mismatch.

Do not start by writing a global event tap because “UniKey does it”. TTS’s ordering lesson is to first make the smallest measurable path correct.

### 2.2 Documentation as a system, not a pile of files

ViFastTts has an unusually disciplined docs hierarchy:

- `docs/README.md` is the single table of contents and defines precedence: `TRANG_THAI.md` (status) → `docs/README.md` (index) → current topical docs → reference docs → `docs/archive/` (historical, not for decisions) (`docs/README.md:4-6`).
- `docs/handoff/TRANG_THAI.md` is the **only** current-status file; it starts with the current round, spec file, code commit, and status (`:1-6`), then a five-line non-technical summary (`:8-14`), production values/hashes (`:16-42`), next actions (`:120-127`), and update conventions (`:130-138`).
- `docs/handoff/DECISIONS.md` is append-only and code-addressed (`V<NN>.A<k>`), not a second status file.
- `docs/handoff/EXCEPTIONS.md` records scoped exceptions: one input, one layer, one mode, one provider, exact hashes, expiry conditions (`:1-14`, `:52-80`). It explicitly says an exception is not “pass everything”.
- `docs/archive/README.md` keeps superseded documents instead of deleting them; archive docs are marked historical and not used for decisions (`docs/README.md:114-137`).

**Why it works:** a new contributor can tell whether a document is current, reference, or history; status cannot drift across five files; every exception has an expiry; every decision has a code.

**ViFastKey takeaway:** adopt a lighter version, not TTS’s full multi-agent ceremony:

- `docs/README.md` — index and precedence.
- `docs/PROJECT_STATUS.md` or keep `ROADMAP.md` as the single status source; do not create parallel status files.
- `docs/DECISIONS.md` — append-only ADR log for macOS choices (`IMK-first`, minimum macOS, Intel support, sandbox, update mechanism).
- `docs/MACOS_EXCEPTIONS.md` — per-app/version workarounds with expiry, e.g. “Chrome 1xx address bar uses preedit; remove when fixed”.
- `docs/archive/` for superseded research.

### 2.3 Ports/adapters, crate boundaries, and “engine knows nothing about the OS”

ViFastTts’s Rust workspace is deliberately layered (`docs/RUST_PORT.md:183-260`, `:1299-1378`):

```text
text (domain, no fs/ORT) ─┐
audio (DSP, no fs/ORT)  ──┼──► engine (async orchestration)
ports (traits/types)    ──┘         ▲
                                   │
                    ort (native) / ort-web (browser)
                                   ▲
                    native / browser composition roots
                                   ▲
        cli | http-server | ffi | desktop | tui | mobile
```

Rules documented and checked with `cargo tree` (`:1477-1499`):

- `text` has no `std::fs`, no ORT, no adapter.
- `audio` only FFT/DSP, no text/engine/ORT.
- `ports` depends on no workspace crate.
- `engine` depends on `text` + `audio` + `ports`; it does not know ORT or the filesystem.
- `ort`/`ort-web` implement ports; they do not depend on engine.
- Only `native`/`browser` wire engine + backend.
- Adapters do not contain chunk/parse/model logic.

This is a direct template for ViFastKey’s macOS work:

```text
vfk-core / vi-syllable     pure Vietnamese logic
        │
vfk-engine (ImeSession, Action, Config, Ports)
        │
vfk-ffi (C ABI, UTF-16, catch_unwind, last_error)
        │
macOS adapter (IMKInputController, AX context, event tap)
```

The adapter may own:

- app/field context collection;
- mapping `Action` to `setMarkedText` / `insertText` / `replacementRange` / pass-through;
- permission and secure-input gates;
- menu bar, onboarding, preferences, OSD;
- diagnostics and logging;
- config persistence.

The adapter must **not** own:

- Telex/VNI parsing, tone placement, macro matching, charset conversion, spelling, or undo semantics;
- a second implementation of “which key means which tone”;
- direct mutation of engine internals.

**ViFastKey gap:** the current `docs/ADAPTER_GUIDE.md` already states this contract (`:7-28`, `:94-101`) and already warns about macOS specifics (`:116-122`). The missing piece is a **testable adapter layer**, not more prose.

### 2.4 Python/Rust boundary and generated artifacts

ViFastTts keeps Python for what Python is good at and Rust for runtime:

- Python remains the training/data/export/eval environment; Rust is the inference/edge runtime (`docs/RUST_PORT.md:18-31`, `:66-81`).
- The frontend data is generated from Python into Rust resources by `tools/export_rust_resources.py`, then embedded with `include_str!` (`docs/RUST_PORT.md:266-275`, `tools/export_rust_resources.py`).
- `tools/rust_parity.py` generates/compares normalized text, parsed structures, 7 IDs, chunks/pauses, and audio hashes; it supports `--matrix`, `--fuzz`, `--reuse`, `--full-custom`, and reports the first divergence (`tools/rust_parity.py:1-30`, `:200+`).
- The parity report documents named exceptions rather than hiding mismatches (`docs/RUST_PORT.md:953-976`).

ViFastKey already has the right shape:

- Python is the oracle (`README.md:20-22`).
- Rust is the port, gated by `conformance/core.jsonl` (13,311 cases) and `engine.jsonl` (3,000 scenarios).
- `make conformance` regenerates the oracle and syncs it into Rust (`Makefile:25-27`).
- CI regenerates and fails if committed oracle/data differs (`ci.yml:32-57`).

**What ViFastKey should add:** generate **adapter fixtures** from the same oracle, not hand-written Swift expectations. A Python/Rust reference exporter should emit, for each scenario:

- config and mode (preedit/replace);
- app identity (bundle ID, localized name, field kind);
- surrounding text/selection/focus events;
- key sequence;
- engine `Action` transcript;
- abstract adapter calls (`setMarkedText`, `insertText`, `replaceRange`, `deleteBackward`, `passThrough`, `commit`);
- UTF-16 delete/cursor values for IMK and grapheme delete count for synthetic backspace.

Then Rust and Swift tests consume the same JSONL. This preserves the “Python oracle” rule while testing the OS boundary.

### 2.5 Reproducibility and version pinning

ViFastTts pins the important moving parts:

- Rust toolchain `1.99.1` with `rustfmt` + `clippy` (`rust/rust-toolchain.toml:1-4`).
- ORT `1.31.1` in model manifests and packaging (`rust/models/n0-q.manifest.json`, `docs/RUST_PORT.md:316-338`).
- Model download SHA256 in a composite action, with cache and retry (`/.github/actions/fetch-model/action.yml:7-27`).
- wasm-bindgen CLI `0.2.129` in CI (`ci.yml:91-94`).
- Workspace version `0.1.1` in `rust/Cargo.toml:31-35` and `pyproject.toml:7`.
- `Cargo.lock` committed.

**ViFastKey takeaway:**

- Add a `rust-toolchain.toml` or pin the toolchain in CI; current ViFastKey CI uses `dtolnay/rust-toolchain@stable` (`ci.yml:71`) while `rust-version = 1.75` (`rust/Cargo.toml:11`). Pin exact versions for reproducible adapter builds.
- Pin Xcode version and XcodeGen version in CI; `xcodebuild -version` is not enough if the project generator changes.
- Generate an `engine_manifest.json` containing `engine_version`, `abi_version`, `config_schema_version`, `git_commit`, `rustc`, target, build date. Show it in diagnostics and assert it in self-test.
- Derive `CFBundleShortVersionString`/`CFBundleVersion` from one source; do not hardcode them in `Info.plist` as TTS does (`package-macos.sh:71-89`).

### 2.6 CI matrix and release packaging

ViFastTts CI has a sensible shape (`ci.yml`):

- `rust`: fmt, clippy `-D warnings`, test on macOS/Ubuntu/Windows, plus wasm checks (`:18-58`).
- `python-lint`: `ruff` only (`:60-72`) — **not** the full pytest suite. The docs explain the full suite is local because torch/onnx are heavy (`docs/RUST_PORT.md:1155-1156`).
- `web-extension`: build wasm + ORT Web + extension zip + web dist (`:74-115`).
- `desktop`: only on tags/manual; package macOS `.app` zip, Linux tar.gz, Windows zip; run `--smoke` from each package (`:117-185`).
- `cli`: same tag-only pattern; run `--help` from package (`:187-244`).
- `model-vfts`: pack/inspect `.vfts`/`.vfa` and upload (`:246-277`).
- `android`: APK build on tag/dispatch (`:279-344`).
- `ios`: simulator build (`:346-382`).
- `release`: download all artifacts, create/update GitHub Release, `--generate-notes`, mark `-rc`/`-test` as prerelease (`:384-410`).

Strengths worth copying:

- tag-only expensive packaging, PRs stay fast;
- `fail-fast: false` so one OS failure does not hide others;
- `if-no-files-found: error`;
- smoke test the **packaged artifact**, not just the build tree;
- pinned model hashes;
- separate artifacts per platform;
- prerelease detection for `-rc`/`-test`.

Weaknesses **not** to copy:

- Full pytest/conformance is not run in TTS CI; ViFastKey already runs it, so keep it.
- The release job uses `if: always()` (`ci.yml:386`), so a failed desktop/mobile job can still publish a partial release. ViFastKey should require all build/sign/notarize jobs to succeed, or publish only explicitly selected assets.
- Global `permissions: contents: write` (`ci.yml:10-11`); ViFastKey should scope permissions per job/environment, especially for signing/notarization.
- macOS is host-arch only and ad-hoc signed; no Intel/universal, no notarization.
- No conformance/parity drift job in TTS CI, despite the docs emphasizing parity. ViFastKey’s `conformance` job is stronger.

**Proposed ViFastKey CI jobs:**

| Job | Trigger | What it does |
|---|---|---|
| `python` | PR/push | pytest 3.10–3.13 + ruff (already) |
| `conformance` | PR/push | export oracle, sync Rust, `git diff --exit-code` (already) |
| `rust` | PR/push | fmt, clippy, test on 3 OS (already) |
| `macos-adapter` | PR/push | XcodeGen, unsigned `xcodebuild`, Swift unit tests with fakes, `--self-test`, `lipo -info`, `plutil -lint` |
| `macos-release` | tag `v*` | universal2 build, Developer ID sign, notarize, staple, `spctl`, package `.pkg`/`.dmg`, upload |
| `macos-nightly` | schedule/manual | real-app UI smoke matrix, fuzz, long stress, benchmarks |
| `licenses` | PR/tag | `cargo-deny`/`cargo-about`, notices in bundle |
| `release` | tag | require all build jobs, checksums, SBOM, release notes, verify assets by downloading them |

### 2.7 Versioning and release notes

TTS uses a workspace version (`rust/Cargo.toml:31-35`), Python version (`pyproject.toml:7`), tags `v0.1.0`/`v0.1.1`, and a date-based `CHANGELOG.md`. The macOS package hardcodes `CFBundleShortVersionString 0.1.1` and `CFBundleVersion 2` (`package-macos.sh:82-83`), which is a drift risk. The README even warns that UI changes on `main` are not in the `v0.1.1` asset (`README.md:40-43`).

**ViFastKey takeaway:**

- One `VERSION` source; scripts update/generate `Info.plist`, docs, and `--version` output.
- `CFBundleShortVersionString` = semantic version (e.g. `0.4.0`).
- `CFBundleVersion` = monotonically increasing integer (CI run number or commit count), required by Sparkle/App Store.
- Tag must match `CFBundleShortVersionString`; CI fails otherwise.
- Release notes from `CHANGELOG.md`, not auto-generated only.
- After publishing, download the asset and verify SHA256, signature, notarization ticket, and `--self-test`; TTS’s own `AGENTS.md:83-89` requires upload verification because “said uploaded but forgot to run it” happened before.

### 2.8 What not to copy from TTS engineering

| Anti-pattern | TTS evidence | Why it matters for ViFastKey |
|---|---|---|
| Ad-hoc signing for a permissioned app | `package-macos.sh:91-93`, `desktop/README.md:42-43` | TCC grants are tied to signing identity; rebuilds/updates will re-prompt or silently fail |
| No notarization | `desktop/README.md:101-102`, `docs/DISTRIBUTION.md:42-47` | Gatekeeper right-click Open is unacceptable for an IME; users may not get permission prompts at all |
| Host-arch-only package | `package-macos.sh:96`, `docs/RUST_PORT.md:1013-1020` | Intel Macs need universal2 or a clear arm64-only decision |
| `.app` zip + drag to Applications | `docs/INSTALL.md:46-66` | An IME must be in `~/Library/Input Methods`; a regular `.app` in `/Applications` is not an input source |
| Non-atomic settings save with ignored errors | `desktop/src/main.rs:264-272` | Corrupt config or crash during save can lose macros/profiles; use atomic write + backup |
| Hardcoded versions in plist | `package-macos.sh:71-89` | Drift between binary, plist, tag, and update feed |
| No tray/menu/auto-update | `desktop/README.md:101-102` | Not acceptable for a daily-use macOS IME; status item, preferences, and update path are product requirements |
| CI skips full pytest/parity | `ci.yml:60-72` | ViFastKey already does better; do not regress while adding Xcode tests |
| Release `if: always()` | `ci.yml:384-410` | Partial releases with missing/unsigned artifacts |
| Broad ATS exception on iOS | `mobile/ios/ViFastTtsMobile/Info.plist:23-27`, `project.yml:34-35` | If ViFastKey ever does networking, allow only local networking; default should be no network |
| `--deep` for signing | `package-macos.sh:91-93` | Sign nested code inside-out; use `--deep` only for verification, not signing |

---

## 3. Desktop/macOS architecture observed in ViFastTts

### 3.1 App lifecycle

The desktop entry point is small and intentional (`desktop/src/main.rs:39-66`):

- `--smoke` is checked **before** creating the window; it runs a headless load/synthesise/WAV check and returns.
- Settings are loaded before the UI.
- The window icon is embedded from the shared brand asset.
- Fonts are installed because egui’s default font lacks full Vietnamese glyph coverage (`:68-88`).

The app uses an `eframe::App` update loop (`:1594-1687`) and a `Drop` implementation that stops the internal server and saves dirty settings (`:1682-1687`). Synthesis runs on a worker thread with `mpsc` commands/events (`:334-395`); the UI never blocks on model loading/synthesis.

**What to copy:**

- A headless entrypoint before UI (`--self-test`, `--doctor`) so CI and support can run it without a GUI session.
- Lifecycle cleanup: stop the event tap/IMK server, commit/cancel composition, flush config on `applicationWillTerminate` and `deactivateServer`.
- A worker/serial queue for long work (updates, diagnostics export, maybe AX queries), but **not** for the per-key transform.
- Embedded fonts/brand assets so the bundle is self-contained.

**What not to copy:**

- `egui` is not an IME app framework; macOS IME needs AppKit `NSApplication`, `IMKServer`, `NSStatusItem`, and AppKit/SwiftUI preferences.
- TTS’s `Drop` is Rust-specific; Swift uses `applicationWillTerminate` + `deinit`, and IMK’s `deactivateServer`/`inputControllerWillClose` are the important composition hooks.
- TTS does not handle sleep/wake, fast user switching, session resign, or input source changes.

### 3.2 Settings, preferences, and config persistence

TTS has two layers:

1. `vifasttts-app::AppSettings` — typed, `#[serde(default)]`, validated (`settings.rs:25-71`), stored under a versioned key `vifasttts_settings_v1` (`:6`), loaded via a `SettingsStore` port (`:74-106`). Unknown/missing fields get defaults; validation rejects non-finite/out-of-range speed/alpha and remote mode without a URL.
2. `DesktopSettings` — wraps `AppSettings` plus desktop-only paths, dialect, output dir, server port/LAN/token, ETA history (`desktop/src/main.rs:141-182`). `load_settings` resolves model paths and `save_settings` writes pretty JSON (`:242-272`).

The UI shows the **resolved file path** that will actually be loaded, not the repo-relative configured path (`desktop/README.md:82-83`, `main.rs:184-214`). There are tests for:
- existing path wins;
- fallback to `<exe>/models`;
- fallback to macOS `Contents/Resources`;
- missing path returns original (`main.rs:1706-1742`).

**What to copy:**

- Typed config + `schema_version` + `serde(default)`/unknown-field tolerance.
- Validation before save; show field-level errors.
- Storage behind a `ConfigStore` port, with a memory store in tests (`settings.rs:109-152`).
- Versioned storage key and migration tests.
- Resolved-path display and “reveal in Finder”.
- ETA/history only as a diagnostics nicety, capped (TTS uses 24 samples, `HISTORY_LIMIT`).

**What to fix:**

- TTS’s `save_settings` ignores errors and writes non-atomically (`main.rs:264-272`). For ViFastKey:
  - write to `config.json.tmp`, `fsync` if appropriate, `rename`/`replaceItemAt`;
  - keep `config.json.bak`;
  - on parse failure, rename to `config.bad.<timestamp>.json`, load defaults, show a banner with “Restore backup”;
  - debounce writes (typing preferences should not hit disk per keystroke);
  - flush on `applicationWillTerminate`, `applicationDidResignActive`, and input source deactivation.
- Do not store tokens/secrets in the config if ViFastKey has no network; if update signing keys ever exist, use Keychain, not JSON.

### 3.3 UI state machine and status/error presentation

TTS uses explicit event enums:

- `BackendEvent::{ModelLoading, ModelReady, ModelError, Info, Started, Finished, Failed}` (`desktop/src/main.rs:310-319`);
- `AppEvent::{Started, Units, AudioChunk, Progress, Finished, Error}` (`app/src/session.rs:30-52`);
- the UI polls events and renders status/errors; the iOS app uses `status`/`error` state and `localizedDescription` (`ContentView.swift:6-17`, `:132-145`, `:238-306`).

This is much better than stringly-typed UI state. For ViFastKey, define one `AdapterState` enum used by the menu bar, onboarding, preferences, diagnostics, and OSD:

```swift
enum AdapterState {
  case starting
  case needsInstall                 // app not in ~/Library/Input Methods
  case needsInputSource             // installed but not selected
  case ready
  case secureInput                  // paused, not an error
  case needsAccessibility           // event-tap/AX mode only
  case needsInputMonitoring         // event-tap mode only
  case tapDisabled(reason, retries)
  case engineError(code, message)
  case configError(path, backupPath)
}
```

Each state has:

- a menu-bar icon/tooltip;
- a one-line status in preferences;
- a primary action button (Open Settings, Install, Restart, Reveal Config, Copy Diagnostics);
- a diagnostics code.

### 3.4 FFI and Swift wrapper

TTS’s C ABI and iOS wrapper are directly reusable patterns:

- All fallible FFI entry points are guarded by `catch_unwind`; errors go to a thread-local `vifasttts_last_error` (`ffi/src/lib.rs:22-37`, `:150-162`).
- New options are added via `_ex` functions instead of changing the old struct (`ffi/src/lib.rs:128-148`), preserving ABI for existing clients.
- Engine handles are raw pointers with explicit free; buffers/strings have `buffer_free`/`string_free` (`:416-500`).
- The iOS wrapper is a `final class` with a serial `DispatchQueue`, copies returned data before freeing pointers, persists the active model path in `UserDefaults`, and falls back to the bundled model if the stored path is missing (`TtsEngine.swift:23-49`, `:57-139`, `:203-220`).
- The bridging header includes the **canonical** Rust header instead of copying the contract (`ViFastTtsMobile-Bridging-Header.h:1-7`).

**ViFastKey takeaway:**

- Add `vfk_abi_version()` and `vfk_engine_version()`; expose them in `--doctor`.
- Keep `VfkAction` layout stable; if it must change, add a new function/struct, not an in-place break.
- In Swift, wrap `VfkSession` in a `final class` with `deinit { vfk_session_free(handle) }`; copy `text` immediately because it is session-owned until the next call.
- Include the canonical `vifastkey.h` from Swift; do not duplicate the struct in Swift.
- Prefer **static linking** of the Rust engine for the first adapter. ViFastKey’s engine is small and pure Rust; unlike TTS’s 86 MB ONNX + 32 MB ORT, there is no need for a dynamic library, `@rpath`, or `libonnxruntime` signing. Static linking removes a whole class of bundle/signing failures.
- Do **not** copy the serial-queue/async model for the key path. TTS synthesises for seconds; ViFastKey’s transform is microseconds (`python/PORTING.md:90-97`). Run the engine synchronously on the IMK/main thread, keep AX/tap work off the hot path, and never block key delivery on disk/network.

### 3.5 macOS packaging observed

`package-macos.sh`:

- Requires model files and ORT, fails early with clear messages (`:22-42`).
- Builds `vifasttts-desktop` and `vifasttts` release binaries (`:44-45`).
- Creates `ViFastTts.app/Contents/MacOS` and `Contents/Resources/licenses` (`:48`).
- Copies a sidecar CLI/server, web assets, ORT dylib, model files, font licence (`:49-59`).
- Builds `AppIcon.icns` from the shared 512 px brand image with `sips` + `iconutil` (`:60-69`).
- Writes `Info.plist` with bundle ID/name/icon/package type/version/min OS (`:71-88`).
- Ad-hoc signs the app (`:91-93`).
- Zips with `ditto -c -k --sequesterRsrc --keepParent` (`:96-100`).

**Reusable:** bundle layout, icon generation, `ditto`, licences folder, early validation, sidecar pattern (if needed), `--smoke` from the packaged app.

**Must change for an IME:**

- Bundle goes to `~/Library/Input Methods/ViFastKey.app` (per-user) or `/Library/Input Methods` (all users), not `/Applications`.
- `Info.plist` needs IME keys:
  - `LSUIElement` (status item, no Dock icon);
  - `NSPrincipalClass` (AppKit app);
  - `InputMethodConnectionName` (unique reverse-DNS);
  - `InputMethodServerControllerClass` (Swift class exposed with `@objc(...)`);
  - `tsInputMethodCharacterRepertoireKey` (`["vi"]`);
  - `CFBundlePackageType=APPL`, stable `CFBundleIdentifier`, versions, min OS;
  - localised `InfoPlist.strings`.
- Use a `.pkg` installer or an in-app “Install” flow; a `.dmg`/`.zip` drag to Applications is wrong for an IME. Homebrew Cask can install a `.pkg`.
- Universal2 + Developer ID + hardened runtime + notarization + staple.
- Include `LICENSE`, `THIRD_PARTY_NOTICES`, font OFL, and Rust dependency notices; TTS only copies the font licence (`:59`).
- Sign nested code inside-out; avoid `--deep` for signing.

### 3.6 What TTS does not demonstrate for macOS

| Area | TTS status | ViFastKey implication |
|---|---|---|
| IMK input controller | absent | design and test from scratch |
| IME `Info.plist` keys | absent | add all required keys; test install/selection |
| Menu-bar lifecycle | absent | `NSStatusItem`, `LSUIElement`, settings window |
| TCC permissions | absent | onboarding, doctor, stable signing |
| Accessibility/AX | absent | field kind, secure field, surrounding text |
| Event tap | absent | fallback only; lifecycle/recovery |
| Secure input | absent | mandatory pass-through gate |
| NSPasteboard injection | absent | last-resort fallback only |
| Notarization/universal | absent | release gate |
| Auto-update | absent | Sparkle or manual update |
| Crash reporting/logging | minimal `println`/`eprintln` | `os_log`, ring buffer, opt-in reports |

---

## 4. macOS adapter/IME lessons for ViFastKey

### 4.1 Decision: IMK first, event tap second

**Recommendation: implement the first adapter as an `IMKInputController` using marked text (preedit).** Add `CGEventTap` only as an explicit “Compatibility mode” for apps where IMK cannot work, default off.

Why IMK first:

- It is the macOS-native input method contract: the system routes key events, manages the input source, and gives you a `IMKTextInput` client.
- Preedit is atomic: `setMarkedText` + `insertText` avoid per-character synthetic events.
- Standard text fields, undo, selection, and IME semantics work better.
- Secure input is handled by the system more predictably.
- It avoids Input Monitoring/Accessibility permission for the normal path, reducing onboarding friction and privacy surface.
- It keeps a plausible path to sandboxing/App Store review (still must be verified), whereas a global event tap is much harder to justify.

Why event tap is second:

- It requires Input Monitoring for listening and Accessibility for active suppression/synthetic events on modern macOS.
- TCC grants are identity-bound; ad-hoc signing makes them unstable.
- Secure input can suppress events or make injection unreliable.
- Backspace/insert timing differs per app (Electron, Firefox, Notion, terminals).
- Synthetic events can loop; markers/PID targeting/serial queues are mandatory.
- Notarization/sandbox and App Review are harder.

**Adapter mapping:**

| Engine `Action` | IMK path | Event-tap fallback |
|---|---|---|
| `PassThrough` | return `false` from `handleEvent` | return event unchanged |
| `Preedit { text, cursor }` | `client.setMarkedText(attributed, selectionRange: cursorUTF16, replacementRange: currentMarkedRange)` | usually not used; fallback uses replace |
| `Replace { delete_before, insert, pass_key }` | prefer `client.insertText(insert, replacementRange: range)` if the range is valid; otherwise preedit/commit | delete graphemes, insert Unicode, optionally pass key |
| `Commit { text, pass_key }` | `client.insertText(text, replacementRange: markedRange)`, unmark; return `false` if `pass_key` | insert text, optionally pass key |

**Critical detail:** in IMK, `NSRange` is UTF-16. In event-tap replace mode, **backspace typically deletes a composed character sequence (grapheme), not a code point**; verify the exact behaviour per app. ViFastKey’s `delete_before` is code points (`docs/COMPATIBILITY.md:120-133`); if output is NFD or contains emoji/combining marks, sending `delete_before` backspaces can over-delete. The event-tap adapter must compute the grapheme count of the text being deleted (or use a `delete_text` prefix from the engine) and verify with surrounding text.

### 4.2 Permissions matrix

| Capability | API/permission | Needed for | ViFastKey policy |
|---|---|---|---|
| IMK preedit/commit | IMK, no TCC | default typing | **P0 default** |
| Read focused element/AX role | Accessibility (`AXIsProcessTrusted`) | field kind, secure field, app context, surrounding verification | optional; request only when needed |
| Listen to key events via tap | Input Monitoring (`CGPreflightListenEventAccess`/`CGRequestListenEventAccess`) | event-tap fallback | explicit opt-in |
| Suppress/post key events | Accessibility + Input Monitoring (verify on target OS) | active tap, synthetic backspace/insert | explicit opt-in, warning |
| Read secure input state | `IsSecureEventInputEnabled()` | password/secure fields | always check; pass through |
| Pasteboard fallback | `NSPasteboard` | last-resort injection | user-visible opt-in; restore clipboard |
| Update check | network | optional | no keystrokes/text; version only |

**P0 rule:** the default install must work without Accessibility or Input Monitoring. If the user never enables compatibility mode, the app should never ask for those permissions.

**TCC identity:** Accessibility/Input Monitoring grants are tied to the code signing identity/designated requirement and bundle identity. Changing ad-hoc signature, Team ID, bundle ID, or sometimes install path creates a new TCC entry. Use a stable Developer ID Application certificate and stable bundle ID **before** testing permissions. Do not use `codesign --sign -` for anything that will be permission-tested.

**Onboarding permission copy:** explain exactly why each permission is needed and what it enables. Do not bundle Accessibility/Input Monitoring into a single scary prompt. Provide “Open System Settings” deep links and a “Recheck” button; poll `AXIsProcessTrusted` and `CGPreflightListenEventAccess` after the user returns.

### 4.3 Event-tap lifecycle and safety

If/when compatibility mode is implemented:

- Create the tap on a dedicated thread with its own `CFRunLoop`, not the main runloop.
- Use `.cgSessionEventTap`, `.headInsertEventTap`, `.defaultTap` (active) or `.listenOnly` (read-only).
- Keep the `CFMachPort`/`CFRunLoopSource` alive; enable with `CGEventTapEnable`.
- Handle `kCGEventTapDisabledByTimeout` and `kCGEventTapDisabledByUserInput`: re-enable with backoff, count retries, and after 3 failures in a short window disable the feature and tell the user to restart.
- Watchdog: heartbeat from the tap thread; if no callback for N seconds while keys are expected, check `CGEventTapIsEnabled` and recreate.
- Mark every synthetic event (`CGEventSource.userData` or a private magic field); the tap must pass marked events through unchanged to avoid loops.
- Pin the target PID (`CGEventPostToPid`) and re-check `frontmostApplication.processIdentifier` before each injection; abort if focus changed.
- Use a serial queue for backspaces/inserts; never block the tap callback on AX/disk/network.
- Handle sleep/wake, fast user switching (`NSWorkspace.sessionDidResignActiveNotification`), screen lock, and input source changes by committing/cancelling composition and recreating the tap if needed.
- Do not install both IMK and event tap active at the same time; if compatibility mode is on, disable the ViFastKey input source or route only selected apps.
- Never process keys when `IsSecureEventInputEnabled()` is true; pass through.
- Never delete blindly: validate `delete_before` against the known surrounding text; if mismatch, pass through or reset.

**Stop rule:** if stress tests show any lost or duplicated character, disable event-tap fallback and ship IMK-only. Key loss is worse than missing a feature.

### 4.4 Text injection, UTF-16, and grapheme boundaries

**IMK path:**

- `setMarkedText(_:selectionRange:replacementRange:)` — marked text is one atomic region; `selectionRange` is relative to the marked text and measured in UTF-16.
- `insertText(_:replacementRange:)` — commit/replace; `replacementRange` measured in UTF-16 in the client’s text storage.
- On focus loss/deactivate/`commitComposition`, commit or cancel the marked text; never leave a dangling composition.
- If `setMarkedText`/`insertText` is unsupported or fails, pass through and reset the session.
- If the client has a non-empty selection, decide policy: clear selection, reset session, then process the key; do not “edit previous word” across a selection.

**Event-tap path:**

- Delete by grapheme clusters for backspace; verify `delete_before` sanity.
- Insert via `CGEventKeyboardSetUnicodeString` on a keyDown/keyUp pair (virtual key 0), targeted at the PID; test apps that ignore Unicode strings.
- If insertion fails, do not fall back to per-character AppleScript; use a user-visible clipboard fallback or pass through.
- Clipboard fallback: snapshot all pasteboard items/types and `changeCount`, write string, send Cmd+V, wait, restore only if `changeCount` is unchanged; never touch `org.nspasteboard.ConcealedType`/transient content; never in secure fields; log nothing.
- Synthetic backspace count must be derived from the actual displayed text, not from `delete_before` code points. For NFC Vietnamese this often coincides; for NFD/emoji it does not. Add fixtures for:
  - `Chào` (NFD, multiple code points per grapheme);
  - `😀`;
  - CJK;
  - combining marks;
  - selection replacement.

**Adapter invariant:** after applying an action, the visible text in the client must end with `session.display()` (ViFastKey’s engine conformance already checks a similar invariant in `rust/crates/vfk-engine/tests/conformance.rs:26-52`). The macOS adapter should assert/verify this in tests and, in production, verify with surrounding text before complex edits.

### 4.5 App/field context and keyboard layout

**App identity:**

- Use `NSRunningApplication.bundleIdentifier` as the canonical key (`com.apple.Terminal`, `com.google.Chrome`, `com.microsoft.VSCode`, `com.tinyspeck.slackmacgap`, `com.hnc.Discord`, `org.mozilla.firefox`, `com.apple.Safari`, `com.apple.TextEdit`, `com.apple.Notes`, `com.apple.iWork.Pages`, `com.microsoft.Word`, `com.apple.dt.Xcode`, `com.googlecode.iterm2`).
- Keep `localizedName` only for display.
- Normalise bundle IDs case-insensitively for lookup but preserve the original for diagnostics.
- Support user profiles keyed by bundle ID; built-in aliases map bundle IDs to preset names.

**ViFastKey gap:** `resolve_profile` currently does:

```rust
let lo = app.to_lowercase();
let k = lo.rsplit('/').next().unwrap_or("");
let k = k.rsplit('\\').next().unwrap_or("");
let k = k.strip_suffix(".exe").unwrap_or(k);
let k = k.strip_suffix(".app").unwrap_or(k);
config.app_profiles.get(k).cloned().unwrap_or_default()
```

(`rust/crates/vfk-engine/src/ports.rs:200-211`). Passing `com.google.Chrome` gives `com.google.chrome`, not `chrome`. Also `excluded_apps` uses `x == a` and `app_modes` uses the raw `app_id` (`session.rs:368-403`). Fix this before the adapter ships:

- Introduce `normalize_app_id(bundle_id, localized_name) -> AppKeys` with exact bundle ID, lowercase bundle ID, last component, and alias.
- Apply the same resolution to `app_profiles`, `excluded_apps`, and `app_modes`.
- Persist bundle IDs in user config, not localized names.
- Add tests for `com.google.Chrome`, `com.apple.Terminal`, `com.microsoft.VSCode`, `com.tinyspeck.slackmacgap`, and a user profile keyed by bundle ID.
- Keep Windows/Linux process-name behavior working; do not break existing tests.

**Field kind:**

- Use AX role/subrole: `AXTextField`, `AXTextArea`, `AXComboBox`, `AXSearchField`, `AXSecureTextField`; treat secure/password, email, URL, number/OTP as `FieldKind` pass-through or url/email policies.
- Cache the focused element; don’t call AX on every key. Set `AXUIElementSetMessagingTimeout` to a small value (e.g. 0.1 s) to avoid hangs.
- Re-read surrounding text before editing a previous word; if it doesn’t end with the expected committed text, fail safe (pass through/reset).
- Use `NSWorkspace.didActivateApplicationNotification` + `AXFocusedUIElementChanged` (if available) with debounce; don’t reset composition on transient focus changes/menus.

**Surrounding text:**

- `IMKTextInput.attributedSubstring(from:)` can provide text before the cursor; if unavailable, pass `nil`.
- Limit length (e.g. last 64–128 characters) and normalise to the configured NFC/NFD form before comparing.
- Do not read surrounding text in secure/password fields.
- For selection, clear/reset first; don’t edit across selection.

**Keyboard layout:**

- In IMK, `NSEvent.characters`/`charactersIgnoringModifiers` already reflect the active layout; use characters for Telex/VNI letters, and keyCode + modifiers for shortcuts.
- In event tap, use `CGEventKeyboardGetUnicodeString`/`UCKeyTranslate` with the current TIS layout; never assume `keyCode == letter`.
- Test U.S., Vietnamese, AZERTY, Dvorak, and a dead-key layout. Pass through when `characters` is empty/control or when Command/Control shortcuts are active.
- Numpad should pass through (ViFastKey `EDGE_CASES.md` C7).
- Do not translate keyboard layout inside the engine; the adapter supplies the character.

### 4.6 Menu bar, app lifecycle, preferences, and hotkeys

TTS has no menu bar, so this is a design section, not a copy section.

**App lifecycle (AppKit):**

- `LSUIElement = true`, `NSApplication.setActivationPolicy(.accessory)`; no Dock icon.
- `applicationDidFinishLaunching`:
  - create `IMKServer(name: connectionName, bundleIdentifier: bundleID)`;
  - install `NSStatusItem`;
  - load config atomically;
  - register workspace/TIS notifications;
  - show onboarding if not installed/selected.
- `applicationWillTerminate`: commit/cancel composition, stop event tap, flush config.
- `applicationShouldTerminateAfterLastWindowClosed = false`; closing preferences should not quit.
- `applicationShouldHandleReopen`: show preferences.
- `deactivateServer`/`inputControllerWillClose`: commit or cancel marked text; reset session; save pending config.
- `NSWorkspace.didWakeNotification` / `sessionDidResignActiveNotification`: reset composition, recheck permissions, restart tap if needed.
- `TISNotifySelectedKeyboardInputSourceChanged`: update status icon/menu; if ViFastKey is not selected, show “Chưa chọn” and offer “Chọn ViFastKey”.

**Menu bar:**

- Template image (black/white) for light/dark mode; 16/18 pt; accessibility label.
- Menu:
  - current mode (Tiếng Việt / English) with checkmark;
  - “Bật/Tắt tiếng Việt” (calls `vfk_toggle_vietnamese`);
  - “Mở Cài đặt…”;
  - “Kiểm tra quyền…” (status);
  - “Chạy chẩn đoán…” (doctor JSON/copy);
  - “Khởi động lại bộ gõ” (if tap/IMK stuck);
  - “Cập nhật…” (if enabled);
  - “Giới thiệu / Quyền riêng tư”;
  - “Thoát”.
- Dynamic status: green/ready, warning/permission, lock/secure input, error.
- `NSMenuDelegate.menuNeedsUpdate` to refresh permission/input-source state.

**Preferences:**

- SwiftUI hosted in `NSWindow` or AppKit; tabs: General, Typing, Macros, Apps, Advanced, Diagnostics.
- One config source; debounced atomic save; inline validation.
- Macro editor: table with key/value, duplicate/empty/length validation, import/export; TTS’s `Dictionary` warnings with line numbers are a good UX model (`app/src/dictionary.rs:37-89`, desktop `dictionary_panel`).
- App profiles: list running apps with bundle IDs; choose preset/custom; show effective output mode/delay; “Add current app”; reset.
- “Try it” window: a real `NSTextView` using the actual input method; don’t reimplement the adapter in the preview. Show status/result.
- Advanced: event-tap compatibility toggle with warning, log level, diagnostics export, reset config, reveal config folder.
- Hotkeys: configurable; avoid swallowing system shortcuts; use input method menu/menu bar for discoverability. A global toggle hotkey is optional; if implemented, use `RegisterEventHotKey` or a documented API and test conflicts. Don’t put hotkey handling in the engine.
- Accessibility: VoiceOver labels, keyboard navigation, high contrast, reduced motion for OSD.

**OSD:**

- Non-activating `NSPanel` (borderless, `.statusBar` level, ignores mouse, joins all spaces, no shadow), fades after ~700 ms.
- Shows “Tiếng Việt”/“English” or “Secure input — tạm dừng”.
- Never steals key focus; never shows in secure fields.

### 4.7 Diagnostics, doctor, and self-test

TTS’s `--smoke` is the right shape (`desktop/src/main.rs:91-135`): headless, loads resources, performs a real operation, checks output, exits non-zero. Extend it for IME.

**`ViFastKey --self-test` (P0):**

- Runs before `NSApplicationMain`/UI, like TTS checks `--smoke` before eframe.
- Loads the bundled engine (static lib), runs:
  - version/ABI/config schema checks;
  - a small adapter fixture subset (engine + action mapping) with a fake client;
  - UTF-16/NFC/NFD conversion checks;
  - config parse/migration/corrupt fallback;
  - bundle layout/resource lookup;
  - `Info.plist` required keys/version/arch;
  - code signature/team ID (can be skipped for unsigned dev with a warning);
  - input source registration check (non-interactive, read-only).
- Prints `SELFTEST OK ...` and exits 0; on failure prints `SELFTEST FAIL <code>: <message>` and exits 1.
- CI runs this from the packaged `.app` on macOS.

**`ViFastKey --doctor [--json]` (P0/P1):**

Fields (redacted by default):

- app version/build, bundle ID, install path, arch, macOS version;
- signing: authority, team ID, hardened runtime, notarization/staple status, designated requirement;
- input source: installed, enabled, selected, input source ID;
- permissions: Accessibility (`AXIsProcessTrusted`), Input Monitoring (`CGPreflightListenEventAccess`), prompt state;
- secure input: `IsSecureEventInputEnabled`;
- event tap: enabled, last callback age, re-enable count, last reason;
- config: path, schema, valid/corrupt/backup, macro count, profile count;
- engine: version, ABI, config schema, last error;
- front app/field kind (optional, `--include-apps`; default omit);
- recent error ring buffer (no text, no keystrokes).
- `--copy` to paste JSON; `--export <path>` to save a support bundle; no automatic upload.

**Error codes:** define a stable table (`VFK-MAC-001` …) with cause, user message, fix, and whether to copy diagnostics. Examples:

| Code | Meaning | User action |
|---|---|---|
| `VFK-MAC-001` | App not installed in Input Methods | Install/ move to `~/Library/Input Methods` |
| `VFK-MAC-002` | Input source not selected | Open Keyboard settings / select ViFastKey |
| `VFK-MAC-003` | Input Monitoring denied | Open Privacy & Security → Input Monitoring |
| `VFK-MAC-004` | Accessibility denied | Open Privacy & Security → Accessibility |
| `VFK-MAC-005` | Secure input active | Explain pause; no action needed |
| `VFK-MAC-006` | Event tap disabled by timeout | Auto-retry; if repeated, restart |
| `VFK-MAC-007` | Config corrupt; backup created | Review/restore backup |
| `VFK-MAC-008` | Engine error/panic; key passed through | Copy diagnostics; restart |
| `VFK-MAC-009` | Signature/identity changed | Re-grant permissions / reinstall |
| `VFK-MAC-010` | Unsupported arch/macOS | Show requirement |

### 4.8 Crash and recovery

- Rust panics are caught at the FFI boundary (`vfk-ffi` uses `catch_unwind`; TTS does the same at `ffi/src/lib.rs:150-162`). On error:
  - reset the session;
  - pass the original key through;
  - increment an error counter;
  - record a redacted log line;
  - show a menu-bar warning only if repeated.
- Swift/ObjC exceptions: install `NSSetUncaughtExceptionHandler` for logging, but do not pretend it can recover. Persist config atomically; do not persist composition/keystrokes (privacy).
- On launch, detect an unclean exit via a marker file; if present, keep `config.json.bak` and show “Phiên trước kết thúc bất thường” in diagnostics; never auto-upload.
- Event-tap timeout is not a crash; recover automatically with backoff.
- If the input method process crashes, macOS may restart it; the current word is lost. This is acceptable if no partial commit was written; ensure actions are atomic (setMarkedText/insertText, not per-character).
- Keep dSYMs for Swift and Rust; archive with the release; optional opt-in crash reports later.

### 4.9 Signing, notarization, universal builds, sandbox, auto-update

**Signing (P0 before external beta):**

- Developer ID Application certificate with stable Team ID.
- Stable bundle ID, e.g. `com.vifastkey.inputmethod`; do not change it between builds.
- Hardened runtime (`--options runtime`), timestamp (`--timestamp`).
- Minimal entitlements; no `disable-library-validation` unless loading unsigned code (avoid).
- Sign nested code first (frameworks/dylibs/helpers), then the app; use `codesign --verify --deep --strict` only for verification, not `--deep` signing.
- `codesign -dr -` to inspect the designated requirement; document it; TCC depends on it.
- `spctl --assess --type execute --verbose=4`.

**Notarization (P1 release):**

- `xcrun notarytool submit ViFastKey.zip --keychain-profile ... --wait`.
- `xcrun stapler staple ViFastKey.app` (and DMG/PKG as appropriate).
- `xcrun stapler validate`, `spctl --assess`.
- Notarize the app before zipping, or notarize the final DMG/PKG; staple the distributed artifact.
- Keep notary credentials in CI secrets/environment with least privilege; never print them.
- Test on a clean macOS user account/VM; Gatekeeper should open without right-click.

**Universal2 (P1):**

- Build Rust staticlib for `aarch64-apple-darwin` and `x86_64-apple-darwin`, `lipo -create` into one `.a`.
- Xcode `ARCHS="arm64 x86_64"`, `ONLY_ACTIVE_ARCH=NO`, `MACOSX_DEPLOYMENT_TARGET=13.0` (or chosen minimum).
- `lipo -info`/`file` gate in CI; run x86_64 slice under Rosetta on Apple Silicon if no Intel runner.
- If Intel is dropped, make it an explicit decision and a clear install error; don’t ship a confusing arm64-only binary under a generic name.

**Sandbox / distribution:**

- Direct notarized distribution: unsandboxed, hardened runtime, stable identity; simplest for IMK + optional AX/tap.
- Mac App Store: sandbox + App Review; IMK-only is the realistic path; event tap/Accessibility may be incompatible. Decide before writing UI/config paths.
- Input methods install in `~/Library/Input Methods`; App Store installs may differ. Test both.
- Do not assume App Sandbox works with a global event tap; prototype and verify, do not guess.

**Auto-update (P2):**

- Sparkle 2 with EdDSA-signed appcast; `SUFeedURL`, `SUPublicEDKey`, `SUEnableAutomaticChecks`, `SUScheduledCheckInterval`.
- Do not update during composition; defer until idle; prompt restart.
- Same bundle ID/Team ID/designated requirement so TCC survives; test update over a granted install.
- If installed in `~/Library/Input Methods`, the user owns the path; if `/Library`, use a `.pkg`/privileged installer.
- Staged rollout (`phasedRolloutInterval`) and a rollback plan; keep previous version.
- If Sparkle is too much for v1, ship “Check for Updates…” linking to GitHub Releases; still sign/notarize.

### 4.10 Privacy, logging, and telemetry

ViFastTts is local-first and explicit about data:

- Extension privacy policy has a data table, permission rationale, no analytics/advertising/sale, local retention/deletion (`docs/EXTENSION_PRIVACY.md:19-38`, `:40-51`, `:53-64`).
- Desktop/mobile TTS does not send text unless remote mode is explicitly configured.
- The project’s demo docs say no text/audio storage for the public server (`docs/NGOC_HUYEN.md` / README).

**ViFastKey policy (P0):**

- No network for typing. No telemetry by default. No keystroke/text/surrounding-text/clipboard logging.
- `os_log` subsystem `com.vifastkey`, categories `ime`, `tap`, `config`, `ui`; use `privacy: .private` for any user-derived string; better, don’t log strings at all.
- Log metadata only: action kind, delete count, app bundle ID hash or category, field kind, latency bucket, error code.
- In-memory ring buffer (e.g. 200 events) for diagnostics; no disk by default; export only on user action.
- Diagnostics/support bundle: redacted JSON + logs; user reviews path; no automatic upload.
- Crash reports: opt-in only; no text; stack traces + OS/app version; disclose.
- Clipboard fallback: user-visible, snapshot/restore, no logging, no concealed/transient content.
- Secure input/password fields: pass through; never read surrounding text; never show preedit.
- Privacy policy in `PRIVACY.md` and app “About/Privacy”; data table similar to TTS extension.
- `PrivacyInfo.xcprivacy` if required for APIs (UserDefaults/file timestamps); declare no tracking/data collection.

---

## 5. UX, onboarding, diagnostics, and status

### 5.1 First-run flow

Recommended 4-step onboarding:

1. **Install**: detect if running from `~/Library/Input Methods`; if not, show “Cài đặt ViFastKey” and copy/move the app (or run the `.pkg`). Explain that macOS input methods live there, not `/Applications`.
2. **Add input source**: `TISRegisterInputSource`/`TISCreateInputSourceList`; show whether ViFastKey is installed/enabled. Button “Mở Cài đặt bàn phím”; optional “Chọn ViFastKey” via `TISSelectInputSource`. If not found, ask logout/login.
3. **Test typing**: a real `NSTextView` with instructions “Gõ `Tiếng Việt`”; detect whether ViFastKey is the selected input source. This validates the whole chain without asking for permissions.
4. **Optional compatibility mode**: only if the user explicitly wants event-tap fallback; explain Input Monitoring/Accessibility, show status, open settings, recheck. Default skip.

Completion: menu bar icon, mode toggle, link to preferences, “Gõ thử trong TextEdit”.

### 5.2 Permission checker

A single `PermissionChecker` view model with states:

```swift
enum PermissionState { case notNeeded, notDetermined, denied, granted, stale, unknown }
```

Rows:

- Input source installed/selected (green/yellow/red).
- Accessibility (only if AX/tap enabled).
- Input Monitoring (only if tap enabled).
- Secure input (informational; “Tạm dừng khi nhập mật khẩu”).
- Signing identity (green “Developer ID”, yellow “ad-hoc — quyền sẽ mất khi cập nhật”).
- Config (valid/backup/default).
- Engine (version/ABI/error).
- Event tap (enabled/last callback/retries).

Buttons: “Mở System Settings”, “Kiểm tra lại”, “Chạy chẩn đoán”, “Sao chép thông tin”. Never claim “granted” from a cached bool; re-query.

### 5.3 Status/OSD

- Menu bar icon: template, accessible label, tooltip.
- States:
  - normal: keyboard/character icon;
  - Vietnamese off: dimmed or “EN”;
  - permission missing: warning badge;
  - secure input: lock badge;
  - engine/tap error: red dot.
- OSD on toggle: non-activating panel, 700 ms, reduced-motion aware.
- Don’t show OSD for every key; only mode changes/errors.

### 5.4 Preferences information architecture

| Tab | Content |
|---|---|
| General | Vietnamese/English default, launch behavior, menu bar icon, updates |
| Typing | input method (Telex/Simple/VNI), tone style, spell check, `w_as_u`, `bracket_keys`, `quick_telex`, `allow_coda_k`, `macro_digits`, `macro_undo`, `auto_capitalize`, `smart_caps` |
| Macros | table editor, import/export, validation, duplicate detection, reset |
| Apps | per-app profiles by bundle ID, presets, exclusion, mode memory, output mode/delay/ACK |
| Advanced | event-tap compatibility mode, log level, diagnostics, config folder, reset |
| Diagnostics | doctor JSON, permission status, last errors, self-test, copy support bundle |

TTS’s dictionary warnings pattern (line numbers, duplicate, fallback) is a good model for macro/profile validation (`app/src/dictionary.rs:37-89`; desktop `dictionary_panel`). TTS’s ETA history (median ms/char, capped) is optional; for ViFastKey, use latency history only in diagnostics.

### 5.5 Error messages

- Vietnamese primary, short cause + fix + code.
- No raw Rust/AppKit errors alone; wrap with `LocalizedError` (`failureReason`, `recoverySuggestion`).
- “Copy diagnostics” button on every error sheet.
- Never include typed text, surrounding text, passwords, clipboard content, or tokens.
- Examples:
  - “Chưa cấp quyền Input Monitoring. Mở System Settings → Privacy & Security → Input Monitoring, bật ViFastKey, rồi bấm Kiểm tra lại.” (`VFK-MAC-003`)
  - “Secure Input đang bật do một ứng dụng khác. ViFastKey tạm dừng để bảo vệ mật khẩu.” (`VFK-MAC-005`)
  - “Event tap bị macOS tắt (timeout). Đã thử bật lại 3 lần; hãy khởi động lại ViFastKey.” (`VFK-MAC-006`)
  - “Config bị hỏng. Đã sao lưu thành `config.bad.2026-10-29.json` và dùng mặc định.” (`VFK-MAC-007`)

### 5.6 What to borrow from TTS UX and what not

Borrow:

- explicit status/error enums and event-driven UI;
- “show resolved path”;
- inline validation/warnings with line numbers;
- `--smoke`/self-test style;
- model/status badges and “copy details” mindset;
- local-first privacy and permission rationale (from extension docs).

Do not borrow:

- no tray/no native menu;
- no onboarding;
- `println`/`eprintln` as production logging;
- direct UI binding to mutable settings without a view model;
- broad ATS exception;
- “right-click Open” as installation UX.

---

## 6. Test strategy for the macOS adapter

### 6.1 Test pyramid

| Tier | What | Where | Frequency |
|---|---|---|---|
| 1. Engine unit/conformance | Python oracle ↔ Rust actions | existing `python/tests`, `rust/conformance` | every PR |
| 2. Adapter unit | Action → IMK/event-tap calls, UTF-16, graphemes, config, permissions | Swift XCTest with fakes | every PR |
| 3. Adapter golden | JSONL fixtures from Python/Rust reference adapter | Rust + Swift tests | every PR |
| 4. Property/fuzz | random strings, actions, contexts; no crash/out-of-bounds | Rust + Swift property tests | nightly |
| 5. Real-app integration | TextEdit, Safari, Chrome, Firefox, Terminal, iTerm2, VS Code, Slack, Discord, Notion, LibreOffice, Pages, Messages, password fields | UI test host + manual matrix | nightly/manual |
| 6. Packaged smoke | `--self-test` from `.app`, signature/notarization/arch/plist | CI tag | every release |
| 7. Performance | key path p50/p99, tap overhead, AX latency, memory | XCTest/criterion | nightly/release |
| 8. Canary | beta users/app matrix, crash/permission reports (opt-in) | release | after beta |

### 6.2 Adapter conformance fixtures

Format proposal (JSONL):

```json
{
  "name": "telex-basic",
  "mode": "preedit",
  "config": {"input_method": "telex", "tone_style": "old"},
  "app": {"bundle_id": "com.apple.TextEdit", "name": "TextEdit", "field": "text"},
  "context": {"before_cursor": "", "selection": [0,0], "secure_input": false},
  "keys": ["t","i","e","e","n","g","s"],
  "actions": [
    {"t":"preedit","text":"t","cursor":1},
    {"t":"preedit","text":"ti","cursor":2},
    {"t":"preedit","text":"tiê","cursor":3},
    {"t":"preedit","text":"tiến","cursor":4},
    {"t":"preedit","text":"tiếng","cursor":5}
  ],
  "calls": [
    {"call":"setMarkedText","text":"t","selection":1},
    {"call":"setMarkedText","text":"ti","selection":2}
  ],
  "final_text": "tiếng"
}
```

Rules:

- Expected `actions` come from the Python oracle/Rust engine; expected `calls` come from a small reference adapter (Python or Rust), not from Swift.
- Include `delete_utf16` for IMK and `delete_graphemes` for event-tap.
- Include app profiles, excluded apps, field kinds, focus/cursor changes, selection, surrounding text, NFD/emoji/CJK.
- `--reuse`/`--stop` options like `tools/rust_parity.py`.
- CI fails on generated fixture drift; fixture changes require a decision record.

### 6.3 Swift test doubles

Define protocols at the adapter boundary:

```swift
protocol TextInputClient {
    var bundleIdentifier: String? { get }
    func selectedRange() -> NSRange
    func markedRange() -> NSRange
    func attributedSubstring(from range: NSRange) -> NSAttributedString?
    func setMarkedText(_ text: Any, selectionRange: NSRange, replacementRange: NSRange)
    func insertText(_ text: Any, replacementRange: NSRange)
    func unmarkText()
}

protocol AppContextProvider { func current() -> AppContext }
protocol PermissionProvider { func accessibility() -> PermissionState; func inputMonitoring() -> PermissionState }
protocol EventPoster { func post(_ event: SyntheticEvent, to pid: pid_t) -> Bool }
```

Tests:

- `FakeIMKClient` records calls; `FakeTextStorage` applies them and enforces UTF-16 ranges.
- `FakeAX` returns role/subrole/secure/selection/surrounding.
- `FakeTap` invokes disable/enable callbacks and records synthetic events/markers.
- `FakeClock` for OSD/debounce/backoff.
- Golden tests compare call transcripts; property tests randomize `Action` and strings.

### 6.4 Real-app matrix

Create `docs/MACOS_TEST_MATRIX.md` with columns:

| App | Bundle ID | Version | Mode | Field | Keys | Expected | Result | macOS | Date |
|---|---|---|---|---|---|---|---|---|---|

Minimum set:

- TextEdit, Notes, Messages, Pages, Safari (address bar + text area), Chrome (address bar + form + Google Docs), Firefox (form + address bar), Terminal, iTerm2, VS Code, Slack, Discord, Notion, LibreOffice Writer, Microsoft Word, Xcode editor, Spotlight, a password field, Keychain Access, 1Password (secure input), a Java/Electron app if available.
- Cases: Telex basic, VNI, tone old/new, `thuowng`, `duowngd`, `Đắk Lắk`, `ass`, `coffee`, `mp3`, `k8s`, macro undo, backspace, space/enter/tab/esc, focus change, cursor move, selection, NFD, emoji/CJK, secure field, app switch.
- Record pass/fail and any workaround; workarounds go to `MACOS_EXCEPTIONS.md` with app version/expiry.

**Automation:** a small `ViFastKeyTestHost` AppKit app with `NSTextView`, select ViFastKey via `TISSelectInputSource`, send `CGEvent` key events, read string, assert. This runs in a logged-in GUI session (nightly/local), not in headless PR CI. For PR CI, fake-client tests are enough.

TTS’s `docs/UI_PARITY.md` is a useful template for a dated feature/UI matrix, and `docs/EVAL_TOOLS.md` is a useful template for a “measure catalog + pitfalls” document. Adapt both rather than inventing a new format from scratch.

### 6.5 CI tiers and gates

- PR: engine tests + adapter unit/golden + `xcodebuild` unsigned + `--self-test` (no permissions).
- Nightly: real-app host tests, fuzz/property, performance, long stress.
- Tag: universal2 build, sign, notarize, staple, `spctl`, package, `--self-test`, download-and-verify release asset.
- Never mark a release successful if signing/notarization/self-test fails; do not use `if: always()` to publish partial assets.

### 6.6 Release smoke checks

`--self-test` should check:

- bundle ID/version/build match tag;
- required `Info.plist` keys present;
- architectures contain arm64 + x86_64 (if universal);
- `codesign --verify --strict`, designated requirement/team;
- `spctl`/staple (release);
- resources: engine, default config, licences, fonts;
- config load/migrate/corrupt fallback;
- engine version/ABI/config schema;
- fixture subset;
- UTF-16/NFC/NFD;
- input source registration (read-only) and selected state;
- no network required.

Manual clean-machine check:

- install `.pkg`; input source appears; select; type in TextEdit; no permission prompt for IMK;
- enable compatibility mode; grant Input Monitoring/Accessibility; type in Chrome/Terminal; disable; permissions not required again for IMK;
- update; TCC grants persist; config/macros persist;
- uninstall; input source removed; config optionally remains.

### 6.7 Benchmarks

- Engine already fast (`python/PORTING.md:90-97`: ~0.5 µs/key Rust incremental).
- Measure adapter `handleEvent` p50/p99 with fake client; target < 1 ms typical, no AX/disk/network on hot path.
- Measure event-tap callback and synthetic injection latency; track underrun/lost-key count.
- Memory over 1M keystrokes; no leaks from `VfkSession`/Swift wrappers.
- Use `os_signpost` intervals; log aggregated p50/p95 once per session, not per key.
- CI benchmark job optional until stable; compare only same runner.

### 6.8 Fuzz/property

- Random UTF-8/UTF-16 strings: emoji, CJK, NFD/combining, ZWJ, control chars.
- Random `Action` sequences: delete counts, cursor positions, pass_key.
- Assert: no crash, no out-of-bounds `NSRange`, no negative/overflow counts, text length invariant, no processing when secure/field policy says off.
- Rust `proptest`/`cargo-fuzz` for engine/FFI; Swift deterministic property tests for adapter.

### 6.9 Failure injection

- `setMarkedText` returns/fails (client nil, wrong range).
- `insertText` no-op/duplicate.
- AX timeout/denied/secure field.
- event tap disabled by timeout/user input; synthetic marker loop; focus changes mid-injection.
- config corrupt/read-only/full disk; update interrupted; signature invalid.
- input source missing/disabled; app not in Input Methods.
- secure input toggles during composition.
- crash during composition; relaunch recovery.

### 6.10 Definition of done for first adapter

- Installs to `~/Library/Input Methods`, appears in Input Sources, selectable.
- Default IMK path works in TextEdit/Notes/Safari/Chrome/Terminal with Telex/VNI, preedit/commit, backspace, space/enter/tab/esc, macros, app profiles.
- No Accessibility/Input Monitoring required for default path.
- Password/secure fields pass through; no text logged.
- Config atomic/migrated/corrupt-safe.
- `--self-test` passes from packaged `.app`; `--doctor` reports all states.
- PR CI builds/tests; tag CI signs/notarizes/staples/verifies universal2.
- Real-app matrix documented; no lost/duplicated characters in stress test.
- Privacy policy and permissions rationale published.
- Event-tap fallback either absent or explicitly opt-in with recovery; not default.

---

## 7. Licensing: what is MIT, what is not, and what not to copy

### 7.1 TTS licence map

| Component | Licence | Evidence |
|---|---|---|
| Repository code | MIT | `LICENSE` |
| Sample model Ngọc Huyền / dataset | Research/non-commercial; dataset `pnnbao-ump/ngochuyen_voice` is CC BY-NC 4.0 | `README.md:145-147`, `docs/NGOC_HUYEN.md:3-9` |
| Model weights/checkpoints/ONNX/bundles | Same non-commercial constraints; not in git | `README.md:188-190`, `.gitignore` |
| Brand/logo/wordmark | Not MIT; owner-provided | `resources/brand/README.md` |
| Be Vietnam Pro font | SIL OFL 1.1 | `resources/fonts/BeVietnamPro-OFL.txt` |
| Reference G2P tools | Mixed; GPL tools only offline/audit, not runtime | `docs/LICENSES.md:16-29`, `:53-58` |
| Vocos pretrained vocoder | MIT | `docs/LICENSES.md:31-39` |

TTS explicitly says code is MIT but data/lexicon/checkpoints have their own licences (`README.md:432-440`, `docs/LICENSES.md:25-29`). Therefore “MIT repo” does **not** mean “all files are MIT”.

### 7.2 Do not copy into ViFastKey

| Path/data | Why |
|---|---|
| `artifacts/**`, `runs/**`, `models/**`, `*.onnx`, `*.pt`, `*.wav` | Model/audio artifacts; non-commercial dataset; large binaries |
| `*.vfa`, `*.vfts`, model bundles | Contain non-commercial weights and model data |
| `data/**` (audio, manifests, features, YouTube data) | Voice data and derived features; separate licences/privacy |
| `deploy/vifasttts_demo/**` data, DB, samples, feedback | User/demo data; not code |
| `resources/brand/**` | Logo/wordmark not MIT |
| `rust/crates/core/domain/text/resources/{overrides_vi,english,products,letter_readings,custom_reading}.json` | TTS pronunciation/lexicon data; provenance separate, irrelevant to IME, and not covered by “code MIT” assumptions |
| TTS docs/model card text | Documentation may be MIT, but model/data statements and brand use are separate; rewrite for ViFastKey |
| GPL/AGPL components referenced in TTS docs (`vPhon`, eSpeak, WikiPron data, GPL wordlists) | Not embedded in TTS runtime, but do not copy into ViFastKey; ViFastKey already has a strict GPL policy (`THIRD_PARTY_NOTICES.md:44-54`) |
| Test audio/model fixtures | Even if test code is MIT, fixtures may derive from restricted data; write your own |

**Do not copy TTS’s `resources/fonts/BeVietnamPro-Regular.ttf` either** unless you need it and retain the OFL notice; ViFastKey already has its own font copy and notice.

### 7.3 What can be borrowed

- Architecture patterns: ports/adapters, headless command/event, manifest/hash verification, phase gates, scoped exceptions, CI matrix, packaged smoke tests.
- Documentation structure: docs index, status/decision/exception separation, archive.
- Test ideas: golden corpus, property/fuzz, parity thresholds, fast/full split, environment-bound baselines.
- UX ideas: status/error enums, inline validation warnings, resolved path display, local-first privacy.
- Code snippets only if MIT-compatible and attributed; better to reimplement in ViFastKey style. If copied, add the MIT copyright notice to `THIRD_PARTY_NOTICES.md`.

### 7.4 ViFastKey release obligations

- `LICENSE` (MIT) in app bundle.
- `THIRD_PARTY_NOTICES.md` plus `Contents/Resources/licenses/`:
  - Be Vietnam Pro OFL;
  - Rust dependencies (generate with `cargo-about`/`cargo-deny`);
  - any SwiftPM dependencies (none ideally);
  - brand assets note (not MIT).
- `About` window links to licence/privacy.
- `cargo-deny` in CI for licence/security advisories.
- Do not use TTS brand/logo or imply endorsement.
- If the macOS adapter is a separate repo/binary, keep the same MIT notice and notices.

---

## 8. Prioritized plan for ViFastKey’s first macOS adapter

Legend:

- **A** = adapter (Swift/AppKit/IMK/CGEventTap)
- **E** = engine/FFI (Rust/Python core)
- **D** = docs/process/CI/release
- **A/E** = boundary contract

### 8.1 P0 — must exist before a usable beta

| ID | Priority | Item | Layer | Why | Acceptance |
|---|---|---|---|---|---|
| P0-1 | P0 | Freeze adapter contract + JSONL conformance corpus | A/E/D | TTS’s best lesson: contract/golden before implementation; ViFastKey already has engine conformance but no OS boundary | 100% fixtures pass in Rust + Swift; first divergence printed; CI drift check |
| P0-2 | P0 | IMK-first architecture; event tap behind explicit compatibility flag | A/D | Native preedit avoids TCC/secure-input/synthetic-event risks; event tap is fallback | Default install uses IMK, no Accessibility/Input Monitoring; fallback off by default |
| P0-3 | P0 | Stable Developer ID + bundle ID before TCC testing | A/D | TCC grants are signing-identity-bound; ad-hoc breaks them | `codesign -dr -` stable; permission survives rebuild/update test |
| P0-4 | P0 | Permission + secure-input gate | A | Never process keys in password/secure fields; permission states must be accurate | Fake tests for AX/Input Monitoring/secure/field kinds; no key loss; pass-through |
| P0-5 | P0 | Event-tap lifecycle/recovery (if fallback present) | A | Timeout/marker/loop/focus bugs cause lost/duplicated text | Tap disable/re-enable test; marker loop test; PID/focus test; stress 100k keys |
| P0-6 | P0 | UTF-16/NFC + grapheme-safe text application | A/E | IMK uses UTF-16; synthetic backspace uses graphemes; NFD/emoji break naive counts | Fixtures for NFD/emoji/CJK; no out-of-bounds; invariant `visible.endsWith(display())` |
| P0-7 | P0 | Config persistence: atomic, versioned, corrupted-safe | A/E | IME config/macros must never be lost; TTS save is non-atomic | Kill during save -> old/new valid; corrupt -> backup+defaults; migration tests |
| P0-8 | P0 | `--self-test` + packaged macOS CI job | A/D | TTS `--smoke` proves the package works; IME needs more checks | CI builds unsigned `.app`, runs self-test from bundle; tag job signed/notarized |
| P0-9 | P0 | IME bundle/Info.plist/install flow | A/D | A normal `.app` in `/Applications` is not an input source | Fresh install -> appears/selectable; version/plist validated; uninstall documented |
| P0-10 | P0 | Privacy/logging policy + redacted doctor | A/D | Keystroke data is highly sensitive; TTS local-first/privacy docs are a good model | Log audit shows no text; doctor JSON redacted; no network by default |

**P0 sequencing:** P0-1 contract → P0-3 signing identity (in parallel with dev) → P0-2/4/6 IMK path → P0-7/9 bundle/config → P0-8 self-test/CI → P0-10 privacy/doctor → P0-5 only if fallback is in scope.

### 8.2 P1 — required for a public, daily-use release

| ID | Item | Layer | Why | Acceptance |
|---|---|---|---|---|
| P1-1 | Menu bar status item + OSD + mode toggle | A | Daily-use IME needs visible state; TTS has no tray | Status states render; toggle shows OSD; VoiceOver label |
| P1-2 | Onboarding + permission checker | A/D | TCC/input-source setup is the biggest UX failure point | 4-step flow; recheck buttons; no permission required for IMK |
| P1-3 | Preferences UI (typing/macros/apps/advanced/diagnostics) | A | TTS only has a simple settings panel; ViFastKey needs profile/macro editing | Live validation, atomic save, import/export, reset |
| P1-4 | Bundle-ID app profiles + macOS presets | E/A | Current `resolve_profile` won’t match `com.google.Chrome` | Tests for bundle IDs; terminal/Chrome/Firefox/Electron presets work |
| P1-5 | Doctor JSON + log export + error codes | A/D | Supportable product, not guesswork | Doctor covers all states; support bundle redacted |
| P1-6 | Real-app matrix + UI test host | A/D | IMK/event-tap behavior differs per app | Matrix documented; TextEdit/Safari/Chrome/Firefox/Terminal/VS Code/Slack pass |
| P1-7 | Universal2 build + notarization + `.pkg` | A/D | TTS’s ad-hoc/host-arch release is not acceptable for IME | `lipo -info` both arch; `spctl`/staple pass; clean-machine install |
| P1-8 | Crash/timeout recovery + stress tests | A | IME must not lose keys or wedge | Recovery tests; 100k keystrokes no loss; repeated tap timeout disables safely |
| P1-9 | Benchmarks + latency budget | A/E | Ensure AX/logging doesn’t ruin key path | p99 target; no AX/disk/network on hot path; memory stable |
| P1-10 | Privacy policy + `PrivacyInfo.xcprivacy` + notices in bundle | D/A | Trust and release compliance | Policy published; licences in app; no telemetry by default |

### 8.3 P2 — after the IMK path is stable

| ID | Item | Layer | Why |
|---|---|---|---|
| P2-1 | Sparkle 2 auto-update with EdDSA appcast | A/D | Keep users current without breaking TCC; defer until signing stable |
| P2-2 | Opt-in crash reports/telemetry | A/D | Improve reliability without keystroke data; no default |
| P2-3 | Clipboard/paste fallback for stubborn apps | A | Last resort; restore pasteboard, never in secure fields |
| P2-4 | Event-tap compatibility mode for specific apps | A | Only where IMK fails; per-app exceptions with expiry |
| P2-5 | Intel/Rosetta test matrix (or explicit arm64-only decision) | A/D | Universal2 fidelity |
| P2-6 | Homebrew Cask / `.dmg` / release channels | D | Distribution after notarization |
| P2-7 | App Store sandbox variant | A/D | Only if IMK-only and review-compatible; verify event tap incompatibility |
| P2-8 | Advanced keyboard layouts (AZERTY/Dvorak/dead keys) | A | International users; pass-through when unsure |
| P2-9 | Localisation (vi/en), accessibility polish, reduced motion | A/D | Product quality |
| P2-10 | SBOM/dSYM/support bundle automation | D | Release/security maturity |

### 8.4 Rough sizing

For one developer familiar with AppKit/IMK:

- IMK MVP (contract, preedit, config, bundle, self-test): ~2–4 weeks.
- Permissions/context/diagnostics/onboarding: ~1–2 weeks.
- Real-app hardening + event-tap fallback (if in scope): ~2–4 weeks.
- Signing/notarization/universal/`.pkg`/release CI: ~1–2 weeks.
- Public beta hardening (crash recovery, stress, docs): ~2–3 weeks.

A realistic first public adapter is **6–10 weeks**; event-tap fallback + auto-update can push it to **10–16 weeks**. TTS’s Rust port estimate was 10–15 weeks (`docs/RUST_PORT.md:790-808`); ViFastKey already has the engine, so most effort is OS integration and hardening, not language logic.

### 8.5 Decisions to record before coding

1. IMK-only v1, or IMK + event-tap fallback? (Recommend IMK-only v1.)
2. Minimum macOS version (13? 14?) and Intel support (universal2 vs arm64-only).
3. Direct notarized distribution vs Mac App Store (or both).
4. Install scope: `~/Library/Input Methods` per-user vs `/Library/Input Methods` all-users.
5. Auto-update: Sparkle now, later, or manual.
6. Whether the app always runs as a menu-bar agent or only when input source selected.
7. App identity canonicalisation: bundle ID + aliases in engine; migration for existing configs.
8. Event-tap scope: which apps/fields, and how users enable/disable.
9. Telemetry/crash reporting: none, opt-in, or local-only.
10. Privacy stance: no network, no text logs, no clipboard logging.

Record these in `docs/DECISIONS.md` with date, options, decision, and revisit condition. TTS’s `DECISIONS.md` is append-only and code-addressed; ViFastKey can use a simpler ADR format.

### 8.6 Risks and stop rules

| Risk | Likelihood | Impact | Mitigation | Stop rule |
|---|---|---|---|---|
| TCC grants reset after rebuild/update | High with ad-hoc | Blocker | Stable Developer ID, bundle ID, path; notarize | Any permission loss after update -> stop release |
| Secure input bypass/text leak | Low–medium | Severe privacy | `IsSecureEventInputEnabled`, AX secure field, pass-through | Any processing in secure field -> stop |
| Event-tap timeout/loop | Medium | Lost/duplicate text | Marker, PID, serial queue, re-enable/backoff, watchdog | Any duplicate/loss in stress -> disable fallback |
| IMK app incompatibility | High | Text corruption | Preedit atomicity, real-app matrix, per-app fallback | Any corruption -> block release |
| UTF-16/NFD deletion bug | Medium | Wrong text | Fixtures, grapheme counts, surrounding verification | Any mismatch -> pass through/fix |
| App profile mismatch | High (current code) | Profiles silently ignored | Bundle-ID normalization + tests | Profiles not applied -> fix before beta |
| Signing/notarization failure | Medium | No install | CI dry run, clean machine, logs | Do not publish unsigned beta |
| Scope creep (IMK + tap + update) | High | Delay | Phase gates; IMK-only v1 | If MVP unstable, cut fallback/update |
| Privacy leak in logs | Low–medium | Trust damage | `os_log` privacy, audit, redaction | Any text in logs -> block release |
| Crash in IME process | Medium | Lost word | FFI catch, atomic actions, logs, recovery | Repeated crash -> disable feature |

---

## 9. Appendices

### Appendix A — TTS file reference map

| Topic | File / lines |
|---|---|
| Port plan, phases, gates | `docs/RUST_PORT.md:16-40`, `:423-467`, `:514-572`, `:740-761`, `:790-808` |
| Crate boundaries | `docs/RUST_PORT.md:183-260`, `:1299-1378`, `:1477-1499` |
| Test tiers/parity | `docs/RUST_PORT.md:693-716`, `:953-976`; `tools/rust_parity.py` |
| Platform matrix | `docs/RUST_PORT.md:1005-1032` |
| Docs index/status | `docs/README.md:4-6`, `:114-137`; `docs/handoff/TRANG_THAI.md:1-6` |
| Decisions/exceptions | `docs/handoff/DECISIONS.md:1-10`; `docs/handoff/EXCEPTIONS.md:1-14`, `:52-80` |
| Distribution/signing | `docs/DISTRIBUTION.md:6-18`, `:42-47`; `docs/INSTALL.md:46-66` |
| Model bundle safety | `docs/VFTS_FORMAT.md:12-56`; `rust/crates/core/domain/vfts/src/lib.rs:78-90`, `:220-325` |
| CI matrix/release | `ci.yml:18-410` |
| Pinned model/toolchain | `.github/actions/fetch-model/action.yml:7-27`; `rust/rust-toolchain.toml:1-4` |
| App settings | `rust/crates/core/application/app/src/settings.rs:6`, `:25-106` |
| Headless commands/events | `rust/crates/core/application/app/src/session.rs:15-52`, `:94-230` |
| ETA/history | `rust/crates/core/application/app/src/history.rs:15-98` |
| Dictionary warnings | `rust/crates/core/application/app/src/dictionary.rs:37-89` |
| Desktop lifecycle/paths | `rust/crates/adapters/inbound/desktop/src/main.rs:39-66`, `:91-135`, `:141-272`, `:1682-1687`, `:1706-1742` |
| macOS package | `rust/crates/adapters/inbound/desktop/package-macos.sh:22-100` |
| FFI safety/ABI | `rust/crates/adapters/inbound/ffi/src/lib.rs:22-37`, `:128-162`, `:416-500` |
| Swift wrapper | `mobile/ios/ViFastTtsMobile/TtsEngine.swift:23-139`, `:203-220`; `ViFastTtsMobile-Bridging-Header.h:1-7` |
| Privacy/permissions | `docs/EXTENSION_PRIVACY.md:19-64`; `docs/EXTENSION_STORE.md:66-105` |
| Licensing | `LICENSE`; `README.md:145-147`; `docs/NGOC_HUYEN.md:3-9`; `docs/LICENSES.md:16-39`, `:53-58`; `resources/brand/README.md` |
| Test examples | `tests/test_parser_matrix.py:1-60`; `tests/test_cli.py:1-100`; `python/tests/test_ffi.py` in ViFastKey |

### Appendix B — ViFastKey current-state map and gaps

| Topic | Current ViFastKey | Gap for macOS |
|---|---|---|
| Engine | Python oracle + Rust port, 13,311 core + 3,000 engine conformance | add adapter fixtures |
| FFI | C ABI, UTF-16, null-safe, catch panic | add `vfk_abi_version`; Swift wrapper tests |
| App identity | `resolve_profile` path/name-based (`ports.rs:200-211`); `excluded_apps`/`app_modes` raw (`session.rs:368-403`) | bundle-ID normalisation/aliases |
| Config | `schema_version`, forward-compatible, JSON merge (`docs/CONFIG.md:3-9`, `:118-123`) | atomic file store, backup, macOS adapter |
| Adapter guide | strong contract (`docs/ADAPTER_GUIDE.md`) | concrete IMK/event-tap implementation + tests |
| macOS plan | IMK Swift, C ABI (`ROADMAP.md:15-19`) | no code yet |
| CI | Python matrix, conformance, Rust 3 OS, wasm | macOS app build/self-test/release |
| Release | none | signing, notarization, universal, `.pkg`, update |
| Licensing | MIT + THIRD_PARTY_NOTICES + GPL policy | bundle notices, cargo-deny, privacy policy |
| Tests | 9 Python test files, Rust integration tests | Swift adapter tests, real-app matrix, benchmarks |

### Appendix C — Adapter conformance fixture example

```json
{"name":"nfd-delete-combining","mode":"replace",
 "app":{"bundle_id":"com.apple.TextEdit","field":"text"},
 "context":{"before_cursor":"Chà","secure_input":false},
 "keys":["Backspace"],
 "actions":[{"t":"replace","delete_before":2,"insert":"","pass_key":false}],
 "imk":{"delete_utf16":2,"cursor_utf16":2},
 "event_tap":{"delete_graphemes":1},
 "final_text":"Ch"}
```

The exact numbers must come from the engine/reference adapter, not from Swift. In this illustrative NFD case, deleting `a` + combining grave is **2 code points / 2 UTF-16 code units but 1 grapheme**. The fixture demonstrates why `delete_before` (code points), UTF-16 (code units), and graphemes are three different units.

### Appendix D — macOS adapter checklist

**Architecture**

- [ ] IMK `IMKServer` + `IMKInputController` implemented.
- [ ] `Action` mapping documented and tested.
- [ ] Event-tap fallback isolated behind a flag (if present).
- [ ] Engine stays pure; no Vietnamese logic in Swift.

**Permissions/security**

- [ ] Default path needs no Accessibility/Input Monitoring.
- [ ] Secure input/password fields pass through.
- [ ] `AXIsProcessTrusted`/`CGPreflightListenEventAccess` states handled.
- [ ] No text/keystroke logging; `os_log` privacy.
- [ ] Clipboard fallback restores pasteboard; no concealed content.

**Text correctness**

- [ ] UTF-16 ranges for IMK.
- [ ] Grapheme counts for synthetic backspace.
- [ ] NFC/NFD, emoji, CJK, selection, focus/cursor changes.
- [ ] No delete beyond known text; pass-through on mismatch.
- [ ] `pass_key` correct for space/enter/tab/esc.

**Lifecycle/UX**

- [ ] `LSUIElement`, status item, menu, preferences, OSD.
- [ ] Onboarding/install/input-source selection.
- [ ] Permission checker + doctor + error codes.
- [ ] Config atomic/migrated/backup.
- [ ] Crash/tap-timeout recovery.

**Release**

- [ ] Stable Developer ID/bundle ID.
- [ ] Universal2 or explicit arm64 decision.
- [ ] Hardened runtime, notarize, staple, `spctl`.
- [ ] `.pkg`/install flow to Input Methods.
- [ ] Version/build/plist/tag consistency.
- [ ] Licences/notices in bundle.
- [ ] `--self-test` from packaged app.
- [ ] Clean-machine install/update/uninstall test.

### Appendix E — Sources read

Primary:

- `ViFastTts/README.md`, `AGENTS.md`, `CHANGELOG.md`, `pyproject.toml`, `Makefile`, `LICENSE`.
- `.github/workflows/ci.yml`, `.github/actions/fetch-model/action.yml`.
- `docs/RUST_PORT.md`, `docs/DISTRIBUTION.md`, `docs/INSTALL.md`, `docs/MODEL_DISTRIBUTION.md`, `docs/VFTS_FORMAT.md`, `docs/README.md`, `docs/ARCHITECTURE.md`, `docs/APPLE_SILICON.md`, `docs/LICENSES.md`, `docs/EXTENSION_PRIVACY.md`, `docs/EXTENSION_STORE.md`, `docs/UI_PARITY.md`, `docs/NGOC_HUYEN.md`, `docs/DEPLOYMENT.md`, `docs/MODEL_SEARCH.md`, `docs/MOBILE_MODELS.md`, `docs/handoff/TRANG_THAI.md`, `docs/handoff/DECISIONS.md`, `docs/handoff/EXCEPTIONS.md`.
- `desktop/README.md`; `rust/crates/adapters/inbound/desktop/{package-macos.sh,src/main.rs}`; `rust/crates/core/application/app/src/{settings,session,history,dictionary}.rs`; `rust/crates/adapters/inbound/ffi/src/lib.rs`; `rust/crates/core/domain/vfts/src/lib.rs`; `rust/Cargo.toml`, `rust/rust-toolchain.toml`.
- `mobile/ios/**` (Swift wrapper, Info.plist, project.yml, build script); `mobile/android/README.md`; `extension/build.sh`; `web/build.sh`.
- `tests/` (parser matrix, CLI, FFI), `tools/rust_parity.py`, `tools/export_rust_resources.py`.

Secondary ViFastKey context:

- `ViFastKey/README.md`, `ROADMAP.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `Makefile`, `.github/workflows/ci.yml`, `THIRD_PARTY_NOTICES.md`, `rust/Cargo.toml`, `python/pyproject.toml`.
- `docs/ADAPTER_GUIDE.md`, `docs/CONFIG.md`, `docs/COMPATIBILITY.md`, `docs/IME_FEATURE_RESEARCH.md`, `python/PORTING.md`, `python/EDGE_CASES.md`.
- `rust/crates/vfk-engine/src/{ports,session}.rs`, `rust/crates/vfk-ffi/include/vifastkey.h`, `python/tests/test_ffi.py`, `rust/crates/vfk-engine/tests/conformance.rs`.

---

**Bottom line:** ViFastTts is a strong model for contract-first engineering, ports/adapters, FFI safety, packaging layout, docs discipline, and test tiering. It is a weak model for macOS IME integration: it has no IMK/event-tap/AX/TCC/signing/notarization implementation. ViFastKey should copy the **engineering discipline**, not the **desktop/TTS runtime**, and should build an IMK-first macOS adapter with a stable signing identity, adapter-level golden tests, atomic config, a real doctor/self-test, and a notarized universal release.
