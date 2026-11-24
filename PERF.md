# PERF — ChipDAW GUI playback CPU: findings and fixes

**Machine:** Windows 11 Pro 26200 · RTX 4070 SUPER (driver 610.88), OpenGL via glutin/winit ·
ViewSonic VX3211-2K, 2560×1440 at **59.951 Hz** · release builds, 1440×900 window, 100% scaling.

**Result:** all targets met, with no visual-quality trade-offs.

| Scenario (target) | Before: CPU | After: CPU (3 runs) | Before: pacing (30 s) | After: pacing (30 s) |
|---|---|---|---|---|
| Idle (~0%) | 0.7% | **0.0–0.1%** | no frames | no frames |
| Demo set, playing (≤ 10%) | 52.6–58.7% | **9.1 / 9.4 / 9.6%** | 6 frames > 20 ms (max 23.6), **4 missed vblanks** | **0 > 20 ms (max 17.7), 0 missed** |
| 200 clips, playing (≤ 20%) | 85.7–88.4% | **9.1 / 9.8 / 9.9%** | 17 > 20 ms (max 28.9), 6 missed | **0 > 20 ms (max 17.9), 0 missed** |
| 200 clips, dragging (≤ 25%) | 83.4–95.0% | **10.0 / 10.9 / 11.2%** | 15 > 20 ms (max 33.1) | **0 > 20 ms (max 19.9), 0 missed** |

- CPU is process CPU (all threads, one core = 100%) over 12 s per scenario (`bench.ps1`).
- Pacing comes from 30 s runs (`pacing.ps1`). "Missed vblanks" are display refreshes that showed no new frame, measured independently of the app with DXGI desktop duplication.
- The ranges in the "Before" columns are the 12 s benchmark and the 30 s pacing run.

Six focused commits went into a vizia fork (§8) and five changes into our own code (§7).

---

## 1. Where "320 Hz" came from (FINDINGS.md was wrong)

The display is **60 Hz (59.951 Hz)**. Three independent sources agree:

| Source | Value |
|---|---|
| DWM composition timing (`DwmGetCompositionTimingInfo`) | rateRefresh 10000000/166802 = **59.951 Hz** |
| Active display path target mode (`QueryDisplayConfig`) | 59951/1000 = **59.951 Hz** |
| `Win32_VideoController.CurrentRefreshRate` (WMI), AMD iGPU entry | 59 |
| `Win32_VideoController.CurrentRefreshRate` (WMI), **NVIDIA entry** | **319** ← the source of the error |

There is one monitor, and it's driven by the NVIDIA GPU (`nvidia-smi display_active = Enabled`). The
"320 Hz" came from the previous session reading the NVIDIA adapter's WMI
`CurrentRefreshRate`, which is **wrong on this driver**. The correct AMD value (59) was dismissed as
belonging to a "second display"; it is actually the iGPU, which has no monitor attached. The
FINDINGS.md reasoning built on top of that error is also wrong: "15.625 ms = 5 vblanks, so the
pacing happens to be even", and "vsync can't be what's limiting".

As the brief suspected, the 16.67 ms spin threshold *is* the 60 Hz frame period (§5, H1).

## 2. Method

- **CPU:** `bench.ps1` launches the release build, then runs idle → Space (demo playing) → F4 (200
  clips playing) → drag a clip in a sine pattern. It samples process CPU for 12 s per scenario.
  Run-to-run noise is about ±0.5–1 point.
- **Frame pacing:** `pacing.ps1` runs 30 s with `CHIPDAW_FRAMELOG` (per-frame submit time and
  per-phase timings from the new vizia `FrameTimings`). In parallel it runs
  `ffmpeg -f lavfi -i ddagrab=…:dup_frames=0 -vf showinfo -f null -`. DXGI desktop duplication
  emits a frame only when the desktop actually changes, so a gap of more than 1.5 vblanks between
  desktop updates means a refresh showed no new frame (judder).
  - Validation: with the app rendering at 125 fps (vsync off), the method reports 1 stall in 20 s.
    An earlier variant that encoded with NVENC reported ~1 per second on the same input, so it was
    discarded.
  - Caveat: in the drag scenario the moving cursor also updates the desktop, which can mask stalls.
    The frame log still covers that scenario.
