# PERF-macos: ChipDAW on macOS (Apple M1, 320 Hz display)

> *Historical report, kept as written under the working title ChipDAW. Since the rename (2026-11-27, [RENAME.md](RENAME.md)) the app is Cartwave by Glintworks: projects save as `.cartwave`, the executable is `cartwave`, and environment variables are `CARTWAVE_*` (the `CHIPDAW_*` names still work).*

This is the companion to [PERF.md](PERF.md) (Windows). It covers the same scenarios and the same targets, plus the
high-refresh question: what should the app do on a display that refreshes 5× faster than the
one PERF.md was tuned on?

**Summary**

- **Build and rendering.** `cargo run --release -p app` builds and runs unchanged from a clean
  checkout. Rendering is correct at 1× (the display's native scale) and 2× (HiDPI).
- **Uncapped.** The app presents ~107–117 fps on the 320 Hz display, in an uneven 2- and 4-vblank
  rhythm (frames alternate 6.25 ms and 12.5 ms on screen). That costs ~18–21% of a P-core.
- **The cap.** The new frame-rate cap (fork commit 7, `F6` / `CHIPDAW_FPS_CAP`) gives exactly
  60.0 or 120.0 fps, with the ideal vblank cadence for a cap that doesn't divide 320 Hz.
- **Targets at the 60 fps cap:**
  - Met: idle 0.1%, 200 clips 11.7%, dragging 15.5%.
  - Narrowly missed: demo set **11.0–11.1% vs ≤ 10%**.
  - Frame time: the app side has 0–1 frames > 20 ms per 30 s. Display-side, 2–7 frames per 30 s
    stayed on screen > 20 ms (7–8 vblanks), and two 50 ms holds occurred during a drag.
- **Where the cost is.** ~70% of what's left is Skia's GL backend driving Apple's
  OpenGL-on-Metal translation layer on the main thread. Our code is ~0.3–0.8% of a core.
- **Recommendation: default cap 60 fps, with 120 fps as an opt-in "smooth motion"
  setting, and drop "display rate"** (§6). The team makes the final call.

---

## 1. Verified environment (§2.1)

Each fact was checked with at least two sources.

| Fact | Value | Sources |
|---|---|---|
| macOS | 26.6.2 (25G83) | `sw_vers`; `uname -a` (Darwin 25.6.0) |
| Machine / chip | Mac mini (Macmini9,1), Apple M1: 4 performance + 4 efficiency cores, 8-core GPU | `system_profiler SPHardwareDataType`; `sysctl hw.perflevel0/1.logicalcpu` = 4 / 4 |
| RAM | 16 GB | `system_profiler`; `sysctl hw.memsize` = 17179869184 |
| Rust | rustc 1.100.0 (0ef1bc386 2026-11-09), `aarch64-apple-darwin`, cargo 1.100.0 | `rustc -Vv`; `rustup show active-toolchain` |
| Tools | Xcode (xcrun 72), `xctrace` (Instruments CLI) | `xcode-select -p`; `xcrun --version` |

### Display refresh rate: 320 Hz is correct

Two displays are attached. The app window opens on the **main** one.

| Source | Main display (GA27T1M) | Second display (XB271HU) |
|---|---|---|
| `NSScreen.maximumFramesPerSecond` | **320** (min = max refresh interval 3.125 ms, so no VRR range) | 60 |
| `CGDisplayCopyDisplayMode().refreshRate` (active mode, ioModeID 104) | **320.0**, 2560×1440 px | 60.0 |
| `CVDisplayLinkGetNominalOutputVideoRefreshPeriod` | 75000/24000000 s = **320.000 Hz** | 400330/24000000 = 59.951 Hz |
| `system_profiler SPDisplaysDataType` | "2560 x 1440 @ 320.00Hz", Main Display: Yes | "@ 60.00Hz" |
| System Settings → Displays | GA27T1M, main display, 2560 × 1440 (Default), **Refresh rate: 320 Hertz** | — |
| Instruments `display-vsyncs-interval` | VSYNC every 3.125 ms on `external-1` | — |
| Window placement | `CGWindowListCopyWindowInfo`: the window is on the 2560×1440 main display | |

All sources agree, so there was no conflict to resolve. The tool is `docs/perf/tools/macos/display_info.swift`.

- The main display offers 50–320 Hz modes.
- **Backing scale is 1.0 on both displays.** No Retina panel is attached, so the "native
  backing scale" here is 1×. 2× was tested separately in a session-only HiDPI mode
  (1280×720 points at 2560×1440 px, 320 Hz), then restored.

### Rendering path

- **From the source:**
  - vizia 0.4's winit backend builds a glutin context (`ContextApi::OpenGl`, core profile, GLES
    fallback). Skia uses it through `skia_safe::gpu::direct_contexts::make_gl`.
  - On macOS glutin's only backend is **CGL**: an `NSOpenGLContext` attached to the winit
    `NSView`. The swap is `-[NSOpenGLContext flushBuffer]`, and the swap interval is set through
    `NSOpenGLCPSwapInterval`.
  - vizia's macOS build *also* enables skia-safe's `metal` feature, but nothing uses it.
  - winit dispatches `RedrawRequested` at the end of a run-loop iteration, not on the display
    cycle. So the swap is the only thing that paces frames.
- **At runtime:**
  - The new one-time log prints
    `GL_VENDOR="Apple" GL_RENDERER="Apple M1" GL_VERSION="4.1 Metal - 90.5"`.
  - `vmmap` shows `AppleMetalOpenGLRenderer.bundle`, `GLEngine` and `AGXMetal13_3` loaded.
  - So this is Apple's deprecated **OpenGL-on-Metal translation layer**. Every GL call is
    translated into Metal work **on the calling (main) thread**. There is no driver worker
    thread, unlike NVIDIA's.
