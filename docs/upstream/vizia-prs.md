# vizia PR descriptions

Against vizia 0.4.0. Measured in ChipDAW, a GUI with a continuously animating arrangement view
(PERF.md, PERF-macos.md, PIANOROLL.md). Suggested order: 11, 2, 5, 1, 6, 4, 3, 7, 9 + 10, 8. The
standalone bug fixes go first; then the pacing work, which builds on 1 and 5.

Each PR's full description is its commit message in `vizia-patches/`. Below: the title, a
two-line summary, and the evidence to quote.

---

## 11 · Fix `modify_timer` / `query_timer` spinning when the timer isn't next due *(bug, small)*

Both loop `while let Some(t) = running_timers.peek()` and only pop when the top of the heap is
the requested timer. If any other running timer is due sooner, the loop never advances: the UI
thread spins at 100% forever. A focused `Textbox`'s caret timer is enough to trigger it.
**Fix:** search the whole heap, edit the matches, and rebuild the heap. **Test:** a new unit test
(two timers, modify/query the later one) hung before the fix; vizia_core 52/52.

## 2 · winit: skip `make_current` when the window's context is already current *(perf, 1 line)*

`make_current` runs on every redraw. On NVIDIA (threaded optimisation) `wglMakeCurrent` spin-waits
for a pending vsync'd swap, so any app redrawing faster than the refresh rate pins a core.
**Evidence:** an 8 px probe at a 15 ms timer went from ~100% to 2.7% CPU. Stacks:
`docs/perf/evidence/probe_15ms_main_thread_spin_sampler.svg`.

## 5 · winit: honour `WindowDescription::vsync` by setting the swap interval *(bug)*

The option was a no-op (the code was commented out), so the swap interval was whatever the driver
defaulted to. **Fix:** `set_swap_interval(Wait(1) | DontWait)` at surface creation, with a warning
on failure. **Evidence:** a 30 s DXGI capture showed 0 missed vblanks.

## 1 · Report per-phase frame timings from the winit backend *(feature, diagnostics)*

`FrameTimings` / `FramePhase`: events, style, layout, draw, flush and present per frame, read with
`Context::frame_timings()`. The overhead is a few `Instant::now()` calls per frame. It was needed to
find everything below.

## 6 · Repaint disjoint damage rects separately instead of their union *(perf)*

One union dirty rect means two small animations in opposite corners repaint the whole window.
**Fix:** `DamageRegion` holds up to 6 pixel-snapped rects, merging the pair that wastes least. One
clipped pass per rect. **Evidence:** 16.3% → 10.3% process CPU (demo playback).

## 4 · Apply nested clips as separate shapes so rect/rrect clips stay analytic *(perf)*

Pre-intersecting nested clips with PathOps yields generic paths. Skia can only apply those as
per-frame software masks: CPU raster plus a texture upload per draw. **Fix:** apply ancestor shapes
one by one with `clip_rect` / `clip_rrect`. **Evidence:** a 200-clip drag went from ~53% to ~17%.
*Rebase onto #803.*

## 3 · Cache shadow geometry and draw view paths in local space *(perf)*

Shadow paths and inset PathOps differences got new identities every frame, so Skia's GPU caches
never hit: re-triangulation and new vertex buffers every frame. **Fix:** cache per
(shadows, scale) and draw under a translate. **Evidence:** 59.5% → 24.3% (demo), 96.8% → 50.5%
(200 clips). *Redo on the new `draw_shadows`.*

## 7 · winit: optional frame-rate limit and the display's refresh rate *(feature)*

`set_max_frame_rate(Option<f64>)` defers redraw requests to fixed-rate slots; nothing is ever
dropped. `display_refresh_rate()` is new. The default is no limit, so nothing changes unless an
app opts in. **Evidence (M1, 320 Hz):** a 60/120 limit gives exactly 60.0/120.1 fps, including
input-driven redraws.

## 9 + 10 · Lock the frame-rate limit to the display's real refresh rate; on Windows let the vsync'd swap pace 1:1 frames *(bug)*

9: a "60 Hz" display really runs at 59.94–59.96 Hz, so 60.000 Hz slots beat against it, causing 5–10
s stretches of stutter every ~20 s. The slots now lock to the precise rate (CoreVideo / DWM), with a
phase anchor on macOS. 10: on Windows the vsync'd `SwapBuffers` already paces 1:1 frames, and
slots with an arbitrary phase made frames miss vblank. Skip the slots there.
**Evidence:** macOS 60 Hz frames > 20 ms 230 → 6 per 120 s. Windows: 0 missed refreshes in 9 of
9 runs (11 in one run before commit 10).

## 8 · winit (macOS): pin the window's colour space to sRGB *(bug)*

A GL window moved to a display with another colour profile flickered between two colour
conversions, 38–67 times per 20 s. **Fix:** `NSWindow.colorSpace = sRGB` at creation. Skia renders
sRGB anyway. **Evidence:** 0 flips after repeated moves both ways (ScreenCaptureKit).

---

Not in the fork, worth separate issues (FINDINGS.md / NOTES.md):

- **CSS `shadow` blur isn't scaled by the DPI factor.** Offset and spread are, so shadows get
  sharper at 150% and 200%.
- **`Textbox::draw` draws shadows before the background**, so inset shadows are painted over.
- **`inset` must be the last token** in a `shadow` value. The CSS-standard leading `inset` fails to
  parse, and the whole declaration is dropped silently.
- **Resizing a window from another process with `SetWindowPos` can panic** in
  `WinState::resize` (`new_surface_with_dimensions` returns `None`) (DESIGN.md §8).