- **Call stacks:**
  - Kernel ETW sampling (1 kHz, plus CSwitch/ReadyThread stacks) via `xperf`. An elevated helper
    runs only `xperf -on`/`-stop`.
  - `samply import` loads the ETL into a profile; a local samply server symbolizes it (Rust and
    Skia from the `profiling` profile PDBs, Windows from the Microsoft symbol server).
  - `analyze.py` weights every sample by its real CPU delta (`threadCPUDelta`), so time spent
    blocked counts as zero. It then buckets samples into the categories below and writes folded
    stacks; inferno renders the flamegraphs.
- **Observer effect:** an active ETW kernel session made the make-current spin (H1) **disappear**.
  The same process measured 95.9% CPU before tracing, 9.6% during tracing, and 97.7% after. For that
  case I wrote a small non-ETW sampler (SuspendThread + GetThreadContext + dbghelp `StackWalk64`,
  1500 samples). Its output is in `docs/perf/evidence/probe_15ms_main_thread_spin_sampler.*`.
- **Rendering regressions:** after every rendering change I pixel-diffed a static 1440×900 frame
  against the previous build (`snap.ps1` / `pixdiff.ps1`), took dynamic captures (drag, dial value
  bubble, meters, playhead), and zoomed into the changed areas.
- The harness scripts are in `docs/perf/tools/`. They take the repo and work directory as parameters (see `docs/perf/tools/README.md`).

Profiles (CPU-weighted flamegraph `.svg`, category report `.txt`, folded stacks `.folded.gz`):
`docs/perf/before/{demo,200,200drag}.*`, `docs/perf/after/{demo,200,200drag}.*`, plus
`docs/perf/evidence/`. The raw ETL traces are not committed; they are system-wide and contain
other processes' data.

## 3. Baseline vs final, all scenarios

The running table for every step is in §7. Per-phase UI-thread timings per frame, from the 30 s
pacing runs:

| | make-current | draw | flush | present | custom views | CPU |
|---|---|---|---|---|---|---|
| Before, demo | 0.12 ms | 2.40 | 1.25 | 0.02 | 1.06 | 52.6% |
| Before, 200 | 1.00 | 4.83 | 2.28 | 0.02 | 3.45 | 85.7% |
| Before, 200 drag | **9.31** | 4.84 | 2.34 | 0.02 | 3.61 | 95.0% |
| After, demo | 0.00 | 0.56 | 0.46 | 15.5 (blocked in vsync) | 0.21 | 9.8% |
| After, 200 | 0.00 | 0.59 | 0.35 | 15.6 (blocked in vsync) | 0.24 | 9.4% |
| After, 200 drag | 0.00 | 0.74 | 0.40 | 15.3 (blocked in vsync) | 0.38 | 10.5% |

## 4. CPU breakdown by category (Step 1.4)

From the ETW profiles: % of **one core**, CPU-weighted, 12 s each. The categories are taken from
the innermost relevant frame.

