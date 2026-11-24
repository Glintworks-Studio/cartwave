# FINDINGS — Vizia GUI bake-off report

**Stack:** vizia 0.4.0 (crates.io) · Skia via `skia-safe 0.93.1` · winit + OpenGL · Rust 1.100 · Windows 11, RTX 4070 SUPER, 2560×1440 @ 59.951 Hz.

> **Update (performance pass, see [PERF.md](PERF.md)).** Several statements below were corrected
> by profiling:
> - The display is **60 Hz**, not "~320 Hz". That figure came from a wrong WMI value (PERF.md §1).
> - The "timer ≤ 16.6 ms spins a core" issue is NVIDIA's `wglMakeCurrent` waiting on a pending
>   vsync swap, triggered by vizia's redundant per-frame `make_current`. It is not timer
>   granularity.
> - Playback CPU was dominated by the GL driver thread, fed by per-frame path re-triangulation,
>   buffer churn and software clip masks. It was not mainly vizia's traversal.
> - With six vizia fork commits and app-side fixes, playback now uses **~9–11%** of a core (was
>   60–100%), with judder-free 60 fps. §2.7–2.9 and §3 below describe the state before that pass.

Run: `cargo run --release -p app`.
Keys: **Space** play/pause · **F3** frame-time overlay · **F4** 200-clip stress set · **F5** reload stylesheet (debug builds) · **Delete** remove selection · **Ctrl+A** select all · **Esc** deselect.
Mouse: drag a clip to move it (snaps to the grid; hold **Alt** to disable snap). Drag vertically to change lane. Drag a clip edge to resize. **Shift**-click adds to the selection; **Ctrl**-drag duplicates. **Ctrl**+wheel zooms; the wheel scrolls. Click or drag the ruler to move the playhead. On dials: drag, **Shift**-drag for fine control, double-click to reset, wheel to step. BPM and POS fields: drag to scrub, double-click to reset.

Screenshots (`docs/screens/`): `main-100.png`, `drag-100.png`, `stress-200clips-f3-100.png`, `main-150.png`, `drag-150.png`, `main-200.png`.
Recording: `docs/screens/demo-100.mp4` (58 s, 60 fps, 1442×932, 100% scaling). The tour is scripted with synthetic input: play, hover, drag a clip to another lane and back, resize an edge, Ctrl-drag duplicate then Delete, dial drag, double-click reset, wheel, BPM scrub and reset, text-field focus, Ctrl+wheel zoom, then F3 + F4 with 200 clips and a drag.

`--size WxH` sets the initial logical window size. At 150%/200% on a 1440p screen, 1440×900 doesn't fit, so the 150% shots use 1400×840 and the 200% shot uses 1260×640, which crops the lower lanes.

---

## 1. Stylesheets alone vs. custom drawing

| Component | How it's built |
|---|---|
| Panels (gradient, inner top highlight, hairline ring, soft drop shadow) | **Stylesheet only** |
| Buttons: gradient, top highlight, hover, pressed (inset shadow + 1 px shift), checked glow, armed record glow | **Stylesheet only**, all states animated with `transition` |
| Track headers (hover, selected tint + accent edge, M/S toggles) | **Stylesheet** plus a small custom lane badge with a pulse light |
| Text fields: inset well, bottom bevel highlight, animated focus glow, pulsing caret | **Stylesheet**, but only after working around a Textbox bug (§2.1) |
| BPM / POS scrub fields | Stylesheet chrome + ~120 lines of custom text and drag handling |
| Arrangement clips (gradient, 5-layer lighting, header strip, note preview, hover/select/drag easing, ghost, snap guide) | **Custom draw** (Skia) |
| Ruler, playhead, grid | Custom draw |
| Rotary dials | Custom draw (radial dome, sweep-gradient "brushed" sheen, rim, specular, glowing arc, pointer, value bubble) |
| Meters, channel lights, transport glyphs | Custom draw |

**Takeaway:** the stylesheet system is strong. Gradients, multiple shadows,
inset shadows, and *transitions on shadows and gradients* all work, which covers
every "chrome" surface. Anything data-driven or with a bespoke lighting stack
belongs in `View::draw`. That canvas is raw Skia, so nothing is out of reach
there; it just costs code.

## 2. Effects that were impossible or ugly, and the workarounds

