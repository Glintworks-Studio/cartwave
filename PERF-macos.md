# PERF-macos: ChipDAW on macOS (Apple M1, 320 Hz display)

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
