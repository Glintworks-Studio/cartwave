# Contrast of the new text (gate 7)

`docs/presets/tools/contrast_pb.py` on the lime scenes: per region, the ink against the region's most common colour (WCAG).

**Lowest: 4.61 : 1** (PASS: every region >= 4.5 : 1)

| Scene | Text | Background | Ink | Ratio | >= 4.5 |
|---|---|---|---|---|---|
| device_studio | device name, badge STUDIO | #2B2B2B | #E6E6E6 | 11.34 | yes |
| device_studio | section title OSCILLATOR | #1F1F1F | #8A8A8A | 4.77 | yes |
| device_studio | section title ENVELOPES | #222222 | #8A8A8A | 4.61 | yes |
| device_studio | envelope tabs, unselected | #1D1D1D | #8A8A8A | 4.88 | yes |
| device_studio | knob labels (first row) | #1B1B1B | #8A8A8A | 4.99 | yes |
| device_studio | knob labels (second row) | #181818 | #8A8A8A | 5.14 | yes |
| device_studio | ENVELOPES knob labels | #1B1B1B | #8A8A8A | 4.99 | yes |
| device_studio | OUTPUT label VOLUME | #161616 | #8A8A8A | 5.24 | yes |
| device_studio | preset field text | #0E0E0E | #E6E6E6 | 15.47 | yes |
| device_locked | badge LOCKED (header, full brightness) | #2A2A2A | #E6E6E6 | 11.50 | yes |
| device_locked | section title OSCILLATOR (locked) | #1F1F1F | #8A8A8A | 4.77 | yes |
| device_locked | knob label DUTY (locked, dimmed device) | #1B1B1B | #8A8A8A | 4.99 | yes |
| device_locked | second-row labels (locked) | #181818 | #8A8A8A | 5.14 | yes |
| device_locked | ENVELOPES knob labels (locked) | #1B1B1B | #8A8A8A | 4.99 | yes |
| device_locked | 4-BIT · PER FRAME (locked) | #141414 | #8A8A8A | 5.34 | yes |
| device_locked | OUTPUT label VOLUME (locked) | #161616 | #8A8A8A | 5.24 | yes |
| device_locked | Voice selector (lock + channel) | #0E0E0E | #E6E6E6 | 15.47 | yes |
| browser_sounds | BROWSER title | #0E0E0E | #8A8A8A | 5.59 | yes |
| browser_sounds | sidebar title COLLECTIONS | #0E0E0E | #8A8A8A | 5.59 | yes |
| browser_sounds | sidebar entries | #0E0E0E | #B0B0B0 | 8.90 | yes |
| browser_sounds | content title SOUNDS | #0E0E0E | #8A8A8A | 5.59 | yes |
| browser_sounds | folder rows | #0E0E0E | #D0D0D0 | 12.52 | yes |
| browser_sounds | item rows (unselected) | #0E0E0E | #D0D0D0 | 12.52 | yes |
| browser_sounds | item row (selected: dark on light) | #BDBDBD | #0E0E0E | 10.28 | yes |
| browser_sounds | search placeholder | #0A0A0A | #808080 | 5.01 | yes |
| picker | picker folders | #0E0E0E | #E6E6E6 | 15.47 | yes |
| picker | picker commands | #0E0E0E | #E6E6E6 | 15.47 | yes |
| dpcm_loop_on | DPCM sample rows | #1B1B1B | #E6E6E6 | 13.80 | yes |
