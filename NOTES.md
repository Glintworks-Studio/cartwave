# NOTES — Vizia capability checklist (CLAUDE.md §3)

## Version choice

| | Version | Date | Notes |
|---|---|---|---|
| crates.io latest | **vizia 0.4.0** | — | Signals-based (`vizia_reactive`), Skia renderer (`skia-safe 0.93.1`), winit backend |
| git `main` | `9f84c1d` | 2026-11-20 | Same architecture; small fixes since 0.4.0 (baseview DPI awareness #806, clip-path rrect #803, modifier queuing #797, chrono→jiff #812) |

**Chosen: crates.io `vizia = "0.4.0"`.** Nothing we need is `main`-only. The
`main` fix for baseview DPI awareness (#806) will matter once we host inside a
plugin. Re-evaluate when 0.4.1+ ships.

How the API was verified: I read the 0.4.0 source in `~/.cargo/registry`
(`vizia_core`, `vizia_style`, `vizia_winit`, `vizia_reactive`, `skia-safe`) and
the repo's `examples/` and `MIGRATION_LENS_TO_SIGNALS.md`. I also wrote two probe
programs, `crates/app/examples/css_probe.rs` and `redraw_probe.rs`, which settle
questions the source alone couldn't answer.

Toolchain: rustc 1.100.0 stable, MSVC, Windows 11. GPU: NVIDIA RTX 4070 SUPER
(OpenGL through `glutin`), with a 2560×1440 display at 59.951 Hz (an earlier "~320 Hz" was a wrong WMI reading; see PERF.md §1).

## Capability matrix (0.4.0)

Legend: ✅ works · ⚠️ works with caveats · ❌ missing → fallback used

| Capability | Status | Detail |
|---|---|---|
| Linear gradient backgrounds | ✅ | `background-image: linear-gradient(to bottom, a, b)`; stops, angles |
| Radial gradient backgrounds | ✅ | `radial-gradient(...)` parsed and drawn |
| `box-shadow` | ⚠️ | Property is named **`shadow`**, not `box-shadow` |
| Multiple shadows | ✅ | Comma-separated list; drawn in reverse order like CSS |
| **Inset** shadows | ⚠️ | Supported, but `inset` must be the **last** token (`2px 3px 6px 0px black inset`). The CSS-standard leading `inset 2px …` silently fails to parse and the whole declaration is dropped. Rendered as a blurred *crescent* (path minus offset path), so it only shows if the fill is lighter than the shadow colour |
| Blur + spread | ⚠️ | Both supported. **Blur radius is NOT multiplied by the DPI scale factor** (offset and spread are), so shadows get relatively sharper at 150%/200% |
| Shadow on `Textbox` | ❌ → fallback | `Textbox::draw` calls `draw_shadows` *before* `draw_background`, so inset shadows are painted over. **Fallback:** wrap the textbox in a `.field` container that owns the inset look, and use `:focus-within` for the focus glow |
| Border radius | ✅ | `corner-radius`, plus per-corner `corner-top-left-radius` etc. |
| Corner shape / smoothing | ✅ | `corner-shape`, `corner-*-smoothing` (squircles) exist; not used |
| Border gradients | ❌ → fallback | Only `border-color`. **Fallback:** draw in a custom view (clip borders, dial rims) |
| Transitions | ✅ | `transition: prop 140ms ease-out`, multiple comma-separated. Animatable: colours, `background-image` (gradients), **`shadow`**, opacity, transform, translate/scale/rotate, sizes, spacing, `caret-color`, … |
| Hover / focus / active / checked states | ✅ | `:hover :active :focus :focus-visible :focus-within :checked :disabled :placeholder-shown`, plus `toggle_class(name, signal)` |
| `:not()` / complex selectors | ✅ | Uses Servo's `selectors` crate |
| Keyframe animations | ✅ | `@keyframes` + `AnimationBuilder`; not needed here |
| Opacity | ✅ | `opacity` (animatable) |
| Backdrop blur / filters | ⚠️ untested | `filter: blur()` and `backdrop-filter: blur()` are parsed and have a draw path (`draw_system` save-layers with an `ImageFilter`). Not visually verified: not needed for the look, and each one forces an extra layer |
| Outline | ✅ | `outline-*` incl. `outline-offset` |
| CSS variables | ⚠️ | `var(--x)` works for colour-like and length properties (`AnimatableVarSet`), not for every property |
| Custom drawing (`View::draw`) | ✅ | Gives a raw **Skia** `Canvas` (`vizia::vg` = `skia_safe`): paths, gradients (`vg::gradient::shaders::*`), `MaskFilter::blur`, sweep gradients, path effects (dash, corner), save layers, text via `Font`/`draw_str`. Default stylesheet drawing is available via `cx.draw_background/shadows/border/text` |
| Drag events | ✅ | `MouseDown/Move/Up`, `cx.mouse()` (physical px), `cx.modifiers()` |
| Mouse capture | ✅ | `cx.capture()` / `cx.release()` |
| Double-click, scroll | ✅ | `WindowEvent::MouseDoubleClick`, `MouseScroll(x, y)` |
| Cursor icons | ✅ | CSS `cursor: ns-resize`, `.cursor(CursorIcon::…)` modifier, or `cx.emit(WindowEvent::SetCursor(..))` for dynamic per-region cursors (clip edges) |
| Keyboard | ✅ | `WindowEvent::KeyDown(Code, Option<Key>)`; bubbles to the root model |
| Hot-reloading stylesheets | ⚠️ | Supported when the sheet is added as a `PathBuf`; it re-reads on `WindowEvent::ReloadStyles` (or `cx.reload_styles()`). No file watcher: we bind **F5** in debug builds. Verified working |
| Fonts | ✅ | `cx.add_font_mem(bytes)`, then `font-family: "Inter"` in CSS. Custom views load their own `Typeface` from the same bytes |
| Timers | ✅ | `cx.add_timer(interval, duration, cb)` + `start_timer`; see the pacing caveat in FINDINGS |
| VSync control | ❌ | `.vsync(bool)` exists on the window builder but the winit backend's swap-interval code is **commented out**; the driver default applies |
| Per-rect damage | ❌ | The draw system unions all dirty entities into **one** dirty rect and repaints every view intersecting it |
| User/zoom scale factor | ❌ (winit) | No API to override the DPI scale in the winit backend, so 150%/200% has to be tested by changing the Windows display scale |

## Fallbacks used (the key bake-off list)

1. **Clips**: everything is custom-drawn in one `Arrangement` view (gradient, five-layer lighting stack, note preview, glow, ghost). One vizia view per clip was not considered, for performance and because of the missing border gradients and the inner-shadow ordering problem.
2. **Inner shadows in custom views**: drawn as clipped gradient strips rather than a blurred inset frame. Skia's blur of a non-rect shape (DRRect) is a CPU mask and was measurable at 200 clips.
3. **Text fields**: a container `.field` owns the inset and focus glow (Textbox shadow-order bug).
4. **Dials, meters, lights, transport glyphs**: procedurally drawn (Skia), as specified.
5. **Buttons, panels, track headers, BPM/POS field chrome**: pure stylesheet.
