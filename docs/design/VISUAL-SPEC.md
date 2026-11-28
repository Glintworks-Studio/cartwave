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
| Lime `#C7D74E` | **toggle on-states and the centre display only** (§13): lit M/S glyphs (text glow `0 0 6px` 85% + `0 0 2px` 90%), the display's active indicators (icon `drop-shadow(0 0 3px .65)`, text `0 0 6px .75`), the selected view icon (`drop-shadow(0 0 3px .55)` **(file)**), the loop brace while looping |
| Neutral stand-in | `#E6E6E6` for the playhead, focus, dial arcs, checked buttons, drag targets; selection is a white outline and a faint glow (§13) |
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
  **Decided 2026-11-26:** the overhang eases out over **48 px** (§13). Built: the 34 px reach
  above; to change in 4b's visual fixups.

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
| Number | Inter 800 italic 40, `#222222`, centred in x 28–56, line box y 16–56, tracking −.02em. **Decided 2026-11-26:** the column's width follows the project's largest track number (§13); built: fixed, so numbers ≥ 10 crowd the name (to change in 4b) |
| Name | x 68, Inter 600 14, `#E6E6E6`, ellipsis at `w − 144` (extra width goes only here) |
| Profile badge, preset | x 68, y 40, as before; the preset ends with "…" at `w − 144` |
| Control block | 120 px, 12 px in from the far edge: x `w − 132` … `w − 12`; never stretches or mirrors inside |
| Column A (56 px, at block + 8) | M / S keys 24 × 24, 8 apart, at y 10 (unchanged keys, lime glyph when on); the pan value at y 42 (Inter 600 10/12, tracking .02em, `#9A9A9A`: `C`, `13L`, `25R`); the pan well 56 × 4 at y 58 (`#050505`, `inset 0 1px 0 #000`, `0 1px 0 #262626`), a 1 × 8 `#4A4A4A` centre tick at 28, a `#8A8A8A` fill from the centre, the 8 × 12 thumb (`#4A4A4A` border, `#5A5A5A → #2E2E2E`, `#A0A0A0` inner top line); thumb centre `28 + pan / 50 · 28` |
| Column B (40 px, 16 after A) | the vertical meter-fader: well 4 px wide at the column's centre, from y 8, **row height − 32** long; the post-fader meter fills from the bottom with the cool-to-hot gradient, stops at **0 / 45 / 70 / 85 / 89 %** of the length (`#2F6FD6`, `#2FB7B0`, `#C7D74E`, `#F0A23A`, `#E0483B`: hot at 0 dB), never past the thumb; the 12 × 8 thumb, centre at `round((1 − (dB + 48) / 54) · length)`; the dB value 4 px under the well (Inter 600 10/12, `#9A9A9A`, one decimal, a real minus `−`, `−inf dB` at silence) |

**Decided 2026-11-26:** header meters use the pan grey with a white clip mark that holds until
clicked (§13). Built: the cool-to-hot gradient in column B above, whose `#C7D74E` stop is lime
outside a toggle; to change in 4b's visual fixups.

**Taller rows** (`E1-row-heights`): the strip runs the full height; the number, the name block and
column A stay in the **72 px band at the top**, exactly as in a 72 px row; only the fader stretches.
**dB scale marks** on rows **≥ 96 px**: 0, −6, −12, −24 dB labelled `0 6 12 24` (Inter 600 9/10,
`#808080`), ticks 4 px `#3A3A3A` at well x −10 … −6 (2 px left of the thumb), labels right-aligned at
well x −12; a label < 12 px from the last shown one is skipped (a 96 px row shows `0 12 24`).

Full name, profile and preset show on hover (a tooltip: content, not help). **Decided 2026-11-26:**
no tooltips anywhere; this goes to the status bar's hover help (§13). To change in 4b.

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

## 4b. Tracks as instruments: badges, the locked state, the Main row (Batch 4a)

Code: `views/chrome.rs` (`badge_of`, `badge_rect`, the locked fader and pan), `views/main_track.rs`,
`theme/palette.rs` (`main_strip`). The CEO's `B4a-main-track` board was not provided (RUNLOG
B4A-D1): the Main row follows the prompt's text on the E1 grid (B4A-D15).

