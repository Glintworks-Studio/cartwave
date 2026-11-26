# MAC-CATCHUP: macOS brought level with Windows (everything since ENGINE-macos.md)

*2026-11-26 · unattended run of MAC_CATCHUP_PROMPT.md · branch `mac/catchup` from `main` ef2ea19
(Polish 2) · Mac mini M1, macOS 26.6.2 · decisions MC-D1… in [docs/RUNLOG.md](docs/RUNLOG.md) ·
harness and raw results in [`docs/mac-catchup/`](docs/mac-catchup/)*

## Summary (one screen)

- **Environment:** Mac mini M1, macOS 26.6.2, rustc 1.100.0; displays GA27T1M 320 Hz (main) and
  XB271HU 59.953 Hz, both backing 1.0; audio: built-in speakers (the main matrix) and a MOTU M2 (USB,
  plugged in during the run); fork pin 60907aad → **2a146e6b** (fork commits 14 and 15).
- **Gates: 6 of 7 pass. Gate 6 (pacing) fails on the 60 Hz display**: 10 isolated 2–3-vblank holds per
  120 s, the stopgap's known residue (PERF-macos §11); the fix is a renderer change. **Merged 2026-11-26** after the owner
  accepted gate 6 as is (§9).
- **§4:** every Mac shortcut, menu, dialog, gesture and layout checked with real input (84 harness
  checks pass); colours sRGB-correct on both displays; lime audit 0; mockup match at 2× within 1 px
  except one reference deviation (explained); export byte-identical to Windows (12/12).
- **Audio:** **0 dropouts in 32 matrix runs** (built-in: A, B, C, D, G × 64/128/256/512; M2: A, C, G
  × 4); fader stress at 64 frames: 0 late, 0 overloads, 0 allocations; in-app device switch 67–93 ms.
- **Performance and heat:** demo 9.3–9.9%, 200 clips 10.7–11.4% (−3 points and −30% instructions per
  frame against the PERF-macos build); 20-minute loop: thermal state nominal throughout, 253 mW.
- **Fixed (8 bugs, all macOS-gated):** ⌘-shortcuts in text fields and dialogs (MC-D3), About in the
  App menu (D5), the Dark menus note (D7), MEM (D10), window tabs (D11), ⌥ + trackpad row height (D14),
  **the 60 fps cap bypassed during clip drags, 126 → 60 fps (fork commit 14, D16)**, **cursor files
  re-read on every move, Overview drag 42 → 27% (fork commit 15, D18)**.
- **Left (each with a proposal, §8):** the 60 Hz holds and full-lane repaint cost (renderer), no
  `.app` bundle (menu title "app", no Finder open), untested trackpad, and three **shared bugs** found
  on the Mac (Return/⌫ act behind dialogs, two colour-preview leaks), not fixed to keep shared code
  untouched.
- **Shared code changed: none** at runtime on Windows (every change is macOS-gated, docs, tools or
  tests; the fork commits are gated too). A Windows gate run isn't needed for this branch, but the
  merge waits for gate 6.

---

## 1. Environment (§2; every fact from two sources)

