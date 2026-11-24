# CLAUDE.md: Chiptune DAW, Vizia GUI Prototype

> Working title: **ChipDAW** (placeholder, do not brand anything)
> Owner: CTO office · Status: GUI bake-off prototype · Language: Rust · GUI: Vizia

## 1. What we are building (and what we are not)

We are a company that uses AI to build excellent software. Our first product is a
chiptune audio workstation for indie game developers, with accurate sound-chip
emulation (starting with NES 2A03 + Konami VRC6 expansion audio) and an
Ableton-style arrangement view.

**This task is ONLY the GUI prototype.** The goal is to find out whether Vizia can
deliver a fully custom, *premium-looking* interface with **no bitmap images**.
The audio engine exists separately and must NOT be integrated yet. Use mock data.

The result will be judged on one screenshot plus a short screen recording, so
visual quality is the top priority. Interaction smoothness comes second.

## 2. Hard constraints

1. **No images for UI.** No PNG/JPG/SVG files, no knob filmstrips, no
   pre-rendered textures. Everything is drawn with stylesheets or procedural
   drawing (paths, gradients, shadows). Fonts are fine (see §6).
2. **Rust + Vizia only.** No JUCE, no web views, no Tauri, no Electron.
3. **Permissive licenses only** (MIT / Apache-2.0 / BSD / Zlib / OFL for fonts).
   No GPL/LGPL/AGPL dependencies. Ask before adding any new crate.
4. **Plugin-ready mindset.** Vizia has a baseview backend for audio plugins
   (e.g. nih-plug). Do not rely on anything that would obviously only work in a
   standalone window, and keep the UI in a library crate that is separate from
   the `main.rs` that opens the window.
5. **No trademarks or copyrighted content.** Do not use "Castlevania", "Konami",
   "Nintendo", or "Ableton" anywhere in the UI, code identifiers, or demo data.
   Use generic labels like "VRC6 Pulse 1" and invented clip names.

## 3. Before writing code: verify the Vizia API

Vizia changes quickly, and AI-generated code tends to invent APIs for it.
**Do not guess.**

1. Check the latest published version on crates.io and compare it with the git
   `main` branch at https://github.com/vizia/vizia. Prefer the latest crates.io
   release unless a feature we need exists only on `main`. Record the choice.