| Category | Before demo | After demo | Before 200 | After 200 | Before drag | After drag |
|---|---|---|---|---|---|---|
| **Our code** (custom-view recording incl. Skia record calls) | 4.0 | 0.4 | 11.4 | 0.5 | 12.1 | 0.7 |
| **vizia** draw traversal + stylesheet recording | 4.7 | 0.8 | 4.9 | 0.7 | 4.2 | 0.8 |
| **Skia CPU rasterization** (blur masks, glyphs, SW clip masks) | 1.7 | 0.1 | 8.4 | 0.1 | 8.6 | 0.1 |
| **Skia flush**, GPU-backend CPU work (op prepare, triangulation, uploads) | 4.7 | 1.1 | 9.5 | 1.1 | 9.0 | 1.4 |
| **OpenGL driver**, main thread (inside Skia calls) | 5.2 | 0.3 | 7.1 | 0.2 | 6.2 | 0.4 |
| **OpenGL driver worker thread** (nvoglv64) | **36.7** | 3.1 | **49.1** | 3.4 | **41.5** | 3.4 |
| **make-current** (spin-wait inside `wglMakeCurrent`) | 0.2 | 0 | 4.2 | 0 | **11.9** | 0 |
| **present** (`SwapBuffers`; after: the vsync wait) | 0.1 | 3.1 | 0.0 | 3.3 | 0.2 | 3.2 |
| Event loop wait / messages, other | 2.9 | 0.1 | 2.5 | 0.1 | 0.6 | 0.4 |
| **Total** | **60.5** | **8.8** | **97.3** | **9.4** | **94.5** | **10.2** |

Answers to Step 1.4:

- **Before:** ~60% of all CPU was the **NVIDIA OpenGL driver's worker thread**. It was processing
  the GL work Skia generated every frame: new vertex buffers for re-triangulated paths, software
  mask textures, and thousands of ops.
- **Before, beyond the driver:** our own code was 4–12% of a core, vizia's traversal ~5%, Skia's
  flush ~5–10%, and Skia CPU rasterization up to ~8.5%.
- **Before, spin-waiting:** the spin inside `wglMakeCurrent` showed up as 4–12% under ETW. Without
  ETW it was a full core whenever redraws outpaced the display (§5, H1).
- **After:** the total is ~9–10%. About 6.5 points of that are a roughly fixed per-present cost in
  the NVIDIA GL stack (worker thread plus the vsync wait inside `SwapBuffers`). Our code, vizia, and
  Skia together are ~2.5%.

## 5. Hypotheses

### H1: the driver spins during vsync waits — **confirmed, but not where expected**

- **Where the spin is.** It sits in **`wglMakeCurrent`**, not in `SwapBuffers`.
  - The non-ETW sampler put 97.7% of the spinning main thread in:
    `vizia_winit … RedrawRequested → glutin make_current → wglMakeCurrent → nvoglv64!DrvReleaseContext`,
    then a busy loop of `GetExitCodeThread` (72%) and `SwitchToThread`/`NtDelayExecution` (20%).
  - vizia calls `make_current` on **every** redraw, even though the context is already current.
  - With threaded optimization, NVIDIA's `DrvReleaseContext` waits for its worker thread by polling.
    The worker is itself blocked in `NtGdiDdDDIWaitForVerticalBlankEvent` while a swap waits for
    vblank. So any redraw request arriving faster than 59.95 Hz spins until vblank.
- **The threshold.** 16.6 ms spun at 96%, 16.7 ms used 15%, and 16.8 ms used 4% (FINDINGS.md). That
  is the frame period exactly, not timer granularity: `timeBeginPeriod(1)` changed nothing, and the
  system timer was already at 1 ms.
- **Which thread.** Per-thread CPU confirms it is the **main** thread (99.4%); the driver worker
  used 1.4%. It is also **foreground-dependent**: the same probe used 2–5% when its window wasn't
  the foreground window.
- **What triggers it in the app.** In the drag scenario, mouse moves request redraws faster than
  60 Hz, so the spin cost 9.3 ms per frame (11.9% of a core even under ETW).
