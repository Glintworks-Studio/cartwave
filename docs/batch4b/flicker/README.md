# K1: the Windows menu bar during a resize (Batch 4b §3.4)

Tool: [`menu_flicker.ps1`](../tools/menu_flicker.ps1) and [`flicker.py`](../tools/flicker.py). The app is
resized by a real mouse drag (Windows' own sizing loop) for 4 s, back and forth between 1400 and
1312 px wide, while ffmpeg's Desktop Duplication grabber (`ddagrab`: every composed frame, up to
120 fps) records the band from the title bar to the top of the client area. A frame is a flash
when the menu bar's median turns light, or when a light patch covers a fifth of it.

| Run | Frames | Flash frames | Largest light share |
|---|---|---|---|
| Baseline (d4f1afd), bottom-right corner, dark menus on | 720 | **0** | 0.017 |
| Baseline, corner, dark menus off | 720 | **0** | 0.017 |
| Baseline, left edge (the window moves), dark menus on | 720 | **0** | 0.008 |
| Baseline, left edge, dark menus off | 720 | **0** | 0.008 |
| Batch 4b, corner, dark menus on | 720 | **0** | 0.017 |
| Batch 4b, corner, dark menus off | 720 | **0** | 0.025 |
| Batch 4b, left edge, dark menus on | 720 | **0** | 0.008 |
| Batch 4b, left edge, dark menus off | 720 | **0** | 0.008 |
| Positive control (`-SelfTest`): a white patch over the bar twice, about 50 ms each | 720 | **12** | 0.289 |

(The light share is the menu text; the bar's median stays at 43, `#2B2B2B`.)

**Result.** 0 flash frames before and after, in every condition, on this machine (Windows 11
build 26200, one 2560 × 1440 display at 100 %, NVIDIA RTX 4070 SUPER). The positive control
shows that the capture and the detector see a 50 ms flash (12 frames for two flashes). An
earlier pass with `gdigrab` at 60 fps also found 0. No frame showed a partly drawn or missing bar
either (each corner-run frame equals the steady bar).

The flicker the owner saw (UX.md K1) doesn't reproduce here, so no code was changed for it: a
change can't be shown to fix something that can't be measured (RUNLOG B4B-D18). The gate (§9.8,
0 flash frames) is met as measured. K1 stays open until the owner can say where it shows (display
scale, a second monitor, the Windows build, or how the window is resized), and this tool can then
measure it there.
