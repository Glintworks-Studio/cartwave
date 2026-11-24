# ChipDAW: Vizia GUI prototype

> **Prototype.** "ChipDAW" is a working title. This repository is a GUI bake-off: it tests
> whether [Vizia](https://github.com/vizia/vizia) can deliver a fully custom, premium-looking
> DAW interface drawn entirely with stylesheets and procedural drawing (no bitmap images).
> There is no audio engine. All data (clips, notes, meters, playhead) is mocked.

![Arrangement view at 200% scaling](docs/screens/main-200.png)

The target product is a chiptune workstation for indie game developers (2A03 + VRC6 channel
layout, arrangement view). This prototype covers only the UI: arrangement clips with layered
lighting, rotary dials, inset text fields, level meters and a channel visualizer.

## Build and run

Requires a recent stable Rust toolchain (built with rustc 1.100).

```sh
cargo run --release -p app
cargo run --release -p app -- --size 1200x800   # initial logical window size
```

Vizia is pulled from a fork, pinned by commit in `[patch.crates-io]` in `Cargo.toml`:
[jeffreyfrederick/vizia @ `chipdaw-perf`](https://github.com/jeffreyfrederick/vizia/tree/chipdaw-perf)
(tag 0.4.0 plus six focused performance fixes, described in [PERF.md](PERF.md) §8). The first
build compiles Skia through `skia-safe`, which downloads prebuilt binaries when they are
available for your target.

Layout: `crates/ui` is the library (views, theme, state; reusable in a plugin host) and
`crates/app` is the standalone window.

## Controls

| Where | Input | Action |
|---|---|---|
| Global | `Space` | Play / stop |
| | `F3` | Frame-time overlay |
| | `F4` | Toggle the 200-clip stress set |
| | `F5` | Reload stylesheets |
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

Set `CHIPDAW_FRAMELOG=<path>` to write a per-frame CSV log (frame timings by phase).

## Documents

- [NOTES.md](NOTES.md): Vizia capability checklist (what the stylesheet system supports, and the fallbacks).
- [FINDINGS.md](FINDINGS.md): bake-off report and verdict.
- [PERF.md](PERF.md): playback CPU and frame-pacing investigation and fixes (Windows).
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
