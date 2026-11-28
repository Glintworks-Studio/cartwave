# Performance: this branch against main (the §2 rule)

Rounds paired by number, the order alternating. Primary: the per-thread cycle counters (UI kcycles per presented frame, audio kcycles per callback), the means over each scenario's window; process CPU % of a core beside them for context. Limits: +0.3 points of a core at 4.7 GHz: UI **+235 k/frame** (60 frames/s), audio **+141 k/callback** (100 callbacks/s). A scenario fails only if its difference is over the limit and more than 2x its noise floor (SE).

**PASS**; 0 runs left out; left out of single scenarios for want of proof: pb-5 (overview_drag, overview_zoom: no Overview hover in its log)

| scenario | n | UI k/frame main | this | Δ (SE) | Audio k/cb main | this | Δ (SE) | verdict | CPU % main | this | Δ (SE) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| idle | 10 | 21642.7 | 21582.8 | -59.9 (389.8) | 87.3 | 89.2 | +1.9 (1.0) | pass | 0.5 | 1.3 | +0.7 (0.2) |
| demo | 10 | 3409.4 | 3400.2 | -9.2 (18.3) | 112.3 | 113.8 | +1.5 (0.9) | pass | 7.5 | 7.5 | -0.1 (0.4) |
| 200 | 10 | 3688.2 | 3721.5 | +33.3 (22.3) | 160.9 | 163.8 | +2.9 (1.5) | pass | 7.8 | 7.9 | +0.1 (0.3) |
| 200drag | 10 | 5473.3 | 4899.4 | -573.9 (55.6) | 139.1 | 141.5 | +2.4 (1.9) | pass | 10.5 | 9.8 | -0.6 (0.2) |
| zoom_wheel | 10 | 8530.5 | 7617.3 | -913.2 (333.7) | 94.3 | 97.8 | +3.5 (1.3) | pass | 17.4 | 15.8 | -1.6 (1.0) |
| scrollbar_drag | 10 | 7151.0 | 6857.1 | -293.9 (108.2) | 171.1 | 172.5 | +1.4 (1.2) | pass | 14.3 | 14.9 | +0.6 (0.5) |
| window_resize | 10 | 18451.7 | 17288.6 | -1163.1 (459.3) | 168.3 | 169.2 | +0.9 (1.8) | pass | 27.0 | 26.1 | -0.9 (1.0) |
| row_resize | 10 | 6844.9 | 6647.0 | -198.0 (140.8) | 148.1 | 150.1 | +1.9 (1.1) | pass | 12.8 | 13.0 | +0.1 (0.3) |
| header_width | 10 | 12039.1 | 11637.1 | -402.0 (318.8) | 86.3 | 89.8 | +3.4 (0.9) | pass | 21.5 | 19.6 | -2.0 (0.4) |
| ruler_zoom | 10 | 6868.3 | 6554.2 | -314.1 (99.9) | 174.8 | 175.0 | +0.1 (2.0) | pass | 13.6 | 13.1 | -0.5 (0.4) |
| overview_drag | 9 | 4011.7 | 4032.2 | +20.5 (47.0) | 167.3 | 166.0 | -1.4 (1.3) | pass | 8.4 | 8.4 | +0.0 (0.2) |
| overview_zoom | 9 | 4244.6 | 4256.1 | +11.5 (36.3) | 155.1 | 156.0 | +0.9 (2.1) | pass | 9.0 | 9.0 | +0.1 (0.5) |
| s64_playing | 10 | 7358.7 | 7434.4 | +75.7 (40.3) | 2483.7 | 2499.1 | +15.5 (19.8) | pass | 18.0 | 18.5 | +0.5 (0.8) |
| s64_scroll | 10 | 21217.4 | 20605.5 | -611.9 (125.6) | 2279.6 | 2293.5 | +13.8 (22.0) | pass | 37.1 | 36.9 | -0.2 (0.9) |
| s64_insert_delete | 10 | 14851.3 | 14892.3 | +41.0 (65.7) | 2497.1 | 2511.9 | +14.8 (19.1) | pass | 28.1 | 27.5 | -0.7 (0.7) |
| s64_header_drag | 10 | 8880.3 | 8820.4 | -59.9 (47.5) | 2493.4 | 2509.6 | +16.2 (17.0) | pass | 19.9 | 19.9 | +0.0 (0.7) |
| pr_playing | 10 | 3049.3 | 3062.2 | +12.9 (32.6) | 127.3 | 128.8 | +1.4 (1.0) | pass | 6.9 | 7.4 | +0.5 (0.4) |
| pr_dragging | 10 | 3950.2 | 4012.2 | +62.0 (49.0) | 127.9 | 130.0 | +2.1 (1.2) | pass | 8.2 | 8.0 | -0.1 (0.3) |
| s64_arrangement | 10 | 7082.3 | 7166.6 | +84.2 (28.3) | 2506.5 | 2532.2 | +25.7 (26.7) | pass | 16.9 | 17.2 | +0.3 (0.6) |
| s64_mixer | 10 | 2541.1 | 2556.1 | +15.1 (37.8) | 2510.0 | 2537.7 | +27.6 (22.2) | pass | 11.1 | 11.0 | -0.2 (0.7) |
| s64_mixer_scroll | 10 | 7767.0 | 7284.8 | -482.3 (32.5) | 2476.9 | 2515.1 | +38.2 (21.0) | pass | 18.9 | 18.2 | -0.6 (0.9) |
| device_playing | 10 | 4826.1 | 4919.3 | +93.3 (68.6) | 115.3 | 116.4 | +1.2 (1.4) | pass | 8.5 | 9.3 | +0.8 (0.3) |
| browser_search | 10 | 3653.7 | 3585.1 | -68.6 (37.5) | 129.4 | 131.4 | +2.0 (1.3) | pass | 8.4 | 7.6 | -0.8 (0.4) |
| browser_scroll | 10 | 5057.3 | 4573.2 | -484.1 (28.1) | 114.1 | 117.6 | +3.5 (1.2) | pass | 9.5 | 8.6 | -0.9 (0.3) |
| step | 0 | – | – | – | – | – | – | this branch only (this branch only: UI 4382 k/frame, audio 103 k/cb) | – | – | – |