1. **Textbox hides inset shadows.** `Textbox::draw` calls `draw_shadows()` *before* `draw_background()`, the reverse of the default `View::draw`. Any inset shadow on a textbox is painted over. *Workaround:* a `.field` container draws the inset well and the `:focus-within` glow; the textbox inside is transparent.
2. **`inset` keyword position.** `shadow: inset 2px 3px 6px black` (standard CSS) fails silently and drops the whole declaration. Only `2px 3px 6px 0px black inset` works. Found with `examples/css_probe.rs`.
3. **Shadow blur is not DPI-scaled.** In `draw_shadows`, offset and spread are multiplied by the scale factor but blur is not, so CSS shadows look tighter at 200% than at 100%. **Confirmed visually** at 200%: the BPM/POS inset shadow is noticeably harder, while custom-drawn shadows (clips, dials, meters) scale correctly. *Workaround (not yet applied):* a per-DPI stylesheet, or move the hero shadows into custom draw.
4. **No border gradients.** Clip borders and dial rims are drawn procedurally.
5. **Inset shadow needs contrast.** vizia draws an inset shadow as a blurred crescent, so a black inset shadow on a near-black field is invisible. This is a design issue rather than a bug, but it cost a round-trip. Fields use a fill slightly lighter than the inset colour.
6. **Blurred non-rect shapes are CPU-rasterised by Skia.** My first inner shadow was a blurred DRRect per clip, which was measurable at 200 clips. *Workaround:* gradient strips clipped to the rounded rect; they look the same at these sizes.
7. **No damage regions.** One union dirty rect per frame. With the playhead (centre), meters (bottom), and POS field (top) all animating, the union is about the whole window, and *every* view in it repaints at 60 fps. This is the main cost during playback (§3).
8. **`Handle::modify` forces a full-window redraw.** It always emits `WindowEvent::Redraw`. Calling it from a per-frame binding repainted the entire window every frame, even when idle (56% CPU at idle). *Workaround:* share state through an `Rc<Cell<_>>` captured by the binding instead.
9. **Animation-timer interval below one 60 Hz frame spins a core.** With a continuously redrawing view, a `cx.add_timer` interval ≤16.6 ms pinned one core at ~100%, while 16.8 ms used ~4% (`examples/redraw_probe.rs`, which redraws one 8 px rect):

   | Interval | CPU (one core) |
   |---|---|
   | 15.0 ms | 99% |
   | 16.6 ms | 96% |
   | 16.7 ms | 15% |
   | 16.8 ms | 4% |
   | 17.0 ms | 3.5% |

   `timeBeginPeriod(1)` made no difference, so this is not timer granularity. Redraw requests arriving faster than ~60 Hz presents cause a busy-wait somewhere in the winit/GL/driver path. vizia's `vsync` flag is a no-op (its swap-interval code is commented out). *Workaround:* a 16.8 ms animation clock; all motion integrates measured `dt`, so speed is exact and pacing is steady. **Risk:** the UI can't animate above ~59.5 fps without burning a core on this setup. This needs investigation on other GPUs, and upstream.
10. **No DPI override in the winit backend**, so 150%/200% can only be tested by changing the Windows display scale. That was done here live through the DisplayConfig API, and the scale was restored to 100% afterwards. At **150% and 200%**, all custom-drawn text, glyphs, 1 px borders, grid lines, and the playhead are pixel-crisp, and layout reflows without jank. The only scale defect seen is §2.3.
11. **Injected Delete key.** With synthetic input (and likely remote-desktop tools), the physical `Code` can be unidentified. The arrangement now also matches the logical `Key::Delete`/`Key::Backspace`.

## 3. Measured performance (release build, 1440×900 window)

Frame interval and worst frame come from the F3 overlay (measured tick-to-tick `dt`). `arr cpu` is CPU time to record the arrangement's Skia commands. Process CPU was sampled over 3 s from the OS.

| Scenario | Frame avg | Worst (1 s) | Arrangement record | Process CPU (one core = 100%) |
|---|---|---|---|---|
| Idle, demo set (43 clips), stopped | — (nothing dirty, no frames drawn) | — | — | **0.0%** |
| Demo set, playing | 16.79 ms (59.6 fps) | 17.2 ms | 0.6 ms | ~60–66% |
| **200 clips**, playing | **16.81 ms (59.5 fps)** | **19.2 ms** | **2.9 ms** | ~97–100% |
| 200 clips, dragging while playing | 16.79 ms | 19.0 ms | 3.5 ms | ~94% |
| 200 clips, stopped | — | — | — | 0.5% |

Frame pacing meets the bar: steady 60 fps with 200 clips while the playhead
moves and the meters animate, and no frame worse than ~19 ms. Two caveats:

- **CPU while playing is high.** Bisecting with views switched off showed about 40% of a core is vizia repainting the stylesheet views under the full-window dirty rect (labels, buttons, panels). My custom drawing adds about 15–20% for the demo set, and about 30% at 200 clips, mostly the arrangement.
- **Not measured:** GPU time (the overlay only sees CPU-side intervals), and other GPUs and OSes.

Optimisations already in place: views redraw only when their signal changes
(meters notify only when values move, the arrangement and dials only while
easing), so idle is 0%. Clips outside the view are culled, notes outside the
view are culled, and there are no per-frame allocations of fonts or paints
beyond small `Vec`s. Possible next steps: cache each clip body as a Skia
`Picture` (redraw only on change), or move the meters and lights out of the
transport/device panels so the dirty union shrinks (limited value while there's
one union rect).

## 4. Rough lines of code per component

Non-blank, non-comment lines:

