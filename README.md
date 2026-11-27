# Cartwave

> **Start here:** [PRODUCT.md](PRODUCT.md) is the current rundown: features, screenshots, history and next steps.

**Cartwave** is made by **Glintworks**: [glintworks.studio](https://glintworks.studio).

> **Prototype.** "ChipDAW" was the app's working title (renamed 2026-11-27: RENAME.md; `.chipdaw`
> projects from before open forever). This repository started as a GUI bake-off: it
> tests whether [Vizia](https://github.com/vizia/vizia) can deliver a fully custom,
> premium-looking DAW interface drawn entirely with stylesheets and procedural drawing (no
> bitmap images). It now plays the arrangement in real time through a clean-room NES 2A03 +
> VRC6 emulator (`crates/chip`, `crates/engine`; see [ENGINE.md](ENGINE.md)), and has a
> piano roll, global undo and `.cartwave` save/load, so songs can be written in it (see
> [PIANOROLL.md](PIANOROLL.md), including the full keyboard and mouse map).

![Arrangement view, prototype redesign, playing](docs/screens/redesign/main-playing-100.png)

The UI follows the designer's prototype ([`docs/design/reference.png`](docs/design/reference.png));
see [DESIGN.md](DESIGN.md) for how it was matched (sampled palette, gradient strokes,
4 px grid). The earlier look is on the `backup/pre-prototype-ui` branch.

The target product is a chiptune workstation for indie game developers (2A03 + VRC6 channel
layout, arrangement view). This prototype covers only the UI: arrangement clips with layered
lighting, rotary dials, inset text fields, level meters and a channel visualizer.

## Build and run

Requires a recent stable Rust toolchain (built with rustc 1.100).

```sh
cargo run --release -p app                      # builds and runs target/release/cartwave(.exe)
cargo run --release -p app -- --size 1200x800   # initial logical window size
```

Vizia is pulled from a fork, pinned by commit in `[patch.crates-io]` in `Cargo.toml`:
[jeffreyfrederick/vizia @ `chipdaw-perf`](https://github.com/jeffreyfrederick/vizia/tree/chipdaw-perf)
(tag 0.4.0 plus focused fixes: performance, frame pacing, macOS colour, a timer hang; see
[PERF.md](PERF.md) §8, [PERF-macos.md](PERF-macos.md) §11 and [PIANOROLL.md](PIANOROLL.md)). The first
build compiles Skia through `skia-safe`, which downloads prebuilt binaries when they are
available for your target.

Layout:

- `crates/chip`: the emulator and sound driver (`no_std`, no dependencies).
- `crates/engine`: the real-time host (cpal output, lock-free queues, song snapshots).
- `crates/ui`: the library (views, theme, state; reusable in a plugin host).
- `crates/app`: the standalone window (the `cartwave` executable). `--render-demo DIR` renders the demo offline to WAV.

## Controls

| Where | Input | Action |
|---|---|---|
| Global | `Space` | Play / stop |
| | `F3` | Frame-time overlay |
| | `F4` | Toggle the 200-clip stress set |
| | `F5` | Reload stylesheets |
| | `F6` | Cycle the frame-rate cap: 60 (default) / 120 / off (shown in F3) |
| | `F7` | Cycle the analog output filter: NES / NES + 440 Hz / Famicom / off (shown in F3). Until you press it, the filter follows the chip: Famicom with VRC6, NES with the 2A03 alone |
| Track headers | `M` / `S` | Mute / solo the channel |
| Arrangement | Click / `Shift`-click | Select / add to selection |
| | Drag clip body | Move (time and lane), snapped to the grid |
| | Drag clip edge | Resize |
| | `Alt` while dragging (macOS: `Cmd`) | Disable snap |
| | `Ctrl`-drag (macOS: `Option`-drag) | Duplicate |
| | `Delete` / `Backspace` | Delete selection |
| | `Ctrl`+`A` (macOS: `Cmd`+`A`), `Esc` | Select all, clear selection |
| | Wheel / two-finger swipe | Scroll horizontally |
| | `Ctrl`/`Cmd`+wheel | Zoom around the cursor |
| Ruler | Click / drag | Move the playhead (`Alt`, macOS `Cmd`: no snap) |
| Dials | Vertical drag, wheel | Change value (`Shift`: fine) |
| | Double-click | Reset to default |
| BPM / Pos fields | Drag, wheel | Scrub value (`Shift`: fine, BPM) |
| | Double-click | Reset (150 BPM / start) |

Environment variables are `CARTWAVE_*`; the pre-rename `CARTWAVE_*` names still work (a debug
build or `CARTWAVE_DEV=1` prints a note when one is used). Set `CARTWAVE_FRAMELOG=<path>` to write a per-frame CSV log (frame timings by phase), and
`CARTWAVE_FPS_CAP=60|120|off` to choose the starting frame-rate cap (default `60`; `display` is an
alias for `off`; see [PERF-macos.md](PERF-macos.md) §6). On macOS the modifier mappings follow Mac conventions; they are
defined in `crates/ui/src/input.rs`.

## Documents

- [NOTES.md](NOTES.md): Vizia capability checklist (what the stylesheet system supports, and the fallbacks).
- [FINDINGS.md](FINDINGS.md): bake-off report and verdict.
- [PERF.md](PERF.md): playback CPU and frame-pacing investigation and fixes (Windows).
- [PERF-macos.md](PERF-macos.md): the same on macOS (Apple M1, 320 Hz), and the frame-rate cap.
- [ENGINE.md](ENGINE.md): the chip engine. Emulation, band-limiting, the real-time architecture,
  stress tests, authenticity checks and known gaps. The listening pack is in
  [`docs/audio/`](docs/audio/), and the tuning table is in [`docs/tuning.md`](docs/tuning.md).
- [CLAUDE.md](CLAUDE.md): the original prototype brief.

Screenshots and a demo recording are in [`docs/screens/`](docs/screens/). Profiles,
flamegraphs and the measurement harness are in [`docs/perf/`](docs/perf/).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.

Third-party components keep their own licenses. The bundled fonts (Inter and JetBrains Mono)
are under the SIL Open Font License 1.1; see `assets/fonts/*-OFL.txt`. Some dependencies
(including Vizia's CSS stack) are MPL-2.0, which is file-level copyleft and compatible with
this license choice.