| Element | Spec |
|---|---|
| **Badge** (x 68, y 40, 18 px, radius 4, 1 px `#383838` ring, Inter 600 11 `#9A9A9A`, 4 px padding) | Locked: the drawn lock glyph (9 × 11; Inter has no 🔒, `docs/batch4a/cmap.txt`) and the channel: `2A03 P1`, `2A03 P2`, `2A03 TRI`, `2A03 NOI`, `2A03 DPCM`, `VRC6 P1`, `VRC6 P2`, `VRC6 SAW`. Studio: the type without a lock: `PULSE`, `TRIANGLE`, `NOISE`, `DPCM`, `VRC6 PULSE`, `SAW`. A track locked to a channel the profile lacks: an amber (`#F0A23A`, the status bar's) 9 × 8 warning triangle first and an amber ring. A click opens the lock menu; the hand cursor over it |
| **Locked header** | The fader thumb at 0 dB and the pan thumb centred, drawn at **40% opacity** (the disabled style of drawn controls, `DISABLED_ALPHA`; "locked means dimmed", §13); the dB value replaced by the lock glyph in `#9A9A9A`, centred in column B; no gestures; hover help names the channel and the register |
| **Channel budget** | TRACKS cell: `P 2/2 · T 1/1 · N 1/1 · D 0/1 · V 1/2 · S 0/1`, Inter 600 10, tracking .1em, `#808080`, right-aligned 16 px in (the "24 BARS" style); a type over budget in `#F07075` (the record red `#E5484D` lightened to 5.3:1 on `#0F0F0F`; never lime). Project Settings lists the full form |
| **Header drag** | A 2 px `#E6E6E6` line on the row where the track lands (top edge moving up, bottom edge moving down); the dragged row lifts by white 4% |
| **Main row** | Pinned under the rows and lanes: a 1 px `#2E2E2E` divider, then 72 px (73 in all: the lanes give it up); an 8 px soft shadow (black 0 → 28%) on the rows above |
| **`main_strip`** | Black → `#1C1C1C` (50%) → `#363636` across the 16 px strip (dark at the outer edge), a 1 px `#4A4A4A` highlight on the outer edge, a 1 px white-6% inner ring. **Black is Main's**: no palette colour or automatic pick has a brightest channel under `#40` (unit test) |
| **Main header** | No number, no M/S. "Main" in the name's place (Inter 600 14 `#E6E6E6`); an `OUT` badge in the badge's place and the output device after it (Inter 500 12 `#808080`, ellipsis); PEAK in column A: label Inter 600 10 tracked .1em `#808080` at y 16, value Inter 600 12 `#E6E6E6` at y 32 (dBFS, one decimal, `−inf`); the master fader in column B (the tracks' meter-fader) with its dB value. It follows the header layouts (side, strip order, width) |
| **Main lane** | `#0D0D0D`, bar lines `#262626`, beat lines `#1D1D1D` (from 48 px per bar), the playhead 1 px `#E6E6E6` at 85%; nothing else (Batch 5: master automation) |

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

## 13. Rules from the 2026-11-26 meeting

Decided by the CEO; recorded by the docs update (RUNLOG DOC-D*). **Rules only:** the batches
implement them, and each line names where today's build differs. Boards:
[`meeting-2026-11-26/`](meeting-2026-11-26/README.md).

| Rule | Spec | Built (Batch 4b, BATCH4B.md) |
|---|---|---|
| **Lime** | `#C7D74E` only for **toggle on-states** and **the centre display** (its indicators). Never selection, meters, instrument displays or track identity | **Built.** The header meters are grey (below). New projects' Pulse tracks get `#4FA8E0` (`tokens::NEW_PULSE`); the palette's lime cell became Sage `#B4C08A`; existing files keep their colours bit for bit (B4B-D13). The lime audit finds `#C7D74E` only on toggle on-states and the display |
| **Instrument displays** | Waveforms and envelopes in **one warm display colour, `#E8DCC0`**. The track colour marks identity only: the strip, the name plate, the device tab | The inspector's macro graphs use the track colour (`graph_bar` / `graph_fill`, §7): to change in the Presets + Browser + device panel batch |
| **Locked means dimmed** | On a locked track, displays and knobs at about **40 % brightness**; labels kept at **≥ 4.5 : 1** | The header's and the mixer strip's fader, pan and knob at 40 % (the strip's fader drawn whole at 45 %, the knob veiled once); the instrument device: Presets + Browser + device panel batch |
| **Selection** | Neutral: a **white outline and a faint glow**, never lime | **Built.** `SELECT_OUTLINE #E6E6E6` 1 px outside the clip or note, a white 22 % glow (10 px blur); the fill keeps its own colour |
| **Header meters** | The **pan grey** (`#8A8A8A`, the pan fill of §4), plus a **white clip mark that holds until clicked** | **Built.** `METER_FILL` = the pan fill; the clip mark 2 px tall at the meter's top (a 4 px hit area), lit past 0 dBFS, reset by a click (B4B-D11) |
| **Mixer meters** | The warm palette, `meter-colours` option 2: segments warm white (`#A09C93 → #E0DCD3 → #A09C93`), amber (`#F5C870` centre, `#B68830` edges), orange (`#FFA757` / `#BF6817`), red above 0 dB (`#FF6C63` / `#BF2C23`); unlit segments their colour at 10 %; the peak mark `#F2F2F2` with a 4 px white glow. Zone boundaries as on the board | **Built** (`views/mixer.rs`; the `mixer` board match: `docs/batch4b/mixer-match/`) |
| **No tooltips** | Hover help goes to the status bar (§10) everywhere | **Built.** No tooltip remains; every former tooltip text is hover help in the status bar (B4B-D12) |
| **Track number column** | Its width follows the project's largest track number | **Built.** 28 px up to 9 tracks, 48 px for two digits, +20 px per digit after (B4B-D3) |
| **The display's overhang** | Eases out over **48 px** | **Built.** α = 0.6 · (1 − y/48)^2.2 (`topbar::overhang_alpha`), within 1 level of the board on every row |
| **Mixer M/S keys** | 24 px, a **darker key body** and a **`#101010` border**, so they separate from the textured panel | **Built** (24 px, `#101010` border, the darker body) |

Mixer knobs: `knobs` option A (machined: brushed cap, knurled skirt, thin white value ring).
Fader caps: plastic, as on the `mixer` board.

## 14. Automation (Batch 5; the starting proposal, for Sam's review)

No board yet: these are the values Batch 5 built (`crates/ui/src/theme/tokens.rs`, `AUTO_*`), to be
reviewed from `docs/screens/batch5/`.

| Element | Value | Note |
|---|---|---|
| Envelope line | `#D6D0C4` (warm white), 1.5 px, anti-aliased | Never lime (the CEO flagged green volume automation), never the track colour (identity only). |
| Breakpoints | 6 px dots, `#D6D0C4`; hovered 8 px | Closer than 3 px apart when zoomed out: only the line shows them. |
| Selected breakpoint | white 1.5 px outline + a white glow at 10 % | Neutral, like every selection. |
| Overridden envelope | `#5A5A5A` (line and dots) | The control was moved by hand: it plays its own value until Re-Enable. |
| No envelope | the control's value as a dashed line, `#D6D0C4` at 35 % (6 px dashes, 4 px gaps) | Click it to start an envelope. |
| Locked lanes | the frame-quantised staircase (one sample per frame tick at least, per pixel when zoomed out) | What the chip plays. |
| Lane band | `#111111`, a step under the rows (`#171717` / `#141414`), with the row hairline `#1A1A1A` | One row height per lane (the header's). |
| Lane time selection | white at 6 %, edges white at 18 % | As the arrangement's time selection. |
| Lane header label / value | `#A8A8A8` on `#111111` (7.9 : 1); value in JetBrains Mono 10 | "Device › Parameter" and a drawn 6 × 3.5 px chevron (Inter has no ▸ / ▾). |
| Automated-control marker | a 4 px dot `#F2F2F2`; `#5A5A5A` while overridden | The reference's LED is red; ours is neither red nor lime. |
| Automation Arm, on | the envelope icon in `#D6D0C4` with a 65 % glow of it | A toggle, but automation never uses lime (B6 over the lime-for-toggles rule). |
| Re-Enable Automation, lit | `#FF8A1F` (the meter's orange) with a 65 % glow | A momentary button: lit only while a control is overridden. |
