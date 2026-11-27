# Task: Find and fix the ChipDAW GUI playback CPU problem

> *Historical report, kept as written under the working title ChipDAW. Since the rename (2026-11-27, [RENAME.md](RENAME.md)) the app is Cartwave by Glintworks: projects save as `.cartwave`, the executable is `cartwave`, and environment variables are `CARTWAVE_*` (the `CHIPDAW_*` names still work).*

You are working on the ChipDAW GUI prototype (Rust, vizia 0.4.0, Skia on OpenGL
via glutin/winit, Windows 11, RTX 4070 SUPER). Read `CLAUDE.md`, `NOTES.md` and
`FINDINGS.md` first. Everything they say about verifying APIs against the vizia
source still applies: **read the source, don't guess.**

The project is now **open source**. Any vizia changes we make should be written
to upstream quality, so we can contribute them back.

## The problem

During playback (playhead moving, meters animating, POS field updating), the
process uses **~60–66% of one core with the 43-clip demo set and ~97–100% with
200 clips**. Idle is 0%. This is a very fast PC, so these numbers are
unacceptable. They also mean the audio engine, which isn't connected yet, will
be competing for CPU.

FINDINGS.md suggests three contributors, but **none of them has been confirmed
with a profiler**:
- ~40% of a core from vizia repainting stylesheet views under a single
  full-window dirty rect
- ~15–30% from our custom drawing, mostly the arrangement
- a core pinned at ~100% when the animation timer interval is ≤16.6 ms, which
  FINDINGS.md works around with a 16.8 ms clock

## IMPORTANT correction to FINDINGS.md

FINDINGS.md says the display runs at **~320 Hz**. **The monitor is actually 60 Hz.**
Your first job is to find out where the 320 Hz figure came from: winit's
reported refresh rate, a second display, a wrong conversion, or something else.
Record the answer. It matters, because the timer-spin finding (spin below
16.67 ms, fine at 16.8 ms) lines up exactly with a **60 Hz frame period**. That
points to frame presentation and vsync, not timer granularity.

## Targets (release build, 1440×900 window, 60 Hz display, this PC)

| Scenario | Process CPU (one core = 100%) | Frame pacing |
|---|---|---|
| Idle | ~0% (unchanged) | no frames |
| Demo set, playing | **≤ 10%** | steady 60 fps; no frame > 20 ms over 30 s |
| 200 clips, playing | **≤ 20%** | same |
| 200 clips, dragging while playing | **≤ 25%** | same |

Also: no visible judder in playhead motion. A 16.8 ms clock on a 16.67 ms
display drops or doubles a frame about every 2 s, so check for that
specifically and fix it if you see it.

You may not reach the targets by lowering visual quality, turning off meters
or lights, lowering the frame rate, or removing effects, unless you first list
that as an explicit trade-off in the report and leave it **off** by default.

## Method: measure first, then fix one thing at a time

### Step 1: Baseline and instrumentation
1. Re-measure the FINDINGS.md table on this 60 Hz setup, so we have a clean baseline.
2. Get **real call stacks**. Use a sampling profiler that works on Windows with
   Rust release builds and symbols (for example `samply`, Windows Performance
   Recorder/Analyzer, or Tracy via `tracing`). Enable `debug = true` in the
   release profile for profiling builds. Save the flamegraphs or profile
   exports to `docs/perf/`.
3. Add lightweight timing spans around: event handling, style/layout, the draw
   traversal, each custom view's `draw`, Skia flush, and **buffer swap/present**.
   Show these in the F3 overlay.
4. Answer, with numbers: **what percentage of the CPU time is in our code, vizia's
   draw traversal, Skia CPU rasterization, the OpenGL driver, and swap/present
   (including any spin-waiting)?**

### Step 2: Test these hypotheses (confirm or rule out each, with evidence)

- **H1: The driver spins during vsync waits.** With the swap interval left at
  the driver default (vizia's swap-interval code is commented out), requesting
  frames faster than 60 Hz may make `SwapBuffers` busy-wait inside the OpenGL
  driver. NVIDIA's OpenGL driver is known to spin rather than sleep in some
  configurations. Check where the spinning core's samples actually land.
  Experiments: explicitly set the swap interval to 1 and to 0; for 0, pace with a
  high-resolution waitable timer or `DwmFlush`; drive animation from the
  presented frame instead of a free-running timer. Check the NVIDIA Control
  Panel settings (Vertical sync, Threaded optimization, Low latency mode) and
  record them, but **the fix must not depend on users changing driver settings.**
- **H2: Full-window repaint.** The single unioned dirty rect makes every view
  repaint each frame. Measure how much CPU the non-animating views cost. Then
  try: (a) per-region damage (several dirty rects, each clipped and repainted
  separately), and/or (b) caching static parts as Skia `Picture`s or layers
  (panels, track headers, the arrangement background grid, and the clip bodies
  when nothing about them has changed).
- **H3: CPU-rasterized blurs in stylesheet shadows.** vizia draws inset shadows
  as a blurred "crescent" path, and Skia may rasterize blurred arbitrary paths
  on the CPU every frame. Count how many blurred shadows are drawn per frame,
  and measure the cost. Fixes to try: cache shadow masks or layers, use Skia's
  fast paths for blurred rounded rectangles, or render shadows once per size
  and reuse them.
- **H4: Text cost.** Check whether labels, monospace values and clip names
  re-shape or re-layout text every frame. Cache shaped text, and only update
  POS and the dial values when their strings actually change.
- **H5: Our arrangement draw.** At 200 clips, recording takes 2.9–3.5 ms. Find
  out what dominates (path building, gradient shader creation, note previews,
  text). Cache gradient shaders and paints per lane color, and cache each
  clip's note preview as a `Picture` that is only rebuilt when the clip changes.
- **H6: Anything else the profiler shows** that isn't on this list. Trust the
  profile over these hypotheses.

### Step 3: Fix
- Apply fixes **one at a time** and re-measure after each. Keep a running table.
- Fix in our own code first. When the fix belongs in vizia (swap interval,
  damage regions, shadow caching, Textbox shadow order, DPI-scaled shadow
  blur), make it in a **local fork**, wire it in with `[patch.crates-io]` (or a
  path/git dependency), and keep each change a small, focused commit with a clear
  message, ready to send upstream as a pull request.
- Don't touch features, layout or visuals beyond what a fix requires.

## Deliverables
1. `PERF.md` containing:
   - the source of the "320 Hz" error, and the corrected display info
   - the baseline table, and a final table for every scenario above
   - a CPU breakdown by category (Step 1.4), before and after
   - each hypothesis: **confirmed / ruled out / partial**, with the evidence
   - each fix: what changed, measured impact, risk
   - any remaining gap to the targets, and what it would take to close it
2. Profiles and flamegraphs, before and after, in `docs/perf/`.
3. The vizia fork changes, listed as separate commits, each with a short
   upstream-PR description (problem, fix, how it was tested).
4. An updated F3 overlay showing per-phase timings and process CPU.
5. A note on whether any finding is likely **NVIDIA-, OpenGL- or Windows-specific**,
   so we know exactly what to check on AMD, Intel and macOS next.

## Rules
- Measure before and after every change. No change is "faster" without numbers.
- Use release builds for every measurement, and close other heavy apps. Take
  CPU samples over at least 10 s, not 3 s.
- If a probe contradicts FINDINGS.md, trust the probe, and say so in PERF.md.
- Ask before adding any new dependency. Licenses must be permissive or
  MPL-2.0-compatible with an open-source project.
- Out of scope: audio engine integration, new UI features, testing on other machines.