- **Swap interval experiments:**
  - **Fix:** skip redundant `make_current` (fork commit 2). After it, the probe uses 2.7% at a
    15 ms timer and 3.3% at an 8 ms timer (was ~100%).
  - **Swap interval 1, explicit** (fork commit 5): `SwapBuffers` now blocks ~15 ms in a kernel wait
    (no spin). Display updates are exactly 60.00/s with 0 missed vblanks.
  - **Swap interval 0 + DWM-aligned high-resolution waitable timer** (in the app host): equally
    perfect pacing, and per-frame UI-thread time roughly halved. But over 3 A/B runs, process CPU
    was unchanged within noise (demo 10.1% vs 10.6%), and dragging got **worse** (19.6% vs 12.0%),
    because input-driven redraws are no longer paced by vsync. **Rejected.** The code is kept in
    `docs/perf/tools/experiments/vblank_clock.rs`.
  - Driving animation from the presented frame instead of a free-running timer (app change F6)
    fixed the judder (§6).
- **NVIDIA Control Panel** (Vertical sync, Threaded optimization, Low latency mode): **not
  recorded.** These settings live in the driver's profile store, which can't be read without
  NVIDIA's tools (nvidia-smi doesn't expose them), and the NVIDIA Control Panel wasn't opened
  during this session. The behaviour is consistent with the defaults (threaded optimization
  auto/on; vsync "use the 3D application setting"). **None of the fixes depend on them:**
  - The app sets the swap interval explicitly.
  - It no longer calls make-current redundantly.

  Worth checking with NVIDIA Profile Inspector if this is investigated further.

### H2: full-window repaint — **confirmed**

- **Before.** The union dirty rect (playhead in the middle, meters bottom-right, POS field top,
  badges left) covered the whole window, so every static view repainted every frame.
- **How it was measured.** Switching off all custom draws still left ~35% of a core (under the
  timer spin at the time), and ~40% in the bisect after it. That was vizia repainting the
  stylesheet views. After the clip/shadow fixes, the static views' share was ~1.5–2 points.
- **(a) Per-region damage (fork commit 6).** Up to 6 merged rects, each repainted under a plain rect
  clip. Demo went from 16.3% to 10.3% and 200 clips from 16.4% to 11.2%.
- **(b) Caching** (our code):
  - Retained GPU layers for the arrangement background, grid, and all settled clips (F4: 200 clips
    41% → 18%), and for the ruler (F10).
  - Pre-rendered A8 shadow masks (F3).
  - Static panels are cheap after commits 3/4/6, so they weren't cached.
- **Also in our code:** views bound to the whole transport signal (transport glyphs, the BPM field)
  repainted on every playhead step; now they repaint only when what they show changes (F7).

### H3: CPU-rasterized blurs in stylesheet shadows — **confirmed (a different mechanism than assumed)**

- **Count.** About 30 inset and 30 outer shadows were drawn per full frame (panels, buttons,
  fields, track headers, device sections).
- **What actually cost the time:**
  - **`SkPathOps` difference per inset shadow per frame:** 13.5% of the main thread.
  - **AA triangulation of those paths** (`GrTriangulator::PathToTriangles`): 22%.
  - **New GL vertex buffers every frame** (`GrGLBuffer::Make`): 10.8%. This also fed the driver
    worker thread.
- **The root cause.** `make_offset` and the fresh shadow paths got **new path identities every
  frame**, so Skia's identity-keyed GPU caches never hit.
- **The fix** (fork commit 3) caches shadow layers in view-local space and draws under a translate.
  Demo dropped from 59.5% to 24.3%, and 200 clips from 96.8% to 50.5%.
- **Related: software clip masks.**
  - vizia pre-intersected nested clips with `PathOps` (window rect ∩ 10 px-radius panel → a 10-verb
    generic path).
  - Skia applies those through `ClipStack::GetSWMaskFP`: CPU rasterization plus a texture upload
    per draw per frame. These masks are tied to the frame's clip-stack generation, so **they are
    never reused across frames**.
  - Preserving path identity (tried first) therefore had no effect; I reverted it after measuring.
  - Applying clip shapes one by one (fork commit 4) took the 200-clip drag from ~53% to ~17%.
    Before that, 85% of it was a 1420×568 mask upload.

### H4: text cost — **partially confirmed (small)**

