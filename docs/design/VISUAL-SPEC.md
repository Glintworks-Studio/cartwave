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
- The overhang, over the values only: black at 78% (0 px), 50% (8), 28% (18), 12% (26), 0 (34),
  plus `inset 0 14px 16px -10px rgba(0,0,0,.95)`, inside the border (radius 7). Rendered once per
  size and scale.

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

## 4. Track rows

Header column 380 px including its right hairline `#1E1E1E`, background `#0A0A0A`. Rows 72 px
plus a `#1A1A1A` hairline (pitch 73) **(file)**. Positions from the row's top-left:

| Element | Position and style |
|---|---|
| Colour strip | x 0–16, full height, square. Unlit `mix(c, black .62) → mix(c, black .35)`; lit `c → mix(c, white .35) (55%) → mix(c, white .7)`, inner glow `inset 0 0 10px` white 55% and a `0 0 10px` spill in the track colour at 35% **(file)**; edges: left `mix(c, black .2)` → `mix(c, white .6)` lit, right `mix(c, black .5)`. Note-ons light it; it fades over 250 ms |
| Number | Inter 800 italic 40, `#222222`, centred in x 28–56, line box y 16–56, tracking −.02em |
| Name | x 68, Inter 600 14, `#E6E6E6`, ellipsis at x 236 |
| Profile badge | x 68, y 40, 18 px tall (16 inside a 1 px `#323232` border) **(file)**, radius 4: lock 9×11 and the chip name, Inter 600 11 `#A8A8A8`, 4 px padding and gap |
| Preset | 8 px after the badge, Inter 500 12 `#808080`, ellipsis at x 236 |
| M / S | 24×24 keys at x 312 / 344, y 16; glyphs Inter 700 12 |
| Volume meter-fader | track 68×4 at x 248, y 52 (`#050505`, `inset 0 1px 0 #000`, `0 1px 0 #262626` below); fill = post-fader level on the fader's scale, never past the thumb, gradient over the full 68 px: `#2F6FD6` 0, `#2FB7B0` 22, `#C7D74E` 44, `#F0A23A` 56, `#E0483B` 68; thumb 8×12 square, `#4A4A4A` border, `#5A5A5A → #2E2E2E`, `#A0A0A0` inner top line |
| Pan | track 40×4 at x 328, a 1×8 `#4A4A4A` centre detent, the same thumb; disabled: "Pan arrives with stereo mixing (Batch 4)." |

Full name, profile and preset show on hover.

## 5. Lanes and clips

Rows alternate `#171717` / `#141414` over their `#1A1A1A` hairline; bar grid `#262626`, beat grid
`#1D1D1D`. Clips start on their grid line and end 1 px short (abutting clips show a 1 px gap; the
owner's call over the mockup's 2 px inset at both ends, RUNLOG DP-D23), and sit 2 px in from the
row's top and bottom: radius 4, body the track colour at 22% → 10% (the
grid shows through), a 16 px header in the track colour inside the border with Inter 600 11
`#111111` text 8 px in, a 1 px border in the track colour at 50%. The time-selection wash:
`rgba(255,255,255,.045)` with 14% edges. No shade under the ruler, no dimming outside the loop
**(file)**.

## 6. Default track colours (file)

Pulse `#C9D24A`, Triangle `#7F86E0`, Noise `#B5AC98`, VRC6 Pulse `#D9579A` (the mockup's tracks);
DPCM `#E39A42` and Saw `#3CC4A6` unchanged.
