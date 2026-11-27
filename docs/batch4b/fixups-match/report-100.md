# The fixups vs the `fixups` board (Batch 4b §9.3)

Tolerance: 1 px for positions; for the overhang, the largest luminance difference on any row (of 255: the gradient is the same if it stays within a couple of levels); for the glow, the lift in levels.

| part | board | app | Δ |
|---|---|---|---|
| number: centre x, top in its row | (50.5, 21) | (50.5, 22) | 1 |
| name: left edge | 89 | 89 | 0 |
| overhang: lum per row (0-51 px under the display's top) | [24, 6, 9, 11, 13, 16, 17, 18, 19, 20, 21, 21, 21] | [23, 7, 9, 12, 14, 16, 17, 19, 20, 20, 21, 21, 21] | 1 |
| outline: its row under the row's top | 1 | 1 | 0 |
| outline: the clip border 1 px under it (its colour) | (121, 126, 51) | (142, 147, 64) | None |
| glow: lift 4 px above the outline (levels) | 5 | 8 | 3 |

Notes:
- The outline sits 1 px outside the clip's border in both (row 1 of the row: the clip starts at 2).
  The border colour under it differs by the clips' own fill (not a fixup: the clip recipe is
  unchanged).
- The glow's lift can't be compared like for like: on the board it falls on the ruler above the
  first row, in the app on the lane of the row above. Its parameters are the board's (white 22 %,
  a 10 px CSS blur = sigma 5, the same conversion every app shadow uses).
- The overhang matches the board within 1 level (of 255) on every row from the display's top to
  48 px down.
- The header meters (the pan grey) and the clip marks are checked by `fader` unit tests and B4B-C1:
  the board's rows show levels and no marks (VISUAL-SPEC §13 wins for those, DOC-D5).