| Component | File(s) | LOC |
|---|---|---|
| Arrangement view: interaction, grid, playhead, snapping | `views/arrangement.rs` (excl. headers) | ~580 |
| Clip painter (lighting stack, notes, ghost) | `views/clip.rs` | 139 |
| Track headers + lane badge | `views/arrangement.rs` (tail) | ~90 |
| Rotary dial | `views/dial.rs` | 321 |
| Skia helpers (fonts, gradients, shadows, text) | `views/gfx.rs` | 274 |
| Ruler | `views/ruler.rs` | 150 |
| Meters | `views/meter.rs` | 131 |
| Scrub fields | `views/text_field.rs` | 123 |
| Transport glyphs | `views/transport.rs` | 97 |
| Channel lights | `views/visualizer.rs` | 67 |
| F3 overlay | `views/overlay.rs` | 63 |
| App model, layout, tick | `lib.rs` | 349 |
| State + pixel↔tick viewport | `state.rs` | 307 |
| Mock data + meter simulation | `mock.rs` | 288 |
| Tokens / stylesheet | `theme/tokens.rs`, `theme/style.css` | 82 / 230 |
| **Total (Rust)** | | **~3,070** |

## 5. How often the AI-generated Vizia code was wrong

Process: before writing UI code, I read the 0.4.0 sources for every API used
(View trait, DrawContext, EventContext, timers, signals, Textbox, modifiers,
style properties, skia-safe 0.93 gradient API). Result:

- **Invented or non-existent APIs: 0.** The first full build (~2,500 lines) failed with **one** vizia-related error: `Chip` was ambiguous because the vizia prelude exports a `Chip` view. That is a name collision, not an invented API; I renamed ours to `SoundChip`. The only other compile error was a Rust borrow issue in my own code.
- **Version mismatches avoided by reading source:** 0.4 replaced Lens with **Signals** (most online examples and model knowledge are Lens-era). `View::draw` takes `&Canvas`, not `&mut Canvas`, as some doc comments still show. skia-safe 0.93 deprecated `Shader::linear_gradient` in favour of `gradient::shaders::linear_gradient(.., &Gradient, ..)`. The CSS property is `shadow`, not `box-shadow`.
- **Runtime-behaviour surprises (the real cost): 5.** All of these compiled fine but rendered or performed wrongly, and each needed a probe or a source read: Textbox shadow order (§2.1), `inset` position (§2.2), `Handle::modify` full redraw (§2.8), timer-interval spinning (§2.9), and invisible inset on dark fills (§2.5).

Lesson: with source-first verification, compile-time mistakes nearly
disappear. The remaining risk is behaviour that only shows up on screen or in a
profiler. Budget for probes.

## 6. Licensing

- Fonts: Inter 4.1 and JetBrains Mono 2.304, both **OFL-1.1**; license files are in `assets/fonts/`.
- No crates added beyond `vizia`.
- ⚠️ **vizia's CSS stack pulls in MPL-2.0 crates:** `cssparser`, `cssparser-color`, `cssparser-macros`, `selectors`, `dtoa-short`. MPL-2.0 is weak, file-level copyleft. Using it unmodified in a closed product is common, but it is **not** on our "MIT/Apache/BSD/Zlib/OFL" list, so it needs a policy decision. Avoiding it means forking vizia's style system.
- Dual-licensed crates with a GPL/LGPL option (`self_cell`: Apache-2.0 OR GPL-2.0; `r-efi`: MIT/Apache/LGPL) are taken under their permissive option. Everything else is MIT, Apache, BSD, Zlib, ISC, BSL-1.0, Unicode, or Unlicense.

## 7. Open items

- Fix §2.3 (CSS shadow blur at high DPI) before the 200% screenshot is used externally.
- Not done: marquee selection, undo, horizontal scrollbar (the wheel scrolls).

## 8. Verdict

**Vizia is viable for the full DAW UI**, with conditions.

**For it:** every visual target in the brief was reached without images, and
mostly cleanly. The stylesheet handles gradients, layered and inset shadows,
and *animated* shadow and gradient transitions, which covers the whole "chrome"
layer with very little code. Custom views get the full Skia API, so the
arrangement, dials, and meters look as good as a native C++/Skia app would.
The model stays separate from the window (`ui` is a library; `app` is 9
lines), which suits the plugin (baseview) path. With source-first
development, AI-written code compiled almost first time.

**Conditions and risks:**
1. Frame pacing and CPU: the sub-16.7 ms timer spin (§2.9), the missing vsync control, and the single dirty rect (§2.7) together mean continuous animation costs ~60–100% of a core. That's acceptable for a prototype but must be solved before shipping, and possibly upstream: per-rect damage, a vsync-driven frame callback, cached layers. Check this first on AMD, Intel, and macOS.
2. Rough edges in widgets: Textbox draw order, `Handle::modify` side effects, and DPI-unscaled shadow blur. Each has a workaround, but expect more of these in less-trodden widgets.
3. MPL-2.0 transitive dependencies need a licensing sign-off.
4. Pre-1.0 churn: the 0.3 → 0.4 Lens → Signals migration shows APIs still move. Pin versions and keep the UI behind our own small set of views.

If items 1 and 3 are acceptable or fixable, I'd continue with Vizia. If
battery and CPU during playback is a hard requirement now, budget time for
upstream rendering work.
