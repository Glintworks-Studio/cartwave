# DESIGNPASS.md: the Option D design pass

Branch **`design/option-d`** (from `main` 23b5da9), pushed, **not merged**: the CEO approves the
comparison screenshots first (§7 has the merge commands). Prompt: `DESIGN_PASS_PROMPT.md`. The
approved mockup is committed as [`docs/design/mockup-D.dc.html`](docs/design/mockup-D.dc.html);
the spec as built is [`docs/design/VISUAL-SPEC.md`](docs/design/VISUAL-SPEC.md); every decision is
in [`docs/RUNLOG.md`](docs/RUNLOG.md) (DP-D1…D22).

## 1. Summary (one screen)

**Built.**
- **Top bar (76 px)**: Arrangement / Mixer view keys, Play / Stop / Record / Loop in the key
  material; the **display** well (indicators, TIME, POSITION, START, LENGTH, BPM) under its
  overhang; **TAP**; the **spectrum** well with four styles; CPU / MEM. The BPM pill and the
  transport capsule are gone.
- **Track rows** (380 × 72 + hairline): full-height strip that lights on note-ons, big italic
  number, name, profile badge, preset, M/S, a **volume meter-fader** (the channel volume; the fill
  is the post-fader level and never passes the thumb), a disabled pan slider.
- **Ruler** (28 px, one hairline, lime brace while looping, the pentagon playhead); alternating
  lanes, the grid, the mockup's clips.
- **Lime** only in its four places (audit: 0 violations); **buttons** in two tiers; **dithering**
  on every gradient (drawn and stylesheet; vizia fork 60907aad).
