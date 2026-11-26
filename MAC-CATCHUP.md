# MAC-CATCHUP: macOS brought level with Windows (everything since ENGINE-macos.md)

*2026-11-26 · unattended run of MAC_CATCHUP_PROMPT.md · branch `mac/catchup` from `main` ef2ea19
(Polish 2) · Mac mini M1, macOS 26.6.2 · decisions MC-D1… in [docs/RUNLOG.md](docs/RUNLOG.md) ·
harness and raw results in [`docs/mac-catchup/`](docs/mac-catchup/)*

## Summary (one screen)

__SUMMARY__

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

__GATES__

## 3. Build, tests and the old Mac results (§3)

- `cargo build --release`, `cargo test --workspace --release`, `cargo clippy --workspace
  --all-targets --release`, all on ARM: __TESTS__
- **Byte-identical to x86.** The golden render (`scale_test_renders_to_the_golden_hash`) and every
  migration test (v2→v3, v3→v4, v4→v5, **v5→v6**: 9 + 11 + 11 + 11 files, render and export hashes)
  compare against hashes recorded by the x86 builds and pass here; v6 includes the Polish 2 gained
  mix at unity (bit-identical to the plain mix, `chip::mixer` test). Third source, the export:
  the committed Windows game export (`docs/export-examples/`, 12 files: the mix, intro, loop, 8
  stems, JSON) re-exported on the Mac with the same command is **byte-identical, 12 of 12**
  (SHA-1). (The `docs/audio/demo_excerpt_*` files are not comparable: the demo song has changed
  since they were rendered, 6c11d6b.)

### 3.2 Pacing on today's build

__PACING__

## 4. Everything since the engine, on macOS (§4)

Checked with real (HID-level) key and mouse events (`macdrive.swift`), the menu bar read through
Accessibility, and each result proven from the saved project, `settings.json`, the dev-mode log or
a screenshot measurement. The scenario harness (`docs/mac-catchup/tools/scen_mac.py`) has __NCHECKS__
checks; results in [`results/scen_mac.txt`](docs/mac-catchup/results/scen_mac.txt).

__TABLE4__

## 5. Audio (§5)

__AUDIO__

## 6. Performance and heat (§6)

__PERF__

## 7. What was fixed

__FIXED__

## 8. What's left (each with a proposed fix)

__LEFT__

## 9. Shared code and the merge

__MERGE__

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
