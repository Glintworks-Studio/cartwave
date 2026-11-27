# K1: the menu bar while the song plays and the window is resized (Batch 4c §3)

Tools: [`menu_flicker_b4c.ps1`](../tools/menu_flicker_b4c.ps1) (from 4b's, plus `-Play`: audio on and
the song playing from launch, `-Project`, `-StepMs`, the right/bottom/top edges, `-Below` to also
capture the top 60 px of the client area, and the app's frame log), [`flicker_b4c.py`](../tools/flicker_b4c.py)
(4b's light test plus a *misdrawn* test: the labels' part of the bar differing from the steady
bar) and [`blocks_b4c.py`](../tools/blocks_b4c.py) (any solid 6 × 6 light block in the whole band that
the steady picture doesn't have). A real mouse drag (Windows' own sizing loop) for 4 s, captured
with ffmpeg's Desktop Duplication grabber (`ddagrab`, every composed frame, up to 120 fps).

[`results.tsv`](results.tsv): every run. Summary:

| Build | Condition | Runs | Flash frames | Misdrawn | New light blocks |
|---|---|---|---|---|---|
| this branch | **playing** (the demo), dark menus on: corner, right, bottom, left, a fast drag (4 ms steps) | 5 | **0** | 0 | 0 |
| this branch | **playing the 64-track project**, dark menus on: corner, right | 2 | **0** | 0 | 0 |
| this branch | playing, with the client strip: corner, right, 64 tracks | 3 | **0** | 0 | **0** |
| this branch | playing, dark menus **off** (corner) | 1 | 0 | 0 | |
| this branch | stopped, with the client strip (corner) | 1 | 0 | 0 | 0 |
| `main` (before) | playing, dark menus on, with the client strip: corner, right, bottom, left | 4 | **0** | 0 | 0 (left: 142, see below) |
| positive control | a white patch over the bar twice, ~50 ms each | 1 | **12** | | **12** |

- The **left-edge** drag moves the window, so the app's own light glyphs (the transport's stop
  square) slide through the fixed capture band: the block test flags them (142 frames on `main`).
  They are not flashes: [the frames](play-corner-frames.png) show the dark bar intact; the light
  and misdrawn tests read 0 for the same run. The block test is valid for the other edges only.
- **Dark menus on or off makes no difference to the bar** (median `#2B2B2B` in both): the bar is
  muda's (`MenuTheme::Dark`, which paints it on the system's menu-bar paint messages), and the
  setting only changes the dropdowns (`menus_dark.rs`). UX.md's "test with dark menus off" can't
  separate the two code paths for the bar.

**Result.** K1 does not reproduce in the CEO's condition (playing while resizing) on this PC, in
17 captures, before or after this batch, while the positive control is caught every time. So no
code was changed for it: a fix can't be shown to fix something that can't be measured, and the
prompt's mechanism (our presents interleaving with the non-client repaint) produced no composed
frame with a light or misdrawn bar in ~11,500 frames of resizing. **What the capture can't see:**
Desktop Duplication records what DWM composes. A glitch after composition (a presentation-mode
change of the GL swap chain, e.g. a hardware overlay plane being taken and released as a resize
starts or ends, or the monitor itself) would not appear in it. The next step needs the owner: a
phone slow-motion video of the flicker (240 fps), with the display scale and how the window was
resized, so the capture can be pointed at the right thing.
