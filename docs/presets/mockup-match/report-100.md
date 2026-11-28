# The device panel against the board (100 %)

`docs/presets/tools/mockup_match_pb.py`: each board part (a patch of the board's render) found in our screenshot by normalised cross-correlation within ±8 px of the same device-local place; offset = ours − board's (x, y). Pass: within 2 px and correlation ≥ 0.5.

**47 of 47 within 2 px** (largest offset 2 px). Display colour #E8DCC0 exact in our studio displays: yes (841 exact pixels; the locked ones are at 40 %: 0).

| Device | Part | Board patch at | Offset (px) | Correlation | Within 2 px |
|---|---|---|---|---|---|
| studio | device box, top-left corner | (0, 0) | anchor: window (224, 602) | 0.96 | – |
| studio | header: colour tab and name | (8, 6) | +0, -1 | 0.89 | yes |
| studio | header: line under it (y only; with the sections' top edge) | (150, 34) | +0, +0 | 0.74 | yes |
| studio | ◀ key | (274, 3) | +1, +0 | 0.99 | yes |
| studio | preset field, left end | (306, 3) | +1, -1 | 0.94 | yes |
| studio | preset field, right end | (540, 3) | +1, +0 | 0.86 | yes |
| studio | ▶ key | (572, 3) | +1, +0 | 0.97 | yes |
| studio | OSCILLATOR corner | (6, 46) | +1, +1 | 0.86 | yes |
| studio | OSCILLATOR title | (20, 60) | +0, +0 | 0.70 | yes |
| studio | ENVELOPES corner | (274, 46) | +1, -1 | 0.87 | yes |
| studio | ENVELOPES title | (288, 60) | +0, +0 | 0.83 | yes |
| studio | OUTPUT corner | (686, 46) | +1, +0 | 0.83 | yes |
| studio | OUTPUT title | (700, 60) | +0, +0 | 0.86 | yes |
| studio | OSCILLATOR display, top-left | (18, 84) | +0, +0 | 1.00 | yes |
| studio | OSCILLATOR display, bottom-right | (216, 168) | +0, +0 | 0.95 | yes |
| studio | ENVELOPES display, top-left | (286, 84) | +0, +0 | 0.86 | yes |
| studio | ENVELOPES display, bottom-right corner | (652, 180) | +0, +0 | 0.97 | yes |
| studio | knob VIB DEPTH | (331, 194) | +0, +0 | 0.85 | yes |
| studio | knob VIB RATE | (456, 194) | +0, +0 | 0.87 | yes |
| studio | knob ARP SPEED | (581, 194) | +0, +0 | 0.87 | yes |
| studio | knob VOLUME | (713, 113) | +0, +0 | 0.84 | yes |
| studio | meter, top (unlit) | (808, 100) | +0, +0 | 0.98 | yes |
| studio | knob DUTY | (32, 194) | +0, +0 | 0.87 | yes |
| studio | knob UNISON | (116, 194) | +0, +0 | 0.87 | yes |
| studio | knob DETUNE | (200, 194) | +0, +0 | 0.89 | yes |
| studio | voice selector | (190, 58) | +0, +0 | 0.93 | yes |
| locked | device box, top-left corner | (0, 0) | anchor: window (224, 602) | 0.95 | – |
| locked | header: colour tab and name | (8, 6) | +0, -1 | 0.89 | yes |
| locked | header: line under it (y only; with the sections' top edge) | (150, 34) | +0, +2 | 0.64 | yes |
| locked | ◀ key | (274, 3) | +1, +0 | 0.99 | yes |
| locked | preset field, left end | (306, 3) | +0, -1 | 0.54 | yes |
| locked | preset field, right end | (540, 3) | +1, +0 | 0.86 | yes |
| locked | ▶ key | (572, 3) | +1, +0 | 0.97 | yes |
| locked | OSCILLATOR corner | (6, 46) | +2, +2 | 0.73 | yes |
| locked | OSCILLATOR title | (20, 60) | +0, +0 | 0.65 | yes |
| locked | ENVELOPES corner | (274, 46) | +1, +1 | 0.73 | yes |
| locked | ENVELOPES title | (288, 60) | +0, +0 | 0.77 | yes |
| locked | OUTPUT corner | (686, 46) | +2, +2 | 0.73 | yes |
| locked | OUTPUT title | (700, 60) | +0, +0 | 0.81 | yes |
| locked | OSCILLATOR display, top-left | (18, 84) | +0, -1 | 0.78 | yes |
| locked | OSCILLATOR display, bottom-right | (216, 168) | +0, +0 | 0.94 | yes |
| locked | ENVELOPES display, top-left | (286, 84) | +0, +0 | 0.97 | yes |
| locked | ENVELOPES display, bottom-right corner | (652, 180) | +0, +0 | 0.94 | yes |
| locked | knob VIB DEPTH | (331, 194) | +0, +0 | 0.78 | yes |
| locked | knob VIB RATE | (456, 194) | +1, +0 | 0.81 | yes |
| locked | knob ARP SPEED | (581, 194) | +0, +0 | 0.80 | yes |
| locked | knob VOLUME | (713, 113) | +0, +0 | 0.69 | yes |
| locked | meter, top (unlit) | (808, 100) | +0, +0 | 0.88 | yes |
| locked | knob DUTY (alone) | (116, 194) | +0, +0 | 0.80 | yes |