- **Fork commit 2 (skip redundant make-current): applies.**
  - glutin's CGL `is_current()` compares `NSOpenGLContext::currentContext()`.
  - Without the commit, every frame would run `update()` + `makeCurrentContext` + `setView`
    (via `run_on_main`).
  - None of these appear in the Time Profiler traces. The make-current phase in the frame log
    is 0.001 ms.
  - The NVIDIA spin it fixed does not exist here. The commit is still correct and free.
- **Fork commit 5 (honour vsync): applies.**
  - glutin maps `SwapInterval::Wait(1)` to `NSOpenGLCPSwapInterval = 1`.
  - With the demo playing, vsync on presents 158–164 fps with 3.5–6 ms per frame blocked in
    `flushBuffer`.
  - `CHIPDAW_VSYNC=0` (a new experiment knob) runs at exactly the 4 ms pump rate (250.0 fps)
    for about +10 points of CPU. So the interval is honoured.
  - But it does **not** produce one present per vblank (§4).

## 2. Build and smoke test (§2.2)

- **Build.** A clean clone builds with no macOS-specific changes; the first build takes ~40 s
  using skia-safe's prebuilt binaries. `cargo build --release` has no warnings, and
  `cargo clippy --workspace --release` is clean.
- **Rendering:**
  - Fonts, gradients, stylesheet shadows, custom-drawn clip lighting, the arrangement
    layer cache, the dials and meters all match the Windows screenshots.
  - At 2× (HiDPI), text, hairlines and the playhead are crisp, and custom shadows scale
    correctly.
  - The known stylesheet-shadow DPI issue (FINDINGS.md §2.3) looks the same as on Windows.
    It is out of scope here.
  - Screenshots are in [`docs/screens/macos/`](docs/screens/macos/): `main`, `drag`, and
    `stress-200clips-f3` at `-100` (1×) and `-200` (2×; 1260×640 window).
- **Input mappings changed on macOS** (all in `crates/ui/src/input.rs`; Windows/Linux are
  unchanged):

| Action | Before (all platforms) | Now on macOS | Why |
|---|---|---|---|
| Select all | Ctrl+A | **Cmd+A** | Platform command key |
| Duplicate while dragging | Ctrl- or Cmd-drag | **Option-drag** | Mac DAW / Finder convention (Option-drag copies) |
| Bypass snap (clips, playhead) | Alt/Option | **Cmd** | Mac DAW convention (Cmd suspends the grid); Option is now taken by duplicate |
| Zoom with the wheel | Ctrl or Cmd | unchanged | Either works |
| Delete clips | Delete or Backspace | unchanged | The Mac "delete" key is Backspace; fn+delete is Delete |
| Wheel on dials / BPM / POS | one step **per event** | one step per accumulated line | A trackpad swipe sends dozens of small pixel deltas and jumped many steps. A mouse notch is still exactly one step |

Trackpad and mouse scrolling in the arrangement already worked smoothly: it scrolls by pixel delta
and zooms by a continuous factor. vizia converts trackpad pixel deltas to lines with a fixed /20.

All of this was verified with injected events (Cmd+A selects all; Option-drag leaves the original
and drags a copy; one wheel notch moves VOLUME 12 → 13; a 40-event continuous scroll moves DUTY by
its accumulated amount). There is no physical trackpad on this Mac mini. **A real trackpad
should be checked by hand.**

## 3. Frame-rate cap (§2.3)

**Setting:** off / 60 / 120 / display rate.
- `F6` cycles it, `CHIPDAW_FPS_CAP=off|60|120|display` sets the start value, and the F3
  overlay shows it.
- The default stays **off**, which is the behaviour PERF.md measured, until the team decides.