- Clip names were truncated with an ellipsis by re-measuring per character every frame
  (`SkFont::measureText`, ~2.5% of process CPU at 200 clips). Fitted strings are now cached per
  (text, size, width) (F3).
- Labels are not re-shaped per frame: vizia caches paragraphs. Text drawing is ≤1% after the
  fixes.
- POS, BPM and dial values now repaint only when their string or value changes (F7, plus the
  pre-existing dial binding).

### H5: our arrangement draw — **confirmed**

- At 200 clips, **`gfx::drop_shadow` alone was 15% of process CPU**. Skia's analytic blurred-RRect
  path requires the rect to exceed `2 × (radius + 3σ)` (nine-patchability). Most clips are 20–60 px
  wide, so they fell back to `GrBlurUtils::DrawShapeWithMaskFilter`, a CPU mask, every frame.
- **The fix:** cached A8 shadow masks. One nine-patch per (radius, σ) for large rects, and exact
  per-size masks for small ones, tinted by paint colour (F3). Then the retained arrangement layer
  (F4).
- The meters drew ~400 rects per frame (LED gaps); that is now one repeating-gradient rect per
  meter (F10).
- Path building, gradient shader creation and note previews were each minor (< 1%) once the above
  was fixed.

### H6: anything else the profiler showed

- **`Handle::modify` forces a full-window redraw.** Already fixed last session and confirmed
  again: it always emits `WindowEvent::Redraw`.
- **Free-running animation clock causes judder** (16.8 ms vs a 16.68 ms display): a missed vblank
  every 5–10 s (measured: 3–4 per 30 s). Fixed by F6.
- **The 4 ms pump costs idle CPU** (1%). Fixed by making the pump adaptive (F8).
- **Stale samply sessions.** samply's `-d` duration flag is ignored in attach mode on Windows, and
  the session blocks until Ctrl+C. Tooling only; it influenced the capture design (§2).

## 6. Judder

- **Before:** the 16.8 ms clock against a 16.680 ms display left **3–4 refreshes per 30 s with no
  new frame** (demo), visible as a hitch every ~7–10 s, plus frames > 20 ms.
- **After:** vsync-blocking present (fork commit 5), plus a 4 ms pump that advances the model
  **once per presented frame**, using the vsync-spaced present interval as dt (F6). The result:
  **1800 frames / 30 s, 0 missed refreshes, max display gap 17.5–18.1 ms** in every scenario.

## 7. Running table (12 s CPU samples; each row includes all previous fixes)

| Step | Change | Idle | Demo | 200 | 200 drag | Notes |
|---|---|---|---|---|---|---|
| Baseline | 16.8 ms clock (FINDINGS state) | 0.7 | 58.7 | 88.4 | 83.4 | |
| F1 | fork 2: skip redundant make-current | 0.4 | 59.5 | 96.8 | 101.1 | removes the spin below 16.67 ms; no effect at 16.8 ms (as expected) |
| F2 | fork 3: cached shadow geometry, local-space paths | 0.4 | **24.3** | **50.5** | 51.9 | |
| F3 | ours: A8 shadow masks for clips, fitted-text cache | 0.3 | 22.7 | 41.4 | 46.0 | |
| F4 | ours: retained arrangement layer | 0.5 | 16.7 | **17.7** | 53.1 | drag regressed: SW clip masks (H3) |
| (F5a) | stable clip-path identity | 0.1 | 17–20 | 25–32 | 92–103 | **reverted**: masks are never cached across frames |
| F5 | fork 4: clip shapes applied individually | 0.5 | 15.6 | 16.1 | **17.3** | |
| F6 | fork 5 + ours: vsync on, model stepped per presented frame | 1.0 | 16.3 | 16.4 | 17.0 | judder fixed; idle +1% from 4 ms pump |
| F7 | fork 6 + ours: damage regions; glyph/BPM repaint only on change | 0.8 | **10.3** | 11.2 | 11.3 | |
| F8 | ours: adaptive pump (4 ms active / 50 ms idle) | **0.1** | 10.7 | 10.4 | 11.7 | |
| (F9) | swap interval 0 + DWM vblank timer | 0.1 | 10.1 | 9.4 | 17.0 | **rejected** (A/B: no demo gain, drag worse) |
| F10 | ours: meter LED gaps as one gradient rect; ruler layer + pre-rendered handle | 0.0–0.1 | **9.1–9.6** | **9.1–9.9** | **10.0–11.2** | 3 runs |