| Fact | Value | Sources |
|---|---|---|
| macOS | 26.6.2 (25G83), Darwin 25.6.0 | `sw_vers`; `uname` |
| Machine / chip | Mac mini (Macmini9,1), Apple M1, 4 P + 4 E cores, 16 GB | `system_profiler SPHardwareDataType`; `sysctl` |
| Rust | rustc 1.100.0 (0ef1bc386 2026-11-09), aarch64-apple-darwin | `rustc -Vv`; the build's host triple |
| App | `main` ef2ea19 at the start; `mac/catchup` (this pass) | `git log` |
| vizia fork | 60907aad at the start (resolved from GitHub: `Cargo.lock` source line, fetched and built); **9110e982** at the end (fork commit 14, §6) | `Cargo.toml` pin; `Cargo.lock` |
| GL | `GL_VENDOR="Apple" GL_RENDERER="Apple M1" GL_VERSION="4.1 Metal - 90.5"` (Apple's GL-on-Metal layer, as in PERF-macos.md) | the app's one-time log; PERF-macos.md §1 |

**Displays** (`display_info.swift`: NSScreen, CGDisplayMode, CVDisplayLink; `system_profiler
SPDisplaysDataType`):

| Display | Role | Mode | Refresh | Backing | Notes |
|---|---|---|---|---|---|
| GA27T1M (CG 2) | main, 2560 × 1440 at (0, 0) | ioModeID 104 | **320.000 Hz** (all sources) | **1.0** | wide-gamut profile |
| XB271HU (CG 1) | second, 2560 × 1440 **above** the main one (global y −1440) | ioModeID 18 | 60 Hz nominal; CVDisplayLink **59.9505 Hz** (PERF-macos §11: really 59.953) | **1.0** | sRGB-like profile |

- The window opens on the **main (320 Hz) display** (`macdrive info`: display 2, backing 1.0, maxFPS
  320), and moves between the displays correctly: `display_hz` in the audio log follows each move
  (§3.2), and the colours stay right on both (§4.7).
- Neither display is Retina. The 2× checks used a session-only HiDPI mode of the second display
  (1280 × 720 pt, 2× backing; restored afterwards, MC-D12).
- The macOS 26 title bar is **32 pt** (not 28): the harness measures it (MC-D8).

**Audio devices** (`coreaudio list` and `system_profiler SPAudioDataType` agree):

| Device | Transport | Rates (Hz) | Buffer frames: current, range | Output latency + safety offset |
|---|---|---|---|---|
| **Mac mini Speakers** (built-in; the main matrix) | built-in | 48000 / 44100–96000 | 512, 15–4096 | 64 + 116 frames, stream latency 408 |
| **MOTU M2** (plugged in during the run, MC-D2) | USB | 48000 / 44100–192000 | 512, 15–4096 | 14 + 14 frames |
| Sound Blaster GS3 (the system default at the start) | Bluetooth | 44100 only | 512, 14–1024 | 9900 frames (224 ms) |
| GA27T1M / XB271HU | DisplayPort / HDMI | 48000 / 32000–48000 | 512, 15–4096 | 88 + 320 / 96 + 576 |

The system's default output was never changed by this run: every run names its device in its own
`settings.json` (Settings ▸ Audio's field), and the device switch went through the app (§5).

**Fonts:** Inter 4.001 and JetBrains Mono 2.304 are loaded from the app's own bytes
(`add_font_mem`); neither is installed on the system, so no system copy can stand in. The Mac
glyphs the menus and the shortcut sheet use (⌘ ⇧ ⌥ ⌃ ⌫ ⌦ ⎋ ↩) are all in the bundled Inter (cmap
checked). One glyph is not: "▸" falls back to a system font in three status messages (shared
code, both platforms; proposed: write ">" as the clip panel already does).

## 2. Gates (§8)

| Gate | Result |
|---|---|
| 1. `cargo test` on ARM; golden and migration hashes equal to x86 | **PASS**: 207 pass + 1 ignored doctest (the Windows set; the CPU-counter test is per platform). Golden, v2→v6 migration hashes from the x86 builds pass; the Windows export re-made byte-identical (12/12) |
| 2. Clippy clean, 0 warnings | **PASS**: `cargo clippy --workspace --all-targets --release` clean; release build 0 warnings |
| 3. §4 table: nothing "broken" without a reason and a fix | **PASS**: the one "broken" row (Return/⌫ behind dialogs) is shared code, with the proposed fix (§8); gaps are bundle, trackpad, unplug |
| 4. Lime audit 0, mockup match ≤ 1 logical px, contrast unchanged | **PASS**: lime 0 (10 scenes); mockup: every element of E1-header and E1-overview at 2×, and E1-row-heights at 1×; at 2× the dB scale labels sit 1.5 px from the @2x PNG but 0.5 px from the spec's HTML box, the PNG itself moving them (MC-D13); one new text (the menus note) uses the existing note style (4.62:1) |
| 5. Audio: 0 dropouts at ≥ 128 frames on the built-in output | **PASS**: 0 in all 20 built-in runs (and at 64, and on the M2) |
| 6. Pacing: 60 fps cap, 0 stalls, both displays | **FAIL on the 60 Hz display** (10 holds > 1.5 vblanks per 120 s; MC-D21). 320 Hz: 60.0 fps, 0 app frames > 20 ms, ideal 5/5/6 cadence, also while dragging (after fork 14) |
| 7. No thermal "critical" in the 20-minute loop | **PASS**: nominal throughout |

## 3. Build, tests and the old Mac results (§3)

- `cargo build --release`, `cargo test --workspace --release`, `cargo clippy --workspace
  --all-targets --release`, all on ARM: 207 tests pass + 1 ignored doctest (the same set as the Windows Polish 2 run apart from the per-platform CPU test), clippy clean, 0 build warnings.
- **Byte-identical to x86.** The golden render (`scale_test_renders_to_the_golden_hash`) and every
  migration test (v2→v3, v3→v4, v4→v5, **v5→v6**: 9 + 11 + 11 + 11 files, render and export hashes)
  compare against hashes recorded by the x86 builds and pass here; v6 includes the Polish 2 gained
  mix at unity (bit-identical to the plain mix, `chip::mixer` test). Third source, the export:
  the committed Windows game export (`docs/export-examples/`, 12 files: the mix, intro, loop, 8
  stems, JSON) re-exported on the Mac with the same command is **byte-identical, 12 of 12**
  (SHA-1). (The `docs/audio/demo_excerpt_*` files are not comparable: the demo song has changed
  since they were rendered, 6c11d6b.)

### 3.2 Pacing on today's build

PERF-macos.md's method: `chipbench` + an Instruments Metal System Trace
(`displayed_surfaces.py`, display side) and the app's frame log (`pacing_mac.py`), default settings
(the **60 fps cap is the default**: every run below sets no cap). Raw: `results/pacing.txt`,
`results/pacing_60_move.txt`.

| Case | App: fps · p99 · max · > 20 ms | Display: updates/s · holds (vblanks: count) | Verdict |
|---|---|---|---|
| 320 Hz, demo, 30 s | 60.0 · 18.2 · 19.9 ms · **0** | 67/s · **5: 1058, 6: 492** (the ideal 5/5/6), 7: 22, 9–10: 6 | cadence ideal; app side clean |
| 320 Hz, 200-clip drag, 30 s (fork 14) | 60.0 · 18.1 · 22.6 ms · 1 | 67/s · **5: 1061, 6: 506**, 7: 21, 8: 2, 16: 1 | cadence ideal (it was 122–126 fps before fork 14, MC-D16) |
| **60 Hz** (59.953), demo, 120 s (the §11 stopgap) | 60.0 · 18.6 · 25.1 ms · 16 | 59.8/s · **1: 7161, 2: 2, 3: 8** | **10 holds > 1 vblank per 120 s** (PERF-macos §11 after the stopgap: 6–9) |
| Moves 320 → 60 → 320 → 60 → 320 Hz while playing, 45 s | 60.0 · 18.5 · 20.5 ms · 3 | 60 Hz segments: 1: 1231, 2: 3, 3: 1 | `display_hz` follows each move; one hold per arrival, as in §11 |

- The 320 Hz display-side counts include this session's terminal, which redraws continuously on
  that display (PERF-macos: other surfaces split holds and add short ones; the 1–4-vblank rows are
  those). The app's own intervals are clean.
- **The 60 Hz display still shows the stopgap's residue**: isolated 2- and 3-vblank holds whose cause
  PERF-macos.md §11 didn't find. By the Windows "stall" definition (a gap over 1.5 vblanks) that is
  10 stalls per 120 s, so **gate 6 is not met on that display** (MC-D21). Proposed fix: frames paced
  by the display link of the window's display (`CVDisplayLink` / `CAMetalDisplayLink`), best done with
  Skia's Metal backend (PERF-macos.md §6–7): a renderer change, outside this pass's remit.

## 4. Everything since the engine, on macOS (§4)

Checked with real (HID-level) key and mouse events (`macdrive.swift`), the menu bar read through
Accessibility, and each result proven from the saved project, `settings.json`, the dev-mode log or
a screenshot measurement. The scenario harness (`docs/mac-catchup/tools/scen_mac.py`) has 87 (84 pass, 0 fail, 3 notes for shared bugs)
checks; results in [`results/scen_mac.txt`](docs/mac-catchup/results/scen_mac.txt).

| # | Feature | Result | Evidence |
|---|---|---|---|
| 4.1 | macOS menu bar with the App menu (About, Settings… ⌘,, Quit ⌘Q there, not under File) | **fixed** (About) / works | AX dump of the menu bar; `mac-menubar.png`. About was the system panel ("app", no licences): now ours (MC-D5) |
| 4.1 | App menu title | **gap** | Reads "app" (the process name): needs an `.app` bundle (MC-D4) |
| 4.1 | Window menu (Minimize ⌘M, Zoom, Bring All to Front); no tab items | **fixed** | View ▸ Show Tab Bar pushed the UI down 36 pt; tabbing off (MC-D11) |
| 4.1 | Every shortcut with ⌘ / ⌥, real key events | works | 65+ harness checks: File, Edit (incl. ⌘Y, ⌦), time commands, ⌘E/⌘J/⌘R/⌘I/⇧⌘M/⌘L, arrows and ⌘2 grid, Space/Home/T, ⌥⌘B, ⌥⇧F, ⌥⌘O, B, W, =/−, Z/X, Return; piano roll ↑/⇧↑/→/⇧→/⌘U/⌘E/⌘J/⌘D/⌫. Each fires once (no menu + key double fire) |
| 4.1 | ⌘-shortcuts inside a text field or with a dialog open | **fixed** | ⌘Z in the search undid the song, ⌘A/⌘C/⌘V hit the clips; with Settings open ⌘Z/⌘D reached the song (MC-D3; before/after in `results/text_menu.txt`) |
| 4.1 | Mouse modifiers: ⌥-drag copies, ⌘-drag no snap, ⌥+wheel row height, ⌥-drag every row, ⌘-click S adds, ⌘+wheel zoom, wheel / ⇧+wheel scroll | works | saved files, screenshots |
| 4.1 | ⌥ + trackpad swipe over a track | **fixed** | 72 → 300 px on one 40 px swipe; now 72 → 96, a mouse notch still 12 px (MC-D14) |
| 4.1 | Shortcut sheet with Mac glyphs | works | `mac-shortcuts.png`; glyphs from the bundled Inter |
| 4.1 | Save prompts (New, Open, Quit, the close button) and the last window | works | Native sheet Save / Don't Save / Cancel; Cancel keeps; closing the last window quits after the sheet; recovery kept (MC-D6) |
| 4.1 | Recent files | works | File ▸ Open Recent newest first with the folder; `settings.json` |
| 4.1 | Opening a `.chipdaw` from Finder / Dock | **gap** | No bundle, no document type (MC-D4). Command-line open and dropping on the window work (shared code) |
| 4.2 | Dark menus setting | works (hidden) + note | Hidden outside Windows; Settings ▸ Display says menus follow the system appearance (MC-D7) |
| 4.3 | Settings (⌘,) and Project Settings (in-app modals) | works | Harness: ⌘, opens, Esc closes; File ▸ Project Settings… |
| 4.4 | File dialogs (rfd): open, save, export, MIDI import, DPCM import | works | Real NSOpenPanel / NSSavePanel driven by keys: a name without extension saves `.chipdaw`; export writes `.wav` + loop files + JSON; MIDI → 8 clips; DPCM → a sample. Open Recent lists `/var` and `/private/var` spellings of one folder (symlink; only under /tmp or /var) |
| 4.5 | Piano roll: editing, the drag fix | works | Every key above; 2,000-note drag 13.1–13.9% at 60 fps (§6). Scales: not built yet (VISION, later batch) |
| 4.6 | Interface scale 100 / 125 / 150 % on both displays | works | Window 1440 × 920 / 1800 × 1150 / 2160 × 1311 content on both displays (150%: macOS shortens it to fit under the menu bar; the lanes scroll); text crisp, layout intact (`tools/scale_mac.py`) |
| 4.7 | Colour management (sRGB tagged, both displays) | works | App = native AppKit window, 0 levels raw; sRGB within 1 level of every token (`mac-color-check.png`, VISUAL-SPEC.md §12) |
| 4.7 | Banding (dithering) | works | Canvas 51 → 24 px longest, mean 29.2 → 1.9; matches Windows (`results/banding.md`) |
| 4.7 | Lime audit | works | **0 violations**, 10 scenes, the Windows checker unchanged (`results/lime-audit.md`) |
| 4.7 | Mockup match at 2× backing | works (one reference deviation) | E1-header and E1-overview pass; row heights: dB scale labels 1.5 px off the @2x PNG, 0.5 px off the HTML spec (MC-D13; `mac-mockup-match.png`) |
| 4.7 | Every Polish 2 layout: row heights, widths 380/520/600, side × order, browser right, status bar, Overview on/off | works | `mac-layouts-4.png`, `mac-overview.png`, `mac-status-bar.png` |
| 4.8 | Ruler drag-zoom, scroll, click, double-click; W; Overview drag, zoom, jump, double-click | works | dev log values per gesture |
| 4.8 | Mouse wheel, ⌥+wheel | works | |
| 4.8 | Trackpad: two-finger scroll and pinch | **untested: no trackpad** | Synthetic continuous scrolls work (and found MC-D14); pinch can't be synthesized |
| 4.9 | Status bar: messages, problem icon, hover help | works | `mac-status-bar.png` (problem via the dev hook) |
| 4.9 | Problem icon on unplugging the USB interface while playing | **needs the user** | Can't unplug unattended; the device-loss path itself is ENGINE-macos test H |
| 4.10 | Export byte-identical to Windows | works | 12/12 files SHA-1 equal to `docs/export-examples` (§3) |
| 4.11 | Colours: the grid popup, Settings page, `next_track_color` tests | works, with two shared bugs | A pick saves, one ⌘Z restores; unit tests pass. Hover-then-Esc keeps the preview, and hover-A-pick-B undoes to A: shared code (MC-D15) |
| — | MEM read-out | **fixed** | Showed "--" on macOS; now the phys footprint, agrees with `top` / `footprint` (MC-D10) |
| — | Frame cap during clip drags | **fixed** | 122–126 fps / 32% → 60 fps / 20% (fork commit 14, MC-D16) |
| — | Cursor in drags | **fixed** | Overview drag 42% → 27% (fork commit 15, MC-D18) |
| — | Return / ⌫ with a dialog open and a clip selected | **broken (shared)** | Return opens the piano roll behind Export; the key reaches the arrangement first. Proposed fix in §8 (MC-D9) |

## 5. Audio (§5)

ENGINE-macos.md's method on today's build (fork 15): `matrix_mac.sh` with the device named per run
(`CHIPDAW_DEVICE`; the system default never changed), CoreAudio's overload report and the engine's
late detector as the dropout signal (no loopback cable, ENGINE-macos §3), tapmon on the same device
as "dropouts visible in the delivered stream". Lengths: MC-D22.

### Built-in output (Mac mini Speakers, 48 kHz): **0 dropouts in all 20 runs**

Every row: tap dropouts 0, late 0, overloads 0, overruns 0, rebuilds 0, allocation violations 0;
the buffer granted exactly as asked. Full table: [`results/matrix_builtin.md`](docs/mac-catchup/results/matrix_builtin.md).

| Test | 64 | 128 | 256 | 512 | Notes |
|---|---|---|---|---|---|
| A. Demo playing, 300 s | 0 · p99 0.7% · gap 1.39 ms | 0 · 0.6% · 2.72 | 0 · 0.6% · 5.39 | 0 · 1.8% · 10.75 | 512 runs on E-cores (P share 0%), as in ENGINE-macos §2.3 |
| B. 200 clips + continuous scripted edits, 300 s | 0 · 0.4% · 1.38 | 0 · 0.3% · 2.71 | 0 · 0.4% · 5.39 | 0 · 0.9% · 10.75 | 70,619 snapshots each, edit → tick ≤ 16.6 ms, 0 dropped |
| C. UI stalls (250 ms every 2 s), 120 s | 0 · 0.7% · 1.38 | 0 · 0.6% · 2.72 | 0 · 0.5% · 5.39 | 0 · 1.8% · 10.74 | 60 injected stalls each |
| D. Window churn (resize, minimise, hide, occlude), 120 s | 0 · 0.5% · 1.38 | 0 · 0.4% · 2.73 | 0 · 0.4% · 5.38 | 0 · 0.9% · 10.73 | |
| G. A/V marker test, 120 s | 0 · 0.5% · 1.39 | 0 · 0.3% · 2.71 | 0 · 0.3% · 5.39 | 0 · 0.9% · 10.76 | tap offsets −11.7 / −12.6 / −5.2 / −4.0 ms, sd ≤ 1.3 (no validated zero point, ENGINE-macos §4) |

(Cells: dropouts · callback cost p99 % of the period · largest callback gap.) The ENGINE-macos
stress matrix that was never completed on the final build is now complete on the built-in output.

### MOTU M2 (USB, 48 kHz)

**0 dropouts in all 12 runs** (A demo and C UI stalls and G A/V, 120 s each, at 64 / 128 / 256 /
512 frames): late 0, overloads 0, rebuilds 0, allocation violations 0; callback p99 ≤ 1.9% of the
period, largest gaps 1.39 / 2.72 / 5.40 / 10.77 ms. Table: [`results/matrix_m2.md`](docs/mac-catchup/results/matrix_m2.md).

### Device switch while playing, fader stress

- **Device switch while playing** (`device_switch_mac.py`, real clicks on Settings ▸ Audio, the
  demo playing): built-in → M2 → built-in → M2 → built-in. 4 rebuilds, no crash, 0 overloads, 0
  allocation violations; gaps **93 / 90 ms** switching to the M2 and **67 / 68 ms** back (Windows,
  BATCH3.md: 23–29 ms to HDMI, ~396 ms to the MOTU; ENGINE-macos: 74–164 ms on a default change).
- **The per-track dB fader under stress** (`fader_stress.py`): 260 fader drags across all 8 tracks in
  120 s while the demo plays at **64 frames** on the built-in output: 92,933 callbacks, **0 late, 0
  overloads, 0 allocation violations** (NoAllocGuard), callback max 1.9% of the period, largest gap
  1.40 ms. The same drag, saved, sets the track to −23.7 dB (so the drags reach the gain).

### Not run, and why
- **Unplug / sleep-wake (ENGINE-macos H, I):** need someone at the machine.
- **System-default switch (test E):** replaced by the in-app switch (MC-D22).
- **Physical loopback ground truth:** no cable (ENGINE-macos §3.2); still the best next check.

## 6. Performance and heat (§6)

`perf_mac.py` + `chipbench` (process CPU, % of one core; P-core share; energy; instructions per
presented frame, the stable measure on Apple Silicon), **60 fps cap**, 12 s per run, fresh launch per
run, builds interleaved. "Base" is the build PERF-macos.md measured (24c9d9c, the old layout; it has
neither the Overview, the E1 header nor today's file format). Medians; raw JSON lines in
`results/perf_320.jsonl` (3 rounds) and `results/perf_60.jsonl` (2 rounds). The 320 Hz Overview rows
are from before fork commit 15; after it they are the 60 Hz values (results/cursor_fix.jsonl).

| Scenario | 320 Hz: today · base | 60 Hz: today · base | M instr / frame (today · base) | PERF-macos.md / Windows (POLISH2.md) |
|---|---|---|---|---|
| Idle | 1.04 · 0.74 | 1.04 · 0.74 | — | 0.1% / 0.37 |
| Demo playing | **9.9 · 13.2** | **9.3 · 12.6** | 10.9 · 15.6 | 11.0–11.1 / 7.83 |
| 200 clips playing | **11.4 · 14.5** | **10.7 · 13.9** | 12.7 · 17.3 | 11.7 / 8.53 |
| 200 clips, dragging | 19.7 · 18.6 | 19.4 · 18.1 | 23.2 · 21.2 | 15.5 / 11.78 |
| Piano roll 2,000 notes, playing | 9.0 | 8.5 | 9.7 | — / 6.50 |
| Piano roll 2,000 notes, dragging | 13.7 | 13.1 | 15.2 | — / 6.76 |
| Time-scrollbar drag | 26.9 · 16.8 | 26.2 · 16.2 | 36.7 · 18.5 | — / 9.17 |
| Row-resize drag | 16.2 | 17.8 | 18.4–20.7 | — / 14.17 |
| Header-width drag | 20.3 | 31.0 | 23.6–42.3 | — / 25.77 |
| Ruler zoom drag | 26.6 | 26.1 | 36.0 | — / 14.93 |
| Overview drag | 42.9 → **27.0** (fork 15) | 26.5 | 47.0 → 37.2 | — / 8.77 |
| Overview zoom drag | 41.8 → **26.1** (fork 15) | 25.7 | 45.4 → 35.4 | — / 9.10 |

Every row presents 60.0 fps (59.8–60.1). CPU % on Apple Silicon moves with P/E placement and clocks
(PERF-macos.md §5): compare within a column and by instructions per frame.

**Regressions over 1 point against PERF-macos.md, explained:**
- **Clip drag +1.1 points (+9% instructions per frame).** The drag now also updates the display's
  START/LENGTH, the Overview and the E1 headers' meters (design pass, polish 2); the same work on
  Windows. Before fork commit 14 it was +14 points at 122–126 fps (MC-D16).
- **Idle +0.3 points.** MEM now updates on macOS (MC-D17).
- **Full-lane gestures (scrollbar, ruler zoom, Overview): 26–27%, about 2× the base's scrollbar and 2.9× the
  Windows figure.** Today's lanes draw more, and every draw pays the GL-on-Metal translation on the
  main thread. Proposed: the Metal backend, or scrolling a cached lane layer (MC-D19; renderer work,
  not made).
- **Header-width drag varies run to run** (15.7–20.4% at 320 Hz in one session, 31% in another at 23.6
  vs 42.3 M instructions/frame): relayout of every row each frame; not investigated further.
- **App-side frames over 20 ms in the new gestures** (row resize ~60, Overview zoom ~110, ruler zoom
  ~40 per 36 s): the first frames of each relayout/zoom step; the cap still presents 60.0 fps. Windows'
  pacing gate didn't measure these gestures. Proposed: measure them on Windows too before tuning.

**Improvements:** demo −3.3 points (−30% instructions per frame) and 200 clips −3.1 points against the
PERF-macos build, from the design and polish passes' caches.

### Heat

20-minute demo loop (`thermal_loop.sh`): the demo playing on the built-in output at the default
settings (60 fps cap, device-default 512-frame buffer), the thermal state every 30 s
(`ProcessInfo.thermalState`; `pmset -g therm` also logged all day in `results/thermal.log`, no sudo):

| | Result |
|---|---|
| Thermal state | **nominal in all 40 samples** (no "fair", "serious" or "critical"; `pmset`: no thermal or performance warning recorded at any point of the run) |
| App CPU (`ps`, 30 s samples) | mean 9.8%, max 11.0% of a core |
| App's own log over 1200 s | 9.9% of a core, 670.7 M instr/s, **252.8 mW**, P-core share 91% |
| Audio in the same 1200 s | 112,496 callbacks, 0 starved / late / overloads, max gap 10.78 ms, 0 allocation violations |

`powermetrics` (SoC temperature) needs sudo and wasn't used. The Mac stayed cool this time: the
earlier "very hot" sessions ran uncapped and traced; the 60 fps default and fork commits 14/15
keep it near 250 mW playing.

## 7. What was fixed

All macOS-only (`#[cfg(target_os = "macos")]`, the macOS menu spec, or fork code under the same
gate), each with before/after evidence:

| # | Bug | Fix | Before → after |
|---|---|---|---|
| MC-D3 | Menu key equivalents ignored the text-field and dialog rules: ⌘Z in a field undid the song, ⌘A/⌘C/⌘V hit clips, ⌘Z/⌘D reached the song behind Settings | `mac_menu_gate` in `run_menu_cmd`: field gets Cut/Copy/Paste/Select All; editor commands wait; File/app commands work | 8 → 9 clips kept; field "qq" (`results/text_menu.txt`) |
| MC-D5 | App menu About = the system panel: "app", no version, licences or font credits (unreachable on Mac) | App menu opens our About; "Hide ChipDAW" | `evidence/about-*.png` |
| MC-D7 | Dark menus: no word on the Mac | A note in Settings ▸ Display (the setting was already hidden) | `evidence/settings-display-mac-note.png` |
| MC-D10 | MEM read "--" | `proc_pid_rusage` phys footprint | "--" → 94 MB (top 95M, footprint 95 MB) |
| MC-D11 | View ▸ Show Tab Bar pushed the UI down 36 pt | Window tabbing off (libobjc call in the app binary) | `evidence/tabbar-before.png`; menu items gone |
| MC-D14 | ⌥ + trackpad swipe: 12 px per event (72 → 300 on one swipe) | `WheelSteps` accumulator | 72 → 96; mouse notch still 12 |
| MC-D16 | 60 fps cap bypassed during clip drags (`Handle::modify` → `WindowEvent::Redraw` → direct redraw) | **Fork commit 14** (9110e982): Redraw goes through the slots | 122–126 fps, 32% → 59.8 fps, 20% (bisected to 281ee66) |
| MC-D18 | Every `SetCursor` rebuilt an NSCursor, some from disk files: Overview drag 42% | **Fork commit 15** (2a146e6b): repeated cursor skipped | 42.3 → 27.0% |

Also: the harness (macOS tools for every §4 check), `chipbench` options (`--grab`, `--sway`, `--park`,
`--drag-mods`, `play-drag`), `matrix_mac.sh` device selection without touching the system default,
UX.md (Mac column in Apple's order, the macOS menu bar as built), VISUAL-SPEC.md §12.

## 8. What's left (each with a proposed fix)

| Item | Why it's left | Proposed fix |
|---|---|---|
| **60 Hz display: 10 holds > 1 vblank per 120 s** (gate 6) | Cause not found (PERF-macos §11); a fix is architecture | Frames paced by the window's display link (`CAMetalDisplayLink`) with Skia's Metal backend |
| **Full-lane repaints 26–27% on the Mac** (scroll, ruler zoom, Overview) | Renderer | Metal backend, or scroll a cached lane layer by a blit |
| **No `.app` bundle**: menu title "app", generic icons, no Finder / Dock open | Packaging belongs with signing and notarisation (release item, out of scope) | A bundle script: Info.plist (`CFBundleName`, identifier, icon, `CFBundleDocumentTypes` for `.chipdaw`, `.mid`, `.wav`), `application:openURLs:` → the existing open path |
| Code signing and notarisation | Out of scope | Release item |
| **Return / ⌫ with a dialog open and a clip selected** go to the arrangement (Return opens the piano roll behind Export) | Shared code (Windows too); fixing it holds the merge | The arrangement and piano-roll key handlers return early while `s.modal` or `s.export_dlg` is set (the root's `Ctx::Modal` rule) |
| **Colour preview leaks** (Esc after hovering a colour keeps it; hover A, pick B, ⌘Z gives A) | Shared code | Esc calls `restore_colours()` like `CloseMenu`; restore the preview before the pick's commit; a hover-A/pick-B/undo unit test |
| Ctrl/⌘-click doesn't toggle a clip (UX A4) | Shared; Shift-click toggles on both | Decide (UX) whether ⌘-click toggles; then one line in `arrangement::on_press` |
| "▸" not in Inter (3 status messages) | Shared | Write ">" as the clip panel does |
| Save sheet's bold line says "ChipDAW", the question below | Cosmetic, rfd maps title → messageText | On macOS pass the question as the title |
| 64-frame buffer not offered in Settings ▸ Audio on Mac | A feature (CoreAudio grants 64, ENGINE-macos §2.2) | Offer 64 on macOS |
| Trackpad pinch and physical two-finger scroll | No trackpad here | Check on a MacBook |
| Unplugging the USB interface while playing (problem icon) | Needs someone at the machine | ENGINE-macos test H with the status bar watched |
| A Retina (P3) panel; ProMotion 120 Hz | Not available | Re-run `color_mac.py` and the pacing traces on a MacBook |
| A cap bypass and a cursor reload on Windows high-refresh displays | Fork 14/15 are macOS-gated by the merge rule | Measure on a 144 Hz Windows display; ungate if it shows |
| Overview / row-resize / ruler-zoom first frames > 20 ms | New gestures, never measured on Windows | Measure on Windows first |

## 9. Shared code and the merge

**Every change is macOS-gated, docs, tools or tests**:
- `crates/ui/src/lib.rs` (`mac_menu_gate`, `#[cfg(target_os = "macos")]`), `perf.rs` (MEM branch),
  `views/chrome.rs` (the wheel accumulator and its field, both gated), `views/modal.rs` (`cfg!` macOS
  note), `menus.rs` (only `spec(mac = true)` and its MacPredefined arms, which Windows never builds;
  plus a unit test), `crates/app/src/main.rs` (`no_window_tabbing`, gated);
- `Cargo.toml` / `Cargo.lock`: the pin moves to fork commit 15. Fork commits 14 and 15 are gated to
  macOS inside vizia_winit; on Windows the fork compiles to the same behaviour as 60907aad;
- docs (this file, RUNLOG, UX.md, VISUAL-SPEC.md §12) and tools (`docs/mac-catchup/`, `chipbench`,
  `matrix_mac.sh`).

**Merge result: merged (2026-11-26).** §1.5 allows the fast-forward only if every gate passes, and
gate 6 fails on the 60 Hz display (a known, pre-existing limitation; the fix is architectural), so
the merge was first held. The owner then accepted gate 6 as is, and `main` was fast-forwarded
ef2ea19 → 18b5860 (`--ff-only`, no force) and pushed; the old `main` is tagged `pre-mac-catchup`
(ef2ea19, pushed). The §9 commands were run as written except `git checkout main`: `main` is checked
out in the `../before-mac-catchup` worktree (clean), so the fast-forward ran there. The 60 Hz holds
stay open for the display-link work (PERF-macos §11).

## 10. Screenshots and how to run

Screenshots in [`docs/screens/mac/`](docs/screens/mac/): `mac-menubar.png` (the App, File, Edit and
View menus), `mac-shortcuts.png` (the sheet with Mac glyphs), `mac-header-e1@2x.png` (the E1 header
at 2× backing), `mac-layouts-4.png` (the four header layouts), `mac-overview.png` (on and off),
`mac-status-bar.png` (a message, the problem icon, hover help, the problem list),
`mac-color-check.png` (the app and a native sRGB window on both displays), `mac-mockup-match.png`
(board | app | difference at 2×). Evidence for the fixes: `docs/mac-catchup/evidence/`.

```sh
cargo build --release
python3 docs/mac-catchup/tools/scen_mac.py --out results.txt      # §4: every shortcut, menu, dialog, gesture
bash docs/mac-catchup/tools/check_text_menu.sh                     # MC-D3 before/after (APP=… for another build)
python3 docs/mac-catchup/tools/color_mac.py                        # §4.7 colour management, both displays
python3 docs/mac-catchup/tools/lime_mac.py <dir> report.md         # the lime audit (Windows checker, Pillow stand-in)
python3 docs/mac-catchup/tools/hidpi_mac.py <dir>                  # mockup match at 2× (switches and restores a display mode)
python3 docs/mac-catchup/tools/screens_mac.py                      # the screenshots above
python3 docs/mac-catchup/tools/perf_mac.py --display 320 new=target/release/app > r.jsonl   # §6
bash docs/mac-catchup/tools/pacing_traces.sh <dir>                 # §3.2 (Instruments traces; deletes its ktrace files)
CHIPDAW_DEVICE="Mac mini Speakers" docs/engine/tools/macos/matrix_mac.sh "A B C D G" "64 128 256 512" 300 120   # §5
```
The terminal needs Accessibility, Screen Recording and (tapmon) System Audio Recording permission.
No Python imaging library is installed on this Mac (installing needs asking): the Windows passes'
checkers run unchanged through `docs/mac-catchup/tools/pilshim` (a pure-Python stand-in for the
Pillow calls they use) on captures converted to sRGB.
