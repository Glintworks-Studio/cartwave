# Visual regression at 150 % (Presets + Browser batch)

Each state: the pixels that changed in place, then those that match the baseline neither in place nor 180 px (the browser's growth) to the left, sorted into the allowed regions (the browser, the bottom pane) and OTHER, every OTHER region listed. Written by `docs/presets/tools/diff_pb.py`.

| State | Changed in place (px) | Changed after the shift (px) | Browser | Bottom pane | OTHER regions |
|---|---|---|---|---|---|
| mixer | 910266 | 251419 | 251419 | 0 | none |

**PASS**: every change is in the browser, the bottom pane or a named region