### Our-code changes (ChipDAW)

| | Change | Risk |
|---|---|---|
| F3 | `gfx::ShadowMasks`: pre-blurred A8 rrect masks (nine-patch, or exact per size), tinted with the paint colour; `draw_text_clipped` caches fitted strings | Bounded caches (cleared when > 1024/4096 entries). Pixel diff: ≤ 3/255 on 3k px (GPU vs CPU blur rounding) |
| F4 | Arrangement retained layer: lanes, grid and settled clips in a GPU surface keyed by a content hash; hovered/easing/dragged clips drawn live on top | Live clips draw above static ones, so z-order differs only where clips overlap; ghost outlines draw above static clips |
| F6 | `AppState::tick`: 4 ms pump, model advanced once per presented frame with dt = present interval; `.vsync(true)` | If the backend doesn't report frame timings, it falls back to wall-clock dt |
| F7 | Transport glyphs and scrub fields repaint only when their displayed state changes | — |
| F8 | Pump interval adapts: 4 ms while playing, presenting or for 250 ms after input; otherwise 50 ms | ≤ 50 ms delay before a non-input-triggered animation starts |
| F10 | Meter LED gaps via `TileMode::Repeat` gradient; ruler static layer; handle image | Pixel-equal at 100% (zoomed comparison); handle glow AA differs sub-pixel |
| — | `perf.rs` + F3 overlay (§9), `CHIPDAW_FRAMELOG`, `--size` unchanged | — |

## 8. vizia fork ([jeffreyfrederick/vizia](https://github.com/jeffreyfrederick/vizia/tree/chipdaw-perf), branch `chipdaw-perf` on tag `0.4.0`)

Wired in via `[patch.crates-io]` in `Cargo.toml`, as a git dependency pinned to commit
`40ef8cfeaefaa09c77171375e037ca04c99e065a` (the six commits below plus commit 7, the frame-rate
limit added during the macOS pass; see [PERF-macos.md](PERF-macos.md) §3). The patches are also
in `docs/perf/vizia-patches/`.

Tests:

- vizia_core unit tests pass (48/48).
- Doctests: on the untouched `0.4.0` tag, 46 vizia_core and 13 vizia_winit doctests already fail
  (Lens-era and missing-trait examples). On the fork the failing sets are **identical by name**;
  only line numbers shift in the two files the fork extends. So the fork adds no failures.

**Rebasing note:** upstream `main` (9f84c1d) has moved on. #803 composes rrect clip intersections,
which is complementary to commit 4: it fixes window ∩ panel, but not rect ∩ rounded-corner cases.
`draw_shadows` has also been rewritten around `rrect`, so commit 3 must be re-measured on `main`.
Commits 1, 2, 5 and 6 address code that is unchanged on `main`.

1. **`75d0a9b7` Report per-phase frame timings from the winit backend**
   - *Problem:* there's no way to see where frame time goes (events, style, layout, draw, flush,
     present).
   - *Fix:* `FrameTimings`/`FramePhase`, plus `BackendContext::record_frame_phase`/`finish_frame`.
     Readable via `Context::frame_timings()`/`EventContext::frame_timings()`. The winit backend
     records all phases, and `WinState::swap_buffers` returns the flush/present split.
   - *Tested:* ChipDAW F3 overlay and per-frame CSV log. Overhead is a few `Instant::now()` calls
     per loop iteration.