- **Behaviour**: TAP (key T; one undo step per sequence); BPM drag, wheel and type-in on the
  display; Record and Mixer disabled with reasons; strips fade over 250 ms; spectrum styles
  (right-click, remembered); browser **Preview** key and arrow-key preview; Project Settings
  **Sample rate / Bit depth** (file format v5, the export's default); **Dark menus** behind
  Settings ▸ Display, default **off**.

**Pixel match** (§2): of 39 measured elements, 31 sit within 1 px of the mockup (26 exactly).
The 8 over 1 px are all explained: 3 by renderer differences (text hinting, glow spread), 3 by
data (a fader value, note previews in two clips), 1 by the measurement window, and the clips'
left edges by the owner's mid-run decision (clips start on the grid line, DP-D23). Display alignment: every cell's ink centre
is within ±0.5 px of the well's middle, and all values share one baseline.

**Banding** (§3): on the stylesheet's bottom-panel gradient the mean run of identical pixels drops
from 25.3 px to 2.0 px; on the drawn canvas gradient from 32.3 px to 2.0 px. Banded rows (runs of
4 px or more): 100% → 48% and 99% → 42%.

**Tests** (§5): DP-01…05 pass; AT-01…19 and AT-07…31 pass; `cargo test --workspace` (golden
render, the new v4 → v5 migration hashes) passes; clippy clean; release build 0 warnings.
**Performance** (§6): demo and 200-clip playback at or below the pre-run build's CPU; dragging
clips +1.3 points (the display's START/LENGTH now follow the drag); pacing clean everywhere.

**Decisions taken alone**: DP-D1…D22 in RUNLOG. The big ones: where the prompt and the file
disagreed the file won (row pitch 73, badge 18 px, the strip's spill,
the sheen inside the border); the fader is the channel volume, disabled with a reason where there
is none (triangle, DPCM, decay envelopes); the project's format got a sample rate as well as a bit
depth (format v5); the clock-format option is retired; the mockup's four track colours were
adopted; the minimum window grew to 1272×920.

## 2. Pixel match against the mockup (§4.1, §4.2)

Method (`docs/design/tools/`): `render_mockup.ps1` renders the mockup at 1440×400 in headless
Chrome (through `support.js`, a stand-in for the mockup tool's runtime, DP-D1);
`mockup_state.ps1` puts the app in the mockup's state (the demo playing, loop on, the browser
closed, 100 px per bar, Triangle soloed, Noise muted, a time selection over bars 9–11);
`compare.py` crops the app's client area to 1440×400 and measures **both images the same way**:
for each element, the bounding box of the pixels of its colour inside a small window around the
mockup's box. The app's lanes are 1–4, the mockup's four chosen tracks are Pulse 1, Triangle,
Noise and VRC6 Pulse 1, so rows are compared row for row.

Images: [`compare-topbar.png`](docs/screens/design-pass/compare-topbar.png),
[`compare-tracks.png`](docs/screens/design-pass/compare-tracks.png) (the app above the mockup).
Full table: [`docs/screens/design-pass/raw/cmp/report.md`](docs/screens/design-pass/raw/cmp/report.md).

Exact (0 px): all six top-bar keys, the left divider, the display, the five cell dividers, TAP,
the spectrum, CPU / MEM, the row hairlines, the strips, the badge, the M/S keys, the pan track and
detent. Within 1 px: the Arrangement icon, the ruler bar line, the TRACKS label,
the number, the name.

**Deviations over 1 px, and why:**

| Element | Δ (l, t, r, b) px | Cause |
|---|---|---|
| TAP text | right +2 | Renderer: Skia's hinted Inter Bold at 11 px advances ~0.7 px wider per letter than Chrome's; the ink starts at the same pixel |
| Loop brace | right +3 | Renderer: the glow. Skia's blur (σ 3) reaches 3 px further at full alpha than Chrome's `box-shadow` at the brace's end; the brace itself ends on the same pixel |
| Preset text | bottom +3 | Renderer: the descender of "q": Skia draws it crisper, so it passes the colour threshold where Chrome's antialiased one doesn't; the tops match |
| Volume thumb | +2 both edges | Data: the demo's Pulse 1 volume is 12/15 (0.80 → 54 px), the mockup's 0.78 (53 px) |
| Clip bodies (Bassline, Walk Down) | bottom +47 | Data: the app draws note previews in the track colour inside clips (kept, DP-D4); the mockup's clips are empty |
| Clip left edges (Intro Lead, Bassline, Walk Down) | left −2 | Intended: the owner asked for clips to start on their grid line with a 1 px gap to the next one, not the mockup's 2 px inset at both ends (DP-D23) |
| Ruler hairline | left −21 | Measurement: in the mockup render the first 21 px of that row are covered by the TRACKS cell's own edge pixels; both images have the line across the full width |

**Display alignment** (ink of value top to label baseline, centre minus the well's middle):
indicators −0.5, TIME 0.0, POSITION +0.5, START +0.5, LENGTH +0.5, BPM 0.0 px: all within
±0.5. Every value's ink ends on the same row (one baseline, 27 px below the well's top; labels on
43). The mono and Inter values differ in cap height (13.1 vs 14.5 px), so the shared baseline is
placed where both faces stay within the tolerance.

## 3. Gradient banding (§3.8)

The measure: run lengths of identical pixels down a column through the gradient (5 neighbouring
columns). An undithered dark gradient shows flat bands as long runs; dithering breaks them into a
fine mix of neighbouring levels. "Before" for the stylesheet gradients is `main` 23b5da9; for the
drawn ones it is this build with the dev-only `CHIPDAW_NO_DITHER=1`.
Image: [`banding-before-after.png`](docs/screens/design-pass/banding-before-after.png)
(contrast-stretched ×3; left before, right after; top bar, bottom panel, canvas, clip, key).

| Gradient | Before: longest / mean run, banded rows | After |
|---|---|---|
| Top bar `#191919 → #0A0A0A` (stylesheet) | 4 / 4.0 px, 100% | 4 / 2.0 px, 61% |
| Bottom panel `#121212 → #0A0A0A` (stylesheet) | 30 / 25.3 px, 100% | 16 / 2.0 px, 48% |
| Canvas under the lanes `#191919 → #0A0A0A` (drawn) | 55 / 32.3 px, 99% | 24 / 2.0 px, 42% |
| Clip body (track colour 22% → 10%, drawn) | 2 / 1.3 px, 0% | 4 / 1.4 px, 2% |
| Key body `#161616 → #424242` (drawn) | 2 / 1.0 px, 0% | 2 / 1.2 px, 0% |

The longest runs that remain are where the gradient sits exactly on one level for a stretch (no
neighbouring level to dither with); the mean run of ~2 px is what the eye sees. The clip and key
gradients were already fine-grained (their columns cross notes and many levels); dithering
doesn't hurt them. Full table: [`banding.md`](docs/screens/design-pass/banding.md).

## 4. Where the prompt and the file disagreed (file won)

Row pitch 73 (72 + hairline); badge 18 px outer; the key sheen
inside the border; the strip's lit spill (§3.5 said no outer glow); the selected view icon's glow
55%; JetBrains Mono 600 (the file's font link only loads 500/700, so the mockup shows 700; both
texts say 600); the cell group at 17 px; the mockup's track colours. Details: RUNLOG DP-D2…D4.
One file value was reverted by the owner mid-run: clips no longer inset 2 px at both ends; they
start on the grid line with a 1 px gap to the next clip (DP-D23).

## 5. Tests

- **Scenarios** (`docs/design/tools/at_dp.ps1`, checker `check_dp.py`): DP-01 TAP (four taps 500 ms
  apart: 118.7, 120.9, 120.7 BPM from real key presses; the unit test proves exactly 120; one undo
  step; Ctrl+Z → 150, Ctrl+Y back), DP-02 spectrum style survives a restart, DP-03 preview on
  select (click, ↓, the Preview key off, the reason while playing), DP-04 disabled reasons (Mixer,
  Record, pan, the triangle's fader), DP-05 Bit depth saved (v5) and shown. All pass.
- **Earlier ATs**: AT-01…19 (`docs/shell/tools/at_shell.ps1`) and AT-07…31
  (`docs/batch3/tools/at_b3.ps1`) pass with coordinates moved to the new layout; AT-28's check
  follows DP-D8; AT-02 leaves the live MEM read-out out of its pixel diff.
- **Unit**: `tap::tests`, `state::fader_tests` (the meter never passes the thumb at six fader
  positions × six levels), `topbar::tests`, file format v5 tests, `tests/migration.rs`
  (`version_4_files_render_bit_identically_in_version_5`, 11 files), the golden render.
- **Lime audit** ([`lime-audit.md`](docs/screens/design-pass/lime-audit.md), screenshots in
  `docs/screens/design-pass/lime/`): stopped, playing, loop off, loop on, solo, mute, inspector,
  Project Settings: 0 lime pixels outside the allowed places.

## 6. Performance (§4.7)

Release builds, the 60 fps cap, this machine (ViewSonic 60 Hz, MOTU output). `bench.ps1` A/B,
interleaved with `main` 23b5da9 (`../prerun-dp`), 3 rounds each; process CPU, % of one core:

| Scenario | Before (mean of 3) | Design pass (mean of 3) |
|---|---|---|
| Idle | 0.8 | 0.7 |
| Demo playing | 9.1 | 8.6 |
| 200 clips playing | 9.5 | 9.2 |
| 200 clips, dragging | 10.8 | 12.1 |

Pacing (`pacing.ps1`, 30 s each, design build): demo, 200 clips and drag: **0 frames over 20 ms,
0 stalls**, 60.0 display updates per second (p99 interval 16.9–17.6 ms). Piano roll, 2,000 notes
(`bench_pr.ps1`): playing 5.6%, dragging 7.0% (before: 8.4%, 8.3%, but the old build doesn't print
the dev line that proves its scenario ran, so those runs are flagged "not reached").

How it got there:
- The first measurement showed about +2 points everywhere. Part of that was the harness:
  `bench.ps1` parked the mouse at (700, 40), empty top bar before but the new display now, holding
  its tooltip open the whole run (10.9% vs 7.5% with the mouse there). It now parks on empty canvas.
- The rest was per-frame blurs and text: the lit strips' glow and spill, the display's glowing
  indicators, its labels and well, and the track rows' text, which redrew every frame because the
  new meters' dirty region covers every row (freezing the meters with the dev-only
  `CHIPDAW_FROZEN_METERS=1` removed 0.7 points). All are now pre-rendered once per state and
  scale, and blitted (strip lighting stays exact: the lit layer is blended at the light level).
- **Remaining: dragging +1.3 points.** While clips are dragged, START and LENGTH follow them, so the
  display redraws on every mouse move; the old capsule showed no selection. Frames stay on time.
  Open item: if it matters, START/LENGTH could update once per frame instead of per move.


## 7. Screenshots

`docs/screens/design-pass/`: `compare-topbar.png`, `compare-tracks.png`,
`browser-before-after.png`, `inspector-before-after.png`, `menus-light-vs-dark.png`,
`banding-before-after.png`, `spectrum-styles.png`, and `lime/` (the audit's scenes).

## 8. Open items

- The minimum window (1272×920) doesn't fit a 1080p display at 125% (864 logical px tall). The
  lanes only scroll while the piano roll is open; letting them scroll always would allow a smaller
  minimum (a UX decision).
- The popups' 1 px frame stays light with Dark menus on (Windows draws it; DARK-MENUS.md).
- MEM reads "--" on macOS until it's implemented there.
- The spectrum's bars reach the top often on the dense demo (the level mapping is today's).
- Dragging clips costs +1.3 points of CPU (§6).

## 9. Merge (after the CEO's OK)

```
git checkout main
git pull --ff-only
git merge --ff-only design/option-d
git push origin main
```

If Dark menus is approved as the default, before merging: set `dark_menus: true` in
`impl Default for Settings` (`crates/ui/src/file.rs`) and update the
`dark_menus_default_off_and_round_trip` test and UX.md §2, then commit on the branch.
