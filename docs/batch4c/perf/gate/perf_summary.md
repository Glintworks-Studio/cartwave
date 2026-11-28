# Batch 4c perf: three builds interleaved, judged on the means

Rounds: 10; runs left out (focus lost / scenario not reached): 0.
Δ = this branch's mean − the other build's (CPU % of one core); SE = the noise floor (standard error of the paired differences). Limits (§6.5): vs main +0.3 everywhere; vs pre-batch4b the 64-track rows +0.4, idle +0.1, row resize +0.3.

| scenario | n | pre-4b | main | 4c | Δ vs main (SE) | ok | Δ vs pre-4b (SE) | limit | ok |
|---|---|---|---|---|---|---|---|---|---|
| idle | 10 | 0.5 | 0.5 | 0.4 | -0.08 (0.28) | yes | -0.11 (0.18) | 0.1 | yes |
| demo | 10 | 8.2 | 7.3 | 7.6 | +0.28 (0.41) | yes | -0.67 (0.54) | – | – |
| 200 | 10 | 7.9 | 7.9 | 7.9 | +0.03 (0.24) | yes | +0.00 (0.17) | – | – |
| 200drag | 10 | 10.6 | 10.7 | 11.2 | +0.53 (0.46) | **no** | +0.67 (0.43) | – | – |
| zoom_wheel | 10 | 17.4 | 16.8 | 17.7 | +0.97 (0.53) | **no** | +0.39 (0.74) | – | – |
| scrollbar_drag | 10 | 15.0 | 14.7 | 14.8 | +0.12 (0.64) | yes | -0.16 (0.79) | – | – |
| window_resize | 10 | 37.0 | 37.0 | 36.6 | -0.39 (1.17) | yes | -0.40 (0.83) | – | – |
| row_resize | 10 | 13.9 | 13.5 | 13.2 | -0.29 (0.45) | yes | -0.66 (0.44) | 0.3 | yes |
| header_width | 10 | 24.9 | 24.4 | 25.2 | +0.81 (0.57) | **no** | +0.25 (0.42) | – | – |
| ruler_zoom | 10 | 16.6 | 14.0 | 17.1 | +3.15 (1.71) | **no** | +0.51 (2.32) | – | – |
| overview_drag | 10 | 8.5 | 8.2 | 11.1 | +2.91 (2.58) | **no** | +2.54 (2.56) | – | – |
| overview_zoom | 10 | 9.0 | 8.5 | 8.6 | +0.09 (0.44) | yes | -0.41 (0.56) | – | – |
| s64_playing | 10 | 17.3 | 17.9 | 18.0 | +0.08 (0.46) | yes | +0.65 (0.61) | 0.4 | **no** |
| s64_scroll | 10 | 36.2 | 38.1 | 38.3 | +0.21 (0.83) | yes | +2.11 (1.32) | 0.4 | **no** |
| s64_insert_delete | 10 | 26.4 | 27.2 | 28.0 | +0.83 (0.63) | **no** | +1.61 (0.97) | 0.4 | **no** |
| s64_header_drag | 10 | 18.8 | 19.6 | 19.3 | -0.34 (0.70) | yes | +0.49 (0.86) | 0.4 | **no** |
| pr_playing | 10 | 6.7 | 7.6 | 6.9 | -0.77 (0.47) | yes | +0.21 (0.37) | – | – |
| pr_dragging | 10 | 8.2 | 8.2 | 8.0 | -0.11 (0.23) | yes | -0.15 (0.39) | – | – |
| s64_arrangement | 10 | – | 17.1 | 16.7 | -0.34 (0.64) | yes | – | 0.4 | – |
| s64_mixer | 10 | – | 11.6 | 10.8 | -0.75 (0.61) | yes | – | 0.4 | – |
| s64_mixer_scroll | 10 | – | 18.9 | 18.4 | -0.53 (0.36) | yes | – | 0.4 | – |

**vs main: NOT MET; vs pre-batch4b: NOT MET**
