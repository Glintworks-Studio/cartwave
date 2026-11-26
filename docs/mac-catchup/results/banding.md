# Banding on macOS (DESIGNPASS.md §3 measure), 320 Hz display, backing 1.0

Run lengths of identical pixels down 5 neighbouring columns (`img runs`); the captured display
values ("raw") and the same converted to sRGB agree on every row. "Off" = `CHIPDAW_NO_DITHER=1`
(drawn gradients undithered; stylesheet gradients are dithered by the vizia fork either way).

| Gradient (content x, y range) | Dither | Longest run | Mean run | Banded rows (runs ≥ 4 px) | Windows (DESIGNPASS.md) |
|---|---|---|---|---|---|
| Canvas under the lanes `#191919 → #0A0A0A`, drawn (x 1300, y 720–895) | off | 51 | 29.2 | 98% | 55 / 32.3 / 99% |
| | **on** | **24** | **1.9** | **43%** | 24 / 2.0 / 42% |
| Top bar `#191919 → #0A0A0A`, stylesheet (x 360, y 2–74) | on (fork) | 4 | 2.1 | 63% | 4 / 2.0 / 61% |
| Bottom panel `#121212 → #0A0A0A`, stylesheet (x 500, inspector open) | on (fork) | 16 | 2.0 | 47% | 16 / 2.0 / 48% |

Same result as Windows: dithering works on the Mac's GL path, and ColorSync's conversion to the
wide-gamut panel adds no banding of its own (raw = sRGB runs).