2. Skim the book (https://book.vizia.dev) and API docs (https://docs.vizia.dev),
   plus the repo's `examples/` folder and the `widget_gallery` package.
3. Confirm which of these style properties are supported **in the version you
   chose**, and write the results into `NOTES.md`:
   - linear / radial gradients as backgrounds
   - `box-shadow` (multiple shadows? **inset**? blur + spread?)
   - border radius, per-corner radius, border gradients
   - transitions / animations on hover, focus and selected states
   - opacity, backdrop blur / filters, outline
   - custom drawing in a view (the `View::draw` canvas and its renderer)
   - drag events, mouse capture, cursor icons
   - hot-reloading stylesheets
4. If a needed property is missing from the stylesheet system, **draw it
   procedurally in a custom view** rather than dropping the effect. Note each
   fallback in `NOTES.md`, because that list is a key bake-off result.

If an API call fails to compile, read the source or docs for the chosen version.
Do not try random variations.

## 4. Scope: what to build

One window, dark theme, about 1440×900 by default, resizable.

### 4.1 Layout
```
┌───────────────────────────────────────────────────────────────┐
│ Transport bar: ▶ ■ ● | BPM [ 150 ] | Pos [ 001.1.1 ] | Chip ▾│
├──────────────┬────────────────────────────────────────────────┤
│ Track headers│ Ruler (bars/beats)                     playhead│
│ ─ Pulse 1    │ [clip][clip]        [clip]                      │
│ ─ Pulse 2    │      [clip]   [clip]                            │
│ ─ Triangle   │ [clip──────────]                                │
│ ─ Noise      │   [clip] [clip] [clip]                          │
│ ─ DPCM       │                                                 │
│ ─ VRC6 Pls 1 │        [clip]                                   │
│ ─ VRC6 Pls 2 │        [clip]                                   │
│ ─ VRC6 Saw   │ [clip──────]                                    │
├──────────────┴────────────────────────────────────────────────┤
│ Device panel: dials (Duty, Volume, Env Attack, Env Decay,      │
│ Sweep, Detune) · text fields · per-channel level meters        │
└───────────────────────────────────────────────────────────────┘
```

### 4.2 Arrangement clips (the most important component)
- Rounded rectangles filled with a **vertical gradient** in the lane's color.
- Lighting stack, from bottom to top:
  1. soft outer drop shadow (offset y ~2px, blur ~6–10px, low alpha)
  2. base gradient (lighter at top, darker at bottom)
  3. 1px **inner top highlight** (white, ~10–18% alpha) for the "lit from above" look
  4. 1px inner border, darker than the base, low alpha
  5. subtle inner shadow at the bottom edge
- A clip header strip with the clip name, plus a **procedurally drawn mini
  note preview** (small bars drawn from mock note data).
- States, all animated (~120–180ms ease):
  - hover: slight brightness lift
  - selected: colored outer glow + brighter border
  - dragging: raised shadow, ~90% opacity, ghost outline at the original spot
- Interactions:
  - **drag to move** horizontally, snapping to the grid (1/16, 1/4, bar; hold
    Alt to disable snap)
  - **drag vertically** to move to another lane
  - **resize from the left/right edges** ("expand"), with a resize cursor and
    a minimum length
  - click to select, Shift-click for multi-select, Delete to remove
  - Ctrl/Cmd-drag to duplicate (stretch goal)
- The playhead moves at 60fps when "playing" (simulated; no audio).

### 4.3 Text fields (BPM, position, clip name, search)
- **Inset look:** a darker gradient background with an **inner shadow** along
  the top and left edges, as if sunk into the panel.
- A thin highlight along the bottom outer edge (a "machined bevel" catching light).
- Focus: an animated soft accent-colored glow, plus a caret that pulses gently.
- BPM and position fields also support click-drag to scrub values.

### 4.4 Rotary dials (procedural, no images)
- Body: a radial or linear gradient for a brushed, slightly domed look, with
  a specular highlight in the upper left.
- A value arc around the dial in the accent color, with a soft glow; the
  unfilled track is dark and inset.
- A pointer line or dot on the body.
- Interaction: vertical drag changes the value, Shift gives fine control,
  double-click resets to default, the scroll wheel adjusts, and a value label
  appears while dragging.
- Must stay crisp at every DPI scale.

### 4.5 Meters and channel visualizer
- One vertical level meter per channel, animated at 60fps with fake data
  (random walk + decay), with gradient fill (green → amber → red), a peak-hold
  line, and an inset well.
- A strip of 8 channel "lights" that pulse when that channel's mock notes play.
  This is the demo-video hook.

### 4.6 Buttons and transport
- Pill/rounded buttons with a gradient, top highlight, and pressed state
  (inset shadow, content shifted 1px down). The record button glows red when armed.

## 5. Visual design tokens

Keep the tokens in one place (`src/theme/tokens.rs` and the main stylesheet) so
the whole look can be retuned quickly.

| Token | Value | Use |
|---|---|---|
| `bg-0` | `#0E0F13` | window background |
| `bg-1` | `#16181F` | panels |
| `bg-2` | `#1E2129` | raised surfaces |
| `bg-inset` | `#0A0B0E` | wells, text fields |
| `line` | `#2A2E38` | dividers |
| `text-hi` | `#E8EAF0` | primary text |
| `text-lo` | `#8A90A0` | secondary text |
| `accent` | `#7C5CFF` | focus, selection, dial arcs |
| `lane-pulse` | `#FF5C8A` | Pulse 1/2 |
| `lane-tri` | `#4CC9F0` | Triangle |
| `lane-noise` | `#B8C0CC` | Noise |
| `lane-dpcm` | `#F4A261` | DPCM |
| `lane-vrc6-pulse` | `#9B5DE5` | VRC6 pulses |
| `lane-vrc6-saw` | `#00F5A0` | VRC6 saw |

- Radius: clips 6px, fields 6px, buttons 8px, panels 10px.
- Spacing scale: 4 / 8 / 12 / 16 / 24.
- Shadows: always low alpha and soft, and usually layered. **Never** harsh
  black drop shadows.
- Gradients: subtle. Typically ±6–12% lightness from top to bottom. If it
  looks like a 2008 web button, it's too strong.
- Generally avoid flat fills on interactive surfaces.

## 6. Fonts
- Bundle one OFL-licensed UI font (e.g. Inter) and one monospace font (e.g.
  JetBrains Mono) for numbers and the position display. Use tabular figures for
  numbers that change.
- Put the license files next to the fonts in `assets/fonts/`.

## 7. Project structure
```
chipdaw-gui/
├── Cargo.toml          # workspace
├── crates/
│   ├── ui/             # library: views, theme, state (plugin-reusable)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── state.rs        # Arrangement, Lane, Clip, Selection, Transport
│   │   │   ├── mock.rs         # demo data + fake meter/playhead generators
│   │   │   ├── theme/{tokens.rs, style.css}
│   │   │   └── views/{arrangement.rs, clip.rs, ruler.rs, dial.rs,
│   │   │              text_field.rs, meter.rs, transport.rs, visualizer.rs}
│   └── app/            # binary: opens the standalone window
├── assets/fonts/
├── NOTES.md            # Vizia capability findings (§3)
└── FINDINGS.md         # bake-off report (§9)
```

Data model, as a starting point:
```rust
struct Clip { id: u32, lane: usize, start_ticks: u32, len_ticks: u32,
              name: String, notes: Vec<MockNote> }
struct MockNote { start: u32, len: u32, pitch: u8 }
// 960 ticks per quarter note. Snap is computed in ticks, never pixels.
```
Keep pixel ↔ tick conversion in one place, with zoom (horizontal) as state.

## 8. Quality bar and performance
- Runs at a steady 60fps with **200 clips** on screen while the playhead moves
  and the meters animate. Add a debug overlay (F3) showing frame time.
- Crisp at 100%, 150% and 200% scaling. Test each.
- No layout jank when resizing the window.
- Hot-reload the stylesheet during development if the chosen version supports it.
- `cargo clippy` is clean, and `cargo build --release` has no warnings.

## 9. Deliverables
1. `cargo run --release -p app` launches the demo with mock data.
2. Screenshots at 100% and 200% scaling in `docs/screens/`.
3. `NOTES.md`: the Vizia capability checklist from §3.
4. `FINDINGS.md`: the bake-off report, which must include:
   - what was achievable with stylesheets alone vs. custom drawing
   - effects that were impossible or ugly, and the workaround used
   - measured frame times (idle, 200 clips, dragging)
   - rough lines of code per component
   - how often AI-generated Vizia code was wrong (invented APIs, version
     mismatches) and how it was fixed
   - an honest verdict: "Vizia is / isn't viable for the full DAW UI, because…"

## 10. Working rules
- Build in this order: theme tokens → static layout → clip visuals → clip
  interactions → dials → text fields → meters/visualizer → polish. Take a
  screenshot after each step.
- Compile and run often; don't write 500 lines before the first build.
- Polish the clips before starting other components. If time runs short, a
  stunning arrangement view beats mediocre everything.
- Do not work around a missing visual feature by adding an image. Draw it, or
  write it up in `FINDINGS.md`.
- Out of scope: audio, MIDI, file save/load, undo/redo (stretch only), plugin
  hosting, chip emulation.
