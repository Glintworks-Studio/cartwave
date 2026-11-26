# Mockup match on macOS (POLISH2.md §4.2 method, `mockup_match_mac.py`)

| Board | 1× (backing 1.0, main display) | 2× (backing 2.0, HiDPI mode on the second display) |
|---|---|---|
| E1-header (4 rows) | PASS, 48/48 elements | **PASS**, every element within 2 device px |
| E1-row-heights (72 / 144 / 240) | PASS (same offsets as Windows, element for element) | 8 elements over: the dB scale labels 0 / 6 / 12 / 24 of the 144 and 240 px rows, (0, 3) device px = 1.5 logical px low; everything else within 1 px |
| E1-overview (Overview on) | — | **PASS** |

**The scale labels at 2×.** Measured ink tops (logical px, relative to the board's HTML box, the
spec under P2-D7):

| Label | Board @1x | Board @2x ÷ 2 | App 1× | App 2× ÷ 2 |
|---|---|---|---|---|
| 0 (144 px row) | +1.0 | 0.0 | +1.0 | +1.5 |
| 6 | +1.0 | 0.0 | +1.0 | +1.5 |
| 12 | +1.0 | +0.5 | +1.0 | +1.5 |
| 24 | +1.0 | +0.5 | +1.0 | +1.5 |

The app's labels move by 0.5 logical px between 1× and 2× (one device px of baseline rounding);
the reference's own @2x render moves them up by 0.5–1 px against its @1x render (Chrome's text
layout at DPR 2). The tool compares app ink with the @2x PNG's ink, so the two shifts add up to
1.5 px. Against the HTML box, the spec, the app is within 0.5 px at both scales, and at 1× it
matches Windows and the @1x board exactly. Not a macOS defect, and no fix: moving the labels
would put the 1× rendering 1 px off. Reports: `docs/screens/mac/mockup-match/`; picture:
`docs/screens/mac/mac-mockup-match.png`.

The 2× session switched the second display (XB271HU) to its HiDPI mode (1280 × 720 pt at
2560 × 1440 px, ioModeID 7, for this session only) and restored ioModeID 18 at the end
(`dispmode current 1` read back 18).
