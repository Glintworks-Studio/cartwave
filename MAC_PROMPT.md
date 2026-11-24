# Task: Publish ChipDAW to GitHub, then build and measure it on macOS (M1)

This prompt has **two parts that run on two different machines**:

- **Part 1:** run on the **Windows PC**, where the project lives today.
- **Part 2:** run on the **Mac mini (M1)**, after cloning from GitHub.

Do only the part for the machine you're on. Read `CLAUDE.md`, `NOTES.md`,
`FINDINGS.md` and `PERF.md` first. The standing rules still apply:

- Read the source; don't guess APIs.
- Measure before and after every change.
- Trust probes over earlier documents.
- **When two sources disagree, resolve the conflict with a third source. Never
  dismiss one.** (The "320 Hz" mistake in FINDINGS.md came from dismissing a
  correct reading.)

The project is **open source**. Treat everything you push as public, even if
the repository starts out private.

---

## PART 1: Windows PC (prepare and push)

### 1.1 Make the vizia fork reachable from another machine
Today the fork is wired in with `[patch.crates-io]` pointing at the local
`../vizia` path. **A clone on the Mac will not build like that.**

1. Push the fork (branch `chipdaw-perf`, on tag `0.4.0`, all six commits) to its
   own GitHub repository. Ask the user which account or org to use, and whether it
   should be a GitHub fork of `vizia/vizia` (preferred, since it makes upstream
   pull requests easy).
2. Change `[patch.crates-io]` to a **git dependency pinned to the exact commit
   hash** (`rev = "…"`), not to a branch.
3. Do a clean build from a fresh clone in a temporary directory to prove it works
   without `../vizia`.

### 1.2 Scrub before the first push
Check everything that will be committed, especially `docs/perf/`:

- Search flamegraph SVGs, folded stacks, CSVs, logs and scripts for the Windows
  username, home-directory paths, machine name, other processes' names, serial
  numbers, and anything else identifying. Remove or redact it. If an artifact
  can't be cleaned reliably, leave it out and note it in PERF.md.
- **Parameterize every script in `docs/perf/tools/`.** No hard-coded paths. Use
  arguments or environment variables with sensible defaults, plus a short usage
  header in each script.
- Confirm that no ETL traces, PDBs, `target/` output or large binaries are
  committed. Add a proper `.gitignore`. If screenshots or videos are large,
  consider Git LFS, and ask first.
- Don't commit secrets, tokens or local config.

### 1.3 Repository basics
- A `README.md`: what this is (prototype), how to build and run, the key
  bindings, and links to the four docs.
- **License:** the company has chosen open source, but **not which license.**
  Do not pick one. Add a `LICENSE` placeholder and ask the user. (Note for the
  decision: the dependencies include MPL-2.0 crates via vizia's CSS stack and
  OFL fonts. Both are compatible with common open-source licenses.)
- Keep the font license files.

### 1.4 Push
- Ask the user for the repository name and visibility before creating it.
- Push with a clean history. Squash or fix any commits that contained data the
  scrub would have removed. **A scrub only counts if old commits are clean too.**
- Report the clone command and the exact commit hash for the Mac session.

---

## PART 2: Mac mini (M1): build and measure

### 2.1 Establish the facts first (no assumptions)
Record each of these, **cross-checked with at least two sources**:

- macOS version, chip, RAM, and the Rust toolchain.
- **Display refresh rate.** The user says 320 Hz. Verify it with macOS APIs
  (e.g. `NSScreen.maximumFramesPerSecond`, the `CGDisplayMode` refresh rate)
  *and* System Settings, and check which mode is actually active. If the
  sources disagree, resolve it before measuring anything.
- **Which rendering path vizia and Skia use on macOS** in this version
  (OpenGL through glutin/CGL? Metal?). Find out from the source, then confirm
  at runtime (log the GL renderer string, or equivalent). Note that Apple's
  OpenGL is deprecated, and on Apple Silicon it runs through a translation layer.
- Whether the fork's vsync / swap-interval change (fork commit 5) and the
  make-current change (fork commit 2) apply on this backend.

### 2.2 Build and smoke-test
- `cargo run --release -p app` from a clean clone. Fix any macOS-only build
  issues with minimal changes, each in its own commit.
- Check that everything renders correctly: fonts, shadows, gradients, the
  arrangement layer cache, and Retina scaling (the display's native backing
  scale). Take screenshots for `docs/screens/macos/`.
- Check the input differences: Cmd vs Ctrl (Ctrl-drag duplicate, Ctrl+wheel
  zoom), trackpad vs mouse scrolling, Delete vs Backspace. Keep behavior
  consistent with platform conventions and note every mapping you change.

### 2.3 The high-refresh question (critical)
The app now advances once **per presented frame**. On a high-refresh display,
it may render at the full refresh rate, which could be several times the CPU
cost of 60 Hz. Measure **both**:

| Mode | What to measure |
|---|---|
| Uncapped: render at the display's refresh rate | CPU, frame pacing |
| Capped at 60 fps | CPU, frame pacing, any judder from a cap that doesn't divide the refresh rate evenly |
| Capped at 120 fps (if the refresh rate allows) | same |

Don't pick the default yourself. Add a frame-rate cap setting (off / 60 / 120 /
display rate), report the trade-offs, and recommend one. **The user and team
make the final product decision.**

### 2.4 Measure the same scenarios as PERF.md
Run idle, demo set playing, 200 clips playing, and 200 clips dragging while
playing, for every cap mode in §2.3.

- **CPU:** process CPU over at least 10–12 s (e.g. `top -pid`/`ps`, or `xctrace`
  / Instruments Time Profiler). Apple Silicon has performance and efficiency
  cores, so "% of one core" is fuzzy. Report CPU %, and if possible energy
  impact (e.g. `powermetrics`, which needs sudo, so ask first).
- **Frame pacing:** the `CHIPDAW_FRAMELOG` per-frame log, plus an independent
  method if one is available (Instruments' display/hitch tooling for this
  backend, or another outside-the-app check). If you can't find an independent
  method, say so explicitly. Don't claim "0 missed frames" from the app's own
  counters alone.
- **Targets** (same as Windows, at the 60 fps cap): idle ~0%, demo ≤ 10%,
  200 clips ≤ 20%, dragging ≤ 25%, and no frame > 20 ms over 30 s.
  Report uncapped results against the same numbers, for comparison.

### 2.5 If the targets are missed: profile, then fix
- Profile with Instruments (Time Profiler) or `samply`, and break the CPU down
  using the same categories as PERF.md §4.
- Re-test the PERF.md hypotheses on this platform. Some fixes may be
  NVIDIA-specific and irrelevant here. Others (path identity, clip masks,
  damage regions) should carry over.
- Same rules as before: fix one thing at a time, measure, pixel-diff, and no
  visual downgrades unless they're listed as opt-in trade-offs.
- Fixes that belong in vizia go into the fork as new focused commits, with
  upstream-PR descriptions.

### 2.6 Deliverables
1. `PERF-macos.md`: verified environment facts (§2.1), the rendering backend,
   results tables for each cap mode, the pacing method and its limits, any
   fixes, and a **comparison with Windows**: which PERF.md findings carried
   over, which didn't, and anything new.
2. A **recommendation on the default frame-rate cap**, with numbers.
3. macOS screenshots.
4. Every change pushed as small commits. Update the fork pin if the fork changed.
5. A short list of anything that should be checked on AMD or Intel later.

## Out of scope
Audio engine integration, new UI features, the Textbox/DPI shadow bugs (unless
they block macOS rendering), and switching rendering backends (Metal/Graphite).
Recommend those if the data supports it; don't implement them.