**Mechanism (fork commit 7,
[`40ef8cfe`](https://github.com/jeffreyfrederick/vizia/commit/40ef8cfeaefaa09c77171375e037ca04c99e065a)):**
- `EventContext::set_max_frame_rate(Option<f64>)`: the winit backend requests a redraw only when
  the next frame slot has arrived, and otherwise sleeps until it.
- The cap has to live in the backend. An app-side throttle only slows the app's own animation:
  a drag sends mouse events at 125 Hz and CSS transitions request frames on their own, and both
  would bypass it.
- Slots are fixed-rate (`slot += 1/cap`), so the average rate equals the cap even when it
  doesn't divide the refresh rate. Pacing from "last present + 1/cap" would round every frame
  up to the next vblank, giving 53 fps for a 60 cap at 320 Hz.
- `display_refresh_rate()` reports winit's monitor refresh rate, updated when the window moves
  or its scale changes. That covers moving the window to the 60 Hz display.
- Measured: the 60 and 120 caps present exactly 60.0 and 120.0 fps in every scenario, including
  while dragging.

## 4. Pacing method and its limits (§2.4)

- **App side.** `CHIPDAW_FRAMELOG` gives per-frame present intervals. On macOS that is the
  `flushBuffer` return time, which is *not* when the frame reaches the screen.
- **Independent, display side (validated):**
  - Instruments' Metal System Trace records the window server's `displayed-surfaces-interval`:
    each composited framebuffer put on a display, and how long it stayed there. We read the
    320 Hz display (`external-1`).
  - "k vblanks" means a frame stayed on screen for k × 3.125 ms. This is the macOS counterpart
    of PERF.md's DXGI desktop duplication.
  - Tools: `chipbench --trace`, `displayed_surfaces.py`.

  Validation against a reference window that changes on every k-th display-link callback
  (`validation/testwin.swift`):

  | Reference | Expected | Measured |
  |---|---|---|
  | static (k = 0) | 0 updates | 8.7 updates/s: other screen activity, here the terminal running the session (noise floor) |
  | every 4th vblank | 80/s, 4-vblank holds | 85/s; 346 of 425 frames held exactly 4 vblanks; the rest were split by the noise above |
  | every vblank | 320/s | **161/s, almost all 2-vblank holds**: in this setup windowed content reaches the screen at most ~160 times per second, even from a native AppKit view |

- **Limits:**
  - It sees the whole display. Other changes (the cursor during drags, a terminal spinner)
    add frames. They can split a long hold but never lengthen one, so "> 20 ms" counts are lower
    bounds.
  - The drag rows for off/display show ~240 updates/s because the moving cursor is composited
    too.
  - Tracing could perturb the app. With the final build, traced and untraced frame rates match
    (uncapped 109 fps traced vs 107–117 untraced; capped rates exact in both).
  - CPU was measured in separate untraced runs anyway, as on Windows.
- **Rejected method: ScreenCaptureKit.** `SCStreamFrameInfo.status == .complete` per window
  (`validation/sckwin.swift`) reported 261 "new frames" in 6 s from a **static** window, and only
  184/s from a 320/s source. It is not a reliable content-change signal at these rates.

## 5. Results (§2.4)

**CPU** is process CPU (all threads; 100% = one core) over 12 s, two untraced runs per cell
(`bench_mac.sh`).
- Both runs were ~99% on P-cores. P-core share, energy and instructions come from
  `proc_pid_rusage`, and `ps` agrees within 0.1 point.
- The scenarios match `bench.ps1`. The drag sends events at 125 Hz, like a mouse; the Windows
  script effectively moved every ~38 ms.

| Cap | Idle | Demo playing | 200 clips playing | 200 clips dragging |
|---|---|---|---|---|
| off (uncapped) | 0.1–0.2% | 18.3–20.9% · 107–117 fps · 554–612 mW | 19.7–20.5% · 107–111 fps · 601–630 mW | 25.3–25.4% · 110 fps · 722–734 mW |
| **60** | **0.1%** | **11.0–11.1%** · 60.0 fps · 336–338 mW | **11.7%** · 60.0 fps · 359–362 mW | **15.5%** · 60.1 fps · 448–452 mW |
| 120 | 0.1% | 20.6–20.7% · 120.0 fps · 637 mW | 22.0% · 120.1 fps · 687–691 mW | 27.4–27.5% · 120.1 fps · 804–810 mW |
| display (320) | 0.1% | 18.1–18.3% · 107 fps · 552–556 mW | 19.5% · 107 fps · 598–599 mW | 24.5–25.4% · 108–110 fps · 713–724 mW |
| *Target (at 60)* | *~0%* | *≤ 10%* | *≤ 20%* | *≤ 25%* |

> **Caveat: "% of a core" is not stable on Apple Silicon.** In an earlier, otherwise identical
> matrix the scheduler ran the capped cases on the **efficiency** cores (P-core share 0.21). The
> 60 fps demo then read **34%** of a core at **128 mW**, against 11% at 338 mW on P-cores. The
> work was the same: 0.91 G instructions/s in both runs. No thermal or power-mode event was
> recorded. E-cores take ~2.5× the time for the same work at ~⅓ of the power.
>
> P-core clocks also varied. During the profiling session the P-cores ran at an effective
> ~1.75 GHz (cycles ÷ CPU time), against ~3.05 GHz in the final matrix. The same build then
> presented ~160 fps uncapped instead of 107–117.
>
> The stable measure is **instructions per frame**: ~13–15 M per presented frame in every run.
> Compare runs by instructions/s or energy, and treat the CPU % targets as "on P-cores at full
> clock".

**Pacing** comes from 30 s runs per cell, with the app frame log and Metal System Trace at the same
time. "Holds" is how many vblanks each displayed frame stayed on screen (count over 30 s).

| Cap | Scenario | App fps · p99 · max interval · > 20 ms | Display: updates/s · max hold · > 20 ms | Hold distribution (vblanks: count) |
|---|---|---|---|---|
| off | demo | 109.4 · 12.7 · 17.3 ms · 0 | 113.8/s · 31.2 ms · 3 | 2: 1486, 4: 1388, 1: 285, 3: 225, ≥5: 10 |
| off | 200 | 108.9 · 14.7 · 19.8 · 0 | 111.9 · 25.0 · 2 | 2: 1583, 4: 1533, 1: 217, ≥5: 5 |
| off | drag | 129.1 · 14.8 · 25.0 · 1 | 238.7 (cursor) · 18.7 · 0 | 1: 6142 (cursor), … |
| **60** | demo | 60.0 · 17.3 · 22.3 · 1 | 60.0 · 21.9 · 2 | **5: 1194, 6: 599**, other: 6 |
| **60** | 200 | 60.0 · 17.4 · 19.4 · 0 | 60.0 · 25.0 · 7 | **5: 1184, 6: 591**, 3: 7, 8: 6, 7: 1 |
| **60** | drag | 60.0 · 17.5 · 18.5 · 0 | 59.9 · 50.0 · 3 | **5: 1184, 6: 595**, 16: 2, other: 3 |
| 120 | demo | 120.0 · 9.1 · 12.0 · 0 | 119.9 · 25.0 · 1 | **3: 2370, 2: 1194**, other: 14 |
| 120 | 200 | 120.0 · 9.0 · 11.0 · 0 | 119.8 · 28.1 · 3 | **3: 2366, 2: 1196**, other: 18 |
| 120 | drag | 120.0 · 9.3 · 11.0 · 0 | 120.0 · 18.7 · 0 | **3: 2365, 2: 1202**, other: 19 |
| display | demo | 107.7 · 12.6 · 15.9 · 0 | 108.2 · 31.2 · 3 | 2: 1408, 4: 1393, 3: 353, 1: 75 |
| display | 200 | 110.6 · 14.7 · 75.5 · 2 | 112.1 · 31.2 · 1 | 2: 1562, 4: 1501, 1: 223, 3: 56 |
| display | drag | 131.2 · 14.8 · 15.1 · 0 | 241.3 (cursor) · 15.6 · 0 | 1: 6307 (cursor), … |

What the pacing shows:

- **60 fps cap.** 320/60 = 5⅓ vblanks, so the best achievable cadence is 5, 5, 6, …, a 2 : 1
  mix. Measured: 1194 : 599. That is the ideal pattern, with ≤ 0.6% outliers.
  - The judder this implies is a 3.1 ms difference between consecutive frames' screen time.
  - The outliers are 7–8-vblank holds (21.9–25 ms) and, twice during a drag, a 16-vblank hold
    (50 ms). The target "no frame > 20 ms over 30 s" is **missed by 2–7 frames per 30 s on the
    display side**, but met on the app side (0–1).
- **120 fps cap.** 320/120 = 2⅔ vblanks, so the ideal is a 1 : 2 mix of 2- and 3-vblank holds.
  Measured: 1194 : 2370, with ~0.5% outliers.
- **Uncapped and "display rate".** The app alternates 2- and 4-vblank holds (6.25 / 12.5 ms on
  screen). Consecutive frames differ in screen time by 2×, which is uneven motion at a frame rate
  well below the display's.
  - Cause: the swap blocks until the previous drawable is consumed, and windowed content is
    composited at most ~160/s here (§4).
  - Rendering faster than ~160 fps buys nothing on this setup, and the extra frames cost CPU.

## 6. Recommendation: default cap

**Default to 60 fps. Offer 120 fps as an opt-in "smooth motion" setting. Drop "display rate"
(or keep it as an alias for off).**

| | 60 | 120 | off / display |
|---|---|---|---|
| CPU, demo / 200 / drag | **11.0 / 11.7 / 15.5%** | 20.7 / 22.0 / 27.5% | 18–21 / 20 / 25% |
| Energy, demo | **~337 mW** | ~637 mW | ~555–610 mW |
| Cadence on 320 Hz | ideal 5/5/6 (±3.1 ms) | ideal 2/3 (±3.1 ms) | uneven 2/4 (±6.25 ms), ~110 fps |
| Targets (demo ≤ 10, 200 ≤ 20, drag ≤ 25) | 200 and drag met, demo +1 | demo, 200 and drag all missed | all missed |

- **60 fps** costs about half as much CPU and energy as anything else, gives the most even
  cadence available on a 320 Hz panel, and matches the Windows baseline and every 60 Hz display.
- **120 fps** is visibly smoother for scrolling and dragging, with cadence as clean as 60. It
  costs about the same CPU as uncapped, but presents *more* frames *more evenly*, so it beats
  uncapped on every axis. That makes it the right "high quality" option.
- **Uncapped and display rate** are strictly worse than 120 on this machine: similar CPU, fewer
  and less even frames.
- **Laptops (not measured; ProMotion displays are variable-rate, 24–120 Hz).** The cap
  presumably still applies and should be re-checked on one.
- Not built: a divisor-aware cap (64 fps = 320/5 would give constant 5-vblank holds). It would
  need vblank-phase-locked slots, which in turn need display-link timestamps that winit doesn't
  expose. The gain over 60 would be removing the 3.1 ms alternation.

## 7. Profile and fixes (§2.5)

Time Profiler, 10 s each (`tp_categories.py`): % of one core, CPU-only samples, attributed to the
innermost recognizable frame.

| Category | 60 cap, demo | 60 cap, drag | Uncapped demo (~160 fps, P-cores at ~1.75 GHz; see §5 caveat) |
|---|---|---|---|
| GL-on-Metal translation (AppleMetalOpenGLRenderer, GLEngine, AGX, IOGPU), main thread | 4.4 | 5.1 | 17.3 |
| Skia GPU flush (op prepare, pipeline state, uniforms) | 2.5 | 3.1 | 12.1 |
| winit / AppKit event loop | 1.7 | 1.8 | 2.7 |
| vizia (traversal, style, layout, text) | 1.6 | 1.0 | 2.9 |
| Skia record | 1.3 | 1.9 | 6.8 |
| **Our code** (custom views, model) | **0.3** | **0.8** | 1.0 |
| Skia CPU raster (blur masks, glyphs) | 0.1 | 0.3 | 0.7 |
| **Total** | **11.8** | **14.1** | **43.4** |
| Threads | main 100% | main 100% | main 100% |

**PERF.md hypotheses re-tested on this platform:**

| PERF.md finding | macOS |
|---|---|
| H1 make-current spin (NVIDIA) | **Does not occur.** No make-current in profiles. Fork commit 2 applies and is harmless |
| H1 vsync wait cost in `SwapBuffers` | `flushBuffer` blocks (sleeping) 3.5–6 ms per frame uncapped; under a cap the limiter sleeps instead. Neither spins |
| Driver worker thread ~3% | **Doesn't exist.** GL-on-Metal does its translation on the main thread (the 4.4% row above) |
| H2 full-window repaint, damage regions (fork 6) | **Carries over** (vizia-level) |
| H3 path identity (fork 3), software clip masks (fork 4) | **Carry over.** No `GetSWMaskFP`, `PathOps` or per-frame `GrGLBuffer::Make` in the profiles; triangulation is 0.4–0.7% (it was 22% of the main thread on Windows before) |
| H5 small blurred rrects off the analytic path (fixed by our A8 masks) | **Carries over.** One remaining per-frame CPU blur mask comes from a stylesheet panel shadow under a damage rect: ~1.7% of frame work |
| Free-running clock judder (F6) | Carries over; the present-driven model step works unchanged |

**Fix attempts (one at a time, measured):**

| Change | Result | Kept? |
|---|---|---|
| Fork commit 7 + `FrameCap` | Enables every capped row above | **Yes** |
| Pump shorter than one 320 Hz frame (0.95 / rate = 2.97 ms) so uncapped could reach 320 fps | Never above ~160 fps (the composite ceiling, §4); "display" cap 64–68% vs "off" 50–53% in the same matrix. That matrix ran on down-clocked/E-cores, so the absolute numbers aren't comparable with §5 | No (reverted: more wakeups, no benefit shown) |
| Pump at half a capped frame (8.3 ms at 60 fps) to cut event-loop wakeups | Wakeups 273 → 126–185/s and 5% fewer instructions, but CPU 11.2–11.7 / 12.4 / 16.3% vs 11.0 / 11.7 / 15.5% | No (reverted) |

**Why the demo target is missed by ~1 point.** About 1.85 ms of P-core time per frame goes on
GL-on-Metal translation, Skia's GPU flush and present. This is a fixed per-frame cost that is
spread across many small draws, with no single hotspot. The levers that remain each save ≤ 0.2–0.6
points:
- skip stylesheet shadow layers that a damage rect can't see;
- fewer draws for the meters (2.4% of uncapped work) and the channel-light glows (1.8%).

Closing the gap properly means **not going through OpenGL**. Skia's Metal backend (skia-safe
already builds it on macOS; vizia would need a `CAMetalLayer` surface) removes the translation layer
and should also allow `CAMetalDisplayLink`-paced, vblank-locked frames. That's a backend switch,
out of scope here; it's the main recommendation.

## 8. Comparison with Windows

| | Windows (PERF.md, RTX 4070 SUPER, 60 Hz) | macOS (M1, 320 Hz, 60 cap) |
|---|---|---|
| Demo / 200 / drag CPU | 9.1–9.6 / 9.1–9.9 / 10.0–11.2% | 11.0–11.1 / 11.7 / 15.5% (P-cores) |
| Idle | 0.0–0.1% | 0.1% |
| Frames > 20 ms (30 s) | 0, display side (DXGI) | app 0–1; display 2–7 |
| Where the rest goes | ~6.5 points fixed NVIDIA GL present cost (worker thread + vsync wait) | ~7 points GL-on-Metal translation + Skia flush on the main thread |
| Thread layout | main + NVIDIA driver worker | main thread only |
| Pacing gate | `SwapBuffers` blocks to vblank: one present per refresh | `flushBuffer` doesn't lock to vblank; compositor ≤ ~160/s for windowed content; the cap supplies the pacing |

- **Carried over:** fork commits 3, 4 and 6 (Skia/vizia-level), our A8 shadow masks and layer
  caches, and the present-driven model step.
- **Didn't carry over:** the make-current spin and the driver worker thread (NVIDIA-specific).
- **New on macOS:**
  - The high-refresh pacing problem and the frame-rate cap.
  - E/P-core placement makes CPU % unstable.
  - GL-on-Metal per-draw translation cost on the main thread.
  - The ~160/s composite ceiling for windowed content.
- **Drag scenario differs:** 125 Hz events here vs ~26 Hz effective on Windows (`bench.ps1`'s
  sleeps), which explains part of the higher drag cost.

## 9. To check on AMD, Intel and other hardware

1. **Intel Macs with AMD or Intel GPUs.** OpenGL is a native driver there, not the Metal
   translation layer. Per-draw cost, swap blocking and vsync behaviour will all differ.
2. **Windows on AMD and Intel GPUs.** Whether the make-current spin (NVIDIA threaded optimization)
   exists, and the fixed per-present cost (PERF.md §10).
3. **High-refresh displays on Windows** (144/240 Hz). The cap is cross-platform (it lives in the
   winit backend). Check that it gives the same clean cadences as on macOS, and whether
   `SwapBuffers` already locks to the higher rate.
4. **Variable-refresh displays** (ProMotion MacBook Pro, G-Sync/FreeSync). Fixed-rate slots
   should be friendly to VRR, but this is unmeasured.
5. **Apple Silicon with Retina panels** (backing scale 2 natively), and M2/M3/M4, where E/P
   scheduling differs.
6. **A real trackpad** on macOS: the scroll accumulator was only tested with synthetic continuous
   events.

## 10. Reproducing

```sh
cargo build --release -p app
docs/perf/tools/macos/bench_mac.sh base 12                    # CPU matrix, all caps x scenarios
# pacing, one cell:
swiftc -O docs/perf/tools/macos/chipbench.swift -o /tmp/chipbench
/tmp/chipbench --app target/release/app --scenario demo --secs 30 --env CHIPDAW_FPS_CAP=60 \
  --framelog /tmp/fl.csv --trace /tmp/demo.trace
python3 docs/perf/tools/macos/pacing_mac.py /tmp/fl.csv 30
python3 docs/perf/tools/macos/displayed_surfaces.py /tmp/demo.trace --hz 320
# profile:
cargo build --profile profiling -p app
/tmp/chipbench --app target/profiling/app --scenario demo --secs 10 --env CHIPDAW_FPS_CAP=60 \
  --trace /tmp/tp.trace --template "Time Profiler"
python3 docs/perf/tools/macos/tp_categories.py /tmp/tp.trace
```

The terminal needs Accessibility permission (synthetic input). Raw traces are not committed: they
are large and system-wide.

## 11. 60 Hz pacing fix

**Status: stopgap fix in fork commit 9 ([`75e643a0`](https://github.com/jeffreyfrederick/vizia/commit/75e643a012f83f19e835b9adcfbff05fae86cdc8), app pin `00c4d17`). Large improvement, but the pass criterion
(≤ 2 frames > 20 ms per 120 s on the 60 Hz display) is not met: 6–9 remain.** They are isolated
3-vblank holds that the build without the fix shows at the same rate. Their cause is not found.

The problem (ENGINE-macos.md §8, test J): the cap's fixed 60.000 Hz slots beat against the 60 Hz
display's real rate. Every ~21 s the slots slide across a vblank boundary, which gives a
multi-second stretch of 2-vblank frames.

### 11.1 Refresh-rate sources and their agreement

60 Hz display (XB271HU, CG display 1, `external-0` in Instruments):

| Source | Rate |
|---|---|
| winit `refresh_rate_millihertz` (`CGDisplayModeGetRefreshRate`) | 60.000 (rounded) |
| `CVDisplayLinkGetNominalOutputVideoRefreshPeriod` | 400330 / 24000000 s = **59.9505 Hz** |
| `NSScreen.minimumRefreshInterval` | 0.016680416 s, the same value |
| `CVDisplayLinkGetActualOutputVideoRefreshPeriod` | the nominal value again (not measured independently) |
| Metal System Trace VSYNC events, least-squares fit over 120 s (5 runs) | **59.9528–59.9533 Hz** |

The display-link and trace sources disagree by 47 ppm. A third source settled it:
- The build without the fix beats against the display, and its hold clusters recur every
  ~21.3 s, which gives 60 − 1/21.3 = **59.953 Hz**.
- Display links started fresh every 10 s and sampled over 2 min gave vblank timestamps that fit
  59.9530–59.9532 Hz.

So the display really runs at 59.953 Hz in host time. The CoreVideo "nominal" figure is the mode's
pixel-clock ratio, 47 ppm off. That error is too small to matter for the slot *rate* (one vblank
of drift every ~6 minutes), but it does matter for *phase*, see 11.3.

The 320 Hz display reports 320.000 from every source.

### 11.2 Slot-rate rule

In `vizia_core::context::frame_slot_rate(cap, refresh)`, with `FRAME_SLOT_RATE_TOLERANCE = 0.02`:
- If `refresh ≤ cap × 1.02`: slots at `refresh` (one frame per vblank).
- Else, if `refresh / cap` is within 2% of a whole number `n`: slots at `refresh / n`.
- Otherwise: slots at `cap`, exactly as before (fork commit 7).

Examples (unit-tested):

| Display | Cap | Slots |
|---|---|---|
| 59.951 Hz | 60 | 59.951 Hz |
| 59.951 Hz | 120 | 59.951 Hz |
| 119.88 Hz | 60 | 59.94 Hz |
| 240 Hz | 60 | 60 Hz |
| 50 Hz | 60 | 50 Hz |
| 320 Hz | 60 / 120 | 60 / 120, unchanged |
| 144 Hz / 75 Hz | 60 | 60, unchanged |

Locking can exceed the cap by up to 2%.

The precise rate comes from `CVDisplayLinkGetNominalOutputVideoRefreshPeriod` for the window's
display (macOS) or `DwmGetCompositionTimingInfo` (Windows). It is used only when it agrees with
winit's rounded rate within 1 Hz. It is re-read on window move and scale change, and once a
second.

### 11.3 Phase findings

**Rate alone was not enough** (measured, 60 Hz display, cap 60, 120 s): with the slots at
59.9505 Hz but free-running, holds fell to 6, but the phase drifted from 0.45 to 0.77 of a vblank
over 120 s. That is the 47 ppm error, about 1 vblank in 6 minutes, so a longer session would
cross the boundary again.

**Where the boundary is.** The "commit phase" is when the app's GL command buffer is committed,
relative to VSYNC in Metal System Trace. Mapped across runs:
- commit phase 0.9 → 0.2 (wrapping through VSYNC): frames are held a vblank or skipped;
- 0.2–0.9: clean.

With the build without the fix, the hold clusters are exactly the stretches where the drifting
phase crosses 0.9–0.2.

**Fix: re-anchor to the display link's timestamps.** While the slots are locked (macOS only):
- Every 10 s a `CVDisplayLink` is started for ~6 vblanks, only to read their host timestamps, and
  then stopped. It never drives rendering.
- Slots are snapped to `anchor + 0.85 × period + k × period`.
- Calibration: commit phase ≈ slot phase − 0.32. At φ = 0.35 the commits landed at 0.05 and were
  held 771 times; at φ = 0.85 they land at 0.52–0.54.

Display-link behaviour measured with probes (`cvnow`, `cvlong`, `cvresync`):
- A **freshly started** link is anchored to the hardware. Anchors 5 s apart move by −0.20 to
  −0.23 ms, which is the 47 ppm drift, with ~0.03 ms jitter.
- A link **left running** produces an exact nominal lattice. It drifts away from the hardware and
  jumps back only after errors of 3–4 ms, so it is restarted for each sample instead.
- Single timestamps can arrive up to ~2 ms early, so the anchor is the *latest* lattice point of
  the sample.
- Estimating the real period from successive anchors failed: a 3 s baseline gave −377 ppm for a
  real −47 ppm. So the nominal period is used, re-anchored every 10 s (≤ 0.5 ms of drift between
  samples).
- Start/stop costs ~0.03 ms. The first callback arrives 22–26 ms after start.
- Traced runs show one slower present (3–12 ms) around each sample. Untraced runs mostly don't.

After a move or a scale change the phase is re-read at once, not at the next 1 s check.
Without that, the first second on the 60 Hz display ran at the new rate with an unknown phase
and showed ~0.7 s of 2-vblank frames after two of three moves (11.4).

**Remaining holds.** Every 60 Hz run (fixed or not, display link or not) has ~4–8 isolated
3-vblank (50 ms) holds per 120 s. In these, a single frame's present blocks 3–12 ms. They are not
in clusters and not tied to the commit phase. They are the reason the ≤ 2 criterion is missed.
The cause is not identified (not investigated further, as this task is the stopgap).

### 11.4 Before / after

Release build, demo playing, Metal System Trace displayed surfaces. "Before" is the build
without the fix (fork commit 8), "after" is fork commit 9.

| Test | Before | After | Pass |
|---|---|---|---|
| 60 Hz, cap 60, 120 s: frames > 20 ms (display) | **230** {1: 6723, 2: 225, 3: 5} | **6** {1: 7179, 2: 1, 3: 5} | **No** (≤ 2) |
| … app-side p99 interval | 31.55 ms | 17.55 ms | |
| … commit phase | sweeps 0 → 1 continuously | 0.52–0.54, sd 0.03 | |
| 60 Hz, cap 120, 120 s: frames > 20 ms | 9 at 120 fps | **9** {1: 7163, 2: 1, 3: 8} at 59.95 fps | **No** (≤ 2) |
| … energy / instructions per second | 753 mW / 2.15 G | 412 mW / 1.18 G | |
| 320 Hz, cap 60, 60 s: cadence (vblanks: count) | 5: 2284, 6: 1129 | 5: 2374, 6: 1184 | Yes (5/5/6) |
| … instructions per frame | 19.6 M | 19.7 M | Yes (+0.5%) |
| 320 Hz, cap 120, 60 s: cadence | 2: 2402, 3: 4755 | 2: 2377, 3: 4773 | Yes (2/3) |
| … instructions per frame | 18.3 M | 18.2 M | Yes |
| Moves 320 ↔ 60 Hz, 3 round trips | rate never refined (60.0) | `display_hz` follows each move within the 1 s log step | Yes (rate) |
| … app frames > 20 ms per segment on the 60 Hz display (3 × ~10.5 s) | — | rate check only once a second: 1 / 7 / 6, with ~0.7 s of 2-vblank frames after two arrivals. Phase re-read on move (commit 9): **1 / 1 / 1**, one hold at each arrival | Yes for the fix; one hold per arrival |
| … same on the 320 Hz display (4 segments) | — | 3 / 3 / 4 / 4 → **2 / 1 / 2 / 1** (22–28 ms holds) | Yes |
| Flash check, 60 Hz, 20 s (`flashcap`) | 8 flagged | 5 flagged | See below |
| Audio, all runs | 0 overloads, 1–2 late | 0 overloads, 1 late | **No** for "0 late" (same before) |

- **Flash check.** Every flagged frame in both builds is the 16 px top segment of a level meter,
  i.e. meter content, not a rendering fault. Neither build reproduces the "0" recorded after
  fork commit 8, so the check now has a floor from the demo's meters. No whole-window flicker
  is flagged in either build. Not a regression.
- **Move runs disturbed by Terminal.** Two further move runs with commit 9 had 287–298 long
  frames from a Terminal window redrawing on the 320 Hz display. In those runs the app's 320 Hz
  holds rose to 10–16 per segment, including the segment before the first move, where the code
  path is the same as before the change. The repeat with Terminal quiet (table above) is the
  clean measurement. Compare move runs only when other surfaces' long frames are at the ~14
  floor.
- **Audio "late".** The 1 late callback happens at startup, while the window is being placed,
  in both builds.

### 11.5 Thermal log

`thermalState` (ProcessInfo) and `pmset -g therm` were read before and after every run, with
≥ 2 min between runs. All 86 entries were **nominal**, with no CPU speed limit, so no run was
discarded. The longest runs were 120 s; there were no soak tests. The full log is below
(timestamps local, 2026-11-24):

```
15:47:55 base60_c60 before thermalState=nominal lowPower=false
15:51:37 base60_c60 after thermalState=nominal lowPower=false
15:54:29 fix60_c60 before thermalState=nominal lowPower=false
15:58:09 fix60_c60 after thermalState=nominal lowPower=false
16:13:13 cal35 before thermalState=nominal lowPower=false
16:15:04 cal35 after thermalState=nominal lowPower=false
16:17:16 cal85 before thermalState=nominal lowPower=false
16:19:08 cal85 after thermalState=nominal lowPower=false
16:20:59 fin60_c60 before thermalState=nominal lowPower=false
16:24:24 fin60_c60 after thermalState=nominal lowPower=false
16:26:34 base60_c120 before thermalState=nominal lowPower=false
16:30:41 base60_c120 after thermalState=nominal lowPower=false
16:32:51 fin60_c120 before thermalState=nominal lowPower=false
16:36:10 fin60_c120 after thermalState=nominal lowPower=false
16:38:20 base320_c60 before thermalState=nominal lowPower=false
16:40:13 base320_c60 after thermalState=nominal lowPower=false
16:42:23 fin320_c60 before thermalState=nominal lowPower=false
16:44:18 fin320_c60 after thermalState=nominal lowPower=false
16:46:28 base320_c120 before thermalState=nominal lowPower=false
16:49:04 base320_c120 after thermalState=nominal lowPower=false
16:51:14 fin320_c120 before thermalState=nominal lowPower=false
16:53:53 fin320_c120 after thermalState=nominal lowPower=false
17:00:28 exp_restart before thermalState=nominal lowPower=false
17:01:16 exp_restart after thermalState=nominal lowPower=false
17:03:21 exp_keep before thermalState=nominal lowPower=false
17:04:08 exp_keep after thermalState=nominal lowPower=false
17:06:13 exp_startonly before thermalState=nominal lowPower=false
17:07:00 exp_startonly after thermalState=nominal lowPower=false
17:09:25 tx_keep before thermalState=nominal lowPower=false
17:11:10 tx_keep after thermalState=nominal lowPower=false
17:13:20 tx_startonly before thermalState=nominal lowPower=false
17:15:03 tx_startonly after thermalState=nominal lowPower=false
17:17:13 tx_restart before thermalState=nominal lowPower=false
17:18:57 tx_restart after thermalState=nominal lowPower=false
17:21:41 dbg before thermalState=nominal lowPower=false
17:22:29 dbg after thermalState=nominal lowPower=false
17:24:49 v2_60_c60 before thermalState=nominal lowPower=false
17:28:07 v2_60_c60 after thermalState=nominal lowPower=false
17:30:43 v2_60_c120 before thermalState=nominal lowPower=false
17:33:59 v2_60_c120 after thermalState=nominal lowPower=false
17:37:17 dbg120 before thermalState=nominal lowPower=false
17:38:39 dbg120 after thermalState=nominal lowPower=false
17:40:30 F_60_c60 before thermalState=nominal lowPower=false
17:43:47 F_60_c60 after thermalState=nominal lowPower=false
17:45:57 F_60_c120 before thermalState=nominal lowPower=false
17:49:14 F_60_c120 after thermalState=nominal lowPower=false
17:51:24 F_320_c60 before thermalState=nominal lowPower=false
17:53:19 F_320_c60 after thermalState=nominal lowPower=false
17:55:29 F_320_c120 before thermalState=nominal lowPower=false
17:58:09 F_320_c120 after thermalState=nominal lowPower=false
18:00:20 F_move before thermalState=nominal lowPower=false
18:02:38 F_move after thermalState=nominal lowPower=false
18:04:49 flash before thermalState=nominal lowPower=false
18:05:26 flash after thermalState=nominal lowPower=false
18:17:42 F_60_c60 before thermalState=nominal lowPower=false
18:21:02 F_60_c60 after thermalState=nominal lowPower=false
18:23:50 F_60_c120 before thermalState=nominal lowPower=false
18:27:10 F_60_c120 after thermalState=nominal lowPower=false
18:30:18 B_320_c60 before thermalState=nominal lowPower=false
18:32:15 B_320_c60 after thermalState=nominal lowPower=false
18:34:27 F_320_c60 before thermalState=nominal lowPower=false
18:36:16 F_320_c60 after thermalState=nominal lowPower=false
18:38:26 B_320_c120 before thermalState=nominal lowPower=false
18:38:26 B_320_c120 after thermalState=nominal lowPower=false
19:43:02 B_320_c60 before thermalState=nominal lowPower=false
19:45:05 B_320_c60 after thermalState=nominal lowPower=false
19:47:39 F_320_c60 before thermalState=nominal lowPower=false
19:49:32 F_320_c60 after thermalState=nominal lowPower=false
19:52:04 B_320_c120 before thermalState=nominal lowPower=false
19:54:41 B_320_c120 after thermalState=nominal lowPower=false
19:57:24 F_320_c120 before thermalState=nominal lowPower=false
19:59:59 F_320_c120 after thermalState=nominal lowPower=false
20:02:42 F_move before thermalState=nominal lowPower=false
20:04:58 F_move after thermalState=nominal lowPower=false
20:06:22 flash_base before thermalState=nominal lowPower=false
20:06:59 flash_base after thermalState=nominal lowPower=false
20:34:00 F_move2 before thermalState=nominal lowPower=false
20:36:26 F_move2 after thermalState=nominal lowPower=false
20:39:32 F_move3 before thermalState=nominal lowPower=false
20:41:56 F_move3 after thermalState=nominal lowPower=false
20:43:22 F_moveP before thermalState=nominal lowPower=false
20:45:58 F_moveP after thermalState=nominal lowPower=false
20:48:37 F_move4 before thermalState=nominal lowPower=false
20:50:56 F_move4 after thermalState=nominal lowPower=false
20:52:36 pin_smoke before thermalState=nominal lowPower=false
20:53:14 pin_smoke after thermalState=nominal lowPower=false
```

Note: each traced run left a 3.5–6 GB `instruments*.ktrace` file in `$TMPDIR`, and these filled
the disk twice. The run script now deletes it after each run and refuses to start with less
than 20 GB free.

### 11.6 Windows status

**Implemented, not compiled or tested.** `dwm_refresh_rate()` reads
`DwmGetCompositionTimingInfo(NULL).rateRefresh` (windows-sys, whose `Win32_Graphics_Dwm`
feature was already enabled, so no new dependency) and feeds the same slot-rate rule. There is no
phase re-anchoring on Windows: `SwapBuffers` locks to vblank there.

Follow-up on the Windows machine:
- build it;
- check that `rateRefresh` matches PERF.md §1's measured rate;
- repeat the cap-60 run on a 59.94/60 Hz display.

DWM's rate is for the composition, not for each monitor, which is why it is only used when it
agrees with winit's per-monitor rate.
