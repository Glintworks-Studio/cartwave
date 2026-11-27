# Batch 4b perf: interleaved A/B, judged on the means

Rounds paired: 10; runs left out (focus lost / scenario not reached): 0.
Pass: Δ ≤ +0.3 or Δ within 2 × the noise floor (the standard error of the paired differences).

| scenario | n | baseline mean (sd) | 4b mean (sd) | Δ | noise floor (SE) | pass |
|---|---|---|---|---|---|---|
| idle | 10 | 0.1 (0.1) | 0.5 (0.6) | +0.37 | 0.19 | yes |
| demo | 10 | 8.0 (1.1) | 7.9 (0.6) | -0.11 | 0.35 | yes |
| 200 | 10 | 8.6 (1.0) | 8.4 (0.7) | -0.13 | 0.26 | yes |
| 200drag | 10 | 11.2 (1.2) | 11.0 (0.9) | -0.26 | 0.48 | yes |
| zoom_wheel | 10 | 16.9 (0.9) | 16.7 (0.9) | -0.18 | 0.38 | yes |
| scrollbar_drag | 10 | 15.0 (1.5) | 15.1 (1.6) | +0.12 | 0.70 | yes |
| window_resize | 10 | 36.2 (2.5) | 35.2 (1.8) | -0.96 | 0.74 | yes |
| row_resize | 10 | 12.9 (0.8) | 13.4 (0.9) | +0.46 | 0.38 | yes |
| header_width | 10 | 25.3 (1.0) | 24.6 (1.5) | -0.76 | 0.53 | yes |
| ruler_zoom | 10 | 17.8 (5.4) | 17.8 (6.2) | +0.08 | 3.12 | yes |
| overview_drag | 10 | 9.1 (0.9) | 8.2 (0.9) | -0.91 | 0.37 | yes |
| overview_zoom | 10 | 8.8 (1.0) | 8.9 (0.7) | +0.14 | 0.42 | yes |
| s64_playing | 10 | 17.2 (1.7) | 18.4 (1.4) | +1.14 | 0.63 | yes |
| s64_scroll | 10 | 37.1 (2.3) | 38.0 (2.3) | +0.92 | 1.08 | yes |
| s64_insert_delete | 10 | 26.6 (1.9) | 28.4 (1.6) | +1.84 | 0.93 | yes |
| s64_header_drag | 10 | 19.2 (1.6) | 20.0 (1.0) | +0.74 | 0.68 | yes |
| pr_playing | 10 | 6.9 (0.8) | 7.1 (1.0) | +0.25 | 0.47 | yes |
| pr_dragging | 10 | 8.2 (0.9) | 8.3 (0.7) | +0.12 | 0.36 | yes |

## The mixer (this branch only; 64 studio tracks playing)

| scenario | n | mean (sd) |
|---|---|---|
| s64_arrangement | 10 | 16.3 (2.0) |
| s64_mixer | 10 | 11.2 (1.4) |
| s64_mixer_scroll | 10 | 18.5 (2.4) |

**Gate 6: pass**