2. **`50636704` winit: skip make_current when the window's context is already current**
   - *Problem:* `wglMakeCurrent` on every redraw; on NVIDIA it synchronizes with the driver thread
     and spin-waits until a pending swap is presented. Any app redrawing faster than the refresh
     rate pins a core.
   - *Fix:* call it only if `!is_current()`. Each window owns its context and surface.
   - *Tested:* an 8 px probe at a 15 ms timer went from ~100% to 2.7% CPU (8 ms timer: 3.3%).
     Stack evidence in `evidence/probe_15ms_main_thread_spin_sampler.svg`.
3. **`68af1a5e` Cache shadow geometry and draw view paths in local space**
   - *Problem:* shadow paths and inset `PathOps` differences are rebuilt every frame, with new path
     identities, so Skia's GPU caches never hit. The result is re-triangulation, new vertex buffers
     and driver load.
   - *Fix:* `CachedData::shadows` keyed by (resolved shadows, scale), invalidated with the outline
     path via `invalidate_path`. Shadows and background colour are drawn from cached local paths
     under a canvas translate.
   - *Tested:* 59.5% → 24.3% (demo), 96.8% → 50.5% (200 clips). Pixel diff: 2.6k px on AA edges,
     max 16/255.
4. **`2ba6f6eb` Apply nested clips as separate shapes so rect/rrect clips stay analytic**
   - *Problem:* pre-intersecting nested clips with `PathOps` produces generic paths. Skia can only
     apply those via per-frame software clip masks (CPU raster plus a texture upload per draw).
   - *Fix:* `CachedData::clip_shapes` (ancestors' shapes, `Rc`-shared), applied with
     `clip_rect`/`clip_rrect`. The combined path is kept for `DrawContext::clip_path()`.
   - *Tested:* 200-clip drag ~53% → ~17%. Pixel diff: 9 px of AA at a panel corner.
5. **`70faa4ca` winit: honour WindowDescription::vsync by setting the swap interval**
   - *Problem:* the `vsync` option was a no-op (the code was commented out); the swap interval was
     whatever the driver or user defaulted to.
   - *Fix:* `set_swap_interval(Wait(1) | DontWait)` at surface creation; a warning on failure.
   - *Tested:* 30 s DXGI capture, 0 missed vblanks; the swap blocks without spinning (after
     commit 2).
6. **`259e1cb3` Repaint disjoint damage rects separately instead of their union**
   - *Problem:* one union dirty rect, so two small animations at opposite corners repaint the whole
     window.
   - *Fix:* `DamageRegion`. Pixel-snapped rects, merged when they touch, capped at 6 by merging the
     least wasteful pair. One clipped pass per rect. No damage still means a full repaint, and
     filter semantics are kept.
   - *Tested:* 16.3% → 10.3% (demo). No stale pixels or seams in dynamic captures.
7. **`40ef8cfe` winit: optional frame-rate limit and the display's refresh rate** (macOS pass)
   - *Problem:* on high-refresh displays there's no way to ask for fewer frames, and app-side
     throttles miss input- and transition-driven redraws.
   - *Fix:* `set_max_frame_rate` defers redraw requests to fixed-rate frame slots in the winit
     backend; `display_refresh_rate()` reports the monitor's rate.
   - *Tested:* on M1 / 320 Hz, 60 / 120 fps caps give exactly 60.0 / 120.0 fps with ideal vblank
     cadence, including while dragging. No change without a limit. vizia_core tests 48/48.

**Not done in the fork** (outside this task's scope; they're features/visual fixes, not
performance): the Textbox shadow draw order and DPI-scaling of CSS shadow blur (both in
FINDINGS.md). They'd be separate PRs.

## 9. F3 overlay

Shows:

- Smoothed presented-frame interval and fps, and the worst interval in the last second.
- **Process CPU** (`GetProcessTimes`, all threads).
- Clip and frame counts.
- **UI-thread time per phase:** events, style, layout, make-current, draw, flush, present.
- **Custom draw time per view:** arrangement, ruler, meters, lights, dials, fields, glyphs, badges,
  overlay.

## 10. What is platform-specific (to check on AMD, Intel, macOS)

macOS has now been measured: see [PERF-macos.md](PERF-macos.md) §8 for which findings carried over.

| Finding | Likely scope | What to check elsewhere |
|---|---|---|
| Make-current spin (fixed by fork 2) | **NVIDIA OpenGL with threaded optimization** on Windows; foreground-dependent | Probe at a 15 ms timer on AMD/Intel GL. The fix is correct everywhere: skipping a redundant make-current is free |
| Vsync wait in `SwapBuffers` costs ~3% of a core; driver worker thread ~3% | NVIDIA GL driver on Windows | Per-present CPU on AMD/Intel; macOS (CGL/Metal via Skia) behaves differently |
| ETW kernel tracing suppresses the spin (observer effect) | Windows + NVIDIA | Use a non-ETW sampler to confirm any spin |
| `Win32_VideoController.CurrentRefreshRate` wrong (319) | NVIDIA WMI provider | Use DWM / DisplayConfig |
| Generic-path clips → per-frame software clip masks (fork 4) | **Skia Ganesh, all platforms/backends** | — (portable fix) |
| Path identity defeats GPU caches (fork 3); small blurred rrects fall off the analytic path (F3) | **Skia, all platforms** | — |
| Full-window repaint (fork 6), `Handle::modify` redraw | **vizia, all platforms** | — |
| Free-running clock vs refresh rate judder | All platforms (period depends on the display) | Verify the presented-frame pacing on 120/144 Hz and VRR displays |

## 11. Remaining cost and what would reduce it further

The targets are met, so there is no gap to close. Of the remaining ~9–10% of a core, **~6.5 points
are a roughly fixed per-present cost in the NVIDIA OpenGL stack**: the driver worker thread
(~3.1–3.4%) plus the vsync wait inside `SwapBuffers` (~3.1–3.3%). Our code, vizia and Skia together
are ~2.5%. Further reductions would need:

1. **A different presentation path.** Skia's Direct3D or Graphite/Dawn backend with a DXGI
   flip-model swap chain and a frame-latency waitable object (or ANGLE). That avoids NVIDIA's GL
   present path entirely; I'd expect most of the 6.5 points to go. It's a large vizia/winit change.
2. **Lower frame rates for slow-changing regions.** For example, meters at 30 Hz. This is a visual
   trade-off, so per the brief it would have to be opt-in and off by default.
3. **Measuring on AMD/Intel** before investing in (1): the fixed cost may be NVIDIA-specific.

## 12. Corrections to FINDINGS.md (probes override)

- The display is **60 Hz**, not ~320 Hz (§1).
- **"Animation timer ≤ 16.6 ms spins a core"**: true, but the cause is NVIDIA's make-current
  synchronization against a pending vsync swap, triggered by vizia's redundant `make_current`. It's
  **not** timer granularity, and **not** a vizia event-loop busy-wait. Fixed in fork commit 2. The
  16.8 ms clock workaround is gone.
- **"vsync flag is a no-op"**: correct for 0.4.0; fixed in fork commit 5.
- **"~40% of a core is vizia repainting stylesheet views; our drawing adds 15–30%"**: the profiler
  says otherwise. ~60% of all CPU was the GL driver's worker thread, fed by per-frame
  re-triangulation, buffer churn and software clip masks (H2/H3). vizia's own traversal was ~5% and
  our recording 4–12%.
- **"Single union dirty rect"**: confirmed; fixed in fork commit 6.
- The FINDINGS.md playback CPU numbers (60–100%) are superseded by the summary table at the top (9–11%).
- vizia doctests: on tag `0.4.0` itself, 13 vizia_winit doctests (`window_modifiers.rs`) and 46
  vizia_core doctests fail to compile (Lens-era examples, missing trait imports). This is
  pre-existing and the same on the fork (§8).
