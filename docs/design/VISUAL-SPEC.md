# VISUAL-SPEC.md: Option D

The approved look, as built in the design pass (branch `design/option-d`). Source of truth:
[`mockup-D.dc.html`](mockup-D.dc.html). Where the prompt and the file disagreed, **the file
won**; those places are marked **(file)** and listed in [DESIGNPASS.md](../../DESIGNPASS.md) §4.
Measured element boxes of the mockup are in [`mockup-boxes.jsonl`](mockup-boxes.jsonl)
(`tools/measure_mockup.py`). Code: tokens in `crates/ui/src/theme/tokens.rs` and `style.css`;
materials in `views/gfx.rs`; the top bar in `views/topbar.rs`; rows, keys and the spectrum in
`views/chrome.rs`; the ruler in `views/ruler.rs`; lanes in `views/arrangement.rs`; clips in
`views/clip.rs`.

## 1. Palette and materials

| Token | Value |
|---|---|
| Window, top bar | gradient `#191919 → #0A0A0A`; the bar has a 1 px top line `#323232` and a bottom hairline `#1A1A1A` |
| Text hi / value / mid / lo | `#E6E6E6` / `#EDEDED` / `#9A9A9A` / `#808080` |
| Icons idle / dim | `#D6D6D6` / `#8C8C8C`; the record ring `#B9534C` |
| Key material | 1 px border `#383838`; body `#161616 → #424242` over the whole border box; a 1 px sheen **inside** the border (the second pixel row, clipped by the inner corners) `#3E3E3E → #999999 (centre) → #3A3A3A` **(file)**; radius 8 on 40/56 px keys, 4 on 24 px keys |
| Recessed well | fill `#151515`, 1 px `#323232`, radius 8; inner bottom highlight `inset 0 -1px 0 rgba(255,255,255,.07)`, outer `0 1px 0 rgba(255,255,255,.04)` |
| Lime `#C7D74E` | **switched-on states only**: lit M/S glyphs (text glow `0 0 6px` 85% + `0 0 2px` 90%), the display's active indicators (icon `drop-shadow(0 0 3px .65)`, text `0 0 6px .75`), the selected view icon (`drop-shadow(0 0 3px .55)` **(file)**), the loop brace while looping |
| Neutral stand-in | `#E6E6E6` for the playhead, focus, dial arcs, checked buttons, drag targets |
| Button tiers | key material: view, transport, TAP, M/S. Every stylesheet button (dialogs, panels, browser): flat `#151515`, plain 1 px `#323232` (hover `#4A4A4A`, checked `#9A9A9A`), no sheen, no gradient stroke. Notices: no stroke |
| Dithering | every gradient: drawn ones (Skia `Paint::set_dither`) and stylesheet ones (vizia fork 60907aad) |
| Grid | every layout distance a multiple of 4, except timeline positions (they follow the zoom), 1 px hairlines, and the 73 px row pitch (72 + hairline) **(file)** |

## 2. Top bar: 76 px, 16 px side padding, three groups spaced between

**Left** (gap 12): view keys 40×40, gap 8 (Arrangement: lime glowing icon; Mixer: dim, disabled
with its reason); a 1×28 divider `#323232`; transport keys 40×40, gap 8: Play, Stop, Record
(disabled, reason), Loop. Transport icons are neutral `#D6D6D6`: the display shows the state.
Icons are the mockup's SVG paths at the mockup's pixel positions.

**Centre** (gap 12): the display well 560×56, then TAP 56×56 (Inter 700 11, tracking .08em,
`#C8C8C8`).

Display cells, centred as a group at `floor((560 − 525) / 2)` = 17 px (the mockup's group starts at
17.5; its dividers land on the pixel below) **(file)**. Cell widths 72 / 96 / 80 / 64 / 64 / 64,
dividers 1×32 `#2A2A2A` with 8 px margins.

| Cell | Value | Label |
|---|---|---|
| Indicators | 2×2 grid, gaps 8×8: Play and Loop icons; "48k", "24-bit" (Inter 700 11). Lime while active, `#4A4A4A` while not | — |
| TIME | `m:ss.cc`, Inter 600 20, tabular | TIME |
| POSITION | `bar.beat.16th` (1-based, unpadded), JetBrains Mono 600 18 | POSITION |
| START / LENGTH | the time selection, else the selected clips' span, else "–"; LENGTH is 0-based (`2.0.0`) | START, LENGTH |
| BPM | `150.00`, Inter 600 20 | BPM |

