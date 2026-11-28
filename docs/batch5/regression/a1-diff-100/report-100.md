# Visual regression at 100 % (Presets + Browser batch)

Each state: the pixels that changed in place, then those that match the baseline neither in place nor 180 px (the browser's growth) to the left, sorted into the allowed regions (the browser, the bottom pane) and OTHER, every OTHER region listed. Written by `docs/presets/tools/diff_pb.py`.

| State | Changed in place (px) | Changed after the shift (px) | Browser | Bottom pane | OTHER regions |
|---|---|---|---|---|---|
| mixer | 413703 | 125950 | 125692 | 0 | (912, 960, 984, 968) 258 px (the strips' scrollbar thumb: shorter, the viewport is 180 px narrower (the 420 px browser, PRESETS.md)) |

**PASS**: every change is in the browser, the bottom pane or a named region
