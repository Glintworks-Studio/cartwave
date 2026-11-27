# SHELL: Batch 1 (safety) and Batch 2 (app shell)

*2026-11-25 · unattended run of SHELL_PROMPT.md · branch `app-shell/batch-1-2` · Windows 11,
RTX 4070 SUPER, 59.951 Hz display · decisions logged in [docs/RUNLOG.md](docs/RUNLOG.md)*

## Summary

**Git.** `main` fast-forwarded through `stabilize/v1.1` and then `docs/ux-audit` (no merge
commits, no conflicts). On `main`: 153 tests pass, golden render included; `main` pushed
(`dd590e8..1ca4a75`). Five untracked prompt files were committed to `wip/prompts-2026-11-25`
(local only, nothing discarded). `app-shell/batch-1-2` was cut from the updated `main`.

**Gates: all pass.**

| Gate | Result |
|---|---|
| Acceptance tests AT-01, 02, 03, 04, 06, 15, 16, 18, 19 (scenario harness, `docs/shell/tools/at_shell.ps1`) | **9/9 PASS** (49 checks; saved files, window titles, the native dialogs' text and screenshots) |
| F-30 unit tests (no shortcut conflicts in any context, menu shortcuts = the table) | **PASS** (`shortcuts::tests`, `menus::tests`) |
| `cargo test --release --workspace` (golden render included) | **167 pass** (153 before + 14 new) |
| `cargo clippy --workspace --all-targets` / `cargo build --release` | clean / 0 warnings |
| PERF.md UI scenarios at the 60 fps cap (3 runs) | demo **8.6–9.8%** (≤ 10), 200 clips **9.0–10.5%** (≤ 20), 200-clip drag **11.8–12.8%** (≤ 25); A/B against the pre-run build: no regression |
| PERF.md pacing, 30 s each | demo / 200 / drag: **0 frames > 20 ms, 0 stalls**, 60.0 updates/s |
| Piano roll, 2,000 notes | playing **7.2%**, dragging **7.8%** |
| ENGINE.md stress A and C, 120 s, loopback, 23 kHz pilot | **0 pilot dropouts**, 0 starved / late, 0 allocation violations (HDMI endpoint: the MOTU is not connected) |
| Every harness re-run after the `CARTWAVE_DEV` change | all reach their scenario (proof per harness below) |

**Built.**
- **Batch 1:** save prompts and a dirty flag (F-01), debug keys and test hooks only in dev mode
  (F-02), the text-field rule (F-03), dimmed and locked VRC6 lanes under the 2A03 profile (F-04),
  and one shortcut table (F-30).
- **Batch 2:** native menus (muda), Open Recent, About, a generated shortcut sheet, the Settings
  panel (Audio applies live; Display, Editing, Files, Warnings), Project Settings (profile, filter),
  and a slimmer top bar (tempo only). The grid moved into the arrangement's right-click menu, the
  clock format into View, and Home rewinds. The decision-3 keys are in: Quantize Ctrl/⌘+U,
  Follow Ctrl+Shift+F, Ctrl+I free.
- **Docs:** `CLAUDE.md` replaced, `VISION.md` decisions recorded, and the new `UX.md` (the build
  spec).

**Decisions taken alone** (all in RUNLOG, D1–D22). The ones that matter most:
- The interface-scale setting is shown disabled, with the reason. The fork's winit backend
  ignores `user_scale_factor`, and fixing that means publishing the vizia fork, which this run
  may not do.
- The save prompt is a native task dialog with real Save / Don't Save / Cancel buttons. That
  needs rfd's `common-controls-v6` and a manifest, embedded by linker arguments (no new crate).
- Clip order no longer counts as content. A click raised a clip, which made the song "unsaved"
  and added an undo step. The undo step was a bug before this run too.
- The Warnings tab works, but nothing can be silenced yet: a native dialog can't hold a "Don't ask
  again" box. F-47 brings the first one.
- Two harnesses were already broken before this run (`pianoroll/scenario.ps1`) or broke with the
  slimmer top bar (`stabilize/screens_v11.ps1`). Both were updated, and both pass.

**Merged:** yes, see the end of this file.

---

## 1. Git step

| Step | Result |
|---|---|
| `git fetch --all`; working tree | Clean except 5 untracked `*_PROMPT.md` files, committed to `wip/prompts-2026-11-25` (not pushed) |
| `git merge --ff-only stabilize/v1.1` | Fast-forward `dd590e8..c0037d3` |
| `git merge --ff-only docs/ux-audit` | Fast-forward `c0037d3..1ca4a75` |
| Tests + golden on `main` | 153 pass |
| `git push origin main` | `dd590e8..1ca4a75` |

## 2. Documents

- **CLAUDE.md** is `docs/ux-audit/CLAUDE.proposed.md`. It points to VISION.md first, and has no
  "Konami" or "Ableton-style". Changes from the proposal:
  - the "proposed" banner is gone;
  - three standing rules from the brief were missing and are added (measure before and after;
    trust probes over earlier documents; every benchmark proves what it tests);
  - it now points to UX.md, SHELL.md and `shortcuts.rs`.
- **VISION.md:** tempo is in the top bar, and a single click deletes a breakpoint. Both changes
  are in place, with a "Decisions" note at the bottom.
- **UX.md** comes from UX-DRAFT, with every priority accepted. The CEO's six decisions are written
  in, and so is the correction that the frame-rate cap defaults to **60 fps** (60 / 120; Off only in
  dev mode). Follow is Ctrl+Shift+F on Windows and ⌥⇧F on macOS, the reference's own Mac key
  (R135).

## 3. Batch 1

### F-01 Never lose work silently (AT-01, 21 checks)
- **Dirty flag:** the last committed undo document differs from the saved one, or a saved field
  outside undo differs (profile, filter, loop, mute/solo). Undo back to the saved state is clean
  again. New, Open and Save mark the document clean. A song made from a MIDI file and a recovered
  session start out unsaved.
- **Save / Don't Save / Cancel**, a native task dialog with the app window as its parent. It
  appears for: New, Open, Open Recent, New from MIDI File, dropping a `.cartwave`, `.chipdaw` or `.mid`, closing
  the window (also Alt+F4) and Quit. A clean document never gets the prompt. The window close is
  cancelled by the root model consuming `WindowClose`: models see the event before the window
  view, so no fork change was needed.
- **Title:** `*name — Cartwave`, or `Untitled` when the project has no file.
- **Recovery after an unsaved quit** (Settings > Files, on by default). Don't Save on quit writes
  the work to `recovery.cartwave` (`recovery.chipdaw` before the rename) and marks the session as `unsaved-quit`. The next start offers it
  with "Unsaved changes from your last session" (`file::LastSession`, unit-tested).

### F-02 Debug keys only in dev mode (AT-02)
- `dev::enabled()` is true in a debug build or with `CARTWAVE_DEV=1`. F3–F7 and every test hook
  (`CARTWAVE_STRESS`, `_PROJECT`, `_OPEN_CLIP`, `_PIANO_ZOOM`, `_EXPORT_TO`, `_FRAMELOG`,
  `_FPS_CAP`, `_SONG`, `_PILOT`, `_AUTO_EDIT`, `_AVSYNC`, …) go through it. Plain configuration
  stays open: `CARTWAVE_DATA_DIR`, `CARTWAVE_AUDIO=0`, `CARTWAVE_VSYNC=0`.
- AT-02: in a release build without dev mode, F3–F7 leave the saved project byte-identical and the
  window **0 px** different. The control run in dev mode changes 154,704 px (F3 overlay) and swaps
  in the 200-clip set (F4).
- **Every harness** sets `CARTWAVE_DEV=1`: 13 PowerShell scripts and 2 macOS scripts. In dev mode
  the app prints one stderr line per hook it applied, and the harnesses check for it:

| Harness | Proof it reached its scenario |
|---|---|
| `perf/bench.ps1` (×9) | "stress set reached" on every run (F4 line) |
| `perf/pacing.ps1` demo/stress/drag | "scenario … reached" (dev line, F4 line, ≥ 100 frame-log rows) |
| `pianoroll/bench_pr.ps1` | "scenario reached" (project and open-clip lines) |
| `engine/matrix.ps1` A, C | audio log written; pilot loopback blocks counted; C shows its 60 injected UI stalls |
| `stabilize/scenario_v11.ps1` | 16/16 file checks PASS |
| `stabilize/screens_v11.ps1` | export through the dialog: 28 files, `smpl` PASS |
| `pianoroll/scenario.ps1` | file check PASS (1 clip, 6 notes, volume 13) |
| `pianoroll/screens.ps1`, `ux-audit/screens_ux.ps1`, `probe_textfocus.ps1`, `perf/snap.ps1` | screenshots written and inspected. The committed images were restored afterwards: they document earlier passes |
| `perf/trace_app.ps1` | **not re-run**: it needs the elevated xperf helper. Its launch and F4 path are the same as bench's, and it now has the same proof line (and the stale drag coordinates fixed) |
| macOS `bench_mac.sh`, `matrix_mac.sh` | updated; tested in the Mac pass |

### F-03 Typing never triggers shortcuts (AT-03)
- The root model tracks the text field being edited (vizia's `TextEvent::StartEdit` / `EndEdit`
  bubble up to it). While that field has focus, the key context is `Text`, and no app shortcut
  matches it. Only the field's own keys work (Esc, Enter, Tab, Ctrl+A/C/X/V).
- While a dialog is open the context is `Modal`: only Esc, which closes it.
- AT-03: typing "a b" in the search box fills the field and leaves the transport stopped.
  Delete and Ctrl+Z in the field change nothing in the song. A clip renamed in the inspector keeps
  its new name after Ctrl+Z in the field.

### F-04 Nothing pretends to work (AT-04)
- Under the 2A03 profile, the VRC6 lanes and their headers are drawn under a 62% black veil. The
  header reads "Off in the 2A03 profile". Hovering it, or the lane, says how to turn VRC6 on.
- Those lanes take no clicks, drags, double-clicks, context menus or preset drops. Clips can't
  be moved onto them, Select All skips them, the piano roll won't open on them, and Insert MIDI
  Clip leaves them out.
- Switching the profile to 2A03 while VRC6 clips exist asks first: "The VRC6 lanes have 11
  clip(s)…". Channel layouts are templates that ask before changing the profile: Switch / Keep /
  Cancel (decision 5).
- AT-04 pixel check: the VRC6 headers' mean brightness against the 2A03 headers falls from 1.00
  to 0.64. The headers are dark surfaces, so the veil shows most in the text and colour strips.

### F-30 One shortcut table
`crates/ui/src/shortcuts.rs` lists every action once: id, name, Windows keys, macOS keys, the
contexts where it applies (App, Arrangement, PianoRoll, Text, Modal) and whether it is dev-only.
Everything reads from it: the root, arrangement and piano-roll key handlers (`lookup`), the menus'
shortcut text and the context menus (`label`), the macOS accelerators (`accelerator`), the browser
and piano-roll hints, and the Help sheet (`sheet`). Its unit tests check:
- no key conflicts in any context, on either platform;
- the decision-3 keys (and that Q and Cmd+I are free);
- platform labels;
- that every accelerator parses in muda;
- that the sheet lists every entry.

## 4. Batch 2

### Menus (AT-06; screenshots 01–06)
- muda 0.22, behind the `menus` feature (on by default; a plugin build can leave it out).
  `menus::spec` is the menu structure as plain data, holding only features this build has.
- **File:** New, Open…, Open Recent ▸, Save, Save As…, New from MIDI File…, Import ▸ Audio as DPCM
  Sample…, Export Audio…, Project Settings…, Exit.
- **Edit:** Undo, Redo, Cut / Copy / Paste (notes), Duplicate, Delete, Select All, Quantize,
  Settings….
- **Create:** MIDI Clip. **View:** Browser ✓, Follow ✓, Clock Shows Bars ✓. **Help:** Keyboard
  Shortcuts, About.
- **macOS** (compiled, not run): the App menu (About, Settings ⌘,, Services, Hide…, Quit ⌘Q) and
  the Window menu, with the accelerators registered from the table.
- **Windows:** no accelerator table. Items show the key as `label\tkey` (what muda itself would
  draw), and keys go through our own path, so nothing double-fires. Items are enabled from the app
  state (Undo when there's history, Paste when the clipboard has notes, …).
- **The window grows by the bar's height,** so the content keeps its 1440×900 and the harness
  coordinates still hold. An Alt-drag (the no-snap modifier) doesn't activate the bar when Alt is
  released.
- The Edit menu acts on the editor that last had the mouse: the piano roll while it's open and
  clicked, else the arrangement.

### Open Recent, About, shortcut sheet (AT-18; 06, 16, 17)
- Open Recent: the 10 newest, in `settings.json`, with Clear Menu. Missing files are removed at
  start and when clicked, with a notice.
- About: version, the git commit (from `build.rs`), the licence, font credits and the provenance
  summary from ASSETS.md.
- The sheet is generated per platform and grouped (File, Edit, Create, Transport, Arrangement,
  View, Piano roll, Text fields, Dialogs; plus Dev mode when dev mode is on).

### Settings (AT-15; 07–11, 23)
An in-app modal (backdrop, Esc closes, no shortcuts reach the document), opened with Ctrl+, or
Edit > Settings…. Everything is in `settings.json`, with defaults; old files load, and unknown
fields are ignored (unit-tested).

| Tab | Settings | Notes |
|---|---|---|
| Audio | Output device (system default or named), buffer size (with the latency in ms), sample rate 44.1 / 48 / 96 kHz | Applied **live through the engine's rebuild path** (`Engine::reconfigure`, `ERR_RECONFIG`): the host drops the stream and rebuilds it; the core keeps song, transport and chip state. A sample-rate change rebuilds only the band-limited step buffer and the output filter, at the current level (`Synth::set_sample_rate`, unit-tested). **Measured gap while playing: 26–34 ms** (6 switches: 256, 1024 frames, 44.1, 96, 48 kHz, device default) |
| Display | Frame-rate cap **60** (default) / 120 (Off in dev mode only); interface scale; clock format | Interface scale is shown disabled with its reason (RUNLOG D12) |
| Editing | Default arrangement grid, default piano-roll grid, follow playhead | Applied at once and on start |
| Files | Autosave interval (Off, 1, 2, 5, 10 min), offer recovery after an unsaved quit, open last project at start | The data folder is shown |
| Warnings | The silenced warnings with Reset, and Reset all | Nothing can be silenced yet (D13) |

### Project Settings (AT-16; 12, 18, 19)
File > Project Settings…: hardware profile (2A03 / 2A03 + VRC6) and output filter (follow the
profile, Off, NES, NES + 440 Hz, Famicom). Both were already saved in format v3 (`chip`,
`filter`), so there is **no format bump**.

### The slim top bar (AT-19; 13, 22)
- The NEW / OPEN / SAVE / MIDI IN / EXPORT keys and the SNAP, CHIP and FILTER pills are gone. The
  BPM pill stays. Ctrl+S still saves.
- The arrangement grid is in the right-click menu on empty lane space: Grid: 1/16 / 1/4 / Bar,
  with a tick on the current one.
- The clock no longer changes format on a click or rewinds on a double-click. The format is in
  View > Clock Shows Bars and in Settings; Home rewinds.
- The piano-roll hint is generated: "… Ctrl+U quantize …" on Windows, Option / Cmd words on
  macOS.
- Contrast of the new text, from the tokens: notes #808080 on #141414 = **4.66:1**, labels
  #9A9A9A = 6.5:1, the dimmed-lane hints #D9D9D9 on near-black > 10:1.

## 5. Performance (gate 3)

`bench.ps1`, 12 s per scenario, 60 fps cap. An alternating A/B against the pre-run build
(`1ca4a75`, built in a worktree), three rounds each, plus three plain runs of the branch:

| Build | idle | demo (≤ 10%) | 200 clips (≤ 20%) | 200-clip drag (≤ 25%) |
|---|---|---|---|---|
| before (`1ca4a75`), 3 runs | 0.1–0.5% | 9.1 / 9.5 / 9.6% | 8.8 / 10.2 / 11.7% | 12.0 / 13.1 / 14.0% |
| this branch, 6 runs | 0.3–1.3% | 8.6–9.8% (mean 9.3) | 9.0–10.5% (mean 9.7) | 11.8–12.8% (mean 12.3) |

Pacing (30 s, frame log + desktop duplication): demo 9.3% CPU, 200 clips 9.8%, drag 12.5%. All
three had 0 frames over 20 ms and 0 stalls, max gap ≤ 17.7 ms. The menu bar and the new per-tick
work (title, menu state, dirty check) cost nothing measurable.

## 6. Engine (gate 3)

| Test | Callbacks | Starved / late | Cost mean / p99 / max | Pilot dropouts | Alloc violations |
|---|---|---|---|---|---|
| A (idle-to-play, 120 s, 512) | 11,998 | 0 / 0 | 0.21 / 0.50 / 1.6% | **0** | 0 |
| C (UI stalls, 120 s, 512) | 12,023 | 0 / 0 | 0.21 / 0.60 / 3.2% | **0** | 0 |

Device: VX3211-2K (NVIDIA HDMI), 48 kHz, 480–1056 frames per callback. The MOTU is not
connected, as in the stabilize run.

## 7. Known gaps and follow-ups
- **macOS:** compiled only. Untested: the App and Window menus, ⌘ accelerators, `init_for_nsapp`
  timing, and the save prompt as an NSAlert. For the Mac pass.
- **Interface scale** needs a small fork change: apply `WindowDescription::user_scale_factor` in
  `vizia_winit` at window creation and on `ScaleFactorChanged`. It wasn't made, because the fork
  can't be published in this run.
- **"Don't ask again"** arrives with the first in-app warning dialog (F-47, the MIDI drop
  preview).
- Win32 menu popups use the system's light theme. muda themes only the bar.
- While a native menu or dialog is open, Windows runs a modal loop: drawing pauses, audio
  doesn't (FEASIBILITY.md §1).
- `trace_app.ps1` wasn't re-run (it needs elevation).
- The M/S keys on dimmed VRC6 rows stay clickable: muting a silent lane changes no content.

## 8. How to run
```
cargo build --release
pwsh -File docs\shell\tools\at_shell.ps1            # all acceptance tests (or -Only AT-01,AT-04)
cargo test --release -p ui --lib shortcuts menus    # F-30 / AT-06 unit tests
pwsh -File docs\perf\tools\bench.ps1 -Label x       # PERF (prints "stress set reached")
```
The harness drives the real native dialogs through UI Automation (`docs/shell/tools/dialog.ps1`).
The menus are opened with their Alt mnemonics, because vizia's accessibility tree hides the menu
bar from UI Automation. Pixel checks use `docs/shell/tools/pixels.ps1`.

## 9. Merge
All gates pass, so `app-shell/batch-1-2` was merged into `main` (fast-forward) and pushed (see
the RUNLOG's last entry for the commit).