- Every value sits on one baseline (27 px below the well's top), every label on another (43 px):
  Inter 600 10, line 12, tracking .1em, `#808080` (the 10 px exception, UX.md). Each cell's ink
  (value top to label baseline) centres on the well within ±0.5 px (measured, DESIGNPASS.md §2).
- The overhang, over the values only: black at 62% (0 px), 40% (8), 22% (18), 10% (26), 0 (34),
  plus `inset 0 14px 16px -10px rgba(0,0,0,.8)`, inside the border (radius 7): the mockup's
  (78 / 50 / 28 / 12 %, inset .95) toned to ~80% by the CEO in the polish pass, same 34 px
  reach. Rendered once per size and scale. The spectrum well has none.

**Right** (gap 12): the spectrum well 216×56 (no overhang): 24 square bars 4 px wide, 4 px apart,
12 px side padding, 8 px bottom padding; bar gradient `#4A4A4A → #9A9A9A (60%) → #D6D6D6`; a 2 px
peak mark `#C8C8C8` at 55%, 4 px above its bar. One spectrum of the whole mix. Styles (right-click):
Bars, LED Segments, Line, Peak Dots; all neutral. Then CPU / MEM: a 2×2 grid 84 px wide (the
mockup's 83.44), Inter 600 11, tracking .06em, labels `#808080`, values `#E6E6E6` right-aligned,
gaps 12×8.

## 3. Ruler: 28 px

`#0F0F0F`; **one** hairline `#1E1E1E` where it meets the tracks. Labelled bars: a full-height
1 px line `#3A3A3A` and the number (Inter 500 11, `#808080`, 4 px after the line, baseline 18);
beats 8 px `#3A3A3A`; sixteenths 4 px `#262626`. The TRACKS cell beside it: Inter 600 11, tracking
.08em, 16 px in, the same baseline. Loop brace: 4 px along the top, `#D3E05E → #A9B83A` with a
`0 0 6px` 45% glow while looping; `#5A5A5A → #404040` while off. Playhead marker: the mockup's
pentagon (`#E6E6E6`, 1 px white stroke) with a stem to the ruler's bottom; the line in the lanes is
1 px `#E6E6E6` at 85%.

## 4. Track rows (polish 2: E1)

Boards: [`polish2-ref/E1-header`](polish2-ref/E1-header.html), `E1-row-heights`,
`E1-header-width-side`, `E1-header-side-order`; element boxes measured from the HTML
(`docs/polish2/tools/measure_ref.py` → `docs/polish2/boxes/`). Code: `views/header_layout.rs`
(geometry), `views/chrome.rs` (`TrackRow`, `VolumeFader`, `PanSlider`).

Header column **380 px** (380 … 600, the boundary drag) including its 1 px hairline `#1E1E1E` on
the lanes' side, background `#0A0A0A`. Rows **72 … 300 px** (4 px steps) plus a `#1A1A1A` hairline.
Positions in the "strip-first" frame (logical px from the row's top-left, header `w` wide):

| Element | Position and style |
|---|---|
| Colour strip | x 0–16, full row height, square. Unlit `mix(c, black .62) → mix(c, black .35)`; lit `c → mix(c, white .28) (55%) → mix(c, white .55)`, inner glow `inset 0 0 10px` white **40%**, a `0 0 10px` spill in the track colour at **25%** toward the header's inside (polish 2: toned down ~20%); edges: left `mix(c, black .2)` → `mix(c, white .6)` lit, right `mix(c, black .5)`. Note-ons light it; it fades over 250 ms |
| Number | Inter 800 italic 40, `#222222`, centred in x 28–56, line box y 16–56, tracking −.02em |
| Name | x 68, Inter 600 14, `#E6E6E6`, ellipsis at `w − 144` (extra width goes only here) |
| Profile badge, preset | x 68, y 40, as before; the preset ends with "…" at `w − 144` |
| Control block | 120 px, 12 px in from the far edge: x `w − 132` … `w − 12`; never stretches or mirrors inside |
| Column A (56 px, at block + 8) | M / S keys 24 × 24, 8 apart, at y 10 (unchanged keys, lime glyph when on); the pan value at y 42 (Inter 600 10/12, tracking .02em, `#9A9A9A`: `C`, `13L`, `25R`); the pan well 56 × 4 at y 58 (`#050505`, `inset 0 1px 0 #000`, `0 1px 0 #262626`), a 1 × 8 `#4A4A4A` centre tick at 28, a `#8A8A8A` fill from the centre, the 8 × 12 thumb (`#4A4A4A` border, `#5A5A5A → #2E2E2E`, `#A0A0A0` inner top line); thumb centre `28 + pan / 50 · 28` |
| Column B (40 px, 16 after A) | the vertical meter-fader: well 4 px wide at the column's centre, from y 8, **row height − 32** long; the post-fader meter fills from the bottom with the cool-to-hot gradient, stops at **0 / 45 / 70 / 85 / 89 %** of the length (`#2F6FD6`, `#2FB7B0`, `#C7D74E`, `#F0A23A`, `#E0483B`: hot at 0 dB), never past the thumb; the 12 × 8 thumb, centre at `round((1 − (dB + 48) / 54) · length)`; the dB value 4 px under the well (Inter 600 10/12, `#9A9A9A`, one decimal, a real minus `−`, `−inf dB` at silence) |

**Taller rows** (`E1-row-heights`): the strip runs the full height; the number, the name block and
column A stay in the **72 px band at the top**, exactly as in a 72 px row; only the fader stretches.
**dB scale marks** on rows **≥ 96 px**: 0, −6, −12, −24 dB labelled `0 6 12 24` (Inter 600 9/10,
`#808080`), ticks 4 px `#3A3A3A` at well x −10 … −6 (2 px left of the thumb), labels right-aligned at
well x −12; a label < 12 px from the last shown one is skipped (a 96 px row shows `0 12 24`).

Full name, profile and preset show on hover (a tooltip: content, not help).

## 4a. Header layouts (polish 2 §5)

Settings ▸ Look & Feel. "Outside" means the strip is at the window's edge. Mirrored rows place
every box at `w − x − width`; text stays left-aligned; the control block never mirrors inside.

| Headers | Strip | Left to right | Mirrored | Hairline |
|---|---|---|---|---|
| Left | Outside | strip · number · name · controls (default) | no | right edge |
| Left | By the clips | controls · name · number · strip | yes | right edge, under the strip |
| Right | By the clips | strip · number · name · controls | no | left edge, under the strip |
| Right | Outside | controls · name · number · strip | yes | left edge |

The TRACKS and OVERVIEW cells sit in the header column (their label 16 px in from its left). The
browser moves across as a whole, unmirrored, with its 1 px divider on the lanes' side. The width
boundary is a 4 px grab band inside the column on the lanes' side (ew-resize cursor).

## 5. Lanes and clips

Rows alternate `#171717` / `#141414` over their `#1A1A1A` hairline, each as tall as its track
header (polish 2); bar grid `#262626`, beat grid
`#1D1D1D`. Clips start on their grid line and end 1 px short (abutting clips show a 1 px gap; the
owner's call over the mockup's 2 px inset at both ends, RUNLOG DP-D23), and sit 2 px in from the
row's top and bottom: radius 4, body the track colour at 22% → 10% (the
grid shows through), a 16 px header in the track colour inside the border with Inter 600 11
`#111111` text 8 px in, a 1 px border in the track colour at 50%. The time-selection wash:
`rgba(255,255,255,.045)` with 14% edges. No shade under the ruler, no dimming outside the loop
**(file)**.

## 6. The browser header (polish pass)

A 24 px title row inside the browser's 4 px top padding: "BROWSER" in the inspector's
section-title style (Inter 600 11, `#9A9A9A`), the 24 × 24 Preview key (plain hairline tier)
centred on the row, 8 px to the shortcut text, 12 px in from both edges like the list. The
search field and the list stay where they were.

## 7. Colours (polish pass)

- **Palette:** `crates/ui/src/theme/palette.rs`, 70 base colours (5 rows light → dark × 14
  OKLCh hues), ours; nothing within OKLab ΔE 0.06 of the toggle lime except the legacy pulse
  `#C9D24A`.
- **Recipes** (the only way a surface gets a palette colour's shades): `strip` (unlit
  `mix(black .62) → mix(black .35)`, lit `c → mix(white .28) → mix(white .55)` since polish 2, edges),
  `clip` (header; body 22% → 10%; border 50%; notes `mix(white .12)` at 92%; hover and
  selection lighten), `marker` (browser: `c → mix(black .35)`), `graph_bar` / `graph_fill`
  (inspector macros), `swatch` (menus: `mix(white .08) → mix(black .12)`), `overview` (polish 2:
  the base colour at 50% and a 1 px top line in the base colour).
- **Clip header text:** `#111111`, or `#F2F2F2` when the dark text wouldn't reach 4.5:1.
- **Colour grid:** inside the context menu: 16 px swatches on a 20 px pitch, 14 to a row, 8 px
  in; the current colour has a 2 px `#E6E6E6` ring, the hovered one a 1 px `#9A9A9A` ring.

## 8. Window

Start size 1440 × 920; minimum **1296 × 720** (the top bar's groups need 1269 px; the lanes
scroll when they don't fit, above the inspector too). A 1080p laptop at 125% (1536 × 816) fits.

## 9. Default track colours (file)

Pulse `#C9D24A`, Triangle `#7F86E0`, Noise `#B5AC98`, VRC6 Pulse `#D9579A` (the mockup's tracks);
DPCM `#E39A42` and Saw `#3CC4A6` unchanged.

## 10. The status bar (polish 2 §6)

24 px along the window's bottom, under the inspector or piano roll: `#0F0F0F`, a 1 px top hairline
`#1E1E1E`, Inter 500 11, 16 px side padding. Left (`role=status`): a 10 × 8 check (idle) or a 4 px
dot (a message), 8 px, then the text in `#9A9A9A` (6.4:1). With a persistent problem a 12 × 11 amber
(`#F0A23A`, the meter's hot stop: not lime) warning triangle comes first. Right: the hover help,
right-aligned, `#808080` (4.9:1). The lanes give up its height and scroll.

## 11. The Overview (polish 2 §7)

32 px between the top bar and the ruler, its strip aligned with the lanes (`#0A0A0A`); the label
cell in the header column (the TRACKS cell's look, `OVERVIEW` and the song length `24 BARS`, Inter
600 10, `#808080`, right-aligned 16 px in). The whole song spans the strip. Rows from y 3, 6 px
tall, 1 px apart, shrinking to 1 px with more tracks, then merging tracks (50+ still fit). Clips:
the `overview` recipe, 1 px short of their end. The visible part: 1 px `#8C8C8C`, radius 2, white 4%
with an 8% inner top highlight, y 2 … 31 (not lime). The loop: a 2 px lime line at 55% along the top,
only while the loop is on. The playhead: 1 px `#E6E6E6` at 60%. The clip layer is cached and redrawn
only when clips, colours, the track count or the song length change.

## 12. macOS: colour management (Mac catch-up)

Every token above is **sRGB**, and on macOS the window says so: vizia fork commit 8 sets the
window's colour space to sRGB (`NSWindow.colorSpace`), so the window server converts what Skia
draws into each display's own profile, as it does for any native AppKit view.

Measured (MAC-CATCHUP.md §4.7, `docs/mac-catchup/tools/color_mac.py`): the app and a native
AppKit window of the same sRGB colours side by side on both displays, captured and read as
Digital Color Meter's "Display in sRGB" does (converted from the display's profile).

| | 320 Hz panel (wide gamut) | 60 Hz panel (sRGB-like) |
|---|---|---|
| App vs native window, raw display values | identical (0 levels) on all 6 flat swatches | identical |
| App in sRGB vs the token | ≤ 1 level (pulse header `#C8D24A` for `#C9D24A`) | 0 |
| Lime `#C7D74E` (the display's indicator text) | `#C6D74E` | `#C7D74E` |
| Raw values the panel gets, e.g. VRC6 `#D9579A` | `#C16598` (gamut mapping, as for native apps) | `#D9579A` |

- **Check colours in sRGB mode.** On a wide-gamut display the raw (device) values differ from
  the tokens by design; Digital Color Meter's "Display native values" shows those, "Display in
  sRGB" shows the tokens.
- Screenshots taken with `screencapture` carry the display's profile; convert them to sRGB
  before comparing pixels with Windows screenshots (the Mac harness's `img` tool does).
- Dithering (fork 60907aad) and ColorSync's conversion don't interact: banding measured on the
  raw values and on the sRGB-converted ones is the same (`docs/mac-catchup/results/banding.md`).
- Untested here: a Retina (P3) laptop panel, and the 2× path on a native 2× display (the 2×
  checks ran in a HiDPI mode of an sRGB-like monitor).
