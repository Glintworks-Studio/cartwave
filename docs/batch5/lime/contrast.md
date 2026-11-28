# Contrast of the new text (gate 7)

`docs/presets/tools/contrast_pb.py` on the lime scenes: per region, the ink against the region's most common colour (WCAG).

**Lowest: 4.98 : 1** (PASS: every region >= 4.5 : 1)

| Scene | Text | Background | Ink | Ratio | >= 4.5 |
|---|---|---|---|---|---|
| auto_mode | lane header label (Mixer > Volume) | #111111 | #A8A8A8 | 7.94 | yes |
| auto_mode | lane header value | #111111 | #A8A8A8 | 7.94 | yes |
| auto_locked | locked lane header label (Chip Synth > Duty) | #111111 | #A8A8A8 | 7.94 | yes |
| auto_locked | locked lane header value | #111111 | #A8A8A8 | 7.94 | yes |
| auto_overridden | overridden lane header value | #111111 | #838383 | 4.98 | yes |
