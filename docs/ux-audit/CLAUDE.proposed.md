# CLAUDE.md: ChipDAW

> **Proposed replacement** for the root `CLAUDE.md`, from the UX audit (2026-11-25). It is not
> active. Replacing the real file is the CEO's call.

> Working title: **ChipDAW** (placeholder; don't brand anything) · Rust · GUI: Vizia (our fork) ·
> MIT OR Apache-2.0

## 0. Read VISION.md first

**Every session starts by reading [`VISION.md`](VISION.md).** It is the signed-off product vision
and overrides every older document, prompt or report, including this file. If a task conflicts
with it, stop and say so; don't build against it. New ideas go to [`IDEAS.md`](IDEAS.md), not
into a build.

Then read what the task touches:
- UX work: `docs/ux-audit/` (INVENTORY, FINDINGS, FEASIBILITY, UX-DRAFT) and, once it exists, `UX.md`.
- Product state: `PRODUCT.md`, `STABILIZE.md`.
- Rendering/pacing: `PERF.md` §8/§11 and `DESIGN.md` before touching drawing or the frame loop.
- Engine: `ENGINE.md` (the audio thread never allocates or locks).
- Piano roll, undo, file format: `PIANOROLL.md`. Export: `docs/EXPORT.md`. Assets: `docs/ASSETS.md`.

## 1. What we are building

A chiptune music workstation for indie game developers and musicians who already know a modern
DAW. **Ableton Live is the only workflow reference**: we copy its behaviour (gestures, shortcuts,
menus), never its look, names, icons or text. Sound comes from clean-room emulation of real
sound chips, starting with the **NES 2A03 and the VRC6 expansion chip**; more systems follow as
hardware profiles (VISION.md roadmap).

## 2. Hard constraints

1. **No images for UI.** No PNG/JPG/SVG, filmstrips or textures. Everything is stylesheets or
   procedural drawing. Fonts are fine (`assets/fonts/`, OFL, licences next to them).
2. **Rust + Vizia only.** No JUCE, web views, Tauri or Electron.
3. **Permissive licences only** (MIT / Apache-2.0 / BSD / Zlib; OFL for fonts). No GPL/LGPL/AGPL.
   **Ask before adding any new crate.**
4. **Plugin-ready.** The UI stays a library crate (`crates/ui`) separate from `crates/app`.
   Don't rely on anything that only works in a standalone window without a plugin fallback.
5. **No trademarks or copyrighted content in the product.** No "Ableton", console-maker,
   publisher or game names in the UI, code identifiers, presets or demo data (VISION.md
   "Naming"). Hardware names that describe what we emulate ("2A03", "VRC6", "NES", "Famicom") are
   allowed. Reference documents (audits, research) may name products as references.
6. **Every asset provably ours** and listed in `docs/ASSETS.md`. Never extract patches, samples or
   music from game ROMs.

## 3. Standing rules

- **Read the source, don't guess.** Vizia changes quickly and AI-written Vizia code invents APIs:
  check the fork's source (`../vizia`, pinned in `Cargo.toml`) before using an API.
- **When two sources disagree, resolve it with a third, never dismiss one** (measurements,
  docs, tools).
- **Behaviour claims about the reference** cite the official Live reference manual (chapter and
  section), paraphrased. If the manual doesn't say it, mark it "unverified". The verified list is
  `docs/ux-audit/manual-refs.md`.
- **Every specified behaviour gets an automated test** (unit test or scenario script), per
  VISION.md "How we work".
- **Budgets are requirements** (VISION.md principle 8): PERF.md and ENGINE.md targets; the audio
  thread never allocates; the model steps once per presented frame with vsync on (no free-running
  animation clock).
- **Keep tokens in one place** (`crates/ui/src/theme/tokens.rs` + `style.css`); every layout
  distance on the 4 px grid; soft, layered, low-alpha shadows; subtle gradients.
- **One shortcut table** drives handlers, menus, labels and the help sheet (once it exists); never
  hard-code a key name in a label.
- **File format changes** bump the version and ship an exact migration with hash tests
  (`crates/ui/tests/migration.rs`).
- **Compile and run often**; `cargo clippy --workspace --all-targets` clean and `cargo build
  --release` without warnings before calling work done. `cargo` may not be on PATH: use
  `~/.cargo/bin/cargo`.
- **Vizia fork:** if it changes, push it and update the rev in all nine `[patch.crates-io]`
  entries.

## 4. Working rules

- Commit on a branch; don't merge to `main` or push `main` without the owner's go-ahead.
- Keep app-driving shell steps small (launch; send input; read a log; stop the process).
- Out of scope unless a prompt says otherwise: audio recording tracks, third-party plugin
  hosting, exporting to hardware music formats (VISION.md "Not in the first build").

## 5. Project layout

```
crates/chip    no_std emulator, driver, instruments, presets (no allocation)
crates/engine  real-time audio thread, lock-free commands, song snapshots
crates/ui      the interface (library; plugin-reusable)
crates/app     the standalone window
docs/          reports, scenario/bench tools, screenshots, songs, the UX audit
```
